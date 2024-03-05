use anyhow::Result;
use ethers::middleware::Middleware;
use ethers::prelude::{H256, U256};
use ethers::providers::{Http, Provider};
use ethers::types::{Address, I256};
use ethers::utils::format_units;
use std::str::FromStr;
use std::sync::Arc;
use uniswap_watcher::{IERC20, RPC_URL_HTTP};

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
    let address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>()?;
    let weth = IERC20::new(address, client);
    let symbol = weth.symbol().call().await?;
    let decimals = weth.decimals().call().await?;
    assert_eq!(symbol, "WETH");
    assert_eq!(decimals, 18);
    Ok(())
}

#[tokio::test]
async fn price() -> Result<()> {
    let amount0 = -I256::from(3500000000usize);
    let amount1 = I256::from(1100000000000000000usize);

    let amount0 = format_units(amount0, 6)?.parse::<f64>()?;
    let amount1 = format_units(amount1, 18)?.parse::<f64>()?;
    let price = (amount0 / amount1).abs();
    println!("{price:?}");

    Ok(())
}
