use crate::{compute_gas_fee_eth, RPC_URL_HTTP, VERSION};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider, H256};
use std::str::FromStr;

#[derive(Clone)]
pub struct Application {
    pub version: String,
    pub client: Provider<Http>,
}

#[derive(serde::Deserialize)]
struct Args {
    tx_hash: String,
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
}

#[get("/")]
async fn home(controller: web::Data<Application>) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("v{}", controller.version))
}

#[get("/tx_fee")]
async fn tx_fee(controller: web::Data<Application>, args: web::Query<Args>,) -> impl Responder {
    let fee = controller
        .get_tx_fee(args.tx_hash.as_str())
        .await
        .unwrap();
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("{}", fee))
}
