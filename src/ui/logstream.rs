use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::{App, LogLevel};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = app
        .log_lines
        .iter()
        .map(|log_line| {
            let style = match log_line.level {
                LogLevel::Emergency | LogLevel::Alert | LogLevel::Critical | LogLevel::Error => {
                    Style::default().fg(Color::Red)
                }
                LogLevel::Warning => Style::default().fg(Color::Yellow),
                LogLevel::Notice => Style::default().fg(Color::Cyan),
                LogLevel::Info => Style::default().fg(Color::Green),
                LogLevel::Debug => Style::default().fg(Color::DarkGray),
                LogLevel::Unknown => Style::default(),
            };
            Line::styled(&log_line.content, style)
        })
        .collect();

    // Calculate visible area height
    let inner_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();

    // Calculate scroll position (scroll from bottom by default)
    let scroll = if app.log_scroll_offset == 0 {
        total_lines.saturating_sub(inner_height) as u16
    } else {
        total_lines
            .saturating_sub(inner_height)
            .saturating_sub(app.log_scroll_offset) as u16
    };

    let title = format!(" Laravel Log ({} lines) ", app.log_lines.len());

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}
