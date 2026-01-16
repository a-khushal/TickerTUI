use serde_json::Value;

#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
    #[allow(dead_code)]
    pub last_update: u64,
}

#[allow(dead_code)]
pub async fn fetch_orderbook(symbol: &str) -> Result<OrderBook, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("https://api.binance.com/api/v3/depth?symbol={}&limit=20", symbol);
    
    let res = client.get(&url).send().await?.json::<Value>().await?;
    
    let bids: Vec<OrderBookEntry> = res
        .get("bids")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|entry| {
            let arr = entry.as_array()?;
            Some(OrderBookEntry {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();
    
    let asks: Vec<OrderBookEntry> = res
        .get("asks")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|entry| {
            let arr = entry.as_array()?;
            Some(OrderBookEntry {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();
    
    Ok(OrderBook {
        bids,
        asks,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

pub async fn stream_orderbook(symbol: &str) -> tokio::sync::mpsc::Receiver<OrderBook> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let symbol_lower = symbol.to_lowercase();
    let url = format!("wss://stream.binance.com:9443/ws/{}@depth20@100ms", symbol_lower);
    
    tokio::spawn(async move {
        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut _write, mut read) = ws_stream.split();
                    
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    if let Some(book) = parse_orderbook(&json) {
                                        let _ = tx.send(book).await;
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

fn parse_orderbook(json: &Value) -> Option<OrderBook> {
    let bids: Vec<OrderBookEntry> = json
        .get("bids")?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let arr = entry.as_array()?;
            Some(OrderBookEntry {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();
    
    let asks: Vec<OrderBookEntry> = json
        .get("asks")?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let arr = entry.as_array()?;
            Some(OrderBookEntry {
                price: arr[0].as_str()?.parse().ok()?,
                quantity: arr[1].as_str()?.parse().ok()?,
            })
        })
        .collect();
    
    Some(OrderBook {
        bids,
        asks,
        last_update: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

use futures_util::StreamExt;