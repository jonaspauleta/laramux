use ratatui::{prelude::*, widgets::Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = app.status_message.as_deref().unwrap_or(
        "↑↓:Navigate  q:Queue  v:Vite  s:Serve  b:Reverb  r:Restart All  c:Clear  Ctrl+C:Quit",
    );

    let paragraph =
        Paragraph::new(status_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(paragraph, area);
}
