mod about;
mod artisan;
mod config;
mod logs;
mod make;
mod processes;
mod quality;

pub use about::render as render_about;
pub use artisan::render as render_artisan;
pub use config::render as render_config;
pub use logs::render as render_logs;
pub use make::render as render_make;
pub use processes::render as render_processes;
pub use quality::render as render_quality;

/// The available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Processes,
    Logs,
    Artisan,
    Make,
    Quality,
    Config,
    About,
}

impl Tab {
    /// Get all tabs in display order
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Processes,
            Tab::Logs,
            Tab::Artisan,
            Tab::Make,
            Tab::Quality,
            Tab::Config,
            Tab::About,
        ]
    }

    /// Get the display name for the tab
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Processes => "Processes",
            Tab::Logs => "Logs",
            Tab::Artisan => "Artisan",
            Tab::Make => "Make",
            Tab::Quality => "Quality",
            Tab::Config => "Config",
            Tab::About => "About",
        }
    }

    /// Get the keyboard shortcut for the tab
    pub fn shortcut(&self) -> &'static str {
        match self {
            Tab::Processes => "1",
            Tab::Logs => "2",
            Tab::Artisan => "3",
            Tab::Make => "4",
            Tab::Quality => "5",
            Tab::Config => "6",
            Tab::About => "?",
        }
    }

    /// Get the next tab (wraps around)
    pub fn next(&self) -> Tab {
        match self {
            Tab::Processes => Tab::Logs,
            Tab::Logs => Tab::Artisan,
            Tab::Artisan => Tab::Make,
            Tab::Make => Tab::Quality,
            Tab::Quality => Tab::Config,
            Tab::Config => Tab::About,
            Tab::About => Tab::Processes,
        }
    }

    /// Get the previous tab (wraps around)
    pub fn previous(&self) -> Tab {
        match self {
            Tab::Processes => Tab::About,
            Tab::Logs => Tab::Processes,
            Tab::Artisan => Tab::Logs,
            Tab::Make => Tab::Artisan,
            Tab::Quality => Tab::Make,
            Tab::Config => Tab::Quality,
            Tab::About => Tab::Config,
        }
    }
}
