use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Candle {
    pub open_time: u64,     
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: u64,
    pub quote_volume: String,
    pub number_of_trades: u64,
    pub taker_buy_base: String,
    pub taker_buy_quote: String,
    pub ignore: String,
}

pub async fn fetch_klines(symbol: &str, interval: &str, limit: u32) -> Result<Vec<Candle>, reqwest::Error> {
    let client = Client::new();
    let url = "https://api.binance.com/api/v3/klines";
    let limit_str = limit.to_string();
    let res = client
        .get(url)
        .query(&[
            ("symbol", symbol),
            ("interval", interval),
            ("limit", &limit_str),
        ])
        .send()
        .await?
        .json::<Vec<Vec<Value>>>()
        .await?;
    
    let candles: Vec<Candle> = res
        .into_iter()
        .map(|arr| Candle {
            open_time: arr[0].as_u64().unwrap_or(0),
            open: arr[1].as_str().unwrap_or("0").to_string(),
            high: arr[2].as_str().unwrap_or("0").to_string(),
            low: arr[3].as_str().unwrap_or("0").to_string(),
            close: arr[4].as_str().unwrap_or("0").to_string(),
            volume: arr[5].as_str().unwrap_or("0").to_string(),
            close_time: arr[6].as_u64().unwrap_or(0),
            quote_volume: arr[7].as_str().unwrap_or("0").to_string(),
            number_of_trades: arr[8].as_u64().unwrap_or(0),
            taker_buy_base: arr[9].as_str().unwrap_or("0").to_string(),
            taker_buy_quote: arr[10].as_str().unwrap_or("0").to_string(),
            ignore: arr[11].as_str().unwrap_or("0").to_string(),
        })
        .collect();
    
    Ok(candles)
}
