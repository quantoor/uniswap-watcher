use ethers::prelude::TransactionReceipt;
use ethers::utils::format_units;

pub const RPC_URL_HTTP: &str = "https://eth.drpc.org";
pub const RPC_URL_WS: &str = "wss://ethereum-rpc.publicnode.com";
pub const POOL_ADDRESS: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";

pub async fn compute_gas_fee(tx: &TransactionReceipt) -> anyhow::Result<f64, &'static str> {
    let gas_price = tx.effective_gas_price.ok_or("No gas price available")?;
    let gas = tx.gas_used.ok_or("No gas price available")?;

    let gas_eth = gas_price * gas;
    // println!(
    //     "gas_price={gas_price:?} gas={gas:?} tx={:?} fee={:?} ETH",
    //     tx.transaction_hash, gas_eth
    // );
    let s: String = format_units(gas_eth, "ether").unwrap();
    Ok(s.parse().unwrap())
}
