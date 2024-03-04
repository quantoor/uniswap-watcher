use anyhow::Result;
use uniswap_watcher::binance_client::BinanceClient;
use uniswap_watcher::BINANCE_HOST;

#[tokio::test]
async fn ticker() -> Result<()> {
    let client = BinanceClient::new(BINANCE_HOST.into());
    let res = client.get_ticker("ETHUSDT").await?;
    println!("{res:?}");
    Ok(())
}

#[tokio::test]
async fn kline() -> Result<()> {
    let client = BinanceClient::new(BINANCE_HOST.into());
    let res = client.get_kline("ETHUSDT", 1709314843000).await?;
    println!("{res:?}");
    Ok(())
}
