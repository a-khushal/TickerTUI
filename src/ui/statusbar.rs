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
    pub loading: bool,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            connected: true,
            symbol: String::new(),
            loading: false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let status_color = if self.connected {
            Color::Green
        } else {
            Color::Red
        };
        let status_text = if self.connected { "●" } else { "○" };
        let mode_text = if self.loading { "LOADING" } else { "LIVE" };
        let mode_color = if self.loading {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let text = Line::from(vec![
            Span::styled(
                format!("{} ", status_text),
                Style::default().fg(status_color),
            ),
            Span::styled("CONNECTED", Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(mode_text, Style::default().fg(mode_color)),
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
            Span::raw(":Zoom"),
        ]);

        let para = Paragraph::new(text).block(Block::default());
        frame.render_widget(para, area);
    }
}
