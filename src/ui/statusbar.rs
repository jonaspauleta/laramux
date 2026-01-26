use ratatui::{prelude::*, widgets::Paragraph};

use super::theme::Theme;
use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let default_help = "↑↓:Navigate  q:Queue  v:Vite  s:Serve  b:Reverb  r:Restart All  c:Clear  Ctrl+C:Quit";

    let line = match &app.status_message {
        Some(msg) => Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Theme::ACCENT)),
            Span::styled(msg.as_str(), Style::default().fg(Theme::TEXT)),
        ]),
        None => Line::from(vec![
            Span::styled(" ◉ ", Style::default().fg(Theme::ACCENT)),
            Span::styled(default_help, Style::default().fg(Theme::TEXT_DIM)),
        ]),
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(Theme::STATUS_BAR_BG));

    frame.render_widget(paragraph, area);
}
