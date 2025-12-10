use crate::data::Candle;
use ratatui::style::{Style, Modifier, Color};
use ratatui::{DefaultTerminal, layout::{Rect, Alignment}, text::{Line, Span, Text}, widgets::{Block, Paragraph, Borders}};
use std::io;

#[derive(Debug, Clone)]
pub struct Chart {
    pub candles: Vec<Candle>,
    pub exit: bool
}

impl Chart {
    pub fn run(&mut self, candles: Vec<Candle>, terminal: &mut DefaultTerminal) -> io::Result<()> {
        if !self.exit {
            terminal.draw(|frame| self.draw(candles, frame))?;
        }
        Ok(())
    }

    fn draw(&mut self, candles: Vec<Candle>, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let block = Block::default()
            .title("Candlestick Chart")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        
        let text = Text::from(Line::from(vec![
            Span::styled(
                format!("Num candles: {}", candles.len()),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]));
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center);

        let info_height = 3;
        let info_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: info_height,
        };
        frame.render_widget(paragraph, info_area);

        let chart_area = Rect {
            x: inner.x,
            y: inner.y + info_height,
            width: inner.width,
            height: inner.height.saturating_sub(info_height),
        };

        if candles.is_empty() || chart_area.height < 2 {
            return;
        }

        self.render_candlesticks(&candles, chart_area, frame);
    }

    fn render_candlesticks(&self, candles: &[Candle], area: Rect, frame: &mut ratatui::Frame) {
        if candles.is_empty() {
            return;
        }

        let parsed_candles: Vec<(f64, f64, f64, f64)> = candles
            .iter()
            .filter_map(|c| {
                let open = c.open.parse::<f64>().ok()?;
                let high = c.high.parse::<f64>().ok()?;
                let low = c.low.parse::<f64>().ok()?;
                let close = c.close.parse::<f64>().ok()?;
                Some((open, high, low, close))
            })
            .collect();

        if parsed_candles.is_empty() {
            return;
        }

        let max_visible = area.width as usize;
        let candles_to_show = if parsed_candles.len() > max_visible {
            &parsed_candles[parsed_candles.len() - max_visible..]
        } else {
            &parsed_candles[..]
        };

        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;

        for (_open, high, low, _close) in candles_to_show {
            min_price = min_price.min(*low);
            max_price = max_price.max(*high);
        }

        let price_range = max_price - min_price;
        let candle_width = ((area.width as usize) / candles_to_show.len().max(1)).max(1);
        let chart_height = area.height as usize;

        for (idx, (open, high, low, close)) in candles_to_show.iter().enumerate() {
            let x = area.x as usize + idx * candle_width + candle_width / 2;

            let high_y = ((max_price - high) / price_range * (chart_height - 1) as f64).ceil() as u16;
            let low_y = ((max_price - low) / price_range * (chart_height - 1) as f64).floor() as u16;
            let open_y = ((max_price - open) / price_range * (chart_height - 1) as f64).round() as u16;
            let close_y = ((max_price - close) / price_range * (chart_height - 1) as f64).round() as u16;

            let is_bullish = close >= open;
            let color = if is_bullish { Color::Green } else { Color::Red };
            let symbol = if is_bullish { '▥' } else { '▤' };

            for y in high_y..=low_y {
                if let Some(cell) = frame.buffer_mut().cell_mut((x as u16, area.y + y)) {
                    cell.set_char('│').set_fg(color);
                }
            }

            let body_top = open_y.min(close_y);
            let body_bottom = open_y.max(close_y);

            for y in body_top..=body_bottom {
                if let Some(cell) = frame.buffer_mut().cell_mut((x as u16, area.y + y)) {
                    cell.set_char(symbol).set_fg(color);
                }
            }
        }

        let label_count = (chart_height / 4).max(2);
        for i in 0..=label_count {
            let y = (i * (chart_height - 1) / label_count.max(1)) as u16;
            let price = max_price - (i as f64 / label_count.max(1) as f64) * price_range;
            let label = format!("{:.2}", price);
            
            if area.x > label.len() as u16 {
                for (j, ch) in label.chars().enumerate() {
                    if let Some(cell) = frame.buffer_mut().cell_mut((area.x - label.len() as u16 + j as u16, area.y + y)) {
                        cell.set_char(ch).set_fg(Color::Gray);
                    }
                }
            }
        }
    }
}
