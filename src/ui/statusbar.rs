use ratatui::{prelude::*, widgets::Paragraph};

use super::theme::Theme;
use crate::app::App;

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

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let line = match &app.status_message {
        Some(msg) => Line::from(vec![
            Span::styled(" ● ", Style::default().fg(Theme::ACCENT)),
            Span::styled(msg.clone(), Style::default().fg(Theme::TEXT)),
        ]),
        None => {
            let mut spans = vec![Span::styled(" ◉  ", Style::default().fg(Theme::ACCENT))];

            // Navigation
            spans.extend(key_hint("↑↓", "Navigate"));
            spans.push(separator());

            // Process hotkeys
            spans.extend(key_hint("q", "Queue"));
            spans.push(separator());
            spans.extend(key_hint("v", "Vite"));
            spans.push(separator());
            spans.extend(key_hint("s", "Serve"));
            spans.push(separator());
            spans.extend(key_hint("h", "Horizon"));
            spans.push(separator());
            spans.extend(key_hint("b", "Reverb"));
            spans.push(separator());

            // Actions
            spans.extend(key_hint("r", "Restart"));
            spans.push(separator());
            spans.extend(key_hint("c", "Clear"));
            spans.push(separator());
            spans.extend(key_hint("Ctrl+C", "Quit"));

            Line::from(spans)
        }
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(Theme::STATUS_BAR_BG));

    frame.render_widget(paragraph, area);
}
