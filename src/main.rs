use actix_web::{web, App, HttpServer};
use uniswap_watcher::controller::{home, tx_fee, Application};
use uniswap_watcher::subscribe_logs;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Subscribing to logs...");
    tokio::spawn(subscribe_logs());

    println!("Serving...");
    let state = Application::new().unwrap();
    let address = format!("127.0.0.1:{}", 8080);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(web::scope("").service(home).service(tx_fee))
    })
    .bind(address)?
    .run()
    .await
}