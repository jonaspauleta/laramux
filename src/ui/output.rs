use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (title, lines) = match app.selected_process() {
        Some(process) => {
            let display_name = app.registry.display_name(&process.id);
            let title = format!(" {} Output {} ", display_name, process.status.indicator());

            let lines: Vec<Line> = process
                .output
                .iter()
                .map(|line| {
                    let style = if line.is_error {
                        Style::default().fg(Color::Red)
                    } else if line.is_stderr {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    Line::styled(&line.content, style)
                })
                .collect();

            (title, lines)
        }
        None => (" No Process Selected ".to_string(), vec![]),
    };

    // Calculate visible area height
    let inner_height = area.height.saturating_sub(2) as usize; // Account for borders
    let total_lines = lines.len();

    // Get scroll offset from selected process
    let scroll_offset = app.selected_process().map(|p| p.scroll_offset).unwrap_or(0);

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

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}
