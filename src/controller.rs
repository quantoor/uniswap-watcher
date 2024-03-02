use crate::binance_client::BinanceClient;
use crate::db::{get_tx_fee_from_db, insert_tx_fee, DatabaseSettings};
use crate::{compute_gas_fee_eth, TxFee, BINANCE_HOST, RPC_URL_HTTP, VERSION};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider};
use ethers::types::{BlockId, TxHash};
use ethers::utils::hex::ToHexExt;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{error, info};

#[derive(Clone)]
pub struct Application {
    pub version: String,
    pub client: Provider<Http>,
    pub binance_client: BinanceClient,
    pub db_connection: PgPool,
}

impl Application {
    pub fn new(db_connection: PgPool) -> Result<Application> {
        Ok(Self {
            version: VERSION.into(),
            client: Provider::<Http>::try_from(RPC_URL_HTTP).unwrap(),
            binance_client: BinanceClient::new(BINANCE_HOST),
            db_connection,
        })
    }

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
        let res = self.get_tx_fee(tx_hash).await;
        if res.is_err() {
            error!(
                "Could not get fee for tx_hash={}",
                tx_hash.encode_hex_with_prefix()
            );
            return res;
        }
        let res = res.unwrap();

        // Store fee in db
        if insert_tx_fee(&res.clone(), &self.db_connection)
            .await
            .is_err()
        {
            error!(
                "Error inserting in db tx_hash={}",
                tx_hash.encode_hex_with_prefix()
            );
        }

        // Return result
        Ok(res)
    }

    pub async fn get_tx_fee(&self, tx_hash: &TxHash) -> Result<TxFee> {
        info!(
            "Getting fee for tx_hash={}",
            tx_hash.encode_hex_with_prefix()
        );

        // todo remove all unwrap
        // todo check db

        // Get transaction receipt for given transaction hash
        let tx_receipt = self.client.get_transaction_receipt(*tx_hash).await?;
        if tx_receipt == None {
            panic!("tx hash not found") // fixme
        }
        let tx = tx_receipt.unwrap();

        // Use transaction receipt to compute the gas fee: gas_fee = gas_used * gas_price
        let fee_eth = compute_gas_fee_eth(&tx).await.unwrap();

        // Get from Binance the price of ETH/USDT at the time of the transaction (with 1 min precision)
        let block = self
            .client
            .get_block(BlockId::Hash(tx.block_hash.unwrap()))
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
            tx_hash: tx.transaction_hash.encode_hex_with_prefix(),
            fee_eth,
            fee_usdt,
        })
    }

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

#[get("/")]
async fn home(controller: web::Data<Application>) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("v{}", controller.version))
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
