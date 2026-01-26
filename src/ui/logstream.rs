use ratatui::{
    prelude::*,
    widgets::{Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use super::theme::Theme;
use crate::app::{App, LogLevel};

/// Format a log line with timestamp, right-aligned level badge, and message
fn format_log_line(log_line: &crate::app::LogLine) -> Line<'static> {
    let level_color = match log_line.level {
        LogLevel::Emergency | LogLevel::Alert | LogLevel::Critical => Theme::LOG_CRITICAL,
        LogLevel::Error => Theme::LOG_ERROR,
        LogLevel::Warning => Theme::LOG_WARNING,
        LogLevel::Notice => Theme::LOG_NOTICE,
        LogLevel::Info => Theme::LOG_INFO,
        LogLevel::Debug => Theme::LOG_DEBUG,
        LogLevel::Unknown => Theme::TEXT_MUTED,
    };

    let level_text = match log_line.level {
        LogLevel::Emergency => "EMERGENCY",
        LogLevel::Alert => "   ALERT",
        LogLevel::Critical => "CRITICAL",
        LogLevel::Error => "   ERROR",
        LogLevel::Warning => "    WARN",
        LogLevel::Notice => "  NOTICE",
        LogLevel::Info => "    INFO",
        LogLevel::Debug => "   DEBUG",
        LogLevel::Unknown => " UNKNOWN",
    };

    // Extract timestamp from content if present, otherwise use placeholder
    let (timestamp, message) = extract_timestamp_and_message(&log_line.content);

    let spans = vec![
        Span::styled(timestamp, Style::default().fg(Theme::TEXT_MUTED)),
        Span::raw("  "),
        Span::styled(
            level_text,
            Style::default()
                .fg(level_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(message, Style::default().fg(Theme::TEXT)),
    ];

    Line::from(spans)
}

/// Extract timestamp and message from Laravel log line
/// Laravel format: [YYYY-MM-DD HH:MM:SS] environment.LEVEL: message
fn extract_timestamp_and_message(content: &str) -> (String, String) {
    // Try to match Laravel log format: [2024-01-15 10:30:45]
    if content.starts_with('[') {
        if let Some(end_bracket) = content.find(']') {
            let timestamp = &content[1..end_bracket];
            // Find the message after the level
            let rest = &content[end_bracket + 1..];
            if let Some(colon_pos) = rest.find(':') {
                let message = rest[colon_pos + 1..].trim();
                return (timestamp.to_string(), message.to_string());
            }
            return (timestamp.to_string(), rest.trim().to_string());
        }
    }

    // Fallback: no timestamp found
    ("                   ".to_string(), content.to_string())
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = app.log_lines.iter().map(format_log_line).collect();

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

    let title = format!(" Laravel Log [{}] ", app.log_lines.len());

    let block = Theme::default_block(&title).padding(Padding::horizontal(1));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);

    // Render scrollbar if content overflows
    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(Theme::SCROLLBAR_THUMB))
            .track_style(Style::default().fg(Theme::SCROLLBAR_TRACK));

        let mut scrollbar_state =
            ScrollbarState::new(total_lines.saturating_sub(inner_height)).position(scroll as usize);

        // Render scrollbar in the inner area (inside border)
        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(2),
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
