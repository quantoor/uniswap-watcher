use anyhow::Result;
use uniswap_watcher::binance_client::BinanceClient;

#[tokio::test]
async fn ticker() -> Result<()> {
    let client = BinanceClient::new("https://api.binance.com".into());
    let res = client.get_ticker("ETHUSDT").await?;
    println!("{res:?}");
    Ok(())
}
