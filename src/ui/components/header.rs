use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs},
};

use crate::app::App;
use crate::ui::tabs::Tab;
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|tab| {
            let shortcut = tab.shortcut();
            let name = tab.name();
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", shortcut),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::raw(name),
            ])
        })
        .collect();

    let selected_index = Tab::all()
        .iter()
        .position(|t| *t == app.active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
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
