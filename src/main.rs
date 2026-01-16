mod data;
mod ui;

use data::{fetch_klines, stream_klines};
use ui::{Chart, LayoutManager};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    chart: Arc<Mutex<Chart>>,
    layout: LayoutManager,
    stream_restart_tx: tokio::sync::mpsc::Sender<(String, String)>,
    show_help: bool,
}

impl AppState {
    async fn switch_symbol(&mut self, symbol: String) {
        let chart_guard = self.chart.lock().await;
        let current_symbol = chart_guard.symbol.clone();
        let interval = chart_guard.interval.clone();
        drop(chart_guard);

        if current_symbol != symbol {
            if let Ok(initial_candles) = fetch_klines(&symbol, &interval, 200).await {
                let mut chart_guard = self.chart.lock().await;
                chart_guard.symbol = symbol.clone();
                chart_guard.candles.clear();
                chart_guard.offset = 0;
                chart_guard.update_candles(initial_candles);
                drop(chart_guard);
                let _ = self.stream_restart_tx.send((symbol, interval)).await;
            }
        }
    }

    async fn switch_interval(&mut self, interval: String) {
        let chart_guard = self.chart.lock().await;
        let symbol = chart_guard.symbol.clone();
        drop(chart_guard);

        if let Ok(initial_candles) = fetch_klines(&symbol, &interval, 200).await {
            let mut chart_guard = self.chart.lock().await;
            chart_guard.interval = interval.clone();
            chart_guard.candles.clear();
            chart_guard.update_candles(initial_candles);
            let _ = self.stream_restart_tx.send((symbol, interval)).await;
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let symbol = "BTCUSDT".to_string();
    let interval = "1m".to_string();

    let initial_candles = fetch_klines(&symbol, &interval, 200).await.unwrap_or_default();
    
    let chart = Arc::new(Mutex::new(Chart::new(symbol.clone(), interval.clone())));
    chart.lock().await.update_candles(initial_candles);

    let (restart_tx, mut restart_rx) = tokio::sync::mpsc::channel(10);
    let chart_clone = chart.clone();
    
    tokio::spawn(async move {
        let mut current_symbol = symbol.clone();
        let mut current_interval = interval.clone();
        let mut rx = stream_klines(&current_symbol, &current_interval).await;
        
        loop {
            tokio::select! {
                candle_opt = rx.recv() => {
                    if let Some(candle) = candle_opt {
                        let mut chart = chart_clone.lock().await;
                        if chart.symbol == current_symbol {
                            chart.add_candle(candle);
                        }
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        rx = stream_klines(&current_symbol, &current_interval).await;
                    }
                }
                restart_opt = restart_rx.recv() => {
                    if let Some((new_symbol, new_interval)) = restart_opt {
                        if new_symbol != current_symbol || new_interval != current_interval {
                            current_symbol = new_symbol;
                            current_interval = new_interval;
                            rx = stream_klines(&current_symbol, &current_interval).await;
                        }
                    }
                }
            }
        }
    });

    let mut app = AppState {
        chart,
        layout: LayoutManager::new(),
        stream_restart_tx: restart_tx,
        show_help: false,
    };

    loop {
        let chart_guard = app.chart.lock().await;
        terminal.draw(|f| {
            if app.show_help {
                render_help(f);
            } else {
                app.layout.render(f, &chart_guard, f.area());
            }
        })?;
        drop(chart_guard);

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.show_help {
                                app.show_help = false;
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('?') | KeyCode::Char('h') => {
                            app.show_help = !app.show_help;
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.chart.lock().await.zoom_in();
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            app.chart.lock().await.zoom_out();
                        }
                        KeyCode::Left => {
                            app.chart.lock().await.pan_left();
                        }
                        KeyCode::Right => {
                            app.chart.lock().await.pan_right();
                        }
                        KeyCode::Up => {
                            if app.layout.selected_symbol > 0 {
                                app.layout.selected_symbol -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.layout.selected_symbol < app.layout.watchlist.len() - 1 {
                                app.layout.selected_symbol += 1;
                            }
                        }
                        KeyCode::Enter => {
                            let new_symbol = app.layout.watchlist[app.layout.selected_symbol].clone();
                            app.switch_symbol(new_symbol.clone()).await;
                        }
                        KeyCode::Char('1') => {
                            app.switch_interval("1m".to_string()).await;
                        }
                        KeyCode::Char('5') => {
                            app.switch_interval("5m".to_string()).await;
                        }
                        KeyCode::Char('H') => {
                            app.switch_interval("1h".to_string()).await;
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            app.switch_interval("1d".to_string()).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_help(frame: &mut ratatui::Frame) {
    use ratatui::{
        layout::Alignment,
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph},
    };

    let help_text = vec![
        Line::from(Span::styled("TickerTUI - Help", Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation:", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  ↑/↓    "),
            Span::styled("Navigate watchlist", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Enter  "),
            Span::styled("Select symbol", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  ←/→    "),
            Span::styled("Pan chart left/right", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Zoom:", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  +/-    "),
            Span::styled("Zoom in/out", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Timeframes:", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  1      "),
            Span::styled("1 minute", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  5      "),
            Span::styled("5 minutes", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  H      "),
            Span::styled("1 hour", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  D      "),
            Span::styled("1 day", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Other:", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  ?/h    "),
            Span::styled("Toggle help", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  q/Esc  "),
            Span::styled("Quit", Style::default().fg(Color::White)),
        ]),
    ];

    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);
    
    frame.render_widget(paragraph, frame.area());
}