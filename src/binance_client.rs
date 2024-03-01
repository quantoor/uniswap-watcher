use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ticker {
    pub symbol: String,
    pub price: String,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct Kline {
    kline_open_time: u64,
    pub open_price: String,
    high_price: String,
    low_price: String,
    close_price: String,
    volume: String,
    kline_close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u64,
    taker_buy_base_asset_volume: String,
    taker_buy_quote_asset_volume: String,
    unused: String,
}

#[derive(Clone)]
pub struct BinanceClient {
    host: String,
}

impl BinanceClient {
    pub fn new(host: &str) -> Self {
        Self { host: host.into() }
    }

    pub async fn get_ticker(&self, symbol: &str) -> reqwest::Result<Ticker> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.host, symbol);
        reqwest::get(url).await?.json::<Ticker>().await
    }

    pub async fn get_kline(&self, symbol: &str, timestamp_ms: u64) -> reqwest::Result<Vec<Kline>> {
        let url = format!(
            "{}/api/v3/klines?symbol={}&interval=1m&startTime={}&limit=1",
            self.host, symbol, timestamp_ms
        );
        reqwest::get(url).await?.json::<Vec<Kline>>().await
    }
}
