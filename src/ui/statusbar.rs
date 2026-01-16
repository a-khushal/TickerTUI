use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub struct StatusBar {
    pub connected: bool,
    pub symbol: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            connected: true,
            symbol: String::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let status_color = if self.connected {
            Color::Green
        } else {
            Color::Red
        };
        let status_text = if self.connected { "●" } else { "○" };

        let text = Line::from(vec![
            Span::styled(
                format!("{} ", status_text),
                Style::default().fg(status_color),
            ),
            Span::styled("CONNECTED", Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled("Q", Style::default().fg(Color::Yellow)),
            Span::raw(":Quit "),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw(":Help "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(":Nav "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(":Select "),
            Span::styled("+/-", Style::default().fg(Color::Yellow)),
            Span::raw(":Zoom"),
        ]);

        let para = Paragraph::new(text).block(Block::default());
        frame.render_widget(para, area);
    }
}