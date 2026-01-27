mod components;
mod layout;
pub mod tabs;
pub mod theme;

pub use layout::TabLayout;

use ratatui::prelude::*;

use crate::app::App;
use tabs::Tab;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let layout = TabLayout::new(frame.area());

    // Render header with tabs
    components::render_header(frame, layout.header, app);

    // Render active tab content
    match app.active_tab {
        Tab::Processes => tabs::render_processes(frame, layout.content, app),
        Tab::Logs => tabs::render_logs(frame, layout.content, app),
        Tab::Artisan => tabs::render_artisan(frame, layout.content, app),
        Tab::Make => tabs::render_make(frame, layout.content, app),
        Tab::Quality => tabs::render_quality(frame, layout.content, app),
        Tab::Config => tabs::render_config(frame, layout.content, app),
        Tab::About => tabs::render_about(frame, layout.content, app),
    }

    // Render status bar
    components::render_statusbar(frame, layout.status_bar, app);
}
