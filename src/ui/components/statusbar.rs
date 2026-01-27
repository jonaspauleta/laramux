use ratatui::{prelude::*, widgets::Paragraph};

use crate::app::App;
use crate::ui::tabs::Tab;
use crate::ui::theme::Theme;

/// Build a key hint with styled key and description
fn key_hint(key: &str, desc: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            key.to_string(),
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(":{}", desc), Style::default().fg(Theme::TEXT_DIM)),
    ]
}

/// Build a separator
fn separator() -> Span<'static> {
    Span::styled(" │ ", Style::default().fg(Theme::TEXT_MUTED))
}

/// Format bytes as human readable
fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else {
        format!("{:.0}M", bytes as f64 / MB as f64)
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let line = match &app.status_message {
        Some(msg) => Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Theme::ACCENT)),
            Span::styled(msg.clone(), Style::default().fg(Theme::TEXT)),
        ]),
        None => {
            let mut spans = vec![Span::styled(" ◉  ", Style::default().fg(Theme::ACCENT))];

            // Context-aware hints based on active tab
            match app.active_tab {
                Tab::Processes => {
                    if app.processes_tab.is_output_view() {
                        spans.extend(key_hint("Enter", "Back"));
                        spans.push(separator());
                        spans.extend(key_hint("c", "Clear"));
                        spans.push(separator());
                        spans.extend(key_hint("r", "Restart"));
                    } else {
                        spans.extend(key_hint("j/k", "Navigate"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "View Output"));
                        spans.push(separator());
                        spans.extend(key_hint("s", "Start"));
                        spans.push(separator());
                        spans.extend(key_hint("x", "Stop"));
                        spans.push(separator());
                        spans.extend(key_hint("r", "Restart"));
                        spans.push(separator());
                        spans.extend(key_hint("R", "Restart All"));
                    }
                }
                Tab::Logs => {
                    if app.logs_tab.input_mode {
                        spans.extend(key_hint("Esc", "Exit Search"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Confirm"));
                    } else {
                        spans.extend(key_hint("/", "Search"));
                        spans.push(separator());
                        spans.extend(key_hint("f", "Filter"));
                        spans.push(separator());
                        spans.extend(key_hint("j/k", "Scroll"));
                        spans.push(separator());
                        spans.extend(key_hint("g/G", "Top/Bottom"));
                        spans.push(separator());
                        spans.extend(key_hint("c", "Clear"));
                    }
                }
                Tab::Artisan => {
                    if app.artisan_tab.input_mode || app.artisan_tab.search_mode {
                        spans.extend(key_hint("Esc", "Cancel"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Confirm"));
                    } else {
                        spans.extend(key_hint("j/k", "Command"));
                        spans.push(separator());
                        spans.extend(key_hint("/", "Search"));
                        spans.push(separator());
                        spans.extend(key_hint("i", "Args"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Run"));
                        spans.push(separator());
                        spans.extend(key_hint("c", "Clear"));
                    }
                }
                Tab::Make => {
                    if app.make_tab.input_mode || app.make_tab.search_mode {
                        spans.extend(key_hint("Esc", "Cancel"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Confirm"));
                    } else {
                        spans.extend(key_hint("j/k", "Command"));
                        spans.push(separator());
                        spans.extend(key_hint("/", "Search"));
                        spans.push(separator());
                        spans.extend(key_hint("i", "Name"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Run"));
                        spans.push(separator());
                        spans.extend(key_hint("c", "Clear"));
                    }
                }
                Tab::Quality => {
                    if app.quality_tab.input_mode {
                        spans.extend(key_hint("Esc", "Cancel"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Confirm"));
                    } else {
                        spans.extend(key_hint("h/l", "Category"));
                        spans.push(separator());
                        spans.extend(key_hint("j/k", "Tool"));
                        spans.push(separator());
                        spans.extend(key_hint("i", "Args"));
                        spans.push(separator());
                        spans.extend(key_hint("Enter", "Run"));
                        spans.push(separator());
                        spans.extend(key_hint("c", "Clear"));
                    }
                }
                Tab::Config => {
                    spans.extend(key_hint("j/k", "Navigate"));
                    spans.push(separator());
                    spans.extend(key_hint("Space", "Toggle"));
                    spans.push(separator());
                    spans.extend(key_hint("s", "Save"));
                    spans.push(separator());
                    spans.extend(key_hint("r", "Reset"));
                }
                Tab::About => {
                    spans.extend(key_hint("1-6", "Switch Tab"));
                    spans.push(separator());
                    spans.extend(key_hint("Tab", "Next Tab"));
                }
            }

            // Global shortcuts
            spans.push(separator());
            spans.extend(key_hint("Ctrl+C", "Quit"));

            Line::from(spans)
        }
    };

    // Create the main paragraph
    let paragraph = Paragraph::new(line).style(Style::default().bg(Theme::STATUS_BAR_BG));
    frame.render_widget(paragraph, area);

    // Render system stats on the right side
    let stats = &app.system_stats;
    let cpu_str = format!("CPU:{:>5.1}%", stats.cpu_usage);
    let mem_str = format!(
        "RAM:{}/{}",
        format_bytes(stats.used_memory),
        format_bytes(stats.total_memory)
    );

    let stats_text = format!(" {} │ {} ", cpu_str, mem_str);
    let stats_width = stats_text.len() as u16;

    if area.width > stats_width + 10 {
        let stats_area = Rect {
            x: area.x + area.width.saturating_sub(stats_width),
            y: area.y,
            width: stats_width,
            height: 1,
        };

        let stats_line = Line::from(vec![
            Span::styled(" ", Style::default().bg(Theme::STATUS_BAR_BG)),
            Span::styled(
                cpu_str,
                Style::default()
                    .fg(Theme::TEXT_DIM)
                    .bg(Theme::STATUS_BAR_BG),
            ),
            Span::styled(
                " │ ",
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .bg(Theme::STATUS_BAR_BG),
            ),
            Span::styled(
                mem_str,
                Style::default()
                    .fg(Theme::TEXT_DIM)
                    .bg(Theme::STATUS_BAR_BG),
            ),
            Span::styled(" ", Style::default().bg(Theme::STATUS_BAR_BG)),
        ]);

        let stats_paragraph =
            Paragraph::new(stats_line).style(Style::default().bg(Theme::STATUS_BAR_BG));
        frame.render_widget(stats_paragraph, stats_area);
    }
}
