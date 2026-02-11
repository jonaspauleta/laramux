use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::{App, ConfigFocus, ConfigSection, CustomProcessDraft, CustomToolDraft};
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

/// Sail configuration options
const SAIL_OPTIONS: &[&str] = &["Auto-detect", "Force Enabled", "Force Disabled"];

/// Log level options for default_filter
const LOG_LEVEL_OPTIONS: &[&str] = &[
    "(none)",
    "debug",
    "info",
    "notice",
    "warning",
    "error",
    "critical",
    "alert",
    "emergency",
];

/// Quality tool category options
const CATEGORY_OPTIONS: &[&str] = &["quality", "testing"];

fn section_description(section: &ConfigSection) -> &'static str {
    match section {
        ConfigSection::Disabled => "Prevent built-in processes from starting automatically.",
        ConfigSection::Overrides => {
            "Customize commands, args, and restart behavior for built-in processes."
        }
        ConfigSection::Custom => "Define additional processes to run alongside the built-in ones.",
        ConfigSection::Sail => "Control whether commands run through Docker via Laravel Sail.",
        ConfigSection::Logs => "Configure log display limits, filtering, and additional log files.",
        ConfigSection::QualityDisabledTools => "Hide specific tools from the Quality tab.",
        ConfigSection::QualityCustomTools => {
            "Add custom quality or testing tools with their own commands."
        }
        ConfigSection::QualityDefaultArgs => "Set default arguments always passed to a tool.",
        ConfigSection::ArtisanFavorites => {
            "Pin frequently-used artisan commands to the top of the list."
        }
        ConfigSection::MakeFavorites => "Pin frequently-used make commands to the top of the list.",
    }
}

fn override_field_description(field: usize) -> Option<&'static str> {
    match field {
        0 => Some("Override the default command binary"),
        1 => Some("Additional arguments passed to the command"),
        2 => Some("Run from a different directory"),
        3 => Some("What happens when the process exits"),
        _ => None,
    }
}

fn custom_field_description(field: usize) -> Option<&'static str> {
    match field {
        0 => Some("Unique identifier for this process"),
        1 => Some("Label shown in the sidebar"),
        2 => Some("The executable to run"),
        3 => Some("Arguments passed to the command"),
        4 => Some("Single key to toggle this process"),
        5 => Some("Run from a different directory"),
        6 => Some("What happens when the process exits"),
        _ => None,
    }
}

fn custom_tool_field_description(field: usize) -> Option<&'static str> {
    match field {
        0 => Some("Unique identifier for this tool"),
        1 => Some("Label shown in the quality tab"),
        2 => Some("The executable to run"),
        3 => Some("Arguments passed to the command"),
        4 => Some("Group: quality or testing"),
        _ => None,
    }
}

fn default_args_field_description(field: usize) -> Option<&'static str> {
    match field {
        0 => Some("Name of the tool to configure"),
        1 => Some("Arguments always passed to this tool"),
        _ => None,
    }
}

