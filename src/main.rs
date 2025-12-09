mod data;
mod ui;

use data::fetch_klines;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let candles = fetch_klines("BTCUSDT", "1h", 1000).await?;
    println!("{:?}", candles[0]);
    Ok(())
}
