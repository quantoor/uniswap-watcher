use anyhow::{anyhow, Error, Result};
use ethers::abi::Address;
use ethers::middleware::Middleware;
use ethers::prelude::{Http, Provider, TransactionReceipt, H256, I256};
use ethers::types::{Bytes, TxHash};
use ethers::utils::format_units;
use ethers::utils::hex::ToHexExt;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::ops::BitXor;
use std::time;
use tracing::info;

/// Get transaction receipt for given transaction hash
pub async fn try_get_tx_receipt(
    tx_hash: TxHash,
    eth_client: &Provider<Http>,
) -> Result<TransactionReceipt> {
    // The tx receipt may not be found immediately after receiving the event log,
    // so a retry logic is used to try fetch the receipt every second for 5 seconds
    let mut count = 0;
    loop {
        match eth_client.get_transaction_receipt(tx_hash).await? {
            Some(receipt) => {
                return Ok(receipt);
            }
            None => {
                count += 1;
                if count >= 5 {
                    // Reached max retries, and receipt not found
                    return Err(anyhow!(
                        "Tx receipt for tx hash {} not found after 5 retries",
                        tx_hash.encode_hex_with_prefix()
                    ));
                }
                info!(
                    "Tx receipt for tx hash {} not found, wait 1 sec and retry",
                    tx_hash.encode_hex_with_prefix()
                );
                tokio::time::sleep(time::Duration::from_millis(1000)).await;
            }
        }
    }
}

/// Given a tx receipt, computes the gas fee in ETH
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

/// Convert a hex string into a BigInt
pub fn hex_to_int256(s: &str) -> BigInt {
    // https://stackoverflow.com/questions/67165852/kotlin-convert-hex-string-to-signed-integer-via-signed-2s-complement
    let x = BigInt::parse_bytes(
        b"8000000000000000000000000000000000000000000000000000000000000000",
        16,
    )
    .unwrap();

    // Convert last 64 characters of the string to BigInt
    let substring = &s[s.len() - 64..];
    let y = BigInt::parse_bytes(substring.as_bytes(), 16).unwrap();

    // Perform XOR and subtraction
    y.bitxor(&x) - x
}

/// Takes given the data of a swap event on the WETH-USDC-500 pool, compute the price as amount_usdc/amount_weth
pub fn log_data_to_price(data: Bytes) -> Result<f64> {
    let data_str = data.encode_hex_with_prefix().clone()[2..].to_string();
    let amount0 = data_str[..64].to_string(); // get first 32 bytes - amount0
    let amount1 = data_str[64..128].to_string(); // get second 32 bytes - amount1
    let amount0 = hex_to_int256(amount0.as_str());
    let amount1 = hex_to_int256(amount1.as_str());
    let amount0 = amount0
        .to_isize()
        .ok_or(anyhow!("error converting bigint {:?}", amount0))?;
    let amount1 = amount1
        .to_isize()
        .ok_or(anyhow!("error converting bigint {:?}", amount1))?;
    let amount0 = I256::from(amount0);
    let amount1 = I256::from(amount1);
    // Note: this function is specific for the pool 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640 on mainnet
    let amount_usdc = format_units(amount0, 6)?.parse::<f64>()?;
    let amount_weth = format_units(amount1, 18)?.parse::<f64>()?;
    Ok((amount_usdc / amount_weth).abs())
}

/// Given a tx hash, return the swap price if a swap event is found for the given topic and address
pub async fn tx_hash_to_price(
    swap_topic: H256,
    pool_address: Address,
    tx_hash: TxHash,
    eth_client: &Provider<Http>,
) -> Result<f64> {
    let tx_receipt = try_get_tx_receipt(tx_hash, eth_client).await?;
    let logs: Vec<_> = tx_receipt
        .logs
        .iter()
        .filter(|log| log.topics[0] == swap_topic && log.address == pool_address)
        .collect();
    if logs.is_empty() {
        return Err(anyhow!(
            "no swap log event found for tx hash {}",
            tx_hash.encode_hex_with_prefix()
        ));
    }
    log_data_to_price(logs[0].clone().data)
}
