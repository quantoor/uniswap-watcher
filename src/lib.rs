pub mod binance_client;
pub mod db;
pub mod util;

use crate::binance_client::BinanceClient;
use crate::db::{get_tx_fee_from_db, DatabaseSettings, TxFee};
use crate::util::{compute_gas_fee_eth, try_get_tx_receipt, tx_hash_to_price};
use actix_web::dev::Server;
use actix_web::http::header::ContentType;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use config;
use config::{Config, File, FileFormat};
use ethers::addressbook::Address;
use ethers::contract::{abigen, Contract, LogMeta};
use ethers::middleware::Middleware;
use ethers::prelude::{BlockId, Http, Provider, TxHash, ValueOrArray, Ws, H256};
use ethers::utils::hex::ToHexExt;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use sqlx;
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tracing::{error, info};

abigen!(
    AggregatorInterface,
    r#"[
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    ]"#,
);

abigen!(
    IERC20,
    r#"[
        function decimals() external view returnns (uint8)
        function symbol() external view returns (string)
    ]"#,
);

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AppConfig {
    pub application_port: u16,
    pub rpc_url_http: String,
    pub rpc_url_ws: String,
    pub pool_address: String,
    pub swap_topic: String,
    pub binance_host: String,
    pub database: DatabaseSettings,
}

impl AppConfig {
    pub fn new() -> Result<AppConfig, config::ConfigError> {
        let config = Config::builder()
            .add_source(File::new("configuration", FileFormat::Yaml))
            .build()?;
        config.try_deserialize::<AppConfig>()
    }
}

#[derive(Clone)]
pub struct Application {
    pub config: AppConfig,
    pub eth_client: Provider<Http>,
    pub binance_client: BinanceClient,
    pub sender: Sender<TxFee>,
    pub db_connection: PgPool,
}

impl Application {
    pub fn new(
        config: AppConfig,
        sender: Sender<TxFee>,
        db_connection: PgPool,
    ) -> Result<Application> {
        Ok(Self {
            config: config.clone(),
            eth_client: Provider::<Http>::try_from(config.rpc_url_http.as_str()).unwrap(),
            binance_client: BinanceClient::new(config.binance_host.as_str()),
            sender,
            db_connection,
        })
    }

    /// Given a tx hash, tries to get the tx fee from db.
    /// If the tx hash is not found, computes the tx fee on-chain, and then stores the result in db.
    pub async fn try_get_or_insert(&self, tx_hash: &TxHash) -> Result<TxFee> {
        // Try get fee from db
        info!(
            "Try get from db tx_hash={}",
            tx_hash.encode_hex_with_prefix()
        );
        let res = get_tx_fee_from_db(tx_hash, &self.db_connection).await;
        if res.is_ok() {
            return Ok(res.unwrap());
        }

        // If fee not found, get it from blockchain
        let res = match self.get_tx_fee(tx_hash).await {
            Ok(res) => res,
            Err(err) => {
                error!(
                    "Could not get fee for tx_hash={}: {}",
                    tx_hash.encode_hex_with_prefix(),
                    err
                );
                return Err(err);
            }
        };

        // Send result to queue to be inserted in db
        if self.sender.send(res.clone()).is_err() {
            error!("Could not send to queue tx fee {:?}", res.clone());
        }

        // Return result
        Ok(res)
    }

    /// Given a tx hash,
    pub async fn get_tx_fee(&self, tx_hash: &TxHash) -> Result<TxFee> {
        info!(
            "Getting fee for tx_hash={}",
            tx_hash.encode_hex_with_prefix()
        );

        // Get transaction receipt for given transaction hash
        let tx_receipt = try_get_tx_receipt(*tx_hash, &self.eth_client).await?;

        // Use transaction receipt to compute the gas fee: gas_fee = gas_used * gas_price
        let fee_eth = compute_gas_fee_eth(&tx_receipt).await?;

        // Get from Binance the price of ETH/USDT at the time of the transaction (with 1 min precision)
        let block = self
            .eth_client
            .get_block(BlockId::Hash(tx_receipt.block_hash.unwrap()))
            .await?
            .unwrap();
        let timestamp_ms = block.timestamp.as_u64() * 1000;
        let ticker = self
            .binance_client
            .get_kline("ETHUSDT", timestamp_ms)
            .await?;
        let eth_usdt_price = ticker[0].clone().open_price;
        let eth_usdt_price: f64 = eth_usdt_price.parse().unwrap();

        // Compute gas fee in USDT
        let fee_usdt = fee_eth * eth_usdt_price;

        Ok(TxFee {
            tx_hash: tx_receipt.transaction_hash.encode_hex_with_prefix(),
            fee_eth,
            fee_usdt,
        })
    }

