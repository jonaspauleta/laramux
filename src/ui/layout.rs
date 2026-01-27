use ratatui::prelude::*;

/// Layout structure for the tab-based design
pub struct TabLayout {
    /// Header area for tab navigation
    pub header: Rect,
    /// Main content area for the active tab
    pub content: Rect,
    /// Status bar at the bottom
    pub status_bar: Rect,
}

impl TabLayout {
    /// Create the application layout from the terminal frame
    /// Layout: Header (2 lines) | Content (flexible) | Status Bar (1 line)
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header with tabs
                Constraint::Min(0),    // Content area
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        Self {
            header: vertical[0],
            content: vertical[1],
            status_bar: vertical[2],
        }
    }
}

// Keep the old layout for backward compatibility if needed
#[allow(dead_code)]
pub struct AppLayout {
    pub sidebar: Rect,
    pub output: Rect,
    pub logs: Rect,
    pub status_bar: Rect,
}

#[allow(dead_code)]
impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let main_area = vertical[0];
        let status_bar = vertical[1];

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(1),
                Constraint::Percentage(80),
            ])
            .split(main_area);

        let sidebar = horizontal[0];
        let content_area = horizontal[2];

        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Length(1),
                Constraint::Percentage(30),
            ])
            .split(content_area);

        let output = content[0];
        let logs = content[2];

        Self {
            sidebar,
            output,
            logs,
            status_bar,
        }
    }
}
