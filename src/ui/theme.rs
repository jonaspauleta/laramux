use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders},
};

use crate::process::types::ProcessStatus;

/// Claude Code inspired color theme
pub struct Theme;

/// Status symbols for modern TUI display
pub mod symbols {
    pub const RUNNING: &str = "●";
    pub const STOPPED: &str = "○";
    pub const RESTARTING: &str = "↻";
    pub const FAILED: &str = "✗";
    pub const SUPERVISED: &str = "◆";
    pub const SELECTOR: &str = "▶";
}

#[allow(dead_code)]
impl Theme {
    /// Primary accent color - Claude's warm orange
    pub const ACCENT: Color = Color::Rgb(217, 119, 87);

    /// Secondary accent - slightly dimmer
    pub const ACCENT_DIM: Color = Color::Rgb(180, 100, 75);

    /// Tertiary accent for subtle highlights
    pub const ACCENT_SUBTLE: Color = Color::Rgb(140, 80, 60);

    /// Border color - subtle gray
    pub const BORDER: Color = Color::Rgb(75, 85, 99);

    /// Border color for focused/active panels
    pub const BORDER_FOCUSED: Color = Color::Rgb(217, 119, 6);

    /// Border color for inactive panels
    pub const BORDER_INACTIVE: Color = Color::Rgb(55, 65, 75);

    /// Background for selections
    pub const SELECTION_BG: Color = Color::Rgb(55, 65, 81);

    /// Text colors
    pub const TEXT: Color = Color::Rgb(229, 231, 235);
    pub const TEXT_DIM: Color = Color::Rgb(156, 163, 175);
    pub const TEXT_MUTED: Color = Color::Rgb(107, 114, 128);
    pub const TEXT_DISABLED: Color = Color::Rgb(75, 85, 99);

    /// Status colors
    pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
    pub const ERROR: Color = Color::Rgb(239, 68, 68);
    pub const WARNING: Color = Color::Rgb(234, 179, 8);
    pub const INFO: Color = Color::Rgb(96, 165, 250);

    /// Log level colors
    pub const LOG_DEBUG: Color = Color::Rgb(107, 114, 128);
    pub const LOG_INFO: Color = Color::Rgb(34, 197, 94);
    pub const LOG_NOTICE: Color = Color::Rgb(96, 165, 250);
    pub const LOG_WARNING: Color = Color::Rgb(234, 179, 8);
    pub const LOG_ERROR: Color = Color::Rgb(239, 68, 68);
    pub const LOG_CRITICAL: Color = Color::Rgb(239, 68, 68);

    /// Status bar background
    pub const STATUS_BAR_BG: Color = Color::Rgb(31, 41, 55);

    /// Scrollbar colors
    pub const SCROLLBAR_THUMB: Color = Color::Rgb(100, 110, 120);
    pub const SCROLLBAR_TRACK: Color = Color::Rgb(40, 50, 60);

    /// Returns the title style for panels
    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    /// Returns a block with rounded borders and consistent styling
    pub fn default_block(title: &str) -> Block<'_> {
        Block::default()
            .title(Title::from(title).alignment(Alignment::Left))
            .title_style(Self::title_style())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Self::BORDER))
    }

    /// Returns a focused block with highlighted border
    pub fn focused_block(title: &str) -> Block<'_> {
        Block::default()
            .title(Title::from(title).alignment(Alignment::Left))
            .title_style(Self::title_style())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Self::BORDER_FOCUSED))
    }

    /// Supervised status color — blue
    pub const SUPERVISED: Color = Color::Rgb(96, 165, 250);

    /// Returns the status symbol for a process status
    pub fn status_symbol(status: ProcessStatus) -> &'static str {
        match status {
            ProcessStatus::Running => symbols::RUNNING,
            ProcessStatus::Stopped => symbols::STOPPED,
            ProcessStatus::Restarting => symbols::RESTARTING,
            ProcessStatus::Failed => symbols::FAILED,
            ProcessStatus::Supervised => symbols::SUPERVISED,
        }
    }

    /// Returns the style for a process status
    pub fn status_style(status: ProcessStatus) -> Style {
        match status {
            ProcessStatus::Running => Style::default().fg(Self::SUCCESS),
            ProcessStatus::Stopped => Style::default().fg(Self::TEXT_MUTED),
            ProcessStatus::Restarting => Style::default().fg(Self::WARNING),
            ProcessStatus::Failed => Style::default().fg(Self::ERROR),
            ProcessStatus::Supervised => Style::default().fg(Self::SUPERVISED),
        }
    }

    /// Returns the status label for non-stopped states
    pub fn status_label(status: ProcessStatus) -> Option<&'static str> {
        match status {
            ProcessStatus::Running => Some("running"),
            ProcessStatus::Restarting => Some("restarting..."),
            ProcessStatus::Failed => Some("failed"),
            ProcessStatus::Supervised => Some("supervised"),
            ProcessStatus::Stopped => None,
        }
    }
}
