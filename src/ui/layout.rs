use crate::data::WatchPrice;
use crate::ui::{Chart, OrderBookPanel, StatusBar, Timeframe, TimeframeSelector, TradeTape};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::collections::HashMap;

pub struct LayoutManager {
    pub watchlist: Vec<String>,
    pub selected_symbol: usize,
    pub watch_prices: HashMap<String, WatchPrice>,
    pub orderbook: OrderBookPanel,
    pub tradetape: TradeTape,
    pub statusbar: StatusBar,
    pub timeframe: TimeframeSelector,
}

impl LayoutManager {
    pub fn new(watchlist: Vec<String>, selected_symbol: usize, timeframe: Timeframe) -> Self {
        let selected_symbol = selected_symbol.min(watchlist.len().saturating_sub(1));
        Self {
            watchlist,
            selected_symbol,
            watch_prices: HashMap::new(),
            orderbook: OrderBookPanel::new(),
            tradetape: TradeTape::new(),
            statusbar: StatusBar::new(),
            timeframe: TimeframeSelector::from_timeframe(timeframe),
        }
    }

    pub fn update_watch_price(&mut self, price: WatchPrice) {
        self.watch_prices.insert(price.symbol.clone(), price);
    }

    pub fn render(&mut self, frame: &mut Frame, chart: &Chart, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(area);

        self.timeframe.render(frame, main_chunks[0]);

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Min(40),
                Constraint::Length(30),
            ])
            .split(main_chunks[1]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(content_chunks[2]);

        self.render_watchlist(frame, content_chunks[0], chart);
        chart.render(frame, content_chunks[1]);
        self.orderbook.render(frame, right_chunks[0]);
        self.tradetape.render(frame, right_chunks[1]);
        self.statusbar.symbol = chart.symbol.clone();
        self.statusbar.render(frame, main_chunks[2]);
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
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                if let Some(price) = self.watch_prices.get(symbol) {
                    let change_color = if price.change_pct >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let line = Line::from(vec![
                        Span::styled(format!("{} {:.2} ", symbol, price.last_price), style),
                        Span::styled(
                            format!("{:+.2}%", price.change_pct),
                            Style::default().fg(change_color),
                        ),
                    ]);
                    return ListItem::new(line);
                }

                if is_current && !chart.candles.is_empty() {
                    if let Some(last) = chart.candles.back() {
                        if let Ok(close) = last.close.parse::<f64>() {
                            let line = Line::from(vec![
                                Span::styled(format!("{} {:.2}", symbol, close), style),
                                Span::styled(" ...", Style::default().fg(Color::Gray)),
                            ]);
                            return ListItem::new(line);
                        }
                    }
                }

                ListItem::new(Line::from(vec![
                    Span::styled(symbol.clone(), style),
                    Span::styled(" ...", Style::default().fg(Color::Gray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::White));
        frame.render_widget(list, vertical[1]);
    }
}
