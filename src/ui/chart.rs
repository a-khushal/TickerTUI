use crate::data::Candle;
use crate::ui::indicators::{calculate_rsi, calculate_sma};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Chart {
    pub candles: VecDeque<Candle>,
    pub symbol: String,
    pub interval: String,
    pub zoom: usize,
    pub offset: usize,
    pub max_candles: usize,
    pub show_sma: bool,
    pub show_rsi: bool,
}

impl Chart {
    pub fn new(symbol: String, interval: String) -> Self {
        Self {
            candles: VecDeque::new(),
            symbol,
            interval,
            zoom: 1,
            offset: 0,
            max_candles: 200,
            show_sma: false,
            show_rsi: false,
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        if let Some(last) = self.candles.back() {
            if candle.open_time == last.open_time {
                *self.candles.back_mut().unwrap() = candle;
                return;
            }
        }
        self.candles.push_back(candle);
        if self.candles.len() > self.max_candles {
            self.candles.pop_front();
        }
    }

    pub fn update_candles(&mut self, mut new_candles: Vec<Candle>) {
        new_candles.sort_by_key(|c| c.open_time);
        for candle in new_candles {
            self.add_candle(candle);
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 2).min(32);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 2).max(1);
    }

    pub fn pan_left(&mut self) {
        let max_offset = self.candles.len().saturating_sub(1);
        self.offset = (self.offset + 1).min(max_offset);
    }

    pub fn pan_right(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub fn toggle_sma(&mut self) {
        self.show_sma = !self.show_sma;
    }

    pub fn toggle_rsi(&mut self) {
        self.show_rsi = !self.show_rsi;
    }

    fn get_visible_count(&self, available_width: usize) -> usize {
        let min_candle_width = 2;
        let max_candles = available_width / min_candle_width;
        let zoom_based = (100 / self.zoom).max(10);
        zoom_based.min(max_candles).max(10)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
                Constraint::Length(3),
            ])
            .split(area);

        let title = format!("{} / {}", self.symbol, self.interval.to_uppercase());
        let title_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(title_block, vertical[0]);

        let chart_area = vertical[1];
        self.render_candlesticks(frame, chart_area);

        let volume_area = vertical[2];
        self.render_volume(frame, volume_area);

