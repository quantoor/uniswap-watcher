use sqlx;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uniswap_watcher::db::DatabaseSettings;

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection =
        PgConnection::connect(&config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[tokio::test]
async fn store() {
    let settings = DatabaseSettings {
        username: "postgres".into(),
        password: "password".into(),
        port: 5432,
        host: "127.0.0.1".into(),
        database_name: "fees".into(),
    };
    configure_database(&settings).await;
}
