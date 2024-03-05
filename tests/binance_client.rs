use anyhow::Result;
use uniswap_watcher::binance_client::BinanceClient;

#[tokio::test]
async fn ticker() -> Result<()> {
    let client = BinanceClient::new("https://api.binance.com");
    let res = client.get_ticker("ETHUSDT").await?;
    println!("{res:?}");
    Ok(())
}

#[tokio::test]
async fn kline() -> Result<()> {
    let client = BinanceClient::new("https://api.binance.com");
    let res = client.get_kline("ETHUSDT", 1709314843000).await?;
    println!("{res:?}");
    Ok(())
}
