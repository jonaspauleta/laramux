use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .process_order
        .iter()
        .map(|id| {
            let process = app.processes.get(id);
            let (indicator, status_text) = match process {
                Some(p) => (p.status.indicator(), p.status),
                None => ("⚫", crate::process::types::ProcessStatus::Stopped),
            };

            let display_name = app.registry.display_name(id);
            let hotkey = app
                .registry
                .hotkey(id)
                .map(|k| format!("[{}]", k))
                .unwrap_or_default();

            let content = format!("{} {} {}", indicator, display_name, hotkey);

            let style = match status_text {
                crate::process::types::ProcessStatus::Running => Style::default().fg(Color::Green),
                crate::process::types::ProcessStatus::Failed => Style::default().fg(Color::Red),
                crate::process::types::ProcessStatus::Restarting => {
                    Style::default().fg(Color::Yellow)
                }
                crate::process::types::ProcessStatus::Stopped => {
                    Style::default().fg(Color::DarkGray)
                }
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Processes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}
