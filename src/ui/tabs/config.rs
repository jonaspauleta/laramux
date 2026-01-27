use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::{App, ConfigFocus, ConfigSection, CustomProcessDraft, OverrideDraft};
use crate::config::RestartPolicy;
use crate::ui::theme::Theme;

/// Built-in process names for overrides
const OVERRIDE_PROCESSES: &[&str] = &["serve", "vite", "queue", "horizon", "reverb"];

/// Restart policy options for enum selection
const RESTART_POLICIES: &[(&str, RestartPolicy)] = &[
    ("never", RestartPolicy::Never),
    ("on_failure", RestartPolicy::OnFailure),
    ("always", RestartPolicy::Always),
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Check for config loading error
    if let Some(ref error) = app.config_error {
        render_config_error(frame, area, error);
        return;
    }

    // Split into main content and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Split main content into sections panel and details panel
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(chunks[0]);

    render_sections_panel(frame, main_chunks[0], app);
    render_details_panel(frame, main_chunks[1], app);
    render_footer(frame, chunks[1], app);
}

fn render_config_error(frame: &mut Frame, area: Rect, error: &str) {
    let block = Block::default()
        .title(" Configuration Error ")
        .title_style(
            Style::default()
                .fg(Theme::ERROR)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::ERROR))
        .padding(Padding::new(2, 2, 1, 1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create error message with helpful hints
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(Theme::ERROR)),
            Span::styled(
                "Failed to load .laramux.json",
                Style::default()
                    .fg(Theme::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    // Split error into multiple lines if needed (simple word wrap)
    let max_width = inner.width.saturating_sub(4) as usize;
    for line in error.lines() {
        if line.len() > max_width {
            // Simple word wrap
            let mut current_line = String::new();
            for word in line.split_whitespace() {
                if current_line.len() + word.len() + 1 > max_width && !current_line.is_empty() {
                    lines.push(Line::from(Span::styled(
                        current_line.clone(),
                        Style::default().fg(Theme::TEXT),
                    )));
                    current_line.clear();
                }
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
            if !current_line.is_empty() {
                lines.push(Line::from(Span::styled(
                    current_line,
                    Style::default().fg(Theme::TEXT),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Theme::TEXT),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("How to fix: ", Style::default().fg(Theme::ACCENT)),
        Span::styled(
            "Edit .laramux.json to correct the error, then restart laramux.",
            Style::default().fg(Theme::TEXT_DIM),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Tip: ", Style::default().fg(Theme::ACCENT)),
        Span::styled(
            "Use a JSON validator or IDE with JSON support to find syntax errors.",
            Style::default().fg(Theme::TEXT_DIM),
        ),
    ]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_sections_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.config_tab.focus == ConfigFocus::Sections;

    let draft = app.config_tab.config_draft.as_ref();
    let custom_count = draft.map(|d| d.custom_count()).unwrap_or(0);

    let items: Vec<ListItem> = ConfigSection::all()
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            let is_selected = idx == app.config_tab.section.index();
            let name = section.name();

            // Add count suffix for Custom section
            let display = if *section == ConfigSection::Custom && custom_count > 0 {
                format!("{} ({})", name, custom_count)
            } else {
                name.to_string()
            };

            let style = if is_selected && is_focused {
                Style::default()
                    .fg(Theme::TEXT)
                    .bg(Theme::SELECTION_BG)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_DIM)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                Span::styled(display, style),
            ]))
        })
        .collect();

    let title = if app.config_tab.has_changes {
        " Sections (modified) "
    } else {
        " Sections "
    };

    let border_color = if is_focused {
        Theme::BORDER_FOCUSED
    } else {
        Theme::BORDER
    };

    let block = Block::default()
        .title(title)
        .title_style(if app.config_tab.has_changes {
            Style::default()
                .fg(Theme::WARNING)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::title_style()
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));

    let list = List::new(items).block(block);

    frame.render_widget(list, area);
}

fn render_details_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    let border_color = if is_focused {
        Theme::BORDER_FOCUSED
    } else {
        Theme::BORDER
    };

    let block = Block::default()
        .title(format!(" {} ", app.config_tab.section.name()))
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    match app.config_tab.section {
        ConfigSection::Disabled => render_disabled_details(frame, inner_area, app),
        ConfigSection::Overrides => render_overrides_details(frame, inner_area, app),
        ConfigSection::Custom => render_custom_details(frame, inner_area, app),
    }
}

fn render_disabled_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => {
            let msg = Paragraph::new("No configuration loaded")
                .style(Style::default().fg(Theme::TEXT_MUTED));
            frame.render_widget(msg, area);
            return;
        }
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    let items: Vec<ListItem> = draft
        .disabled
        .items()
        .iter()
        .enumerate()
        .map(|(idx, (name, disabled))| {
            let is_selected = idx == app.config_tab.selected_item && is_focused;

            let checkbox = if *disabled { "[x]" } else { "[ ]" };
            let checkbox_color = if *disabled {
                Theme::ERROR
            } else {
                Theme::SUCCESS
            };

            let status_text = if *disabled { "disabled" } else { "enabled" };
            let status_color = if *disabled {
                Theme::ERROR
            } else {
                Theme::SUCCESS
            };

            let mut spans = Vec::new();

            if is_selected {
                spans.push(Span::styled("▶ ", Style::default().fg(Theme::ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }

            spans.push(Span::styled(checkbox, Style::default().fg(checkbox_color)));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{:<12}", name),
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));
            spans.push(Span::styled(status_text, Style::default().fg(status_color)));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);

    // Help text at bottom
    if area.height > 8 {
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                "Checked [x] = process will NOT start automatically",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
        frame.render_widget(help, help_area);
    }
}

fn render_overrides_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    let items: Vec<ListItem> = OVERRIDE_PROCESSES
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let is_selected = idx == app.config_tab.selected_item && is_focused;
            let override_draft = draft.overrides.get(*name);
            let has_override = override_draft.map(|o| !o.is_empty()).unwrap_or(false);

            let indicator = if has_override { "●" } else { "○" };
            let indicator_color = if has_override {
                Theme::SUCCESS
            } else {
                Theme::TEXT_MUTED
            };

            let mut spans = Vec::new();

            if is_selected {
                spans.push(Span::styled("▶ ", Style::default().fg(Theme::ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }

            spans.push(Span::styled(
                indicator,
                Style::default().fg(indicator_color),
            ));
            spans.push(Span::raw(" "));

            let name_display = format!("{:<12}", name);
            spans.push(Span::styled(
                name_display,
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));

            // Show summary of override
            if let Some(ovr) = override_draft {
                if !ovr.command.is_empty() {
                    spans.push(Span::styled(
                        format!("cmd: {} ", ovr.command),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
                if ovr.restart_policy != RestartPolicy::Never {
                    spans.push(Span::styled(
                        format!("restart: {:?}", ovr.restart_policy),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Split area for list and details
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(3)])
        .split(area);

    let list = List::new(items);
    frame.render_widget(list, chunks[0]);

    // Show selected override details
    if let Some(name) = OVERRIDE_PROCESSES.get(app.config_tab.selected_item) {
        let ovr = draft.overrides.get(*name);
        render_override_fields(frame, chunks[1], ovr, name, app);
    }
}

fn render_override_fields(
    frame: &mut Frame,
    area: Rect,
    ovr: Option<&OverrideDraft>,
    name: &str,
    app: &App,
) {
    let is_field_view = app.config_tab.is_field_view();
    let is_editing = app.config_tab.is_editing();
    let is_selecting = app.config_tab.is_selecting();
    let edit_field = app.config_tab.edit_field;
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("─── ", Style::default().fg(Theme::BORDER)),
            Span::styled(
                format!("{} Override", name),
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            if is_field_view {
                Span::styled(" (editing) ", Style::default().fg(Theme::ACCENT))
            } else {
                Span::raw("")
            },
            Span::styled("───", Style::default().fg(Theme::BORDER)),
        ]),
        Line::from(""),
    ];

    let fields: Vec<(&str, String, bool)> = vec![
        (
            "Command",
            ovr.map(|o| o.command.clone()).unwrap_or_default(),
            false,
        ),
        (
            "Args",
            ovr.map(|o| o.args.clone()).unwrap_or_default(),
            false,
        ),
        (
            "Working Dir",
            ovr.map(|o| o.working_dir.clone()).unwrap_or_default(),
            false,
        ),
        (
            "Restart",
            ovr.map(|o| restart_policy_str(o.restart_policy))
                .unwrap_or_else(|| "never".to_string()),
            true, // is_enum
        ),
    ];

    for (idx, (label, value, is_enum)) in fields.iter().enumerate() {
        let is_field_selected = is_field_view && edit_field == idx;
        let is_field_editing = is_editing && edit_field == idx;
        let is_field_selecting = is_selecting && edit_field == idx;

        let display_value = if is_field_editing {
            format!("{}▌", edit_buffer)
        } else if is_field_selecting {
            // Show enum options
            render_enum_inline(app.config_tab.enum_selection)
        } else if value.is_empty() {
            "(not set)".to_string()
        } else {
            value.clone()
        };

        let value_style = if is_field_editing || is_field_selecting {
            Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
        } else if is_field_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else if value.is_empty() {
            Style::default().fg(Theme::TEXT_MUTED)
        } else {
            Style::default().fg(Theme::TEXT)
        };

        let prefix = if is_field_selected { "▶ " } else { "  " };
        let enum_marker = if *is_enum && is_field_selected {
            " [↵ to select]"
        } else {
            ""
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(
                format!("{:<12}: ", label),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled(display_value, value_style),
            Span::styled(enum_marker, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    // Env vars (simplified display)
    if let Some(ovr) = ovr {
        if !ovr.env.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Environment:",
                Style::default().fg(Theme::TEXT_DIM),
            )));
            for (key, val) in &ovr.env {
                lines.push(Line::from(Span::styled(
                    format!("  {}={}", key, val),
                    Style::default().fg(Theme::TEXT),
                )));
            }
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_custom_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    if draft.custom.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No custom processes defined",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    " to add a new custom process",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    // Split for list and details (like Overrides)
    let list_height = (draft.custom.len() + 2).min(8) as u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(list_height), Constraint::Min(3)])
        .split(area);

    let items: Vec<ListItem> = draft
        .custom
        .iter()
        .enumerate()
        .map(|(idx, cp)| {
            let is_selected = idx == app.config_tab.selected_item && is_focused;

            let enabled_indicator = if cp.enabled { "●" } else { "○" };
            let enabled_color = if cp.enabled {
                Theme::SUCCESS
            } else {
                Theme::TEXT_MUTED
            };

            let hotkey_display = if cp.hotkey.is_empty() {
                "   ".to_string()
            } else {
                format!("[{}]", cp.hotkey)
            };

            let mut spans = Vec::new();

            if is_selected {
                spans.push(Span::styled("▶ ", Style::default().fg(Theme::ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }

            spans.push(Span::styled(
                enabled_indicator,
                Style::default().fg(enabled_color),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                hotkey_display,
                Style::default().fg(Theme::ACCENT),
            ));
            spans.push(Span::raw(" "));

            let name_display = if cp.display_name.is_empty() {
                if cp.name.is_empty() {
                    "(unnamed)".to_string()
                } else {
                    cp.name.clone()
                }
            } else {
                cp.display_name.clone()
            };

            spans.push(Span::styled(
                name_display,
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));

            // Show command summary
            if !cp.command.is_empty() {
                spans.push(Span::styled(
                    format!("  → {}", cp.command),
                    Style::default().fg(Theme::TEXT_MUTED),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[0]);

    // Show selected process details
    let selected = app
        .config_tab
        .selected_item
        .min(draft.custom.len().saturating_sub(1));
    if let Some(cp) = draft.custom.get(selected) {
        render_custom_fields(frame, chunks[1], cp, app);
    }
}

fn render_custom_fields(frame: &mut Frame, area: Rect, cp: &CustomProcessDraft, app: &App) {
    let is_field_view = app.config_tab.is_field_view();
    let is_editing = app.config_tab.is_editing();
    let is_selecting = app.config_tab.is_selecting();
    let edit_field = app.config_tab.edit_field;
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![Line::from(vec![
        Span::styled("─── ", Style::default().fg(Theme::BORDER)),
        Span::styled(
            "Details",
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if cp.enabled {
                " (enabled) "
            } else {
                " (disabled) "
            },
            Style::default().fg(if cp.enabled {
                Theme::SUCCESS
            } else {
                Theme::TEXT_MUTED
            }),
        ),
        if is_field_view {
            Span::styled("(editing) ", Style::default().fg(Theme::ACCENT))
        } else {
            Span::raw("")
        },
        Span::styled("───", Style::default().fg(Theme::BORDER)),
    ])];

    // Fields: name, display_name, command, args, hotkey, working_dir, restart_policy
    let fields: Vec<(&str, String, bool)> = vec![
        ("Name", cp.name.clone(), false),
        ("Display", cp.display_name.clone(), false),
        ("Command", cp.command.clone(), false),
        ("Args", cp.args.clone(), false),
        ("Hotkey", cp.hotkey.clone(), false),
        ("Work Dir", cp.working_dir.clone(), false),
        ("Restart", restart_policy_str(cp.restart_policy), true), // is_enum
    ];

    for (idx, (label, value, is_enum)) in fields.iter().enumerate() {
        let is_field_selected = is_field_view && edit_field == idx;
        let is_field_editing = is_editing && edit_field == idx;
        let is_field_selecting = is_selecting && edit_field == idx;

        let display_value = if is_field_editing {
            format!("{}▌", edit_buffer)
        } else if is_field_selecting {
            render_enum_inline(app.config_tab.enum_selection)
        } else if value.is_empty() {
            "(not set)".to_string()
        } else {
            value.clone()
        };

        let value_style = if is_field_editing || is_field_selecting {
            Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
        } else if is_field_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else if value.is_empty() {
            Style::default().fg(Theme::TEXT_MUTED)
        } else {
            Style::default().fg(Theme::TEXT)
        };

        let prefix = if is_field_selected { "▶ " } else { "  " };
        let enum_marker = if *is_enum && is_field_selected && !is_field_selecting {
            " [↵ to select]"
        } else {
            ""
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(
                format!("{:<10}: ", label),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled(display_value, value_style),
            Span::styled(enum_marker, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn restart_policy_str(policy: RestartPolicy) -> String {
    match policy {
        RestartPolicy::Never => "never".to_string(),
        RestartPolicy::OnFailure => "on_failure".to_string(),
        RestartPolicy::Always => "always".to_string(),
    }
}

fn render_enum_inline(selected: usize) -> String {
    RESTART_POLICIES
        .iter()
        .enumerate()
        .map(|(idx, (name, _))| {
            if idx == selected {
                format!("[{}]", name)
            } else {
                name.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: 1,
    };

    frame.render_widget(block, area);

    // Build context-sensitive footer
    let mut spans = Vec::new();

    // Show error if any
    if let Some(ref error) = app.config_tab.error {
        spans.push(Span::styled(
            format!("Error: {} ", error),
            Style::default().fg(Theme::ERROR),
        ));
    }

    // Navigation hints based on current state
    if app.config_tab.is_editing() {
        spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled(
            "Confirm",
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)));
    } else if app.config_tab.is_selecting() {
        spans.push(Span::styled("[h/l] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Choose", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled(
            "Confirm",
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)));
    } else if app.config_tab.confirm_delete.is_some() {
        spans.push(Span::styled("[y] ", Style::default().fg(Theme::ERROR)));
        spans.push(Span::styled(
            "Confirm Delete",
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[n/Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)));
    } else if app.config_tab.is_field_view() {
        // Field navigation mode
        spans.push(Span::styled("[j/k] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Field", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[h/Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Back", Style::default().fg(Theme::TEXT_DIM)));

        // Save/reset if changes
        if app.config_tab.has_changes {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "• Unsaved",
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    } else {
        // Standard navigation
        spans.push(Span::styled("[h/l] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Pane", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[j/k] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled(
            "Navigate",
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::raw("  "));

        // Section-specific hints
        match app.config_tab.section {
            ConfigSection::Disabled => {
                spans.push(Span::styled("[Space] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Toggle", Style::default().fg(Theme::TEXT_DIM)));
            }
            ConfigSection::Overrides => {
                spans.push(Span::styled(
                    "[Enter/l] ",
                    Style::default().fg(Theme::ACCENT),
                ));
                spans.push(Span::styled(
                    "Edit Fields",
                    Style::default().fg(Theme::TEXT_DIM),
                ));
            }
            ConfigSection::Custom => {
                spans.push(Span::styled("[a] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Add", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[d] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Delete", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    "[Enter/l] ",
                    Style::default().fg(Theme::ACCENT),
                ));
                spans.push(Span::styled(
                    "Edit Fields",
                    Style::default().fg(Theme::TEXT_DIM),
                ));
            }
        }

        // Save/reset if changes
        if app.config_tab.has_changes {
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[s] ", Style::default().fg(Theme::ACCENT)));
            spans.push(Span::styled("Save", Style::default().fg(Theme::TEXT_DIM)));
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[r] ", Style::default().fg(Theme::ACCENT)));
            spans.push(Span::styled("Reset", Style::default().fg(Theme::TEXT_DIM)));
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "• Unsaved",
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, inner_area);
}
