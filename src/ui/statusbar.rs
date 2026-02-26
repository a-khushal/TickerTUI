use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Live,
    Reconnecting,
    Degraded,
}

pub struct StatusBar {
    pub connection_mode: ConnectionMode,
    pub symbol: String,
    pub loading: bool,
    pub last_error: Option<String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            connection_mode: ConnectionMode::Live,
            symbol: String::new(),
            loading: false,
            last_error: None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (status_text, status_color) = match self.connection_mode {
            ConnectionMode::Live => ("LIVE", Color::Green),
            ConnectionMode::Reconnecting => ("RECONNECTING", Color::Yellow),
            ConnectionMode::Degraded => ("DEGRADED", Color::Red),
        };

        let mode_text = if self.loading { "LOADING" } else { "READY" };
        let mode_color = if self.loading {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let mut spans = vec![
            Span::styled(status_text, Style::default().fg(status_color)),
            Span::raw(" | "),
            Span::styled(mode_text, Style::default().fg(mode_color)),
            Span::raw(" | "),
            Span::styled(self.symbol.clone(), Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled("Q", Style::default().fg(Color::Yellow)),
            Span::raw(":Quit "),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw(":Help "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(":Nav "),
            Span::styled("←→", Style::default().fg(Color::Yellow)),
            Span::raw(":Pan "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(":TF "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(":Select "),
            Span::styled("+/-", Style::default().fg(Color::Yellow)),
            Span::raw(":Zoom "),
            Span::styled("S", Style::default().fg(Color::Yellow)),
            Span::raw(":SMA "),
            Span::styled("R", Style::default().fg(Color::Yellow)),
            Span::raw(":RSI"),
        ];

        if let Some(err) = &self.last_error {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(
                format!("ERR: {}", err),
                Style::default().fg(Color::Red),
            ));
        }

        let text = Line::from(spans);
        let para = Paragraph::new(text).block(Block::default());
        frame.render_widget(para, area);
    }
}
