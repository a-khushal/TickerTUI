mod config;
mod data;
mod ui;

use config::{config_path, load_config, save_config, AppConfig};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use data::orderbook::stream_orderbook;
use data::prices::stream_watchlist_prices;
use data::trades::stream_trades;
use data::{fetch_klines, stream_klines};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use ui::{Chart, ConnectionMode, LayoutManager};

const FETCH_TIMEOUT: Duration = Duration::from_secs(8);
const FETCH_RETRIES: usize = 2;
const FETCH_RETRY_DELAY: Duration = Duration::from_millis(500);
const HEALTH_TICK_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeedState {
    Live,
    Reconnecting,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HealthUpdate {
    kline: FeedState,
    orderbook: FeedState,
    trades: FeedState,
    last_error: Option<String>,
}

impl HealthUpdate {
    fn overall_mode(&self) -> ConnectionMode {
        if self.kline == FeedState::Live
            && self.orderbook == FeedState::Live
            && self.trades == FeedState::Live
        {
            ConnectionMode::Live
        } else if self.kline == FeedState::Degraded
            || self.orderbook == FeedState::Degraded
            || self.trades == FeedState::Degraded
        {
            ConnectionMode::Degraded
        } else {
            ConnectionMode::Reconnecting
        }
    }
}

struct FeedTracker {
    last_message: Option<Instant>,
    state: FeedState,
    reconnect_after: Duration,
    degrade_after: Duration,
}

impl FeedTracker {
    fn new(reconnect_after: Duration, degrade_after: Duration) -> Self {
        Self {
            last_message: None,
            state: FeedState::Reconnecting,
            reconnect_after,
            degrade_after,
        }
    }

    fn mark_live(&mut self, now: Instant) {
        self.last_message = Some(now);
        self.state = FeedState::Live;
    }

    fn mark_reconnecting(&mut self) {
        self.last_message = None;
        self.state = FeedState::Reconnecting;
    }

    fn refresh(&mut self, now: Instant) -> bool {
        let previous = self.state;
        if let Some(last) = self.last_message {
            let elapsed = now.saturating_duration_since(last);
            self.state = if elapsed >= self.degrade_after {
                FeedState::Degraded
            } else if elapsed >= self.reconnect_after {
                FeedState::Reconnecting
            } else {
                FeedState::Live
            };
        } else {
            self.state = FeedState::Reconnecting;
        }

        previous != self.state
    }
}

fn health_reason(update: &HealthUpdate) -> Option<String> {
    if update.overall_mode() == ConnectionMode::Live {
        return None;
    }

    let mut degraded = Vec::new();
    let mut reconnecting = Vec::new();

    for (name, state) in [
        ("kline", update.kline),
        ("orderbook", update.orderbook),
        ("trades", update.trades),
    ] {
        match state {
            FeedState::Degraded => degraded.push(name),
            FeedState::Reconnecting => reconnecting.push(name),
            FeedState::Live => {}
        }
    }

    if !degraded.is_empty() {
        Some(format!("degraded: {}", degraded.join(",")))
    } else if !reconnecting.is_empty() {
        Some(format!("reconnecting: {}", reconnecting.join(",")))
    } else {
        None
    }
}

fn push_health_update(
    tx: &tokio::sync::mpsc::UnboundedSender<HealthUpdate>,
    last_sent: &mut Option<HealthUpdate>,
    next: &HealthUpdate,
) {
    if last_sent.as_ref() != Some(next) {
        let _ = tx.send(next.clone());
        *last_sent = Some(next.clone());
    }
}

fn should_apply_fetch_result(pending_request_id: Option<u64>, incoming_request_id: u64) -> bool {
    pending_request_id == Some(incoming_request_id)
}

fn should_restart_stream(
    current_symbol: &str,
    current_interval: &str,
    new_symbol: &str,
    new_interval: &str,
) -> bool {
    current_symbol != new_symbol || current_interval != new_interval
}

struct FetchResult {
    request_id: u64,
    symbol: String,
    interval: String,
    candles: Result<Vec<data::Candle>, String>,
}

struct AppState {
    chart: Arc<Mutex<Chart>>,
    layout: Arc<Mutex<LayoutManager>>,
    config_path: PathBuf,
    stream_restart_tx: tokio::sync::mpsc::Sender<(String, String)>,
    fetch_result_tx: tokio::sync::mpsc::UnboundedSender<FetchResult>,
    fetch_task: Option<JoinHandle<()>>,
    next_request_id: u64,
    pending_request_id: Option<u64>,
    is_loading: bool,
    connection_mode: ConnectionMode,
    connection_error: Option<String>,
    show_help: bool,
}

async fn fetch_klines_with_retry(
    symbol: &str,
    interval: &str,
    limit: u32,
) -> Result<Vec<data::Candle>, String> {
    let mut last_error = String::from("unknown error");

    for attempt in 1..=FETCH_RETRIES {
        let fetch_result =
            tokio::time::timeout(FETCH_TIMEOUT, fetch_klines(symbol, interval, limit)).await;

        match fetch_result {
            Ok(Ok(candles)) => return Ok(candles),
            Ok(Err(err)) => {
                last_error = format!("attempt {attempt}/{FETCH_RETRIES} failed: {err}");
            }
            Err(_) => {
                last_error = format!(
                    "attempt {attempt}/{FETCH_RETRIES} timed out after {}s",
                    FETCH_TIMEOUT.as_secs()
                );
            }
        }

        if attempt < FETCH_RETRIES {
            tokio::time::sleep(FETCH_RETRY_DELAY).await;
        }
    }

    Err(last_error)
}

impl AppState {
    async fn snapshot_config(&self) -> AppConfig {
        let chart_guard = self.chart.lock().await;
        let layout_guard = self.layout.lock().await;
        AppConfig {
            watchlist: layout_guard.watchlist.clone(),
            selected_symbol: layout_guard.selected_symbol,
            symbol: chart_guard.symbol.clone(),
            timeframe: layout_guard.timeframe.current(),
            zoom: chart_guard.zoom,
        }
        .sanitized()
    }

    async fn persist_config(&self) {
        let config = self.snapshot_config().await;
        if let Err(err) = save_config(&self.config_path, &config) {
            eprintln!("Failed to save config: {}", err);
        }
    }

    fn queue_fetch(&mut self, symbol: String, interval: String, limit: u32) {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        let request_id = self.next_request_id;
        self.pending_request_id = Some(request_id);
        self.is_loading = true;

        if let Some(handle) = self.fetch_task.take() {
            handle.abort();
        }

        let tx = self.fetch_result_tx.clone();
        let handle = tokio::spawn(async move {
            let candles = fetch_klines_with_retry(&symbol, &interval, limit).await;

            let _ = tx.send(FetchResult {
                request_id,
                symbol,
                interval,
                candles,
            });
        });

        self.fetch_task = Some(handle);
    }

    async fn switch_symbol(&mut self, symbol: String) {
        let chart_guard = self.chart.lock().await;
        let current_symbol = chart_guard.symbol.clone();
        let interval = chart_guard.interval.clone();
        drop(chart_guard);

        let limit = {
            let layout_guard = self.layout.lock().await;
            layout_guard.timeframe.current().limit()
        };

        if current_symbol != symbol {
            self.queue_fetch(symbol, interval, limit);
        }
    }

    async fn switch_interval(&mut self, interval: String, limit: u32) {
        let chart_guard = self.chart.lock().await;
        let symbol = chart_guard.symbol.clone();
        let current_interval = chart_guard.interval.clone();
        drop(chart_guard);

        if current_interval != interval {
            self.queue_fetch(symbol, interval, limit);
        }
    }

    async fn switch_timeframe(&mut self, timeframe: ui::Timeframe) {
        let interval = timeframe.binance_interval().to_string();
        let limit = timeframe.limit();
        self.switch_interval(interval, limit).await;
    }

    async fn apply_fetch_result(&mut self, result: FetchResult) {
        if !should_apply_fetch_result(self.pending_request_id, result.request_id) {
            return;
        }

        self.pending_request_id = None;
        self.fetch_task = None;
        self.is_loading = false;

        match result.candles {
            Ok(initial_candles) => {
                let mut chart_guard = self.chart.lock().await;
                chart_guard.symbol = result.symbol.clone();
                chart_guard.interval = result.interval.clone();
                chart_guard.candles.clear();
                chart_guard.offset = 0;
                chart_guard.update_candles(initial_candles);
                drop(chart_guard);
                let _ = self
                    .stream_restart_tx
                    .send((result.symbol, result.interval))
                    .await;
                self.persist_config().await;
            }
            Err(err) => {
                self.connection_error = Some(format!("fetch: {}", err));
                eprintln!("Kline fetch failed: {}", err);
            }
        }
    }

    fn apply_health_update(&mut self, update: HealthUpdate) {
        self.connection_mode = update.overall_mode();
        self.connection_error = update.last_error;
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        eprintln!("TickerTUI needs an interactive terminal (TTY). Run it in a normal shell.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config_path = config_path();
    let initial_config = load_config(&config_path).sanitized();

    let symbol = initial_config.symbol.clone();
    let timeframe = initial_config.timeframe;
    let interval = timeframe.binance_interval().to_string();
    let limit = timeframe.limit();

    let initial_candles = fetch_klines(&symbol, &interval, limit)
        .await
        .unwrap_or_default();

    let chart = Arc::new(Mutex::new(Chart::new(symbol.clone(), interval.clone())));
    {
        let mut chart_guard = chart.lock().await;
        chart_guard.zoom = initial_config.zoom;
        chart_guard.update_candles(initial_candles);
    }

    let (restart_tx, mut restart_rx) = tokio::sync::mpsc::channel::<(String, String)>(10);
    let (fetch_result_tx, mut fetch_result_rx) = tokio::sync::mpsc::unbounded_channel();
    let (health_tx, mut health_rx) = tokio::sync::mpsc::unbounded_channel();
    let chart_clone = chart.clone();
    let layout_clone = Arc::new(Mutex::new(LayoutManager::new(
        initial_config.watchlist.clone(),
        initial_config.selected_symbol,
        timeframe,
    )));

    let layout_for_orderbook = layout_clone.clone();
    let layout_for_trades = layout_clone.clone();
    let layout_for_prices = layout_clone.clone();
    let watchlist_for_prices = initial_config.watchlist.clone();

    tokio::spawn(async move {
        let mut current_symbol = symbol.clone();
        let mut current_interval = interval.clone();
        let (mut rx, mut kline_handle) = stream_klines(&current_symbol, &current_interval);
        let (mut orderbook_rx, mut orderbook_handle) = stream_orderbook(&current_symbol);
        let (mut trades_rx, mut trades_handle) = stream_trades(&current_symbol);
        let (mut watch_prices_rx, mut watch_prices_handle) =
            stream_watchlist_prices(&watchlist_for_prices);

        let mut kline_tracker = FeedTracker::new(Duration::from_secs(12), Duration::from_secs(40));
        let mut orderbook_tracker =
            FeedTracker::new(Duration::from_secs(3), Duration::from_secs(10));
        let mut trades_tracker = FeedTracker::new(Duration::from_secs(3), Duration::from_secs(10));

        let mut health = HealthUpdate {
            kline: FeedState::Reconnecting,
            orderbook: FeedState::Reconnecting,
            trades: FeedState::Reconnecting,
            last_error: Some("reconnecting: kline,orderbook,trades".to_string()),
        };
        let mut last_sent = None;
        push_health_update(&health_tx, &mut last_sent, &health);

        let mut health_tick = tokio::time::interval(HEALTH_TICK_INTERVAL);
        health_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                candle_opt = rx.recv() => {
                    if let Some(candle) = candle_opt {
                        let mut chart = chart_clone.lock().await;
                        if chart.symbol == current_symbol {
                            chart.add_candle(candle);
                        }

                        kline_tracker.mark_live(Instant::now());
                        health.kline = kline_tracker.state;
                        health.last_error = health_reason(&health);
                        push_health_update(&health_tx, &mut last_sent, &health);
                    } else {
                        kline_handle.abort();
                        let (new_rx, new_handle) = stream_klines(&current_symbol, &current_interval);
                        rx = new_rx;
                        kline_handle = new_handle;

                        kline_tracker.mark_reconnecting();
                        health.kline = kline_tracker.state;
                        health.last_error = Some("kline stream dropped; reconnecting".to_string());
                        push_health_update(&health_tx, &mut last_sent, &health);
                    }
                }
                orderbook_opt = orderbook_rx.recv() => {
                    if let Some(book) = orderbook_opt {
                        let mut layout = layout_for_orderbook.lock().await;
                        layout.orderbook.update(book);

                        orderbook_tracker.mark_live(Instant::now());
                        health.orderbook = orderbook_tracker.state;
                        health.last_error = health_reason(&health);
                        push_health_update(&health_tx, &mut last_sent, &health);
                    } else {
                        orderbook_handle.abort();
                        let (new_rx, new_handle) = stream_orderbook(&current_symbol);
                        orderbook_rx = new_rx;
                        orderbook_handle = new_handle;

                        orderbook_tracker.mark_reconnecting();
                        health.orderbook = orderbook_tracker.state;
                        health.last_error = Some("orderbook stream dropped; reconnecting".to_string());
                        push_health_update(&health_tx, &mut last_sent, &health);
                    }
                }
                trade_opt = trades_rx.recv() => {
                    if let Some(trade) = trade_opt {
                        let mut layout = layout_for_trades.lock().await;
                        layout.tradetape.add_trade(trade);

                        trades_tracker.mark_live(Instant::now());
                        health.trades = trades_tracker.state;
                        health.last_error = health_reason(&health);
                        push_health_update(&health_tx, &mut last_sent, &health);
                    } else {
                        trades_handle.abort();
                        let (new_rx, new_handle) = stream_trades(&current_symbol);
                        trades_rx = new_rx;
                        trades_handle = new_handle;

                        trades_tracker.mark_reconnecting();
                        health.trades = trades_tracker.state;
                        health.last_error = Some("trade stream dropped; reconnecting".to_string());
                        push_health_update(&health_tx, &mut last_sent, &health);
                    }
                }
                watch_price_opt = watch_prices_rx.recv() => {
                    if let Some(watch_price) = watch_price_opt {
                        let mut layout = layout_for_prices.lock().await;
                        layout.update_watch_price(watch_price);
                    } else {
                        watch_prices_handle.abort();
                        let (new_rx, new_handle) = stream_watchlist_prices(&watchlist_for_prices);
                        watch_prices_rx = new_rx;
                        watch_prices_handle = new_handle;
                    }
                }
                restart_opt = restart_rx.recv() => {
                    if let Some((new_symbol, new_interval)) = restart_opt {
                        if should_restart_stream(
                            &current_symbol,
                            &current_interval,
                            &new_symbol,
                            &new_interval,
                        ) {
                            current_symbol = new_symbol;
                            current_interval = new_interval;

                            kline_handle.abort();
                            orderbook_handle.abort();
                            trades_handle.abort();

                            let (new_rx, new_kline_handle) = stream_klines(&current_symbol, &current_interval);
                            let (new_orderbook_rx, new_orderbook_handle) = stream_orderbook(&current_symbol);
                            let (new_trades_rx, new_trades_handle) = stream_trades(&current_symbol);

                            rx = new_rx;
                            orderbook_rx = new_orderbook_rx;
                            trades_rx = new_trades_rx;
                            kline_handle = new_kline_handle;
                            orderbook_handle = new_orderbook_handle;
                            trades_handle = new_trades_handle;

                            kline_tracker.mark_reconnecting();
                            orderbook_tracker.mark_reconnecting();
                            trades_tracker.mark_reconnecting();
                            health.kline = kline_tracker.state;
                            health.orderbook = orderbook_tracker.state;
                            health.trades = trades_tracker.state;
                            health.last_error = Some("reconnecting: kline,orderbook,trades".to_string());
                            push_health_update(&health_tx, &mut last_sent, &health);
                        }
                    }
                }
                _ = health_tick.tick() => {
                    let now = Instant::now();
                    let mut changed = false;
                    changed |= kline_tracker.refresh(now);
                    changed |= orderbook_tracker.refresh(now);
                    changed |= trades_tracker.refresh(now);

                    if changed {
                        health.kline = kline_tracker.state;
                        health.orderbook = orderbook_tracker.state;
                        health.trades = trades_tracker.state;
                        health.last_error = health_reason(&health);
                        push_health_update(&health_tx, &mut last_sent, &health);
                    }
                }
            }
        }
    });

    let mut app = AppState {
        chart,
        layout: layout_clone,
        config_path,
        stream_restart_tx: restart_tx,
        fetch_result_tx,
        fetch_task: None,
        next_request_id: 0,
        pending_request_id: None,
        is_loading: false,
        connection_mode: ConnectionMode::Reconnecting,
        connection_error: Some("reconnecting: kline,orderbook,trades".to_string()),
        show_help: false,
    };

    loop {
        while let Ok(result) = fetch_result_rx.try_recv() {
            app.apply_fetch_result(result).await;
        }

        while let Ok(update) = health_rx.try_recv() {
            app.apply_health_update(update);
        }

        let chart_guard = app.chart.lock().await;
        let mut layout_guard = app.layout.lock().await;
        layout_guard.statusbar.loading = app.is_loading;
        layout_guard.statusbar.connection_mode = app.connection_mode;
        layout_guard.statusbar.last_error = app.connection_error.clone();
        terminal.draw(|f| {
            if app.show_help {
                render_help(f);
            } else {
                layout_guard.render(f, &chart_guard, f.area());
            }
        })?;
        drop(chart_guard);
        drop(layout_guard);

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.show_help {
                                app.show_help = false;
                            } else {
                                app.persist_config().await;
                                break;
                            }
                        }
                        KeyCode::Char('?') | KeyCode::Char('h') => {
                            app.show_help = !app.show_help;
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.chart.lock().await.zoom_in();
                            app.persist_config().await;
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            app.chart.lock().await.zoom_out();
                            app.persist_config().await;
                        }
                        KeyCode::Tab => {
                            let mut layout = app.layout.lock().await;
                            layout.timeframe.select_next();
                            let tf = layout.timeframe.current();
                            drop(layout);
                            app.switch_timeframe(tf).await;
                            app.persist_config().await;
                        }
                        KeyCode::BackTab => {
                            let mut layout = app.layout.lock().await;
                            layout.timeframe.select_prev();
                            let tf = layout.timeframe.current();
                            drop(layout);
                            app.switch_timeframe(tf).await;
                            app.persist_config().await;
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.chart.lock().await.toggle_sma();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            app.chart.lock().await.toggle_rsi();
                        }
                        KeyCode::Left => {
                            app.chart.lock().await.pan_left();
                        }
                        KeyCode::Right => {
                            app.chart.lock().await.pan_right();
                        }
                        KeyCode::Up => {
                            let mut layout = app.layout.lock().await;
                            if layout.selected_symbol > 0 {
                                layout.selected_symbol -= 1;
                            }
                            drop(layout);
                            app.persist_config().await;
                        }
                        KeyCode::Down => {
                            let mut layout = app.layout.lock().await;
                            if layout.selected_symbol < layout.watchlist.len().saturating_sub(1) {
                                layout.selected_symbol += 1;
                            }
                            drop(layout);
                            app.persist_config().await;
                        }
                        KeyCode::Enter => {
                            let layout = app.layout.lock().await;
                            let new_symbol = layout.watchlist[layout.selected_symbol].clone();
                            drop(layout);
                            app.switch_symbol(new_symbol.clone()).await;
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
        Line::from(Span::styled(
            "TickerTUI - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
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
        Line::from(vec![Span::styled(
            "Zoom:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  +/-    "),
            Span::styled("Zoom in/out", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Timeframes:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  Tab    "),
            Span::styled("Next timeframe", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Shift+Tab"),
            Span::styled("Previous timeframe", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Indicators:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  S      "),
            Span::styled("Toggle SMA20 overlay", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  R      "),
            Span::styled("Toggle RSI14 overlay", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )]),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_stream_only_when_target_changes() {
        assert!(!should_restart_stream("BTCUSDT", "1h", "BTCUSDT", "1h"));
        assert!(should_restart_stream("BTCUSDT", "1h", "ETHUSDT", "1h"));
        assert!(should_restart_stream("BTCUSDT", "1h", "BTCUSDT", "4h"));
    }

    #[test]
    fn stale_fetch_results_are_rejected() {
        assert!(should_apply_fetch_result(Some(7), 7));
        assert!(!should_apply_fetch_result(Some(7), 6));
        assert!(!should_apply_fetch_result(None, 1));
    }

    #[test]
    fn feed_tracker_transitions_live_reconnecting_degraded() {
        let now = Instant::now();
        let mut tracker = FeedTracker::new(Duration::from_secs(2), Duration::from_secs(5));

        tracker.mark_live(now);
        assert_eq!(tracker.state, FeedState::Live);

        tracker.refresh(now + Duration::from_secs(3));
        assert_eq!(tracker.state, FeedState::Reconnecting);

        tracker.refresh(now + Duration::from_secs(6));
        assert_eq!(tracker.state, FeedState::Degraded);
    }
}
