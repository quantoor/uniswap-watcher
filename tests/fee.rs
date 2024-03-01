use ethers::types::U256;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::H256,
};
use std::str::FromStr;
use uniswap_watcher::{compute_gas_fee_eth, RPC_URL_HTTP};

const TEST_TX_HASH: &str = "0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77";

#[tokio::test]
async fn get_tx_receipt() {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let tx_hash = H256::from_str(TEST_TX_HASH).unwrap();
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
async fn gas_fee_eth() {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let tx_hash = H256::from_str(TEST_TX_HASH).unwrap();
    let tx = client
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();

    let gas_fee = compute_gas_fee_eth(&tx).await.unwrap();
    assert_eq!(gas_fee, 0.0181623707852374);
}
