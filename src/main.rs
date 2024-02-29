use actix_web::http::header::ContentType;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use ethers::prelude::H256;
use ethers::providers::Middleware;
use std::fmt::format;
use std::net::TcpListener;
use std::str::FromStr;
use uniswap_watcher::{compute_gas_fee_eth, subscribe_logs};
use uniswap_watcher::controller::{Application, home, tx_fee};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Serving...");
    let state = Application::new().unwrap();
    let address = format!("127.0.0.1:{}", 8080);
    _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(web::scope("").service(home).service(tx_fee))
    })
    .bind(address)?
    .run()
    .await;
    Ok(())
}
