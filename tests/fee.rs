use ethers::{
    providers::{Http, Middleware, Provider},
    types::H256,
};
use std::str::FromStr;
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
    println!("{receipt:?}");
}

#[tokio::test]
async fn gas_fee_eth() {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();
    let tx_hash =
        H256::from_str("0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7")
            .unwrap();
    let tx = client
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();

    let gas_fee = compute_gas_fee_eth(&tx).await.unwrap();
    assert_eq!(gas_fee, 0.018990909956827312);
}
