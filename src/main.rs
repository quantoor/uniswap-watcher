use env_logger::Env;
use tracing::info;
use uniswap_watcher::{run_server, subscribe_logs};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Subscribing to logs...");
    tokio::spawn(subscribe_logs());

    info!("Serving...");
    let address = format!("127.0.0.1:{}", 8080);
    run_server(address)?.await.expect("Error running server");

    Ok(())
}
