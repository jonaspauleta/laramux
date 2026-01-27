use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::app::App;
use crate::ui::theme::Theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .padding(Padding::new(4, 4, 2, 2));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "LaraMux",
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{}", VERSION),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "A TUI for managing Laravel development",
            Style::default().fg(Theme::TEXT),
        )]),
        Line::from(vec![Span::styled(
            "processes in a single terminal window.",
            Style::default().fg(Theme::TEXT),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            Style::default().fg(Theme::BORDER),
        )]),
        Line::from(""),
        shortcut_line("1-4, ?", "Switch tabs"),
        shortcut_line("Tab / Shift+Tab", "Next / Previous tab"),
        shortcut_line("j / k  or  ↑ / ↓", "Navigate items"),
        shortcut_line("Enter", "Select / Toggle view"),
        shortcut_line("Ctrl+C", "Quit"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Processes Tab",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            Style::default().fg(Theme::BORDER),
        )]),
        Line::from(""),
        shortcut_line("s", "Start selected process"),
        shortcut_line("x", "Stop selected process"),
        shortcut_line("r", "Restart selected process"),
        shortcut_line("R", "Restart all processes"),
        shortcut_line("c", "Clear output (in output view)"),
        shortcut_line("s/v/q/h/b", "Quick restart by hotkey"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Logs Tab",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            Style::default().fg(Theme::BORDER),
        )]),
        Line::from(""),
        shortcut_line("/", "Focus search"),
        shortcut_line("f", "Cycle filter level"),
        shortcut_line("c", "Clear logs"),
        shortcut_line("g / G", "Go to top / bottom"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Commands Tab",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            Style::default().fg(Theme::BORDER),
        )]),
        Line::from(""),
        shortcut_line("h / l  or  ← / →", "Previous / Next category"),
        shortcut_line("i", "Edit arguments"),
        shortcut_line("Enter", "Run command"),
        shortcut_line("c", "Clear output"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Config Tab",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            Style::default().fg(Theme::BORDER),
        )]),
        Line::from(""),
        shortcut_line("Space / Enter", "Toggle checkbox"),
        shortcut_line("s", "Save configuration"),
        shortcut_line("r", "Reset changes"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "github.com/jonaspauleta/laramux",
            Style::default()
                .fg(Theme::TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}

fn shortcut_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<22}", key),
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(Theme::TEXT_DIM)),
    ])
}
