use ratatui::prelude::*;

/// Layout structure for the 3-pane design with gaps
pub struct AppLayout {
    pub sidebar: Rect,
    pub output: Rect,
    pub logs: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    /// Create the application layout from the terminal frame
    /// Layout: 20% sidebar | gap | 80% main (70% output / gap / 30% logs)
    pub fn new(area: Rect) -> Self {
        // Split vertically: main area and status bar
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let main_area = vertical[0];
        let status_bar = vertical[1];

        // Split horizontally: sidebar, gap, and content
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(1), // Gap between sidebar and content
                Constraint::Percentage(80),
            ])
            .split(main_area);

        let sidebar = horizontal[0];
        // horizontal[1] is the gap
        let content_area = horizontal[2];

        // Split content vertically: output, gap, and logs
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Length(1), // Gap between output and logs
                Constraint::Percentage(30),
            ])
            .split(content_area);

        let output = content[0];
        // content[1] is the gap
        let logs = content[2];

        Self {
            sidebar,
            output,
            logs,
            status_bar,
        }
    }
}
