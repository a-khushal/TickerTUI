use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Trade {
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
    #[allow(dead_code)]
    pub timestamp: u64,
}

pub async fn stream_trades(symbol: &str) -> tokio::sync::mpsc::Receiver<Trade> {
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    let symbol_lower = symbol.to_lowercase();
    let url = format!("wss://stream.binance.com:9443/ws/{}@trade", symbol_lower);
    
    tokio::spawn(async move {
        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut _write, mut read) = ws_stream.split();
                    
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    if let Some(trade) = parse_trade(&json) {
                                        let _ = tx.send(trade).await;
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
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
    
    rx
}

fn parse_trade(json: &Value) -> Option<Trade> {
    Some(Trade {
        price: json.get("p")?.as_str()?.parse().ok()?,
        quantity: json.get("q")?.as_str()?.parse().ok()?,
        is_buyer_maker: json.get("m")?.as_bool()?,
        timestamp: json.get("T")?.as_u64()?,
    })
}

use futures_util::StreamExt;