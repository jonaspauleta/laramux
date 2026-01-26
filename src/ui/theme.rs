use ratatui::prelude::*;

/// Claude Code inspired color theme
pub struct Theme;

impl Theme {
    /// Primary accent color - Claude's warm orange
    pub const ACCENT: Color = Color::Rgb(217, 119, 87);

    /// Secondary accent - slightly dimmer
    pub const ACCENT_DIM: Color = Color::Rgb(180, 100, 75);

    /// Border color - subtle gray
    pub const BORDER: Color = Color::Rgb(75, 85, 99);

    /// Border color for focused/active panels
    pub const BORDER_FOCUSED: Color = Color::Rgb(217, 119, 6);

    /// Background for selections
    pub const SELECTION_BG: Color = Color::Rgb(55, 65, 81);

    /// Text colors
    pub const TEXT: Color = Color::Rgb(229, 231, 235);
    pub const TEXT_DIM: Color = Color::Rgb(156, 163, 175);
    pub const TEXT_MUTED: Color = Color::Rgb(107, 114, 128);

    /// Status colors
    pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
    pub const ERROR: Color = Color::Rgb(239, 68, 68);
    pub const WARNING: Color = Color::Rgb(234, 179, 8);
    pub const INFO: Color = Color::Rgb(96, 165, 250);

    /// Status bar background
    pub const STATUS_BAR_BG: Color = Color::Rgb(31, 41, 55);
}
