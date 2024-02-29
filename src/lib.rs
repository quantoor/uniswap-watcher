use anyhow::{Result, Error, anyhow};
use ethers::prelude::TransactionReceipt;
use ethers::utils::format_units;

pub const RPC_URL_HTTP: &str = "https://eth.drpc.org";
pub const RPC_URL_WS: &str = "wss://ethereum-rpc.publicnode.com";
pub const POOL_ADDRESS: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";

pub async fn compute_gas_fee_eth(tx: &TransactionReceipt) -> Result<f64, Error> {
    let gas_price = tx.effective_gas_price.ok_or(anyhow!("effective gas price not found in tx receipt"))?;
    let gas = tx.gas_used.ok_or(anyhow!("gas used not found in tx receipt"))?;
    let gas_eth = gas_price * gas;
    let gas_eth_str = format_units(gas_eth, "ether")?;
    Ok(gas_eth_str.parse()?)
}