    /// Given an array of tx hashes, returns a map of tx hashes to their corresponding tx fees in USDT.
    pub async fn get_tx_fee_batch(&self, tx_hashes: Vec<String>) -> Result<HashMap<TxHash, f64>> {
        let mut res: HashMap<TxHash, f64> = HashMap::new();
        for tx_hash_str in tx_hashes {
            let tx_hash = TxHash::from_str(tx_hash_str.as_str());
            if tx_hash.is_err() {
                error!("Invalid tx hash {}", tx_hash_str);
                continue;
            }
            let tx_hash = tx_hash.unwrap();
            let tx_fee_res = self.try_get_or_insert(&tx_hash).await;
            if tx_fee_res.is_ok() {
                res.insert(tx_hash, tx_fee_res.unwrap().fee_usdt);
            }
        }
        Ok(res)
    }
}

/// Listen to event logs and store in db the tx fees
#[allow(unreachable_code)]
pub async fn subscribe_logs(config: AppConfig, sender: Sender<TxFee>) -> Result<()> {
    let binance_client = BinanceClient::new(config.binance_host.as_str());
    let eth_client = Provider::<Http>::try_from(config.rpc_url_http.as_str()).unwrap();
    let ws_client = Provider::<Ws>::connect(config.rpc_url_ws.as_str()).await?;
    let ws_client = Arc::new(ws_client);

    let event = Contract::event_of_type::<SwapFilter>(ws_client)
        .address(ValueOrArray::Array(vec![config.pool_address.parse()?]));

    let mut stream = event.subscribe_with_meta().await?;
    loop {
        info!("Waiting for swap event...");
        while let Some(Ok((_log, meta))) = stream.next().await {
            let meta: LogMeta = meta;
            let tx_receipt = try_get_tx_receipt(meta.transaction_hash, &eth_client).await?;
            let fee_eth = compute_gas_fee_eth(&tx_receipt).await?;

            info!("Getting ticker for ETHUSDT");
            let ticker = binance_client.get_ticker("ETHUSDT").await?;
            let eth_price = ticker.price.parse::<f64>()?;
            let fee_usdt = fee_eth * eth_price;

            let data = TxFee {
                tx_hash: tx_receipt.transaction_hash.encode_hex_with_prefix(),
                fee_eth,
                fee_usdt,
            };
            info!("Sending new data to queue: {:?}", data.clone());
            if sender.send(data.clone()).is_err() {
                error!("Could not send to queue tx fee {:?}", data.clone());
            }
        }
    }
    Ok(())
}

pub fn run_server(
    app_config: AppConfig,
    address: String,
    sender: Sender<TxFee>,
    db_connection: PgPool,
) -> Result<Server, std::io::Error> {
    let app = Application::new(app_config, sender, db_connection.clone()).unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app.clone()))
            .service(
                web::scope("")
                    .service(home)
                    .service(tx_fee)
                    .service(swap_price),
            )
            .app_data(db_connection.clone())
    })
    .bind(address)?
    .run();
    Ok(server)
}

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Uniswap Watcher")
}

#[get("/tx_fee")]
async fn tx_fee(controller: web::Data<Application>, body: web::Bytes) -> impl Responder {
    if body.is_empty() {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("Missing body");
    };
    let hashes: Vec<String>;
    match serde_json::from_slice::<Vec<String>>(body.as_ref()) {
        Ok(deserialized) => {
            hashes = deserialized;
        }
        Err(err) => {
            return HttpResponse::BadRequest()
                .content_type(ContentType::plaintext())
                .body(format!("Body deserialization failed: {}", err));
        }
    };
    match controller.get_tx_fee_batch(hashes).await {
        Ok(fee) => {
            let res = json!(fee);
            HttpResponse::Ok().json(res)
        }
        Err(err) => HttpResponse::InternalServerError()
            .content_type(ContentType::plaintext())
            .body(format!("Something went wrong: {}", err)),
    }
}

#[derive(Deserialize)]
struct SwapPriceArg {
    tx_hash: String,
}

#[get("/swap_price")]
async fn swap_price(
    controller: web::Data<Application>,
    arg: web::Query<SwapPriceArg>,
) -> impl Responder {
    let swap_topic = H256::from_str(controller.config.swap_topic.as_str()).unwrap();
    let pool_address = controller.config.pool_address.parse::<Address>().unwrap();
    let tx_hash = TxHash::from_str(arg.tx_hash.as_str());
    if tx_hash.is_err() {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(format!("Invalid tx hash {}", arg.tx_hash));
    }
    match tx_hash_to_price(
        swap_topic,
        pool_address,
        tx_hash.unwrap(),
        &controller.eth_client,
    )
    .await
    {
        Ok(fee) => HttpResponse::Ok().body(format!("{}", fee)),
        Err(err) => HttpResponse::InternalServerError()
            .content_type(ContentType::plaintext())
            .body(format!("Something went wrong: {}", err)),
    }
}
