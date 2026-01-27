use ansi_to_tui::IntoText;
use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::app::{App, ProcessesView};
use crate::process::types::ProcessStatus;
use crate::ui::theme::{symbols, Theme};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.processes_tab.view {
        ProcessesView::List => render_list_view(frame, area, app),
        ProcessesView::Output => render_output_view(frame, area, app),
    }
}

fn render_list_view(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .process_order
        .iter()
        .enumerate()
        .map(|(idx, id)| {
            let process = app.processes.get(id);
            let status = process.map(|p| p.status).unwrap_or(ProcessStatus::Stopped);
            let pid = process.and_then(|p| p.pid);

            let display_name = app.registry.display_name(id);

            let is_selected = idx == app.processes_tab.selected_index;

            // Get process stats if running
            let process_stats = pid.and_then(|p| app.system_stats.process_stats.get(&p));

            // Build the line with proper styling
            let mut spans = Vec::new();

            // Selection indicator
            if is_selected {
                spans.push(Span::styled(
                    format!("{} ", symbols::SELECTOR),
                    Style::default().fg(Theme::ACCENT),
                ));
            } else {
                spans.push(Span::raw("  "));
            }

            // Status symbol
            spans.push(Span::styled(
                format!("{} ", Theme::status_symbol(status)),
                Theme::status_style(status),
            ));

            // Process name
            spans.push(Span::styled(
                format!("{:<12}", display_name),
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));

            // CPU/RAM stats for running processes
            if status == ProcessStatus::Running {
                if let Some(stats) = process_stats {
                    let cpu_str = format!("{:>5.1}%", stats.cpu_usage);
                    let mem_mb = stats.memory_bytes as f64 / 1024.0 / 1024.0;
                    let mem_str = format!("{:>6.1}MB", mem_mb);

                    spans.push(Span::styled(cpu_str, Style::default().fg(Theme::TEXT_DIM)));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(mem_str, Style::default().fg(Theme::TEXT_DIM)));
                    spans.push(Span::raw("  "));
                } else {
                    spans.push(Span::styled(
                        "    -      -   ",
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    "               ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ));
            }

            // Action hints
            let action_hint = match status {
                ProcessStatus::Running => "[x]stop [r]restart",
                ProcessStatus::Stopped | ProcessStatus::Failed => "[s]start",
                ProcessStatus::Restarting => "please wait...",
            };
            spans.push(Span::styled(
                action_hint,
                Style::default().fg(Theme::TEXT_MUTED),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let block = Block::default()
        .title(" Processes ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_FOCUSED))
        .padding(Padding::horizontal(1));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Theme::SELECTION_BG));

    let mut state = ListState::default();
    state.select(Some(app.processes_tab.selected_index));

    frame.render_stateful_widget(list, area, &mut state);

    // Add footer with global actions
    let footer_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(4),
        height: 1,
    };

    if footer_area.y > area.y + 2 {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("[R] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Restart All", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("View Output", Style::default().fg(Theme::TEXT_DIM)),
        ]));
        frame.render_widget(footer, footer_area);
    }
}

fn render_output_view(frame: &mut Frame, area: Rect, app: &App) {
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

    // Calculate visible area height
    let inner_height = area.height.saturating_sub(4) as usize; // Account for borders and footer
    let total_lines = lines.len();

    let scroll_offset = scroll_state.map(|(_, offset)| offset).unwrap_or(0);

    // Calculate scroll position
    let scroll = if scroll_offset == 0 {
        total_lines.saturating_sub(inner_height) as u16
    } else {
        total_lines
            .saturating_sub(inner_height)
            .saturating_sub(scroll_offset) as u16
    };

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER_FOCUSED))
        .padding(Padding::horizontal(1));

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

        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(2),
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(4),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // Footer with actions
    let footer_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(4),
        height: 1,
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("Back", Style::default().fg(Theme::TEXT_DIM)),
        Span::raw("  "),
        Span::styled("[c] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("Clear", Style::default().fg(Theme::TEXT_DIM)),
        Span::raw("  "),
        Span::styled("[r] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("Restart", Style::default().fg(Theme::TEXT_DIM)),
        Span::raw("  "),
        Span::styled("[PageUp/Down] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("Scroll", Style::default().fg(Theme::TEXT_DIM)),
    ]));
    frame.render_widget(footer, footer_area);
}
