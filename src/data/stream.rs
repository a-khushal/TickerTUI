use crate::data::Candle;
use futures_util::StreamExt;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn stream_klines(symbol: &str, interval: &str) -> tokio::sync::mpsc::Receiver<Candle> {
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    
    let symbol_lower = symbol.to_lowercase();
    let stream_name = format!("{}@kline_{}", symbol_lower, interval);
    let url = format!("wss://stream.binance.com:9443/ws/{}", stream_name);
    
    tokio::spawn(async move {
        loop {
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut _write, mut read) = ws_stream.split();
                    
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    if let Some(k) = json.get("k") {
                                        if let Some(candle) = parse_kline(k) {
                                            let _ = tx.send(candle).await;
                                        }
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                break;
                            }
                            Err(e) => {
                                eprintln!("WebSocket error: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
    
    rx
}

fn parse_kline(k: &Value) -> Option<Candle> {
    Some(Candle {
        open_time: k.get("t")?.as_u64()?,
        open: k.get("o")?.as_str()?.to_string(),
        high: k.get("h")?.as_str()?.to_string(),
        low: k.get("l")?.as_str()?.to_string(),
        close: k.get("c")?.as_str()?.to_string(),
        volume: k.get("v")?.as_str()?.to_string(),
        close_time: k.get("T")?.as_u64()?,
        quote_volume: k.get("q")?.as_str()?.to_string(),
        number_of_trades: k.get("n")?.as_u64()?,
        taker_buy_base: k.get("V")?.as_str()?.to_string(),
        taker_buy_quote: k.get("Q")?.as_str()?.to_string(),
        ignore: "0".to_string(),
    })
}