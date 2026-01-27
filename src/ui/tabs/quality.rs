use ansi_to_tui::IntoText;
use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Tabs,
    },
};

use crate::app::{App, QualityCategory};
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Category tabs (Quality Tools / Testing)
            Constraint::Length(14), // Tool list + details (side by side)
            Constraint::Length(3),  // Input area
            Constraint::Min(5),     // Output area
        ])
        .split(area);

    render_category_tabs(frame, chunks[0], app);
    render_split_view(frame, chunks[1], app);
    render_input_area(frame, chunks[2], app);
    render_output_area(frame, chunks[3], app);
}

fn render_category_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = QualityCategory::all()
        .iter()
        .map(|cat| Line::from(cat.name()))
        .collect();

    let selected_index = QualityCategory::all()
        .iter()
        .position(|c| *c == app.quality_tab.selected_category)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title(" Category ")
                .title_style(Theme::title_style())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::BORDER)),
        )
        .select(selected_index)
        .style(Style::default().fg(Theme::TEXT_DIM))
        .highlight_style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", Style::default().fg(Theme::TEXT_MUTED)));

    frame.render_widget(tabs, area);
}

fn render_split_view(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_tool_list(frame, chunks[0], app);
    render_tool_details(frame, chunks[1], app);
}

fn render_tool_list(frame: &mut Frame, area: Rect, app: &App) {
    let tools = app.quality_tab.current_tools();
    let available_width = area.width.saturating_sub(6) as usize;
    let max_name_width = available_width.saturating_sub(2);

    let items: Vec<ListItem> = tools
        .iter()
        .enumerate()
        .map(|(idx, tool)| {
            let is_selected = idx == app.quality_tab.selected_tool;

            let mut spans = Vec::new();

            if is_selected {
                spans.push(Span::styled("▶ ", Style::default().fg(Theme::ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }

            let display_name = if tool.display_name.len() > max_name_width {
                format!(
                    "{}…",
                    &tool.display_name[..max_name_width.saturating_sub(1)]
                )
            } else {
                format!("{:<width$}", tool.display_name, width = max_name_width)
            };

            spans.push(Span::styled(
                display_name,
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        " {} ({}) ",
        app.quality_tab.selected_category.name(),
        tools.len()
    );

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::horizontal(1));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Theme::SELECTION_BG));

    let mut state = ListState::default();
    state.select(Some(app.quality_tab.selected_tool));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tool_details(frame: &mut Frame, area: Rect, app: &App) {
    let scroll_offset = app.quality_tab.details_scroll_offset;

    let block = Block::default()
        .title(" Tool Details ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(tool) = app.quality_tab.selected_tool_item() {
        let mut lines = Vec::new();

        // Tool name
        lines.push(Line::from(vec![
            Span::styled("Tool: ", Style::default().fg(Theme::ACCENT)),
            Span::styled(
                tool.display_name.clone(),
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Command
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Theme::ACCENT)),
            Span::styled(tool.command.clone(), Style::default().fg(Theme::TEXT)),
        ]));

        // Default arguments
        if !tool.args.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Default Arguments:".to_string(),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
            for arg in &tool.args {
                lines.push(Line::styled(
                    format!("  {}", arg),
                    Style::default().fg(Theme::TEXT),
                ));
            }
        }

        // Usage hint
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Usage:".to_string(),
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            "  Press [Enter] to run with default arguments.".to_string(),
            Style::default().fg(Theme::TEXT_MUTED),
        ));
        lines.push(Line::styled(
            "  Press [i] to add custom arguments.".to_string(),
            Style::default().fg(Theme::TEXT_MUTED),
        ));

        let total_lines = lines.len();
        let inner_height = inner.height.saturating_sub(1) as usize; // Reserve 1 line for scroll hint
        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll = scroll_offset.min(max_scroll) as u16;

        let paragraph = Paragraph::new(lines).scroll((scroll, 0));
        frame.render_widget(paragraph, inner);

        // Render scrollbar if content overflows
        if total_lines > inner_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(Theme::SCROLLBAR_THUMB))
                .track_style(Style::default().fg(Theme::SCROLLBAR_TRACK));

            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll as usize);

            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(2),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(3),
            };

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

            // Render scroll hint at bottom of details pane
            let hint_area = Rect {
                x: inner.x,
                y: area.y + area.height.saturating_sub(2),
                width: inner.width,
                height: 1,
            };
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("[^j/k] ", Style::default().fg(Theme::ACCENT)),
                Span::styled("Scroll", Style::default().fg(Theme::TEXT_MUTED)),
            ]));
            frame.render_widget(hint, hint_area);
        }
    } else {
        let msg = Paragraph::new("Select a tool to see details")
            .style(Style::default().fg(Theme::TEXT_MUTED));
        frame.render_widget(msg, inner);
    }
}

fn render_input_area(frame: &mut Frame, area: Rect, app: &App) {
    let is_input_mode = app.quality_tab.input_mode;
    let is_running = app.quality_tab.running_command.is_some();
    let is_empty = app.quality_tab.input_buffer.is_empty();

    let border_color = if is_input_mode {
        Theme::BORDER_FOCUSED
    } else {
        Theme::BORDER
    };

    let title = if is_running {
        " Input (press Enter to send) "
    } else {
        " Arguments (optional) "
    };

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));

    let hint = if is_running {
        "Type your response and press Enter".to_string()
    } else {
        "Press [i] to enter arguments (optional)".to_string()
    };

    let content = if is_input_mode {
        Line::from(vec![
            Span::styled(
                app.quality_tab.input_buffer.clone(),
                Style::default().fg(Theme::TEXT),
            ),
            Span::styled("█", Style::default().fg(Theme::ACCENT)),
        ])
    } else if is_empty {
        Line::from(vec![Span::styled(
            hint,
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    } else {
        Line::from(vec![Span::styled(
            app.quality_tab.input_buffer.clone(),
            Style::default().fg(Theme::TEXT_DIM),
        )])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_output_area(frame: &mut Frame, area: Rect, app: &App) {
    let title = if let Some(ref cmd) = app.quality_tab.running_command {
        format!(" Output - {} (running) ", cmd)
    } else {
        " Output ".to_string()
    };

    let lines: Vec<Line> = app
        .quality_tab
        .command_output
        .iter()
        .flat_map(|line| match line.content.as_bytes().into_text() {
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
        })
        .collect();

    let inner_height = area.height.saturating_sub(4) as usize;
    let total_lines = lines.len();

    let scroll_offset = app.quality_tab.output_scroll_offset;
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
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::horizontal(1));

    let paragraph = Paragraph::new(lines).block(block).scroll((scroll, 0));

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

    let is_running = app.quality_tab.running_command.is_some();

    let footer = if is_running {
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[c] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Clear", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[PgUp/Dn] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Scroll", Style::default().fg(Theme::TEXT_DIM)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Run", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[c] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Clear", Style::default().fg(Theme::TEXT_DIM)),
            Span::raw("  "),
            Span::styled("[PgUp/Dn] ", Style::default().fg(Theme::ACCENT)),
            Span::styled("Scroll", Style::default().fg(Theme::TEXT_DIM)),
        ]))
    };
    frame.render_widget(footer, footer_area);
}
