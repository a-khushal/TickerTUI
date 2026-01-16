use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timeframe {
    OneDay,
    SevenDays,
    OneMonth,
    ThreeMonths,
    OneYear,
    YearToDate,
}

impl Timeframe {
    pub fn all() -> Vec<Timeframe> {
        vec![
            Timeframe::OneDay,
            Timeframe::SevenDays,
            Timeframe::OneMonth,
            Timeframe::ThreeMonths,
            Timeframe::OneYear,
            Timeframe::YearToDate,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Timeframe::OneDay => "1D",
            Timeframe::SevenDays => "7D",
            Timeframe::OneMonth => "1M",
            Timeframe::ThreeMonths => "3M",
            Timeframe::OneYear => "1Y",
            Timeframe::YearToDate => "YTD",
        }
    }

    pub fn to_binance_interval(&self) -> &'static str {
        match self {
            Timeframe::OneDay => "5m",
            Timeframe::SevenDays => "15m",
            Timeframe::OneMonth => "1h",
            Timeframe::ThreeMonths => "4h",
            Timeframe::OneYear => "1d",
            Timeframe::YearToDate => "1d",
        }
    }

    pub fn limit(&self) -> u32 {
        match self {
            Timeframe::OneDay => 288,
            Timeframe::SevenDays => 672,
            Timeframe::OneMonth => 720,
            Timeframe::ThreeMonths => 540,
            Timeframe::OneYear => 365,
            Timeframe::YearToDate => 365,
        }
    }
}

pub struct TimeframeSelector {
    pub timeframes: Vec<Timeframe>,
    pub selected: usize,
}

impl TimeframeSelector {
    pub fn new() -> Self {
        Self {
            timeframes: Timeframe::all(),
            selected: 2,
        }
    }

    pub fn current(&self) -> Timeframe {
        self.timeframes[self.selected]
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % self.timeframes.len();
    }

    pub fn select_prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.timeframes.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Timeframe")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text: Vec<Span> = self
            .timeframes
            .iter()
            .enumerate()
            .flat_map(|(idx, tf)| {
                let is_selected = idx == self.selected;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(Color::White)
                };
                vec![
                    Span::styled(tf.label(), style),
                    if idx < self.timeframes.len() - 1 {
                        Span::raw(" ")
                    } else {
                        Span::raw("")
                    },
                ]
            })
            .collect();

        let line = Line::from(text);
        let para = Paragraph::new(line).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(para, inner);
    }
}