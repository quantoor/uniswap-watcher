use ethers::{
    providers::{Http, Middleware, Provider},
    types::H256,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::collections::HashMap;
use std::str::FromStr;
use uniswap_watcher::db::{get_tx_fee_from_db, DatabaseSettings, TxFee};
use uniswap_watcher::{compute_gas_fee_eth, Application, RPC_URL_HTTP};

pub async fn get_db_connection() -> PgPool {
    let settings = DatabaseSettings {
        username: "postgres".into(),
        password: "password".into(),
        port: 5432,
        host: "127.0.0.1".into(),
        database_name: "postgres_db".into(),
    };
    let mut connection = PgConnection::connect(&settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    _ = connection
        .execute(
            r#"
        CREATE TABLE fees
        (
            tx_hash   TEXT             NOT NULL,
            PRIMARY KEY (tx_hash),
            fee_eth   DOUBLE PRECISION NOT NULL,
            fee_usdt  DOUBLE PRECISION NOT NULL
        );
        "#,
        )
        .await;
    _ = sqlx::query(
        r#"
        INSERT INTO fees (tx_hash, fee_eth, fee_usdt)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind("0xf3a23cc9af86832d33e87d717a6490fb75f594220abc88485084516256bae331")
    .bind(0.11)
    .bind(50.3)
    .execute(&mut connection)
    .await;
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect(&settings.connection_string())
        .await
        .expect("Failed to create db connection")
}

#[tokio::test]
async fn gas_fee_eth() {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let tx_hash =
        H256::from_str("0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77")
            .unwrap();
    let tx = client
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();

    let gas_fee = compute_gas_fee_eth(&tx).await.unwrap();
    assert_eq!(gas_fee, 0.0181623707852374);
}

#[tokio::test]
async fn get_tx_fee_single() {
    let db_connection = get_db_connection().await;
    let actual = get_tx_fee_from_db(
        &H256::from_str("0xf3a23cc9af86832d33e87d717a6490fb75f594220abc88485084516256bae331")
            .unwrap(),
        &db_connection,
    )
    .await
    .unwrap();
    let expected = TxFee {
        tx_hash: "0xf3a23cc9af86832d33e87d717a6490fb75f594220abc88485084516256bae331".to_string(),
        fee_eth: 0.11,
        fee_usdt: 50.3,
    };
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn get_tx_fee_batch() {
    let hash1 = "0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77";
    let hash2 = "0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7";
    let hash3 = "0x926484f31f9d99d24b0e984a98483f6459872fbcb7e0abd5f1ce704d70835cee";

    let db_connection = get_db_connection().await;
    let app = Application::new(db_connection).unwrap();
    let actual = app
        .get_tx_fee_batch(vec![
            hash1.to_string(),
            hash2.to_string(),
            hash3.to_string(),
        ])
        .await
        .unwrap();
    let expected = HashMap::from([
        (H256::from_str(hash1).unwrap(), 61.875928038562485),
        (H256::from_str(hash2).unwrap(), 64.23295474697701),
        (H256::from_str(hash3).unwrap(), 409.30215911746706),
    ]);
    assert_eq!(actual, expected);
}
