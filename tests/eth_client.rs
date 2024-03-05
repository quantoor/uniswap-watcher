use anyhow::Result;
use ethers::middleware::Middleware;
use ethers::prelude::{H256, U256};
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use std::str::FromStr;
use std::sync::Arc;
use uniswap_watcher::util::tx_hash_to_price;
use uniswap_watcher::IERC20;

const POOL_ADDRESS: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
const RPC_URL_HTTP: &str = "https://eth.drpc.org";
const SWAP_TOPIC: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

#[tokio::test]
async fn get_tx_receipt() {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let tx_hash =
        H256::from_str("0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77")
            .unwrap();
    let receipt = client
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipt.transaction_hash, tx_hash);
    assert_eq!(receipt.gas_used.unwrap(), U256::from(338200usize));
    assert_eq!(
        receipt.effective_gas_price.unwrap(),
        U256::from(53703047857usize)
    );
}

#[tokio::test]
async fn erc20() -> Result<()> {
    let provider = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let client = Arc::new(provider);
    let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>()?;
    let weth = IERC20::new(weth_address, client);
    let symbol = weth.symbol().call().await?;
    let decimals = weth.decimals().call().await?;
    assert_eq!(symbol, "WETH");
    assert_eq!(decimals, 18);
    Ok(())
}

#[tokio::test]
async fn decode_price() -> Result<()> {
    let eth_client = Provider::<Http>::try_from("https://eth.drpc.org").unwrap();
    let swap_topic = H256::from_str(SWAP_TOPIC).unwrap();
    let pool_address = POOL_ADDRESS.parse::<Address>().unwrap();
    let tx_hash =
        H256::from_str("0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77")
            .unwrap();
    let price = tx_hash_to_price(swap_topic, pool_address, tx_hash, &eth_client).await?;
    assert_eq!(price, 3405.792833770436);
    Ok(())
}
