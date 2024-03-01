use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ticker {
    pub symbol: String,
    pub price: String,
}

pub struct BinanceClient {
    host: String,
}

impl BinanceClient {
    pub fn new(host: String) -> Self {
        Self { host }
    }

    pub async fn get_ticker(&self, symbol: &str) -> reqwest::Result<Ticker> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.host, symbol);
        reqwest::get(url).await?.json::<Ticker>().await
    }

    pub async fn get_kline(&self, symbol: String) -> reqwest::Result<f64> {
        Ok(0.0) // todo
    }
}
