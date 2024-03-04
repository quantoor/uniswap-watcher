use env_logger::Env;
use sqlx::postgres::PgPoolOptions;
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
        host: "host.docker.internal".into(),
        database_name: "postgres_db".into(),
    };
    let db_connection = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(3))
        .connect_lazy(&settings.connection_string())
        .expect("Failed to create db connection");

    info!("Subscribing to logs...");
    tokio::spawn(subscribe_logs(db_connection.clone()));

    info!("Serving...");
    let address = format!("0.0.0.0:{}", 8080);
    run_server(address, db_connection)?
        .await
        .expect("Error running server");

    Ok(())
}
