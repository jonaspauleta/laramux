use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

use crate::app::{App, LogLevel, LogLine};
use crate::ui::theme::Theme;

/// Format a log line with timestamp, right-aligned level badge, and message
fn format_log_line(log_line: &LogLine) -> Line<'static> {
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
fn extract_timestamp_and_message(content: &str) -> (String, String) {
    if content.starts_with('[') {
        if let Some(end_bracket) = content.find(']') {
            let timestamp = &content[1..end_bracket];
            let rest = &content[end_bracket + 1..];
            if let Some(colon_pos) = rest.find(':') {
                let message = rest[colon_pos + 1..].trim();
                return (timestamp.to_string(), message.to_string());
            }
            return (timestamp.to_string(), rest.trim().to_string());
        }
    }
    ("                   ".to_string(), content.to_string())
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Get filtered logs
    let filtered_logs = app.filtered_logs();
    let lines: Vec<Line> = filtered_logs
        .iter()
        .map(|log| format_log_line(log))
        .collect();

    // Calculate visible area height
    let inner_height = area.height.saturating_sub(5) as usize; // Account for borders, header, footer
    let total_lines = lines.len();

    // Calculate scroll position
    let scroll = if app.logs_tab.scroll_offset == 0 {
        total_lines.saturating_sub(inner_height) as u16
    } else {
        total_lines
            .saturating_sub(inner_height)
            .saturating_sub(app.logs_tab.scroll_offset) as u16
    };

    // Build title with filter info
    let filter_info = app.logs_tab.filter_name();
    let file_info = app.logs_tab.file_name();
    let title = format!(
        " Logs [{}] File: {} | Level: {} ",
        filtered_logs.len(),
        file_info,
        filter_info
    );

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_FOCUSED))
        .padding(Padding::horizontal(1));

    // Render search bar if in input mode
    if app.logs_tab.input_mode {
        render_with_search(
            frame,
            area,
            app,
            lines,
            block,
            scroll,
            total_lines,
            inner_height,
        );
    } else {
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(paragraph, area);

        // Render scrollbar if content overflows
        if total_lines > inner_height {
            render_scrollbar(frame, area, total_lines, inner_height, scroll);
        }
    }

    // Render search hint or current search query
    let search_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: 1,
    };

    let search_line = if app.logs_tab.input_mode {
        Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Theme::ACCENT)),
            Span::styled(
                app.logs_tab.search_query.clone(),
                Style::default().fg(Theme::TEXT),
            ),
            Span::styled("█", Style::default().fg(Theme::ACCENT)),
        ])
    } else if !app.logs_tab.search_query.is_empty() {
        Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("\"{}\"", app.logs_tab.search_query),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled(" (press / to edit)", Style::default().fg(Theme::TEXT_MUTED)),
        ])
    } else {
        Line::from(vec![Span::styled(
            "Press / to search",
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    };

    frame.render_widget(Paragraph::new(search_line), search_area);

    // Footer with actions
    let footer_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(4),
        height: 1,
    };

    let footer = if app.logs_tab.input_mode {
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Confirm", Style::default().fg(Theme::TEXT_DIM)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[/] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Search", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[f] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Level", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[F] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("File", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[c] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Clear", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[g/G] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Top/Bottom", Style::default().fg(Theme::TEXT_DIM)),
        ]))
    };
    frame.render_widget(footer, footer_area);
}

#[allow(clippy::too_many_arguments)]
fn render_with_search(
    frame: &mut Frame,
    area: Rect,
    _app: &App,
    lines: Vec<Line>,
    block: Block,
    scroll: u16,
    total_lines: usize,
    inner_height: usize,
) {
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);

    if total_lines > inner_height {
        render_scrollbar(frame, area, total_lines, inner_height, scroll);
    }
}

fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total_lines: usize,
    inner_height: usize,
    scroll: u16,
) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_style(Style::default().fg(Theme::SCROLLBAR_THUMB))
        .track_style(Style::default().fg(Theme::SCROLLBAR_TRACK));

    let mut scrollbar_state =
        ScrollbarState::new(total_lines.saturating_sub(inner_height)).position(scroll as usize);

    let scrollbar_area = Rect {
        x: area.x + area.width.saturating_sub(2),
        y: area.y + 1,
        width: 1,
        height: area.height.saturating_sub(4),
    };

    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}
