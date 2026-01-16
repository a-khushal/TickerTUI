use crate::ui::Chart;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub struct LayoutManager {
    pub watchlist: Vec<String>,
    pub selected_symbol: usize,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            watchlist: vec![
                "BTCUSDT".to_string(),
                "ETHUSDT".to_string(),
                "BNBUSDT".to_string(),
                "SOLUSDT".to_string(),
                "ADAUSDT".to_string(),
            ],
            selected_symbol: 0,
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        chart: &Chart,
        area: Rect,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(40)])
            .split(area);

        self.render_watchlist(frame, chunks[0], chart);
        chart.render(frame, chunks[1]);
    }

    fn render_watchlist(&self, frame: &mut Frame, area: Rect, chart: &Chart) {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .split(area);

        let title_block = Block::default()
            .title("Watchlist")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(title_block, vertical[0]);

        let items: Vec<ListItem> = self
            .watchlist
            .iter()
            .enumerate()
            .map(|(idx, symbol)| {
                let is_selected = idx == self.selected_symbol;
                let is_current = symbol == &chart.symbol;
                let style = if is_current {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                let mut price_text = symbol.clone();
                if is_current && !chart.candles.is_empty() {
                    if let Some(last) = chart.candles.back() {
                        if let Ok(close) = last.close.parse::<f64>() {
                            price_text = format!("{} {:.2}", symbol, close);
                        }
                    }
                }

                ListItem::new(Line::from(Span::styled(price_text, style)))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::White));
        frame.render_widget(list, vertical[1]);
    }
}