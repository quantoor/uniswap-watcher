// todo remove
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

pub mod binance_client;
pub mod controller;
pub mod db;

use crate::binance_client::BinanceClient;
use crate::controller::{home, tx_fee, Application};
use crate::db::{insert_tx_fee, DatabaseSettings, TxFee};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer, Responder};
use anyhow::{anyhow, Error, Result};
use ethers::contract::{abigen, Contract, LogMeta};
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider, TransactionReceipt, ValueOrArray, Ws, H256};
use ethers::utils::format_units;
use ethers::utils::hex::ToHexExt;
use futures_util::StreamExt;
use sqlx;
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool, Row};
use std::sync::Arc;
use std::time;
use tracing::{error, info};

pub const VERSION: &str = "0.0.1";
pub const RPC_URL_HTTP: &str = "https://eth.drpc.org";
pub const RPC_URL_WS: &str = "wss://ethereum-rpc.publicnode.com";
pub const POOL_ADDRESS: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
pub const BINANCE_HOST: &str = "https://api.binance.com";

abigen!(
    AggregatorInterface,
    r#"[
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    ]"#,
);

pub async fn compute_gas_fee_eth(tx: &TransactionReceipt) -> Result<f64, Error> {
    let gas_price = tx
        .effective_gas_price
        .ok_or(anyhow!("effective gas price not found in tx receipt"))?;
    let gas = tx
        .gas_used
        .ok_or(anyhow!("gas used not found in tx receipt"))?;
    let gas_eth = gas_price * gas;
    let gas_eth_str = format_units(gas_eth, "ether")?;
    Ok(gas_eth_str.parse()?)
}

#[allow(unreachable_code)]
pub async fn subscribe_logs(db_connection: PgPool) -> Result<()> {
    // todo app attributes
    let binance_client = BinanceClient::new(BINANCE_HOST.into());
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let ws_client = Provider::<Ws>::connect(RPC_URL_WS).await.unwrap();
    let ws_client = Arc::new(ws_client);

    let event = Contract::event_of_type::<SwapFilter>(ws_client)
        .address(ValueOrArray::Array(vec![POOL_ADDRESS.parse()?]));

    let mut stream = event.subscribe_with_meta().await?;
    loop {
        while let Some(Ok((_log, meta))) = stream.next().await {
            let meta: LogMeta = meta;

            tokio::time::sleep(time::Duration::from_millis(2000)).await; // fixme

            let tx = client
                .get_transaction_receipt(meta.transaction_hash)
                .await
                .unwrap(); // todo handle this
            if tx == None {
                // todo retry
                error!("receipt is none for tx_hash={:?}", meta.transaction_hash);
                continue;
            }
            let tx = tx.unwrap();
            let fee_eth = compute_gas_fee_eth(&tx).await.unwrap();

            info!("getting ticker");
            let ticker = binance_client.get_ticker("ETHUSDT").await?;
            let eth_price = ticker.price;
            let eth_price: f64 = eth_price.parse()?;
            let fee_usdt = fee_eth * eth_price;

            // todo queue
            let data = TxFee {
                tx_hash: tx.transaction_hash.encode_hex_with_prefix(),
                fee_eth,
                fee_usdt,
            };
            insert_tx_fee(&data, &db_connection).await;
        }
    }
    Ok(())
}

pub fn run_server(address: String, db_connection: PgPool) -> Result<Server, std::io::Error> {
    let app = Application::new().unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app.clone()))
            .service(web::scope("").service(home).service(tx_fee))
            .app_data(db_connection.clone())
    })
    .bind(address)?
    .run();
    Ok(server)
}
