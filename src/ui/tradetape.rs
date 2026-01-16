use crate::data::Trade;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::collections::VecDeque;

pub struct TradeTape {
    pub trades: VecDeque<Trade>,
    pub max_trades: usize,
}

impl TradeTape {
    pub fn new() -> Self {
        Self {
            trades: VecDeque::with_capacity(100),
            max_trades: 50,
        }
    }

    pub fn add_trade(&mut self, trade: Trade) {
        self.trades.push_back(trade);
        if self.trades.len() > self.max_trades {
            self.trades.pop_front();
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Trade Tape")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let trades_vec: Vec<_> = self.trades.iter().rev().take(inner.height as usize).collect();
        let items: Vec<ListItem> = trades_vec
            .iter()
            .enumerate()
            .map(|(idx, trade)| {
                let (color, direction) = if idx < trades_vec.len() - 1 {
                    let prev_trade = trades_vec[idx + 1];
                    if trade.price > prev_trade.price {
                        (Color::Green, "↑")
                    } else if trade.price < prev_trade.price {
                        (Color::Red, "↓")
                    } else {
                        if !trade.is_buyer_maker {
                            (Color::Green, "↑")
                        } else {
                            (Color::Red, "↓")
                        }
                    }
                } else {
                    if !trade.is_buyer_maker {
                        (Color::Green, "↑")
                    } else {
                        (Color::Red, "↓")
                    }
                };
                let text = format!(
                    "{} {:>10.2} x {:>10.4}",
                    direction, trade.price, trade.quantity
                );
                ListItem::new(Line::from(Span::styled(text, Style::default().fg(color))))
            })
            .collect();

        let list = List::new(items).style(Style::default().fg(Color::White));
        frame.render_widget(list, inner);
    }
}