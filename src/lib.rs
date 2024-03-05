pub mod binance_client;
pub mod db;

use crate::binance_client::BinanceClient;
use crate::db::{get_tx_fee_from_db, TxFee};
use actix_web::dev::Server;
use actix_web::http::header::ContentType;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Error, Result};
use ethers::contract::{abigen, Contract, LogMeta};
use ethers::middleware::Middleware;
use ethers::prelude::{BlockId, Http, Provider, TransactionReceipt, TxHash, ValueOrArray, Ws};
use ethers::types::I256;
use ethers::utils::format_units;
use ethers::utils::hex::ToHexExt;
use futures_util::StreamExt;
use serde_json::json;
use sqlx;
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time;
use tracing::{error, info};

pub const VERSION: &str = "0.0.1";
pub const RPC_URL_HTTP: &str = "https://eth.drpc.org";
pub const RPC_URL_WS: &str = "wss://ethereum-rpc.publicnode.com";
pub const POOL_ADDRESS: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
pub const BINANCE_HOST: &str = "https://api.binance.com";

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

#[derive(Clone)]
pub struct Application {
    pub version: String,
    pub eth_client: Provider<Http>,
    pub binance_client: BinanceClient,
    pub sender: Sender<TxFee>,
    pub db_connection: PgPool,
}

impl Application {
    pub fn new(sender: Sender<TxFee>, db_connection: PgPool) -> Result<Application> {
        Ok(Self {
            version: VERSION.into(),
            eth_client: Provider::<Http>::try_from(RPC_URL_HTTP).unwrap(),
            binance_client: BinanceClient::new(BINANCE_HOST),
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

/// Given a tx receipt, computes the gas fee in ETH
pub async fn compute_gas_fee_eth(tx: &TransactionReceipt) -> Result<f64, Error> {
    let gas_price = tx
        .effective_gas_price
        .ok_or(anyhow!("effective gas price not found in tx receipt"))?;
    let gas = tx
        .gas_used
        .ok_or(anyhow!("gas used not found in tx receipt"))?;
    let gas_eth = gas_price * gas;
    let gas_eth_str = format_units(gas_eth, "ether")?;
    Ok(gas_eth_str.parse()?)
}

/// Take in input the blockchain amounts of USDC and WETH, and return the price WETH/USDC
pub fn get_price(amount_usdc: I256, amount_weth: I256) -> Result<f64> {
    let amount_usdc = format_units(amount_usdc, 6)?.parse::<f64>()?;
    let amount_weth = format_units(amount_weth, 18)?.parse::<f64>()?;
    Ok((amount_usdc / amount_weth).abs())
}

/// Listen to event logs and store in db the tx fees
#[allow(unreachable_code)]
pub async fn subscribe_logs(sender: Sender<TxFee>) -> Result<()> {
    let binance_client = BinanceClient::new(BINANCE_HOST.into());
    let eth_client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let ws_client = Provider::<Ws>::connect(RPC_URL_WS).await.unwrap();
    let ws_client = Arc::new(ws_client);

    let event = Contract::event_of_type::<SwapFilter>(ws_client)
        .address(ValueOrArray::Array(vec![POOL_ADDRESS.parse()?]));

    let mut stream = event.subscribe_with_meta().await?;
    loop {
        info!("Waiting for swap event...");
        while let Some(Ok((log, meta))) = stream.next().await {
            let log: SwapFilter = log;
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

            // Compute the swap price
            match get_price(log.amount_0, log.amount_1) {
                Ok(price) => {
                    info!("Swap occurred at price {} ETH/USDC", price);
                }
                Err(err) => {
                    error!(
                        "Could not compute the price for tx hash {}: {}",
                        tx_receipt.transaction_hash.encode_hex_with_prefix(),
                        err
                    );
                }
            }
        }
    }
    Ok(())
}

/// Get transaction receipt for given transaction hash
pub async fn try_get_tx_receipt(
    tx_hash: TxHash,
    eth_client: &Provider<Http>,
) -> Result<TransactionReceipt> {
    // The tx receipt may not be found immediately after receiving the event log,
    // so a retry logic is used to try fetch the receipt every second for 5 seconds
    let mut count = 0;
    loop {
        match eth_client.get_transaction_receipt(tx_hash).await? {
            Some(receipt) => {
                return Ok(receipt);
            }
            None => {
                count += 1;
                if count >= 5 {
                    // Reached max retries, and receipt not found
                    return Err(anyhow!(
                        "Tx receipt for tx hash {} not found after 5 retries",
                        tx_hash.encode_hex_with_prefix()
                    ));
                }
                info!(
                    "Tx receipt for tx hash {} not found, wait 1 sec and retry",
                    tx_hash.encode_hex_with_prefix()
                );
                tokio::time::sleep(time::Duration::from_millis(1000)).await;
            }
        }
    }
}

pub fn run_server(
    address: String,
    sender: Sender<TxFee>,
    db_connection: PgPool,
) -> Result<Server, std::io::Error> {
    let app = Application::new(sender, db_connection.clone()).unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app.clone()))
            .service(web::scope("").service(home).service(tx_fee))
            .app_data(db_connection.clone())
    })
    .bind(address)?
    .run();
    Ok(server)
}

#[get("/")]
async fn home(controller: web::Data<Application>) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("Uniswap Watcher v{}", controller.version))
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
