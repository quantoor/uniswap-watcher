use crate::{compute_gas_fee_eth, RPC_URL_HTTP, VERSION};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse, Responder};
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider, H256};
use std::str::FromStr;

#[derive(Clone)]
pub struct Application {
    pub version: String,
    pub client: Provider<Http>,
}

impl Application {
    pub fn new() -> anyhow::Result<Application> {
        Ok(Self {
            version: VERSION.into(),
            client: Provider::<Http>::try_from(RPC_URL_HTTP).unwrap(),
        })
    }
}

#[get("/")]
async fn home(controller: web::Data<Application>) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("v{}", controller.version))
}

#[get("/tx_fee")]
async fn tx_fee(controller: web::Data<Application>) -> impl Responder {
    let tx_hash =
        H256::from_str("0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7") // fixme
            .unwrap();
    let tx = controller
        .client
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap(); // todo handle this
    if tx == None {
        panic!("tx hash not found") // fixme
    }
    let tx = tx.unwrap();
    let fee = compute_gas_fee_eth(&tx).await.unwrap();
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("{}", fee))
}
