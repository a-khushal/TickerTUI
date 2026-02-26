use futures_util::StreamExt;
use serde_json::Value;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct WatchPrice {
    pub symbol: String,
    pub last_price: f64,
    pub change_pct: f64,
}

pub fn stream_watchlist_prices(
    symbols: &[String],
) -> (tokio::sync::mpsc::Receiver<WatchPrice>, JoinHandle<()>) {
    let (tx, rx) = tokio::sync::mpsc::channel(500);
    let streams = symbols
        .iter()
        .map(|symbol| format!("{}@miniTicker", symbol.to_lowercase()))
        .collect::<Vec<_>>()
        .join("/");

    let url = format!("wss://stream.binance.com:9443/stream?streams={}", streams);

    let handle = tokio::spawn(async move {
        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut _write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    if let Some(price) = parse_mini_ticker(&json) {
                                        if tx.send(price).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    });

    (rx, handle)
}

fn parse_mini_ticker(json: &Value) -> Option<WatchPrice> {
    let data = json.get("data")?;
    let symbol = data.get("s")?.as_str()?.to_string();
    let close = data.get("c")?.as_str()?.parse::<f64>().ok()?;
    let open = data.get("o")?.as_str()?.parse::<f64>().ok()?;
    let change_pct = if open > 0.0 {
        ((close - open) / open) * 100.0
    } else {
        0.0
    };

    Some(WatchPrice {
        symbol,
        last_price: close,
        change_pct,
    })
}
