use env_logger::Env;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::mpsc;
use tracing::info;
use uniswap_watcher::db::run_queue_receiver;
use uniswap_watcher::{run_server, subscribe_logs, AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let app_config = match AppConfig::new() {
        Ok(config) => config,
        Err(err) => panic!("Failed to load app config: {}", err),
    };
    info!("Loaded app config");

    let docker = env::var("DOCKER").is_ok(); // whether the application is running in docker
    let settings = app_config.database.clone();
    let db_connection = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(3))
        .connect_lazy(&settings.connection_string(docker))
        .expect("Failed to create db connection");

    info!("Spawning queue receiver");
    let (sender, receiver) = mpsc::channel();
    tokio::spawn(run_queue_receiver(receiver, db_connection.clone()));

    info!("Subscribing to logs");
    tokio::spawn(subscribe_logs(app_config.clone(), sender.clone()));

    info!("Serving...");
    let address = format!("0.0.0.0:{}", app_config.application_port);
    run_server(app_config.clone(), address, sender, db_connection)?
        .await
        .expect("Error running server");

    Ok(())
}
