# TickerTUI

A Bloomberg-style terminal UI for real-time cryptocurrency trading data. Displays live candlestick charts, order books, trade tape, and market data from Binance.

## Tech Stack

- **Rust** - Core language
- **ratatui** - TUI framework
- **crossterm** - Terminal I/O
- **tokio** - Async runtime
- **reqwest** - HTTP client (REST API)
- **tokio-tungstenite** - WebSocket client
- **serde/serde_json** - JSON parsing

## Project Structure

```
src/
├── main.rs              # Entry point, event loop, async tasks
├── data/
│   ├── fetch.rs         # REST API calls for historical candles
│   ├── stream.rs        # WebSocket stream for live candles
│   ├── orderbook.rs     # Order book data fetching and streaming
│   ├── trades.rs        # Trade tape data streaming
│   └── mod.rs           # Module exports
└── ui/
    ├── chart.rs         # Candlestick chart rendering
    ├── layout.rs        # Multi-panel layout manager
    ├── orderbook.rs     # Order book panel UI
    ├── tradetape.rs     # Trade tape panel UI
    ├── timeframe.rs     # Timeframe selector component
    ├── statusbar.rs     # Status bar component
    ├── indicators.rs    # Technical indicators (RSI, SMA)
    └── mod.rs           # Module exports
```

## Build & Run

```bash
cargo build --release
cargo run
```

**Controls:**
- `↑/↓` - Navigate watchlist
- `←/→` - Pan chart left/right
- `Tab/Shift+Tab` - Switch timeframes
- `Enter` - Select symbol
- `+/-` - Zoom in/out
- `S` - Toggle SMA20 overlay
- `R` - Toggle RSI14 overlay
- `?` - Help
- `q` - Quit

Settings are auto-saved to `.tickertui.json` (watchlist, selected symbol, timeframe, and zoom).
