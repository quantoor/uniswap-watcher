use ethers::prelude::TxHash;
use ethers::utils::hex::ToHexExt;
use sqlx::{FromRow, PgPool};
use tracing::info;

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

#[derive(Debug, FromRow)]
pub struct TxFee {
    pub tx_hash: String,
    pub fee_eth: f64,
    pub fee_usdt: f64,
}

pub async fn insert_tx_fee(data: &TxFee, pool: &PgPool) -> anyhow::Result<()> {
    info!("Inserting TxFee={:?}", data);
    let res = sqlx::query(
        r#"
        INSERT INTO fees (tx_hash, fee_eth, fee_usdt)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(data.tx_hash.clone())
    .bind(data.fee_eth)
    .bind(data.fee_usdt)
    .execute(pool)
    .await?;
    info!("rows affected: {}", res.rows_affected());
    Ok(())
}

pub async fn get_tx_fee_from_db(tx_hash: &TxHash, pool: &PgPool) -> anyhow::Result<TxFee> {
    println!("{}", tx_hash.encode_hex_with_prefix());
    let res = sqlx::query_as::<_, TxFee>(
        r#"
        SELECT * FROM fees
        WHERE tx_hash = $1
        "#,
    )
    .bind(tx_hash.encode_hex_with_prefix())
    .fetch_one(pool)
    .await?;
    Ok(res)
}
