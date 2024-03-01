use actix_web::{web, App, HttpServer};
use anyhow::Result;
use ethers::providers::Middleware;
use uniswap_watcher::controller::{home, tx_fee, Application};
use uniswap_watcher::subscribe_logs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Subscribing to logs...");
    tokio::spawn(subscribe_logs());

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
