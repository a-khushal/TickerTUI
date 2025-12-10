mod data;
mod ui;

use data::fetch_klines;
use ui::chart::Chart;
use std::io;
use ratatui::DefaultTerminal;
use ratatui::backend::CrosstermBackend;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let candles = fetch_klines("BTCUSDT", "1h", 10000).await.unwrap();

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = DefaultTerminal::new(backend)?;
    terminal.clear()?;

    let mut chart = Chart {
        candles,
        exit: false,
    };

    chart.run(chart.candles.clone(), &mut terminal)?;
    Ok(())
}