        let stats_area = vertical[3];
        self.render_stats(frame, stats_area);
    }

    fn clamp_x(inner: Rect, chart_width: usize, x_pos: usize) -> u16 {
        let max_x = inner.x.saturating_add(inner.width.saturating_sub(1));
        let x_offset = x_pos.min(chart_width.saturating_sub(1)) as u16;
        inner.x.saturating_add(x_offset).min(max_x)
    }

    fn map_price_to_y(inner: Rect, max_price: f64, price_range: f64, value: f64) -> Option<u16> {
        if inner.height == 0
            || !max_price.is_finite()
            || !price_range.is_finite()
            || price_range <= 0.0
        {
            return None;
        }
        if !value.is_finite() {
            return None;
        }

        let normalized = ((max_price - value) / price_range).clamp(0.0, 1.0);
        let y_offset = (normalized * f64::from(inner.height.saturating_sub(1))).round() as u16;
        let max_y = inner.y.saturating_add(inner.height.saturating_sub(1));
        Some(inner.y.saturating_add(y_offset).min(max_y))
    }

    fn render_candlesticks(&self, frame: &mut Frame, area: Rect) {
        if self.candles.is_empty() || area.width < 10 || area.height < 5 {
            return;
        }

        let chart_width = area.width.saturating_sub(13) as usize;
        let chart_height = area.height.saturating_sub(2);
        if chart_width == 0 || chart_height == 0 {
            return;
        }

        let visible_count = self.get_visible_count(chart_width);
        let start_idx = self
            .candles
            .len()
            .saturating_sub(visible_count + self.offset);
        let end_idx = self.candles.len().saturating_sub(self.offset);
        let visible_candles: Vec<&Candle> = self
            .candles
            .iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .collect();

        if visible_candles.is_empty() {
            return;
        }

        let parsed: Vec<(f64, f64, f64, f64, f64)> = visible_candles
            .iter()
            .filter_map(|c| {
                Some((
                    c.open.parse().ok()?,
                    c.high.parse().ok()?,
                    c.low.parse().ok()?,
                    c.close.parse().ok()?,
                    c.volume.parse().ok()?,
                ))
            })
            .collect();

        if parsed.is_empty() {
            return;
        }

        let (min_price, max_price) = parsed.iter().fold(
            (f64::MAX, f64::MIN),
            |(min, max), (_open, high, low, _close, _vol)| (min.min(*low), max.max(*high)),
        );

        let price_range = (max_price - min_price).max(0.0001);
        let candle_count = parsed.len();
        let spacing = chart_width / candle_count.max(1);

        let inner = Rect {
            x: area.x + 13,
            y: area.y + 1,
            width: chart_width as u16,
            height: chart_height,
        };

        for (idx, (open, high, low, close, _vol)) in parsed.iter().enumerate() {
            let x_pos = (idx * spacing) + (spacing / 2);
            let x = Self::clamp_x(inner, chart_width, x_pos);

            let Some(high_y) = Self::map_price_to_y(inner, max_price, price_range, *high) else {
                continue;
            };
            let Some(low_y) = Self::map_price_to_y(inner, max_price, price_range, *low) else {
                continue;
            };
            let Some(open_y) = Self::map_price_to_y(inner, max_price, price_range, *open) else {
                continue;
            };
            let Some(close_y) = Self::map_price_to_y(inner, max_price, price_range, *close) else {
                continue;
            };

            let is_bullish = close >= open;
            let color = if is_bullish { Color::Green } else { Color::Red };

            let body_top = open_y.min(close_y);
            let body_bottom = open_y.max(close_y);

            if high_y < low_y {
                for y in high_y..=low_y {
                    if y >= inner.y && y < inner.y + inner.height {
                        let cell = &mut frame.buffer_mut()[(x, y)];
                        cell.set_char('│').set_fg(color);
                    }
                }
            }

            if body_top <= body_bottom {
                for y in body_top..=body_bottom {
                    if y >= inner.y && y < inner.y + inner.height {
                        let cell = &mut frame.buffer_mut()[(x, y)];
                        cell.set_char('█').set_fg(color);
                    }
                }
            }
        }

        let full_candles: Vec<Candle> = self.candles.iter().cloned().collect();
        if self.show_sma {
            let sma_values = calculate_sma(&full_candles, 20);
            self.draw_sma_overlay(
                frame,
                inner,
                start_idx,
                candle_count,
                spacing,
                chart_width,
                max_price,
                price_range,
                &sma_values,
            );
        }

        if self.show_rsi {
            let rsi_values = calculate_rsi(&full_candles, 14);
            self.draw_rsi_overlay(
                frame,
                inner,
                start_idx,
                candle_count,
                spacing,
                chart_width,
                &rsi_values,
            );
        }

        let label_count = 5.min(chart_height as usize / 2);
        for i in 0..=label_count {
            let y = inner.y
                + ((i as u16) * (chart_height.saturating_sub(1)) / label_count.max(1) as u16);
            let price = max_price - (i as f64 / label_count.max(1) as f64) * price_range;
            let label = format!("{:>11.2}", price);

            for (j, ch) in label.chars().enumerate() {
                let x_pos = area.x + (j as u16);
                if x_pos < area.x + 13 && y < area.y + area.height {
                    let cell = &mut frame.buffer_mut()[(x_pos, y)];
                    cell.set_char(ch).set_fg(Color::Gray);
                }
            }
        }

        let latest = parsed.last().unwrap();
        let latest_price = latest.3;
        let change = latest_price - parsed.first().unwrap().0;
        let change_pct = (change / parsed.first().unwrap().0) * 100.0;

        let price_label = format!("{:.2}", latest_price);
        let change_label = format!("{:+.2} ({:+.2}%)", change, change_pct);
        let change_color = if change >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };

        let price_text = Line::from(vec![
            Span::styled("Price: ", Style::default().fg(Color::Gray)),
            Span::styled(
                price_label,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(change_label, Style::default().fg(change_color)),
        ]);

        let price_para = Paragraph::new(price_text);
        frame.render_widget(
            price_para,
            Rect {
                x: area.x + 13,
                y: area.y.saturating_add(area.height.saturating_sub(1)),
                width: area.width.saturating_sub(13),
                height: 1,
            },
        );
    }

    fn render_volume(&self, frame: &mut Frame, area: Rect) {
        if self.candles.is_empty() || area.width < 10 || area.height < 2 {
            return;
        }

        let chart_width = area.width.saturating_sub(13) as usize;
        if chart_width == 0 {
            return;
        }
        let visible_count = self.get_visible_count(chart_width);
        let start_idx = self
            .candles
            .len()
            .saturating_sub(visible_count + self.offset);
        let end_idx = self.candles.len().saturating_sub(self.offset);
        let visible_candles: Vec<&Candle> = self
            .candles
            .iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .collect();

        if visible_candles.is_empty() {
            return;
        }

        let volumes: Vec<f64> = visible_candles
            .iter()
            .filter_map(|c| c.volume.parse().ok())
            .collect();

        if volumes.is_empty() {
            return;
        }

        let max_volume = volumes.iter().fold(0.0f64, |a, &b| a.max(b));
        if max_volume == 0.0 {
            return;
        }

        let chart_height = area.height.saturating_sub(1);
        if chart_height == 0 {
            return;
        }
        let spacing = chart_width / volumes.len().max(1);

        let inner = Rect {
            x: area.x + 13,
            y: area.y,
            width: chart_width as u16,
            height: chart_height,
        };

        for (idx, volume) in volumes.iter().enumerate() {
            let x_pos = (idx * spacing) + (spacing / 2);
            let x = Self::clamp_x(inner, chart_width, x_pos);
            let height = ((volume / max_volume) * chart_height as f64) as u16;

            if height > 0 {
                let start_y = inner.y + inner.height - height;
                for y in start_y..inner.y + inner.height {
                    if y < area.y + area.height {
                        let cell = &mut frame.buffer_mut()[(x, y)];
                        cell.set_char('▊').set_fg(Color::Yellow);
                    }
                }
            }
        }

        let volume_label = format!("Vol: {:.2}", max_volume);
        let label_text = Line::from(Span::styled(volume_label, Style::default().fg(Color::Gray)));
        let label_para = Paragraph::new(label_text);
        frame.render_widget(
            label_para,
            Rect {
                x: area.x,
                y: area.y,
                width: 12,
                height: 1,
            },
        );
    }

    fn render_stats(&self, frame: &mut Frame, area: Rect) {
        if self.candles.is_empty() {
            return;
        }

        let latest = self.candles.back().unwrap();
        let open: f64 = latest.open.parse().unwrap_or(0.0);
        let high: f64 = latest.high.parse().unwrap_or(0.0);
        let low: f64 = latest.low.parse().unwrap_or(0.0);
        let close: f64 = latest.close.parse().unwrap_or(0.0);
        let volume: f64 = latest.volume.parse().unwrap_or(0.0);

        let change = close - open;
        let change_pct = if open > 0.0 {
            (change / open) * 100.0
        } else {
            0.0
        };
        let change_color = if change >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };

        let (latest_sma, latest_rsi) = self.latest_indicator_values();
        let sma_value_text = latest_sma
            .map(|value| format!("{:.2}", value))
            .unwrap_or_else(|| "n/a".to_string());
        let rsi_value_text = latest_rsi
            .map(|value| format!("{:.1}", value))
            .unwrap_or_else(|| "n/a".to_string());

        let stats_text = Line::from(vec![
            Span::styled("O: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.2}  ", open), Style::default().fg(Color::White)),
            Span::styled("H: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.2}  ", high), Style::default().fg(Color::Green)),
            Span::styled("L: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.2}  ", low), Style::default().fg(Color::Red)),
            Span::styled("C: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.2}  ", close), Style::default().fg(Color::White)),
            Span::styled("Vol: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}  ", volume),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("Chg: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:+.2}%", change_pct),
                Style::default()
                    .fg(change_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  SMA20: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if self.show_sma {
                    sma_value_text
                } else {
                    "off".to_string()
                },
                Style::default().fg(if self.show_sma {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled("  RSI14: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if self.show_rsi {
                    rsi_value_text
                } else {
                    "off".to_string()
                },
                Style::default().fg(if self.show_rsi {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
        ]);

        let stats_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        let stats_para = Paragraph::new(stats_text).block(stats_block);
        frame.render_widget(stats_para, area);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_sma_overlay(
        &self,
        frame: &mut Frame,
        inner: Rect,
        start_idx: usize,
        candle_count: usize,
        spacing: usize,
        chart_width: usize,
        max_price: f64,
        price_range: f64,
        sma_values: &[Option<f64>],
    ) {
        for idx in 0..candle_count {
            let global_idx = start_idx + idx;
            let Some(Some(sma_value)) = sma_values.get(global_idx) else {
                continue;
            };

            let x_pos = (idx * spacing) + (spacing / 2);
            let x = Self::clamp_x(inner, chart_width, x_pos);
            let Some(y) = Self::map_price_to_y(inner, max_price, price_range, *sma_value) else {
                continue;
            };

            if y >= inner.y && y < inner.y + inner.height {
                let cell = &mut frame.buffer_mut()[(x, y)];
                cell.set_char('•').set_fg(Color::Cyan);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_rsi_overlay(
        &self,
        frame: &mut Frame,
        inner: Rect,
        start_idx: usize,
        candle_count: usize,
        spacing: usize,
        chart_width: usize,
        rsi_values: &[Option<f64>],
    ) {
        let band_height = (inner.height / 4).max(2);
        let band_top = inner.y + inner.height.saturating_sub(band_height);

        for idx in 0..candle_count {
            let global_idx = start_idx + idx;
            let Some(Some(rsi_value)) = rsi_values.get(global_idx) else {
                continue;
            };

            let rsi_clamped = rsi_value.clamp(0.0, 100.0);
            let x_pos = (idx * spacing) + (spacing / 2);
            let x = Self::clamp_x(inner, chart_width, x_pos);
            let y_offset =
                ((100.0 - rsi_clamped) / 100.0 * (band_height.saturating_sub(1)) as f64) as u16;
            let max_band_y = band_top.saturating_add(band_height.saturating_sub(1));
            let y = band_top.saturating_add(y_offset).min(max_band_y);

            if y >= inner.y && y < inner.y + inner.height {
                let cell = &mut frame.buffer_mut()[(x, y)];
                cell.set_char('·').set_fg(Color::Magenta);
            }
        }
    }

    fn latest_indicator_values(&self) -> (Option<f64>, Option<f64>) {
        let all_candles: Vec<Candle> = self.candles.iter().cloned().collect();
        let sma = calculate_sma(&all_candles, 20);
        let rsi = calculate_rsi(&all_candles, 14);

        (sma.last().copied().flatten(), rsi.last().copied().flatten())
    }
}
