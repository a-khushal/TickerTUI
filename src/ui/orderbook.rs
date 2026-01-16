use crate::data::OrderBook;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct OrderBookPanel {
    pub orderbook: Option<OrderBook>,
    pub max_entries: usize,
}

impl OrderBookPanel {
    pub fn new() -> Self {
        Self {
            orderbook: None,
            max_entries: 10,
        }
    }

    pub fn update(&mut self, book: OrderBook) {
        self.orderbook = Some(book);
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Order Book")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(book) = &self.orderbook {
            let asks_height = (inner.height.saturating_sub(2) / 2).min(self.max_entries as u16);
            let bids_height = inner.height.saturating_sub(2).saturating_sub(asks_height);

            let asks_area = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: asks_height,
            };

            let bids_area = Rect {
                x: inner.x,
                y: inner.y + asks_height,
                width: inner.width,
                height: bids_height,
            };

            self.render_side(&book.asks, asks_area, frame, true);
            self.render_side(&book.bids, bids_area, frame, false);
        } else {
            let text = Line::from(Span::styled(
                "Loading...",
                Style::default().fg(Color::Gray),
            ));
            let para = Paragraph::new(text).alignment(Alignment::Center);
            frame.render_widget(para, inner);
        }
    }

    fn render_side(
        &self,
        entries: &[crate::data::orderbook::OrderBookEntry],
        area: Rect,
        frame: &mut Frame,
        is_asks: bool,
    ) {
        let color = if is_asks { Color::Red } else { Color::Green };

        let header = Line::from(vec![
            Span::styled(
                format!("{:>12} {:>12}", "Price", "Size"),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]);
        let header_para = Paragraph::new(header);
        frame.render_widget(header_para, Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        });

        let display_entries: Vec<_> = entries
            .iter()
            .take(area.height.saturating_sub(1) as usize)
            .collect();

        for (idx, entry) in display_entries.iter().enumerate() {
            let y = area.y + 1 + idx as u16;
            if y < area.y + area.height {
                let price_str = format!("{:>12.2}", entry.price);
                let qty_str = format!("{:>12.4}", entry.quantity);
                let line = Line::from(vec![
                    Span::styled(price_str, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(qty_str, Style::default().fg(Color::White)),
                ]);
                let para = Paragraph::new(line);
                frame.render_widget(para, Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                });
            }
        }
    }
}