/// Render a vertical enum selector dropdown appended to lines
fn render_enum_selector<'a>(
    lines: &mut Vec<Line<'a>>,
    options: &[&str],
    selected_idx: usize,
    indent: usize,
) {
    for (idx, option) in options.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_DIM)
        };
        let padding = " ".repeat(indent);
        lines.push(Line::from(vec![
            Span::raw(padding),
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(option.to_string(), style),
        ]));
    }
}

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
            Span::styled("! ", Style::default().fg(Theme::ERROR)),
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
    let artisan_fav_count = draft.map(|d| d.artisan_favorites.len()).unwrap_or(0);
    let make_fav_count = draft.map(|d| d.make_favorites.len()).unwrap_or(0);
    let disabled_tools_count = draft.map(|d| d.quality.disabled_tools.len()).unwrap_or(0);
    let custom_tools_count = draft.map(|d| d.quality.custom_tools.len()).unwrap_or(0);
    let default_args_count = draft.map(|d| d.quality.default_args.len()).unwrap_or(0);

    let items: Vec<ListItem> = ConfigSection::all()
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            let is_selected = idx == app.config_tab.section.index();
            let name = section.name();

            // Add count suffix for sections with dynamic items
            let display = match section {
                ConfigSection::Custom if custom_count > 0 => {
                    format!("{} ({})", name, custom_count)
                }
                ConfigSection::QualityDisabledTools if disabled_tools_count > 0 => {
                    format!("{} ({})", name, disabled_tools_count)
                }
                ConfigSection::QualityCustomTools if custom_tools_count > 0 => {
                    format!("{} ({})", name, custom_tools_count)
                }
                ConfigSection::QualityDefaultArgs if default_args_count > 0 => {
                    format!("{} ({})", name, default_args_count)
                }
                ConfigSection::ArtisanFavorites if artisan_fav_count > 0 => {
                    format!("{} ({})", name, artisan_fav_count)
                }
                ConfigSection::MakeFavorites if make_fav_count > 0 => {
                    format!("{} ({})", name, make_fav_count)
                }
                _ => name.to_string(),
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

            let prefix = if is_selected { "> " } else { "  " };
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
        ConfigSection::Sail => render_sail_details(frame, inner_area, app),
        ConfigSection::Logs => render_logs_details(frame, inner_area, app),
        ConfigSection::QualityDisabledTools => {
            render_quality_disabled_tools(frame, inner_area, app)
        }
        ConfigSection::QualityCustomTools => render_quality_custom_tools(frame, inner_area, app),
        ConfigSection::QualityDefaultArgs => render_quality_default_args(frame, inner_area, app),
        ConfigSection::ArtisanFavorites => {
            render_favorites_details(frame, inner_area, app, "artisan")
        }
        ConfigSection::MakeFavorites => render_favorites_details(frame, inner_area, app, "make"),
    }
}

