// todo remove
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use env_logger::Env;
use sqlx::PgPool;
use tracing::info;
use uniswap_watcher::db::DatabaseSettings;
use uniswap_watcher::{run_server, subscribe_logs};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let settings = DatabaseSettings {
        username: "postgres".into(),
        password: "password".into(),
        port: 5432,
        host: "127.0.0.1".into(),
        database_name: "fees".into(),
    };
    let db_connection = PgPool::connect(&settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");

    info!("Subscribing to logs...");
    tokio::spawn(subscribe_logs(db_connection.clone()));

    info!("Serving...");
    let address = format!("127.0.0.1:{}", 8080);
    run_server(address, db_connection)?
        .await
        .expect("Error running server");

    Ok(())
}
