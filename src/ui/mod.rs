mod layout;
mod logstream;
mod output;
mod sidebar;
mod statusbar;

pub use layout::AppLayout;

use ratatui::prelude::*;

use crate::app::App;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let layout = AppLayout::new(frame.area());

    sidebar::render(frame, layout.sidebar, app);
    output::render(frame, layout.output, app);
    logstream::render(frame, layout.logs, app);
    statusbar::render(frame, layout.status_bar, app);
}