fn render_description_line(section: &ConfigSection) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", section_description(section)),
        Style::default().fg(Theme::TEXT_MUTED),
    ))
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

    let mut lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (idx, (name, disabled)) in draft.disabled.items().iter().enumerate() {
        let is_selected = idx == app.config_tab.selected_item && is_focused;

        let indicator = if *disabled { "○" } else { "●" };
        let indicator_color = if *disabled {
            Theme::TEXT_MUTED
        } else {
            Theme::SUCCESS
        };

        let status_text = if *disabled { "disabled" } else { "enabled" };
        let status_color = if *disabled {
            Theme::TEXT_MUTED
        } else {
            Theme::SUCCESS
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

        lines.push(Line::from(spans));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_overrides_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;
    let is_editing = app.config_tab.is_editing();
    let is_selecting = app.config_tab.is_selecting();
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (proc_idx, proc_name) in OVERRIDE_PROCESSES.iter().enumerate() {
        let base_idx = proc_idx * 5;
        let ovr = draft.overrides.get(*proc_name);

        // Header row
        let is_header_selected = app.config_tab.selected_item == base_idx && is_focused;
        let has_override = ovr.map(|o| !o.is_empty()).unwrap_or(false);

        let indicator = if has_override { "●" } else { "○" };
        let indicator_color = if has_override {
            Theme::SUCCESS
        } else {
            Theme::TEXT_MUTED
        };

        let header_prefix = if is_header_selected { "▶ " } else { "  " };
        let header_style = Style::default()
            .fg(Theme::TEXT)
            .add_modifier(Modifier::BOLD);

        lines.push(Line::from(vec![
            Span::styled(header_prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(indicator, Style::default().fg(indicator_color)),
            Span::styled(format!(" {}", capitalize(proc_name)), header_style),
        ]));

        // Field rows
        let fields: [(&str, String, bool); 4] = [
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
                true,
            ),
        ];

        for (field_idx, (label, value, is_enum)) in fields.iter().enumerate() {
            let flat_idx = base_idx + 1 + field_idx;
            let is_selected = app.config_tab.selected_item == flat_idx && is_focused;
            let is_field_editing = is_editing && app.config_tab.selected_item == flat_idx;
            let is_field_selecting = is_selecting && app.config_tab.selected_item == flat_idx;

            let display_value = if is_field_editing {
                format!("{}|", edit_buffer)
            } else if value.is_empty() {
                "(not set)".to_string()
            } else {
                value.clone()
            };

            let value_style = if is_field_editing || is_field_selecting {
                Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
            } else if is_selected {
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD)
            } else if value.is_empty() {
                Style::default().fg(Theme::TEXT_MUTED)
            } else {
                Style::default().fg(Theme::TEXT)
            };

            let prefix = if is_selected { "  ▶ " } else { "    " };
            let enum_marker = if *is_enum && is_selected && !is_field_selecting {
                " [Enter to select]"
            } else {
                ""
            };

            if is_field_selecting {
                // Show current value then vertical dropdown
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                    Span::styled(
                        format!("{:<12} ", label),
                        Style::default().fg(Theme::TEXT_DIM),
                    ),
                ]));
                let restart_options: Vec<&str> =
                    RESTART_POLICIES.iter().map(|(name, _)| *name).collect();
                render_enum_selector(
                    &mut lines,
                    &restart_options,
                    app.config_tab.enum_selection,
                    18,
                );
            } else {
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                    Span::styled(
                        format!("{:<12} ", label),
                        Style::default().fg(Theme::TEXT_DIM),
                    ),
                    Span::styled(display_value, value_style),
                    Span::styled(enum_marker, Style::default().fg(Theme::TEXT_MUTED)),
                ]));
            }

            // Field description hint (only for selected field)
            if is_selected && !is_field_editing && !is_field_selecting {
                if let Some(desc) = override_field_description(field_idx) {
                    lines.push(Line::from(Span::styled(
                        format!("      {}", desc),
                        Style::default().fg(Theme::TEXT_MUTED),
                    )));
                }
            }
        }

        // Spacing between processes
        if proc_idx < OVERRIDE_PROCESSES.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    let para = Paragraph::new(lines).scroll((app.config_tab.scroll_offset as u16, 0));
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
            render_description_line(&app.config_tab.section),
            Line::from(""),
            Line::from(Span::styled(
                "  No custom processes defined.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Add a custom process to run alongside built-in ones. ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(" to add.", Style::default().fg(Theme::TEXT_MUTED)),
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
                    format!("  -> {}", cp.command),
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
        Span::styled("--- ", Style::default().fg(Theme::BORDER)),
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
        Span::styled("---", Style::default().fg(Theme::BORDER)),
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
            format!("{}|", edit_buffer)
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
            " [Enter to select]"
        } else {
            ""
        };

        if is_field_selecting {
            // Vertical enum dropdown
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    format!("{:<10}: ", label),
                    Style::default().fg(Theme::TEXT_DIM),
                ),
            ]));
            let restart_options: Vec<&str> =
                RESTART_POLICIES.iter().map(|(name, _)| *name).collect();
            render_enum_selector(
                &mut lines,
                &restart_options,
                app.config_tab.enum_selection,
                14,
            );
        } else {
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

        // Field description hint
        if is_field_selected && !is_field_editing && !is_field_selecting {
            if let Some(desc) = custom_field_description(idx) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", desc),
                    Style::default().fg(Theme::TEXT_MUTED),
                )));
            }
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_sail_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;
    let is_selecting = app.config_tab.is_selecting();
    let is_selected = app.config_tab.selected_item == 0 && is_focused;

    let current_label = match draft.sail {
        None => "Auto-detect",
        Some(true) => "Force Enabled",
        Some(false) => "Force Disabled",
    };

    let mut lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    let prefix = if is_selected { "▶ " } else { "  " };

    if is_selecting {
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("Sail Mode:", Style::default().fg(Theme::TEXT_DIM)),
        ]));
        render_enum_selector(&mut lines, SAIL_OPTIONS, app.config_tab.enum_selection, 4);
    } else {
        let value_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_DIM)
        };

        let hint = if is_selected {
            " [Enter to select]"
        } else {
            ""
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("Sail Mode:  ", Style::default().fg(Theme::TEXT_DIM)),
            Span::styled(current_label, value_style),
            Span::styled(hint, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_logs_details(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;
    let is_editing = app.config_tab.is_editing();
    let is_selecting = app.config_tab.is_selecting();
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    // Item 0: max_lines
    let is_selected = app.config_tab.selected_item == 0 && is_focused;
    let is_item_editing = is_editing && app.config_tab.selected_item == 0;
    let display = if is_item_editing {
        format!("{}|", edit_buffer)
    } else if draft.logs.max_lines.is_empty() {
        "(default: 100)".to_string()
    } else {
        draft.logs.max_lines.clone()
    };

    let value_style = if is_item_editing {
        Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
    } else if is_selected {
        Style::default()
            .fg(Theme::TEXT)
            .add_modifier(Modifier::BOLD)
    } else if draft.logs.max_lines.is_empty() {
        Style::default().fg(Theme::TEXT_MUTED)
    } else {
        Style::default().fg(Theme::TEXT)
    };

    let prefix = if is_selected { "▶ " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
        Span::styled("Max Lines:      ", Style::default().fg(Theme::TEXT_DIM)),
        Span::styled(display, value_style),
    ]));

    // Item 1: default_filter
    let is_selected = app.config_tab.selected_item == 1 && is_focused;
    let is_item_selecting = is_selecting && app.config_tab.selected_item == 1;

    let prefix = if is_selected { "▶ " } else { "  " };

    if is_item_selecting {
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("Default Filter:", Style::default().fg(Theme::TEXT_DIM)),
        ]));
        render_enum_selector(
            &mut lines,
            LOG_LEVEL_OPTIONS,
            app.config_tab.enum_selection,
            4,
        );
    } else {
        let display = if draft.logs.default_filter.is_empty() {
            "(none)".to_string()
        } else {
            draft.logs.default_filter.clone()
        };

        let value_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else if draft.logs.default_filter.is_empty() {
            Style::default().fg(Theme::TEXT_MUTED)
        } else {
            Style::default().fg(Theme::TEXT)
        };

        let hint = if is_selected {
            " [Enter to select]"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("Default Filter: ", Style::default().fg(Theme::TEXT_DIM)),
            Span::styled(display, value_style),
            Span::styled(hint, Style::default().fg(Theme::TEXT_MUTED)),
        ]));
    }

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Additional Log Files:",
        Style::default()
            .fg(Theme::TEXT_DIM)
            .add_modifier(Modifier::BOLD),
    )));

    // Items 2+: files
    if draft.logs.files.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(Theme::TEXT_MUTED),
        )));
    } else {
        for (idx, file) in draft.logs.files.iter().enumerate() {
            let item_idx = idx + 2;
            let is_selected = app.config_tab.selected_item == item_idx && is_focused;
            let is_item_editing = is_editing && app.config_tab.selected_item == item_idx;

            let display = if is_item_editing {
                format!("{}|", edit_buffer)
            } else if file.is_empty() {
                "(empty)".to_string()
            } else {
                file.clone()
            };

            let value_style = if is_item_editing {
                Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
            } else if is_selected {
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD)
            } else if file.is_empty() {
                Style::default().fg(Theme::TEXT_MUTED)
            } else {
                Style::default().fg(Theme::TEXT)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                Span::styled(display, value_style),
            ]));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_favorites_details(frame: &mut Frame, area: Rect, app: &App, kind: &str) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let favorites = match kind {
        "artisan" => &draft.artisan_favorites,
        "make" => &draft.make_favorites,
        _ => return,
    };

    let is_focused = app.config_tab.focus == ConfigFocus::Details;
    let is_editing = app.config_tab.is_editing();
    let edit_buffer = &app.config_tab.edit_buffer;

    if favorites.is_empty() {
        let msg = Paragraph::new(vec![
            render_description_line(&app.config_tab.section),
            Line::from(""),
            Line::from(Span::styled(
                format!("  No {} favorites defined.", kind),
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Pin commands you use often so they appear first. ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(" to add.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    let mut all_lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (idx, fav) in favorites.iter().enumerate() {
        let is_selected = idx == app.config_tab.selected_item && is_focused;
        let is_item_editing = is_editing && idx == app.config_tab.selected_item;

        let display = if is_item_editing {
            format!("{}|", edit_buffer)
        } else if fav.is_empty() {
            "(empty)".to_string()
        } else {
            fav.clone()
        };

        let value_style = if is_item_editing {
            Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
        } else if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else if fav.is_empty() {
            Style::default().fg(Theme::TEXT_MUTED)
        } else {
            Style::default().fg(Theme::TEXT)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        all_lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("* ", Style::default().fg(Theme::WARNING)),
            Span::styled(display, value_style),
        ]));
    }

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}

fn render_quality_disabled_tools(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let tools = &draft.quality.disabled_tools;
    let is_focused = app.config_tab.focus == ConfigFocus::Details;
    let is_editing = app.config_tab.is_editing();
    let edit_buffer = &app.config_tab.edit_buffer;

    if tools.is_empty() {
        let msg = Paragraph::new(vec![
            render_description_line(&app.config_tab.section),
            Line::from(""),
            Line::from(Span::styled(
                "  No disabled tools.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Add a tool name to hide it from the Quality tab. ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(" to add.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    let mut all_lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (idx, tool) in tools.iter().enumerate() {
        let is_selected = idx == app.config_tab.selected_item && is_focused;
        let is_item_editing = is_editing && idx == app.config_tab.selected_item;

        let display = if is_item_editing {
            format!("{}|", edit_buffer)
        } else if tool.is_empty() {
            "(empty)".to_string()
        } else {
            tool.clone()
        };

        let value_style = if is_item_editing {
            Style::default().fg(Theme::ACCENT).bg(Theme::SELECTION_BG)
        } else if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT)
        };

        let prefix = if is_selected { "▶ " } else { "  " };
        all_lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("x ", Style::default().fg(Theme::ERROR)),
            Span::styled(display, value_style),
        ]));
    }

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}

fn render_quality_custom_tools(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let tools = &draft.quality.custom_tools;
    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    if tools.is_empty() {
        let msg = Paragraph::new(vec![
            render_description_line(&app.config_tab.section),
            Line::from(""),
            Line::from(Span::styled(
                "  No custom tools defined.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Create custom quality or testing tools with their own commands. ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(" to add.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    if app.config_tab.is_field_view() {
        // Show fields for selected tool
        let selected = app
            .config_tab
            .selected_item
            .min(tools.len().saturating_sub(1));
        if let Some(tool) = tools.get(selected) {
            render_quality_tool_fields(frame, area, tool, app);
        }
        return;
    }

    let mut all_lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (idx, tool) in tools.iter().enumerate() {
        let is_selected = idx == app.config_tab.selected_item && is_focused;

        let name_display = if tool.display_name.is_empty() {
            if tool.name.is_empty() {
                "(unnamed)"
            } else {
                &tool.name
            }
        } else {
            &tool.display_name
        };

        let prefix = if is_selected { "▶ " } else { "  " };
        let name_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_DIM)
        };

        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled("● ", Style::default().fg(Theme::SUCCESS)),
            Span::styled(name_display.to_string(), name_style),
        ];

        if !tool.command.is_empty() {
            spans.push(Span::styled(
                format!("  -> {}", tool.command),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }

        spans.push(Span::styled(
            format!("  [{}]", tool.category),
            Style::default().fg(Theme::TEXT_MUTED),
        ));

        all_lines.push(Line::from(spans));
    }

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}

fn render_quality_tool_fields(frame: &mut Frame, area: Rect, tool: &CustomToolDraft, app: &App) {
    let is_field_view = app.config_tab.is_field_view();
    let is_editing = app.config_tab.is_editing();
    let is_selecting = app.config_tab.is_selecting();
    let edit_field = app.config_tab.edit_field;
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![Line::from(vec![
        Span::styled("--- ", Style::default().fg(Theme::BORDER)),
        Span::styled(
            "Tool Details",
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        if is_field_view {
            Span::styled(" (editing) ", Style::default().fg(Theme::ACCENT))
        } else {
            Span::raw("")
        },
        Span::styled("---", Style::default().fg(Theme::BORDER)),
    ])];

    let fields: Vec<(&str, String, bool)> = vec![
        ("Name", tool.name.clone(), false),
        ("Display", tool.display_name.clone(), false),
        ("Command", tool.command.clone(), false),
        ("Args", tool.args.clone(), false),
        ("Category", tool.category.clone(), true), // is_enum
    ];

    for (idx, (label, value, is_enum)) in fields.iter().enumerate() {
        let is_field_selected = is_field_view && edit_field == idx;
        let is_field_editing = is_editing && edit_field == idx;
        let is_field_selecting = is_selecting && edit_field == idx;

        let display_value = if is_field_editing {
            format!("{}|", edit_buffer)
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
            " [Enter to select]"
        } else {
            ""
        };

        if is_field_selecting {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    format!("{:<10}: ", label),
                    Style::default().fg(Theme::TEXT_DIM),
                ),
            ]));
            render_enum_selector(
                &mut lines,
                CATEGORY_OPTIONS,
                app.config_tab.enum_selection,
                14,
            );
        } else {
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

        // Field description hint
        if is_field_selected && !is_field_editing && !is_field_selecting {
            if let Some(desc) = custom_tool_field_description(idx) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", desc),
                    Style::default().fg(Theme::TEXT_MUTED),
                )));
            }
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_quality_default_args(frame: &mut Frame, area: Rect, app: &App) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    let args = &draft.quality.default_args;
    let is_focused = app.config_tab.focus == ConfigFocus::Details;

    if args.is_empty() {
        let msg = Paragraph::new(vec![
            render_description_line(&app.config_tab.section),
            Line::from(""),
            Line::from(Span::styled(
                "  No default args configured.",
                Style::default().fg(Theme::TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Set arguments that are always passed to a tool. ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("[a]", Style::default().fg(Theme::ACCENT)),
                Span::styled(" to add.", Style::default().fg(Theme::TEXT_MUTED)),
            ]),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    if app.config_tab.is_field_view() {
        // Show fields for selected default args entry
        let selected = app
            .config_tab
            .selected_item
            .min(args.len().saturating_sub(1));
        if let Some((tool_name, tool_args)) = args.get(selected) {
            render_default_args_fields(frame, area, tool_name, tool_args, app);
        }
        return;
    }

    let mut all_lines = vec![
        render_description_line(&app.config_tab.section),
        Line::from(""),
    ];

    for (idx, (tool_name, tool_args)) in args.iter().enumerate() {
        let is_selected = idx == app.config_tab.selected_item && is_focused;

        let prefix = if is_selected { "▶ " } else { "  " };
        let name_style = if is_selected {
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_DIM)
        };

        let display_name = if tool_name.is_empty() {
            "(unnamed)"
        } else {
            tool_name
        };

        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(format!("{:<15}", display_name), name_style),
        ];

        if !tool_args.is_empty() {
            spans.push(Span::styled(
                format!("-> {}", tool_args),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }

        all_lines.push(Line::from(spans));
    }

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}

fn render_default_args_fields(
    frame: &mut Frame,
    area: Rect,
    tool_name: &str,
    tool_args: &str,
    app: &App,
) {
    let is_field_view = app.config_tab.is_field_view();
    let is_editing = app.config_tab.is_editing();
    let edit_field = app.config_tab.edit_field;
    let edit_buffer = &app.config_tab.edit_buffer;

    let mut lines = vec![Line::from(vec![
        Span::styled("--- ", Style::default().fg(Theme::BORDER)),
        Span::styled(
            "Default Args",
            Style::default()
                .fg(Theme::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        if is_field_view {
            Span::styled(" (editing) ", Style::default().fg(Theme::ACCENT))
        } else {
            Span::raw("")
        },
        Span::styled("---", Style::default().fg(Theme::BORDER)),
    ])];

    let fields: Vec<(&str, &str)> = vec![("Tool Name", tool_name), ("Args", tool_args)];

    for (idx, (label, value)) in fields.iter().enumerate() {
        let is_field_selected = is_field_view && edit_field == idx;
        let is_field_editing = is_editing && edit_field == idx;

        let display_value = if is_field_editing {
            format!("{}|", edit_buffer)
        } else if value.is_empty() {
            "(not set)".to_string()
        } else {
            value.to_string()
        };

        let value_style = if is_field_editing {
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

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Theme::ACCENT)),
            Span::styled(
                format!("{:<10}: ", label),
                Style::default().fg(Theme::TEXT_DIM),
            ),
            Span::styled(display_value, value_style),
        ]));

        // Field description hint
        if is_field_selected && !is_field_editing {
            if let Some(desc) = default_args_field_description(idx) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", desc),
                    Style::default().fg(Theme::TEXT_MUTED),
                )));
            }
        }
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

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
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

    // Show unsaved indicator first if applicable
    let has_changes = app.config_tab.has_changes;

    // Show error if any
    if let Some(ref error) = app.config_tab.error {
        spans.push(Span::styled(
            format!("Error: {} ", error),
            Style::default().fg(Theme::ERROR),
        ));
    }

    // Navigation hints based on current state
    if app.config_tab.is_editing() {
        if has_changes {
            push_unsaved(&mut spans);
        }
        spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled(
            "Confirm",
            Style::default().fg(Theme::TEXT_DIM),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Cancel", Style::default().fg(Theme::TEXT_DIM)));
    } else if app.config_tab.is_selecting() {
        if has_changes {
            push_unsaved(&mut spans);
        }
        spans.push(Span::styled("[j/k] ", Style::default().fg(Theme::ACCENT)));
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
        if has_changes {
            push_unsaved(&mut spans);
        }
        spans.push(Span::styled("[j/k] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Field", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[h/Esc] ", Style::default().fg(Theme::ACCENT)));
        spans.push(Span::styled("Back", Style::default().fg(Theme::TEXT_DIM)));

        if has_changes {
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[s] ", Style::default().fg(Theme::ACCENT)));
            spans.push(Span::styled("Save", Style::default().fg(Theme::TEXT_DIM)));
        }
    } else {
        // Standard navigation
        if has_changes {
            push_unsaved(&mut spans);
            spans.push(Span::styled("[s] ", Style::default().fg(Theme::ACCENT)));
            spans.push(Span::styled("Save", Style::default().fg(Theme::TEXT_DIM)));
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[r] ", Style::default().fg(Theme::ACCENT)));
            spans.push(Span::styled("Reset", Style::default().fg(Theme::TEXT_DIM)));
            spans.push(Span::raw("  |  "));
        }

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
                spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
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
            ConfigSection::Sail => {
                spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Select", Style::default().fg(Theme::TEXT_DIM)));
            }
            ConfigSection::Logs => {
                spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[a] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled(
                    "Add File",
                    Style::default().fg(Theme::TEXT_DIM),
                ));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[d] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Delete", Style::default().fg(Theme::TEXT_DIM)));
            }
            ConfigSection::QualityDisabledTools => {
                spans.push(Span::styled("[a] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Add", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[d] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Delete", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
            }
            ConfigSection::QualityCustomTools => {
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
            ConfigSection::QualityDefaultArgs => {
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
            ConfigSection::ArtisanFavorites | ConfigSection::MakeFavorites => {
                spans.push(Span::styled("[a] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Add", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[d] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Delete", Style::default().fg(Theme::TEXT_DIM)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[Enter] ", Style::default().fg(Theme::ACCENT)));
                spans.push(Span::styled("Edit", Style::default().fg(Theme::TEXT_DIM)));
            }
        }
    }

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, inner_area);
}

fn push_unsaved(spans: &mut Vec<Span<'static>>) {
    spans.push(Span::styled(
        "● Unsaved",
        Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw("  "));
}
