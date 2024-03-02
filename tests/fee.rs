use ethers::types::U256;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::H256,
};
use std::collections::HashMap;
use std::str::FromStr;
use uniswap_watcher::controller::Application;
use uniswap_watcher::{compute_gas_fee_eth, RPC_URL_HTTP};

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
async fn gas_fee_usdt() {
    let hash1 = "0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77";
    let hash2 = "0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7";
    let hash3 = "0x926484f31f9d99d24b0e984a98483f6459872fbcb7e0abd5f1ce704d70835cee";
    let app = Application::new().unwrap();
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
