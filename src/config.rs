use crate::ui::Timeframe;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub watchlist: Vec<String>,
    pub selected_symbol: usize,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub zoom: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        let watchlist = default_watchlist();
        Self {
            symbol: watchlist[0].clone(),
            watchlist,
            selected_symbol: 0,
            timeframe: Timeframe::OneMonth,
            zoom: 1,
        }
    }
}

impl AppConfig {
    pub fn sanitized(mut self) -> Self {
        if self.watchlist.is_empty() {
            self.watchlist = default_watchlist();
        }

        if self.selected_symbol >= self.watchlist.len() {
            self.selected_symbol = self.watchlist.len().saturating_sub(1);
        }

        if !self.watchlist.iter().any(|s| s == &self.symbol) {
            self.symbol = self.watchlist[self.selected_symbol].clone();
        }

        self.zoom = self.zoom.clamp(1, 32);
        self
    }
}

pub fn default_watchlist() -> Vec<String> {
    vec![
        "BTCUSDT".to_string(),
        "ETHUSDT".to_string(),
        "BNBUSDT".to_string(),
        "SOLUSDT".to_string(),
        "ADAUSDT".to_string(),
    ]
}

pub fn config_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".tickertui.json")
}

pub fn load_config(path: &Path) -> AppConfig {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return AppConfig::default(),
    };

    serde_json::from_str::<AppConfig>(&contents)
        .map(|cfg| cfg.sanitized())
        .unwrap_or_default()
}

pub fn save_config(path: &Path, config: &AppConfig) -> std::io::Result<()> {
    let payload = serde_json::to_string_pretty(config)?;
    std::fs::write(path, payload)
}
