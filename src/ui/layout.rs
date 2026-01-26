use ratatui::prelude::*;

/// Layout structure for the 3-pane design
pub struct AppLayout {
    pub sidebar: Rect,
    pub output: Rect,
    pub logs: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    /// Create the application layout from the terminal frame
    /// Layout: 20% sidebar | 80% main (70% output / 30% logs)
    pub fn new(area: Rect) -> Self {
        // Split vertically: main area and status bar
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let main_area = vertical[0];
        let status_bar = vertical[1];

        // Split horizontally: sidebar and content
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(main_area);

        let sidebar = horizontal[0];
        let content_area = horizontal[1];

        // Split content vertically: output and logs
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(content_area);

        let output = content[0];
        let logs = content[1];

        Self {
            sidebar,
            output,
            logs,
            status_bar,
        }
    }
}
