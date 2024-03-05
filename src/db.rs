use anyhow::Result;
use ethers::prelude::TxHash;
use ethers::utils::hex::ToHexExt;
use sqlx::{FromRow, PgPool};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tracing::{error, info};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self, docker: bool) -> String {
        let host = if docker {
            "host.docker.internal"
        } else {
            self.host.as_str()
        };
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, host, self.port, self.database_name
        )
    }
}

#[derive(Clone, Debug, FromRow, PartialEq)]
pub struct TxFee {
    pub tx_hash: String,
    pub fee_eth: f64,
    pub fee_usdt: f64,
}

/// Insert tx in db
pub async fn insert_tx_fee(data: &TxFee, pool: &PgPool) -> Result<()> {
    info!("Inserting in db TxFee={:?}", data);
    _ = sqlx::query(
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
    Ok(())
}

/// Get tx fee from db
pub async fn get_tx_fee_from_db(tx_hash: &TxHash, pool: &PgPool) -> Result<TxFee> {
    info!(
        "Getting from db tx_hash {}",
        tx_hash.encode_hex_with_prefix()
    );
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

/// Keep consuming elements from the queue and insert them in db
pub async fn run_queue_receiver(rx: Receiver<TxFee>, pool: PgPool) {
    info!("Running queue receiver");
    loop {
        match rx.try_recv() {
            Ok(txfee) => match insert_tx_fee(&txfee, &pool).await {
                Ok(_) => info!("Receiver inserted new data in db"),
                Err(err) => error!("Error inserting tx fee {:?} in db: {:?}", txfee, err),
            },
            Err(err) => {
                match err {
                    std::sync::mpsc::TryRecvError::Empty => {
                        // No message to consume, sleep 100 ms
                        tokio::time::sleep(Duration::from_millis(100)).await
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        info!("Channel closed, exit");
                        break;
                    }
                }
            }
        }
    }
}
