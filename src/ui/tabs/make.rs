use ansi_to_tui::IntoText;
use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::app::App;
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14), // Command list + details (side by side)
            Constraint::Length(3),  // Input area
            Constraint::Min(5),     // Output area
        ])
        .split(area);

    render_split_view(frame, chunks[0], app);
    render_input_area(frame, chunks[1], app);
    render_output_area(frame, chunks[2], app);
}

fn render_split_view(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_command_list(frame, chunks[0], app);
    render_command_details(frame, chunks[1], app);
}

fn render_command_list(frame: &mut Frame, area: Rect, app: &App) {
    let favorites = app
        .config
        .as_ref()
        .map(|c| c.make_favorites())
        .unwrap_or(&[]);
    let commands = app.make_tab.current_command_display(favorites, app.is_sail);

    let title = format!(" Make Commands ({}) ", commands.len());

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area for search bar and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let search_area = chunks[0];
    let list_area = chunks[1];

    // Render search bar
    let search_line = if app.make_tab.search_mode {
        Line::from(vec![
            Span::styled("/", Style::default().fg(Theme::ACCENT)),
            Span::styled(
                app.make_tab.search_query.clone(),
                Style::default().fg(Theme::TEXT),
            ),
            Span::styled("█", Style::default().fg(Theme::ACCENT)),
        ])
    } else if !app.make_tab.search_query.is_empty() {
        Line::from(vec![
            Span::styled("/", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                app.make_tab.search_query.clone(),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ])
    } else {
        Line::from(vec![Span::styled(
            "Press / to search, f to favorite",
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    };
    frame.render_widget(Paragraph::new(search_line), search_area);

    let available_width = list_area.width.saturating_sub(4) as usize;
    let max_name_width = available_width.saturating_sub(4); // Room for star

    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .map(|(idx, (name, _full_cmd, is_favorite))| {
            let is_selected = idx == app.make_tab.selected_command;

            let mut spans = Vec::new();

            if is_selected {
                spans.push(Span::styled("▶ ", Style::default().fg(Theme::ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }

            // Show star for favorites
            if *is_favorite {
                spans.push(Span::styled("★ ", Style::default().fg(Theme::WARNING)));
            } else {
                spans.push(Span::raw("  "));
            }

            let display_name = if name.len() > max_name_width {
                format!("{}…", &name[..max_name_width.saturating_sub(1)])
            } else {
                format!("{:<width$}", name, width = max_name_width)
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

    let list = List::new(items).highlight_style(Style::default().bg(Theme::SELECTION_BG));

    let mut state = ListState::default();
    state.select(Some(app.make_tab.selected_command));

    frame.render_stateful_widget(list, list_area, &mut state);
}

fn render_command_details(frame: &mut Frame, area: Rect, app: &App) {
    let scroll_offset = app.make_tab.details_scroll_offset;
    let favorites = app
        .config
        .as_ref()
        .map(|c| c.make_favorites())
        .unwrap_or(&[]);

    let block = Block::default()
        .title(" Command Details ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max_width = inner.width.saturating_sub(2) as usize;

    if let Some(cmd) = app.make_tab.selected_make_command(favorites) {
        let mut lines = Vec::new();

        // Command name
        lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Theme::ACCENT)),
            Span::styled(
                format!("php artisan {}", cmd.name),
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Description
        if !cmd.description.is_empty() {
            lines.push(Line::from(""));
            if cmd.description.len() > max_width && max_width > 0 {
                for chunk in cmd.description.as_bytes().chunks(max_width) {
                    if let Ok(s) = std::str::from_utf8(chunk) {
                        lines.push(Line::styled(
                            s.to_string(),
                            Style::default().fg(Theme::TEXT_DIM),
                        ));
                    }
                }
            } else {
                lines.push(Line::styled(
                    cmd.description.clone(),
                    Style::default().fg(Theme::TEXT_DIM),
                ));
            }
        }

        // Arguments
        if !cmd.arguments.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Arguments:".to_string(),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
            for (name, is_required, desc) in &cmd.arguments {
                let required_marker = if *is_required { " (required)" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}", name), Style::default().fg(Theme::TEXT)),
                    Span::styled(
                        required_marker.to_string(),
                        Style::default().fg(Theme::WARNING),
                    ),
                ]));
                if !desc.is_empty() {
                    let available = max_width.saturating_sub(4);
                    let truncated = if desc.len() > available && available > 3 {
                        format!("{}...", &desc[..available.saturating_sub(3)])
                    } else {
                        desc.clone()
                    };
                    lines.push(Line::styled(
                        format!("    {}", truncated),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
            }
        }

        // Options - filter common global options
        let filtered_options: Vec<_> = cmd
            .options
            .iter()
            .filter(|(name, _, _)| {
                !matches!(
                    name.as_str(),
                    "--help"
                        | "--quiet"
                        | "--silent"
                        | "--verbose"
                        | "--version"
                        | "--ansi"
                        | "--no-ansi"
                        | "--no-interaction"
                        | "--env"
                )
            })
            .collect();

        if !filtered_options.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Options:".to_string(),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
            for (name, shortcut, desc) in filtered_options {
                let shortcut_str = if !shortcut.is_empty() {
                    format!("{}, ", shortcut)
                } else {
                    String::new()
                };
                lines.push(Line::from(vec![Span::styled(
                    format!("  {}{}", shortcut_str, name),
                    Style::default().fg(Theme::TEXT),
                )]));
                if !desc.is_empty() {
                    let available = max_width.saturating_sub(4);
                    let truncated = if desc.len() > available && available > 3 {
                        format!("{}...", &desc[..available.saturating_sub(3)])
                    } else {
                        desc.clone()
                    };
                    lines.push(Line::styled(
                        format!("    {}", truncated),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
            }
        }

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
        let msg = Paragraph::new("Select a command to see details")
            .style(Style::default().fg(Theme::TEXT_MUTED));
        frame.render_widget(msg, inner);
    }
}

fn render_input_area(frame: &mut Frame, area: Rect, app: &App) {
    let is_input_mode = app.make_tab.input_mode;
    let is_running = app.make_tab.running_command.is_some();
    let is_empty = app.make_tab.input_buffer.is_empty();
    let favorites = app
        .config
        .as_ref()
        .map(|c| c.make_favorites())
        .unwrap_or(&[]);

    // Check if selected make command has required arguments
    let has_required_args = app
        .make_tab
        .selected_make_command(favorites)
        .map(|cmd| cmd.arguments.iter().any(|(_, required, _)| *required))
        .unwrap_or(false);

    let border_color = if is_input_mode {
        Theme::BORDER_FOCUSED
    } else if has_required_args && is_empty && !is_running {
        Theme::WARNING
    } else {
        Theme::BORDER
    };

    let title = if is_running {
        " Input (press Enter to send) "
    } else if has_required_args {
        " Arguments (required) "
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
    } else if let Some(cmd) = app.make_tab.selected_make_command(favorites) {
        if !cmd.arguments.is_empty() {
            let args_hint: Vec<String> = cmd
                .arguments
                .iter()
                .map(|(name, required, _)| {
                    if *required {
                        format!("<{}>", name)
                    } else {
                        format!("[{}]", name)
                    }
                })
                .collect();
            format!("Press [i] to enter: {}", args_hint.join(" "))
        } else {
            "Press [i] to enter arguments or flags (optional)".to_string()
        }
    } else {
        "Select a command first".to_string()
    };

    let content = if is_input_mode {
        Line::from(vec![
            Span::styled(
                app.make_tab.input_buffer.clone(),
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
            app.make_tab.input_buffer.clone(),
            Style::default().fg(Theme::TEXT_DIM),
        )])
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_output_area(frame: &mut Frame, area: Rect, app: &App) {
    let title = if let Some(ref cmd) = app.make_tab.running_command {
        format!(" Output - {} (running) ", cmd)
    } else {
        " Output ".to_string()
    };

    let lines: Vec<Line> = app
        .make_tab
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

    let scroll_offset = app.make_tab.output_scroll_offset;
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

    let is_running = app.make_tab.running_command.is_some();

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
