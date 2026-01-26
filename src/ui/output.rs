use ansi_to_tui::IntoText;
use ratatui::{
    prelude::*,
    widgets::{Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use super::theme::Theme;
use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (title, lines, scroll_state) = match app.selected_process() {
        Some(process) => {
            let display_name = app.registry.display_name(&process.id);
            let status_symbol = Theme::status_symbol(process.status);
            let title = format!(" {} {} ", display_name, status_symbol);

            let lines: Vec<Line> = process
                .output
                .iter()
                .flat_map(|line| {
                    // Parse ANSI codes, fall back to plain text on error
                    match line.content.as_bytes().into_text() {
                        Ok(text) => text.lines,
                        Err(_) => {
                            // Fallback: apply basic styling based on stderr/error
                            let style = if line.is_error {
                                Style::default().fg(Theme::ERROR)
                            } else if line.is_stderr {
                                Style::default().fg(Theme::WARNING)
                            } else {
                                Style::default().fg(Theme::TEXT)
                            };
                            vec![Line::styled(line.content.clone(), style)]
                        }
                    }
                })
                .collect();

            let total_lines = lines.len();
            let scroll_offset = process.scroll_offset;

            (title, lines, Some((total_lines, scroll_offset)))
        }
        None => (" No Process Selected ".to_string(), vec![], None),
    };

    // Calculate visible area height (account for borders and padding)
    let inner_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();

    // Get scroll offset from selected process
    let scroll_offset = scroll_state.map(|(_, offset)| offset).unwrap_or(0);

    // Calculate scroll position (scroll from bottom by default)
    let scroll = if scroll_offset == 0 {
        // Auto-scroll to bottom
        total_lines.saturating_sub(inner_height) as u16
    } else {
        // Manual scroll position
        total_lines
            .saturating_sub(inner_height)
            .saturating_sub(scroll_offset) as u16
    };

    let block = Theme::focused_block(&title).padding(Padding::horizontal(1));

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
