use crate::{compute_gas_fee_eth, RPC_URL_HTTP, VERSION};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider, H256};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone)]
pub struct Application {
    pub version: String,
    pub client: Provider<Http>,
}

impl Application {
    pub fn new() -> Result<Application> {
        Ok(Self {
            version: VERSION.into(),
            client: Provider::<Http>::try_from(RPC_URL_HTTP).unwrap(),
        })
    }

    pub async fn get_tx_fee(&self, tx_hash: &str) -> Result<f64> {
        let tx_hash = H256::from_str(tx_hash)?;
        // todo check db
        let tx = self.client.get_transaction_receipt(tx_hash).await?;
        if tx == None {
            panic!("tx hash not found") // fixme
        }
        let tx = tx.unwrap();
        let fee = compute_gas_fee_eth(&tx).await.unwrap();
        Ok(fee)
    }

    pub async fn get_tx_fee_batch(&self, tx_hashes: Vec<String>) -> Result<HashMap<String, f64>> {
        let mut res: HashMap<String, f64> = HashMap::new();
        for tx_hash in tx_hashes {
            let fee = self.get_tx_fee(tx_hash.as_str()).await;
            res.insert(tx_hash, fee.unwrap());
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
