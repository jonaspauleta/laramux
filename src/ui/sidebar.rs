use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState, Padding},
};

use super::theme::{symbols, Theme};
use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .process_order
        .iter()
        .enumerate()
        .map(|(idx, id)| {
            let process = app.processes.get(id);
            let status = process
                .map(|p| p.status)
                .unwrap_or(crate::process::types::ProcessStatus::Stopped);

            let display_name = app.registry.display_name(id);
            let hotkey = app
                .registry
                .hotkey(id)
                .map(|k| format!("[{}]", k))
                .unwrap_or_default();

            let is_selected = idx == app.selected_index;

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
                display_name,
                if is_selected {
                    Style::default()
                        .fg(Theme::TEXT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_DIM)
                },
            ));

            // Hotkey
            spans.push(Span::styled(
                format!(" {}", hotkey),
                Style::default().fg(Theme::TEXT_MUTED),
            ));

            // Status label for non-stopped states
            if let Some(label) = Theme::status_label(status) {
                spans.push(Span::styled(
                    format!(" {}", label),
                    Theme::status_style(status).add_modifier(Modifier::ITALIC),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let block = Theme::default_block(" Processes ").padding(Padding::horizontal(1));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Theme::SELECTION_BG));

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}
