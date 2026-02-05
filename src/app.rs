#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::config::LaramuxConfig;
use crate::process::types::{
    OutputLine, Process, ProcessConfig, ProcessId, ProcessRegistry, ProcessStatus,
};
use crate::process::{FullArtisanCommand, QualityTool};
use crate::ui::tabs::Tab;

/// Default maximum number of log lines to display
pub const DEFAULT_MAX_LOG_LINES: usize = 100;

/// System resource statistics
#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    /// Overall CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// Overall memory usage percentage (0-100)
    pub memory_usage: f32,
    /// Total memory in bytes
    pub total_memory: u64,
    /// Used memory in bytes
    pub used_memory: u64,
    /// Per-process stats keyed by PID
    pub process_stats: HashMap<u32, ProcessStats>,
}

/// Per-process resource statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessStats {
    /// CPU usage percentage for this process
    pub cpu_usage: f32,
    /// Memory usage in bytes for this process
    pub memory_bytes: u64,
}

/// A line from a log file
#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
    pub level: LogLevel,
    pub file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
    Unknown,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "notice" => LogLevel::Notice,
            "warning" => LogLevel::Warning,
            "error" => LogLevel::Error,
            "critical" => LogLevel::Critical,
            "alert" => LogLevel::Alert,
            "emergency" => LogLevel::Emergency,
            _ => LogLevel::Unknown,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            LogLevel::Error | LogLevel::Critical | LogLevel::Alert | LogLevel::Emergency
        )
    }

    /// Get display name for the level
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Notice => "Notice",
            LogLevel::Warning => "Warning",
            LogLevel::Error => "Error",
            LogLevel::Critical => "Critical",
            LogLevel::Alert => "Alert",
            LogLevel::Emergency => "Emergency",
            LogLevel::Unknown => "Unknown",
        }
    }

    /// All log levels for filtering
    pub fn all() -> &'static [LogLevel] {
        &[
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Notice,
            LogLevel::Warning,
            LogLevel::Error,
            LogLevel::Critical,
            LogLevel::Alert,
            LogLevel::Emergency,
        ]
    }

    /// Get next filter level (cycles through)
    pub fn next_filter(&self) -> Option<LogLevel> {
        match self {
            LogLevel::Debug => Some(LogLevel::Info),
            LogLevel::Info => Some(LogLevel::Notice),
            LogLevel::Notice => Some(LogLevel::Warning),
            LogLevel::Warning => Some(LogLevel::Error),
            LogLevel::Error => Some(LogLevel::Critical),
            LogLevel::Critical => None, // Return to "All"
            _ => Some(LogLevel::Debug),
        }
    }
}

// ============================================================================
// Processes Tab State
// ============================================================================

/// View mode for the Processes tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessesView {
    #[default]
    List,
    Output,
}

/// State for the Processes tab
#[derive(Debug, Default)]
pub struct ProcessesTabState {
    pub view: ProcessesView,
    pub selected_index: usize,
    pub output_scroll_offset: usize,
}

impl ProcessesTabState {
    pub fn is_output_view(&self) -> bool {
        self.view == ProcessesView::Output
    }

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            ProcessesView::List => ProcessesView::Output,
            ProcessesView::Output => ProcessesView::List,
        };
    }
}

// ============================================================================
// Logs Tab State
// ============================================================================

/// State for the Logs tab
#[derive(Debug, Default)]
pub struct LogsTabState {
    pub search_query: String,
    pub filter_level: Option<LogLevel>,
    pub scroll_offset: usize,
    pub input_mode: bool,
    pub selected_file: Option<String>,
    pub available_files: Vec<String>,
}

impl LogsTabState {
    pub fn cycle_filter(&mut self) {
        self.filter_level = match self.filter_level {
            None => Some(LogLevel::Debug),
            Some(level) => level.next_filter(),
        };
    }

    pub fn filter_name(&self) -> &'static str {
        match self.filter_level {
            None => "All",
            Some(level) => level.name(),
        }
    }

    pub fn cycle_file(&mut self) {
        if self.available_files.is_empty() {
            return;
        }

        self.selected_file = match &self.selected_file {
            None => Some(self.available_files[0].clone()),
            Some(current) => {
                let current_idx = self
                    .available_files
                    .iter()
                    .position(|f| f == current)
                    .unwrap_or(0);
                let next_idx = current_idx + 1;
                if next_idx >= self.available_files.len() {
                    None // Back to "All"
                } else {
                    Some(self.available_files[next_idx].clone())
                }
            }
        };
    }

    pub fn file_name(&self) -> &str {
        match &self.selected_file {
            None => "All",
            Some(file) => file,
        }
    }
}

// ============================================================================
// Artisan Tab State
// ============================================================================

/// A resolved command ready to execute
#[derive(Debug, Clone)]
pub struct ResolvedCommand {
    pub display_name: String,
    pub command: String,
    pub args: Vec<String>,
}

/// State for the Artisan tab
#[derive(Debug, Default)]
pub struct ArtisanTabState {
    pub selected_command: usize,
    pub input_buffer: String,
    pub input_mode: bool,
    pub command_output: VecDeque<OutputLine>,
    pub output_scroll_offset: usize,
    pub running_command: Option<String>,
    pub artisan_commands: Vec<FullArtisanCommand>,
    pub search_query: String,
    pub search_mode: bool,
    pub details_scroll_offset: usize,
}

impl ArtisanTabState {
    fn filtered_commands_with_favorites<'a>(
        &'a self,
        favorites: &[String],
    ) -> Vec<(&'a FullArtisanCommand, bool)> {
        let mut commands: Vec<_> = if self.search_query.is_empty() {
            self.artisan_commands.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.artisan_commands
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().contains(&query)
                        || cmd.description.to_lowercase().contains(&query)
                })
                .collect()
        };

        // Sort: favorites first, then alphabetically
        commands.sort_by(|a, b| {
            let a_fav = favorites.contains(&a.name);
            let b_fav = favorites.contains(&b.name);
            match (a_fav, b_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        commands
            .into_iter()
            .map(|cmd| (cmd, favorites.contains(&cmd.name)))
            .collect()
    }

    pub fn command_count(&self, favorites: &[String]) -> usize {
        self.filtered_commands_with_favorites(favorites).len()
    }

    pub fn selected_artisan_command(&self, favorites: &[String]) -> Option<&FullArtisanCommand> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        filtered.get(self.selected_command).map(|(cmd, _)| *cmd)
    }

    /// Returns (name, description, is_favorite)
    pub fn current_command_display(&self, favorites: &[String]) -> Vec<(String, String, bool)> {
        self.filtered_commands_with_favorites(favorites)
            .iter()
            .map(|(cmd, is_fav)| (cmd.name.clone(), cmd.description.clone(), *is_fav))
            .collect()
    }

    pub fn selected_command_resolved(
        &self,
        user_args: &str,
        favorites: &[String],
        is_sail: bool,
    ) -> Option<ResolvedCommand> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        let (cmd, _) = filtered.get(self.selected_command)?;

        let (command, mut args) = if is_sail {
            (
                "./vendor/bin/sail".to_string(),
                vec!["artisan".to_string(), cmd.name.clone()],
            )
        } else {
            (
                "php".to_string(),
                vec!["artisan".to_string(), cmd.name.clone()],
            )
        };

        if !user_args.is_empty() {
            for arg in user_args.split_whitespace() {
                args.push(arg.to_string());
            }
        }

        args.push("--ansi".to_string());

        Some(ResolvedCommand {
            display_name: format!("artisan {}", cmd.name),
            command,
            args,
        })
    }

    /// Get the command name for the currently selected command (for toggling favorites)
    pub fn selected_command_name(&self, favorites: &[String]) -> Option<String> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        filtered
            .get(self.selected_command)
            .map(|(cmd, _)| cmd.name.clone())
    }

    pub fn add_output(&mut self, line: OutputLine) {
        if self.command_output.len() >= 1000 {
            self.command_output.pop_front();
        }
        self.command_output.push_back(line);
    }

    pub fn clear_output(&mut self) {
        self.command_output.clear();
        self.output_scroll_offset = 0;
    }
}

// ============================================================================
// Make Tab State
// ============================================================================

/// State for the Make tab
#[derive(Debug, Default)]
pub struct MakeTabState {
    pub selected_command: usize,
    pub input_buffer: String,
    pub input_mode: bool,
    pub command_output: VecDeque<OutputLine>,
    pub output_scroll_offset: usize,
    pub running_command: Option<String>,
    pub make_commands: Vec<FullArtisanCommand>,
    pub search_query: String,
    pub search_mode: bool,
    pub details_scroll_offset: usize,
}

impl MakeTabState {
    fn filtered_commands_with_favorites<'a>(
        &'a self,
        favorites: &[String],
    ) -> Vec<(&'a FullArtisanCommand, bool)> {
        let mut commands: Vec<_> = if self.search_query.is_empty() {
            self.make_commands.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.make_commands
                .iter()
                .filter(|cmd| {
                    cmd.name.to_lowercase().contains(&query)
                        || cmd.description.to_lowercase().contains(&query)
                })
                .collect()
        };

        // Sort: favorites first, then alphabetically
        commands.sort_by(|a, b| {
            let a_fav = favorites.contains(&a.name);
            let b_fav = favorites.contains(&b.name);
            match (a_fav, b_fav) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        commands
            .into_iter()
            .map(|cmd| (cmd, favorites.contains(&cmd.name)))
            .collect()
    }

    pub fn command_count(&self, favorites: &[String]) -> usize {
        self.filtered_commands_with_favorites(favorites).len()
    }

    pub fn selected_make_command(&self, favorites: &[String]) -> Option<&FullArtisanCommand> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        filtered.get(self.selected_command).map(|(cmd, _)| *cmd)
    }

    /// Returns (display_name, full_command, is_favorite)
    pub fn current_command_display(
        &self,
        favorites: &[String],
        is_sail: bool,
    ) -> Vec<(String, String, bool)> {
        self.filtered_commands_with_favorites(favorites)
            .iter()
            .map(|(cmd, is_fav)| {
                let display_name = cmd
                    .name
                    .strip_prefix("make:")
                    .map(|s| {
                        let mut chars = s.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        }
                    })
                    .unwrap_or_else(|| cmd.name.clone());
                let full_command = if is_sail {
                    format!("sail artisan {}", cmd.name)
                } else {
                    format!("php artisan {}", cmd.name)
                };
                (display_name, full_command, *is_fav)
            })
            .collect()
    }

    pub fn selected_command_resolved(
        &self,
        user_args: &str,
        favorites: &[String],
        is_sail: bool,
    ) -> Option<ResolvedCommand> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        let (cmd, _) = filtered.get(self.selected_command)?;

        let (command, mut args) = if is_sail {
            (
                "./vendor/bin/sail".to_string(),
                vec!["artisan".to_string(), cmd.name.clone()],
            )
        } else {
            (
                "php".to_string(),
                vec!["artisan".to_string(), cmd.name.clone()],
            )
        };

        if !user_args.is_empty() {
            for arg in user_args.split_whitespace() {
                args.push(arg.to_string());
            }
        }

        args.push("--ansi".to_string());

        let display_name = cmd
            .name
            .strip_prefix("make:")
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .unwrap_or_else(|| cmd.name.clone());

        Some(ResolvedCommand {
            display_name,
            command,
            args,
        })
    }

    /// Get the command name for the currently selected command (for toggling favorites)
    pub fn selected_command_name(&self, favorites: &[String]) -> Option<String> {
        let filtered = self.filtered_commands_with_favorites(favorites);
        filtered
            .get(self.selected_command)
            .map(|(cmd, _)| cmd.name.clone())
    }

    pub fn add_output(&mut self, line: OutputLine) {
        if self.command_output.len() >= 1000 {
            self.command_output.pop_front();
        }
        self.command_output.push_back(line);
    }

    pub fn clear_output(&mut self) {
        self.command_output.clear();
        self.output_scroll_offset = 0;
    }
}

// ============================================================================
// Quality Tab State
// ============================================================================

/// Category within the Quality tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityCategory {
    #[default]
    QualityTools,
    Testing,
}

impl QualityCategory {
    pub fn all() -> &'static [QualityCategory] {
        &[QualityCategory::QualityTools, QualityCategory::Testing]
    }

    pub fn name(&self) -> &'static str {
        match self {
            QualityCategory::QualityTools => "Quality Tools",
            QualityCategory::Testing => "Testing",
        }
    }

    pub fn next(&self) -> QualityCategory {
        match self {
            QualityCategory::QualityTools => QualityCategory::Testing,
            QualityCategory::Testing => QualityCategory::QualityTools,
        }
    }

    pub fn previous(&self) -> QualityCategory {
        self.next()
    }
}

/// State for the Quality tab (quality tools + testing)
#[derive(Debug, Default)]
pub struct QualityTabState {
    pub selected_category: QualityCategory,
    pub selected_tool: usize,
    pub input_buffer: String,
    pub input_mode: bool,
    pub command_output: VecDeque<OutputLine>,
    pub output_scroll_offset: usize,
    pub running_command: Option<String>,
    pub quality_tools: Vec<QualityTool>,
    pub testing_tools: Vec<QualityTool>,
    pub details_scroll_offset: usize,
}

impl QualityTabState {
    pub fn current_tools(&self) -> &[QualityTool] {
        match self.selected_category {
            QualityCategory::QualityTools => &self.quality_tools,
            QualityCategory::Testing => &self.testing_tools,
        }
    }

    pub fn tool_count(&self) -> usize {
        self.current_tools().len()
    }

    pub fn selected_tool_item(&self) -> Option<&QualityTool> {
        self.current_tools().get(self.selected_tool)
    }

    pub fn selected_command_resolved(&self, user_args: &str) -> Option<ResolvedCommand> {
        let tool = self.selected_tool_item()?;

        let (non_flags, flags): (Vec<_>, Vec<_>) =
            tool.args.iter().partition(|arg| !arg.starts_with('-'));

        let mut args: Vec<String> = non_flags.into_iter().cloned().collect();

        if !user_args.is_empty() {
            for arg in user_args.split_whitespace() {
                args.push(arg.to_string());
            }
        }

        args.extend(flags.into_iter().cloned());

        Some(ResolvedCommand {
            display_name: tool.display_name.clone(),
            command: tool.command.clone(),
            args,
        })
    }

    pub fn add_output(&mut self, line: OutputLine) {
        if self.command_output.len() >= 1000 {
            self.command_output.pop_front();
        }
        self.command_output.push_back(line);
    }

    pub fn clear_output(&mut self) {
        self.command_output.clear();
        self.output_scroll_offset = 0;
    }
}

// ============================================================================
// Config Tab State
// ============================================================================

use crate::config::{CustomProcess, CustomTool, OverrideConfig, RestartPolicy};

/// Available configuration sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigSection {
    #[default]
    Disabled,
    Overrides,
    Custom,
}

impl ConfigSection {
    pub fn all() -> &'static [ConfigSection] {
        &[
            ConfigSection::Disabled,
            ConfigSection::Overrides,
            ConfigSection::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ConfigSection::Disabled => "Disabled",
            ConfigSection::Overrides => "Overrides",
            ConfigSection::Custom => "Custom",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            ConfigSection::Disabled => 0,
            ConfigSection::Overrides => 1,
            ConfigSection::Custom => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ConfigSection::Disabled,
            1 => ConfigSection::Overrides,
            2 => ConfigSection::Custom,
            _ => ConfigSection::Disabled,
        }
    }
}

/// Which panel has focus in the Config tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigFocus {
    #[default]
    Sections,
    Details,
}

/// Edit mode for Config tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigEditMode {
    #[default]
    Browse,
    EditText,
    SelectOption,
    Confirm,
}

/// Detail view mode - whether viewing item list or item fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigDetailView {
    #[default]
    ItemList, // Navigating the list of items
    ItemFields, // Navigating fields within a selected item
}

/// Sub-section within Quality section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualitySubSection {
    #[default]
    DisabledTools,
    CustomTools,
    DefaultArgs,
}

impl QualitySubSection {
    pub fn name(&self) -> &'static str {
        match self {
            QualitySubSection::DisabledTools => "Disabled Tools",
            QualitySubSection::CustomTools => "Custom Tools",
            QualitySubSection::DefaultArgs => "Default Args",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            QualitySubSection::DisabledTools => QualitySubSection::CustomTools,
            QualitySubSection::CustomTools => QualitySubSection::DefaultArgs,
            QualitySubSection::DefaultArgs => QualitySubSection::DisabledTools,
        }
    }
}

/// State for the Config tab
#[derive(Debug, Default)]
pub struct ConfigTabState {
    pub config_draft: Option<ConfigDraft>,
    pub section: ConfigSection,
    pub focus: ConfigFocus,
    pub selected_item: usize,
    pub scroll_offset: usize,
    pub edit_mode: ConfigEditMode,
    pub edit_buffer: String,
    pub edit_field: usize,
    pub has_changes: bool,
    pub error: Option<String>,
    pub confirm_delete: Option<usize>,
    pub detail_view: ConfigDetailView,
    /// For enum selection mode - the currently selected option index
    pub enum_selection: usize,
}

impl ConfigTabState {
    pub fn is_editing(&self) -> bool {
        matches!(self.edit_mode, ConfigEditMode::EditText)
    }

    pub fn is_selecting(&self) -> bool {
        matches!(self.edit_mode, ConfigEditMode::SelectOption)
    }

    pub fn is_field_view(&self) -> bool {
        self.detail_view == ConfigDetailView::ItemFields
    }
}

/// Editable copy of disabled flags
#[derive(Debug, Clone, Default)]
pub struct DisabledDraft {
    pub serve: bool,
    pub vite: bool,
    pub queue: bool,
    pub horizon: bool,
    pub reverb: bool,
}

impl DisabledDraft {
    pub fn items(&self) -> [(&'static str, bool); 5] {
        [
            ("Serve", self.serve),
            ("Vite", self.vite),
            ("Queue", self.queue),
            ("Horizon", self.horizon),
            ("Reverb", self.reverb),
        ]
    }

    pub fn toggle(&mut self, index: usize) {
        match index {
            0 => self.serve = !self.serve,
            1 => self.vite = !self.vite,
            2 => self.queue = !self.queue,
            3 => self.horizon = !self.horizon,
            4 => self.reverb = !self.reverb,
            _ => {}
        }
    }
}

/// Editable copy of an override config
#[derive(Debug, Clone, Default)]
pub struct OverrideDraft {
    pub command: String,
    pub args: String,
    pub working_dir: String,
    pub env: Vec<(String, String)>,
    pub restart_policy: RestartPolicy,
}

impl OverrideDraft {
    pub fn from_override(cfg: &OverrideConfig) -> Self {
        Self {
            command: cfg.command.clone().unwrap_or_default(),
            args: cfg.args.clone().map(|a| a.join(" ")).unwrap_or_default(),
            working_dir: cfg.working_dir.clone().unwrap_or_default(),
            env: cfg
                .env
                .clone()
                .map(|e| e.into_iter().collect())
                .unwrap_or_default(),
            restart_policy: cfg.restart_policy.unwrap_or_default(),
        }
    }

    pub fn to_override(&self) -> Option<OverrideConfig> {
        // Only create override if something is actually set
        if self.command.is_empty()
            && self.args.is_empty()
            && self.working_dir.is_empty()
            && self.env.is_empty()
            && self.restart_policy == RestartPolicy::Never
        {
            return None;
        }

        Some(OverrideConfig {
            command: if self.command.is_empty() {
                None
            } else {
                Some(self.command.clone())
            },
            args: if self.args.is_empty() {
                None
            } else {
                Some(self.args.split_whitespace().map(String::from).collect())
            },
            working_dir: if self.working_dir.is_empty() {
                None
            } else {
                Some(self.working_dir.clone())
            },
            env: if self.env.is_empty() {
                None
            } else {
                Some(self.env.iter().cloned().collect())
            },
            restart_policy: if self.restart_policy == RestartPolicy::Never {
                None
            } else {
                Some(self.restart_policy)
            },
        })
    }

    pub fn is_empty(&self) -> bool {
        self.command.is_empty()
            && self.args.is_empty()
            && self.working_dir.is_empty()
            && self.env.is_empty()
            && self.restart_policy == RestartPolicy::Never
    }
}

/// Editable copy of a custom process
#[derive(Debug, Clone, Default)]
pub struct CustomProcessDraft {
    pub name: String,
    pub display_name: String,
    pub command: String,
    pub args: String,
    pub hotkey: String,
    pub enabled: bool,
    pub working_dir: String,
    pub env: Vec<(String, String)>,
    pub restart_policy: RestartPolicy,
}

impl CustomProcessDraft {
    pub fn from_custom(cp: &CustomProcess) -> Self {
        Self {
            name: cp.name.clone(),
            display_name: cp.display_name.clone(),
            command: cp.command.clone(),
            args: cp.args.join(" "),
            hotkey: cp.hotkey.map(|c| c.to_string()).unwrap_or_default(),
            enabled: cp.enabled,
            working_dir: cp.working_dir.clone().unwrap_or_default(),
            env: cp
                .env
                .clone()
                .map(|e| e.into_iter().collect())
                .unwrap_or_default(),
            restart_policy: cp.restart_policy.unwrap_or_default(),
        }
    }

    pub fn to_custom(&self) -> CustomProcess {
        CustomProcess {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            command: self.command.clone(),
            args: if self.args.is_empty() {
                vec![]
            } else {
                self.args.split_whitespace().map(String::from).collect()
            },
            hotkey: self.hotkey.chars().next(),
            enabled: self.enabled,
            working_dir: if self.working_dir.is_empty() {
                None
            } else {
                Some(self.working_dir.clone())
            },
            env: if self.env.is_empty() {
                None
            } else {
                Some(self.env.iter().cloned().collect())
            },
            restart_policy: if self.restart_policy == RestartPolicy::Never {
                None
            } else {
                Some(self.restart_policy)
            },
        }
    }

    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }
}

/// Editable copy of a custom tool
#[derive(Debug, Clone, Default)]
pub struct CustomToolDraft {
    pub name: String,
    pub display_name: String,
    pub command: String,
    pub args: String,
    pub category: String,
}

impl CustomToolDraft {
    pub fn from_tool(tool: &CustomTool) -> Self {
        Self {
            name: tool.name.clone(),
            display_name: tool.display_name.clone(),
            command: tool.command.clone(),
            args: tool.args.join(" "),
            category: tool.category.clone(),
        }
    }

    pub fn to_tool(&self) -> CustomTool {
        CustomTool {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            command: self.command.clone(),
            args: if self.args.is_empty() {
                vec![]
            } else {
                self.args.split_whitespace().map(String::from).collect()
            },
            category: self.category.clone(),
        }
    }

    pub fn new_quality() -> Self {
        Self {
            category: "quality".to_string(),
            ..Default::default()
        }
    }
}

/// Editable copy of logs config
#[derive(Debug, Clone, Default)]
pub struct LogsDraft {
    pub max_lines: String,
    pub files: Vec<String>,
    pub default_filter: String,
}

/// Editable copy of quality config
#[derive(Debug, Clone, Default)]
pub struct QualityDraft {
    pub disabled_tools: Vec<String>,
    pub custom_tools: Vec<CustomToolDraft>,
    pub default_args: Vec<(String, String)>,
}

/// Complete editable copy of the configuration
#[derive(Debug, Clone, Default)]
pub struct ConfigDraft {
    pub sail: Option<bool>,
    pub disabled: DisabledDraft,
    pub overrides: HashMap<String, OverrideDraft>,
    pub custom: Vec<CustomProcessDraft>,
    pub quality: QualityDraft,
    pub logs: LogsDraft,
    pub artisan_favorites: Vec<String>,
    pub make_favorites: Vec<String>,
}

impl ConfigDraft {
    pub fn from_config(config: Option<&LaramuxConfig>) -> Self {
        match config {
            Some(cfg) => Self {
                sail: cfg.sail,
                disabled: DisabledDraft {
                    serve: cfg.disabled.serve,
                    vite: cfg.disabled.vite,
                    queue: cfg.disabled.queue,
                    horizon: cfg.disabled.horizon,
                    reverb: cfg.disabled.reverb,
                },
                overrides: cfg
                    .overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), OverrideDraft::from_override(v)))
                    .collect(),
                custom: cfg
                    .custom
                    .iter()
                    .map(CustomProcessDraft::from_custom)
                    .collect(),
                quality: QualityDraft {
                    disabled_tools: cfg.quality.disabled_tools.clone(),
                    custom_tools: cfg
                        .quality
                        .custom_tools
                        .iter()
                        .map(CustomToolDraft::from_tool)
                        .collect(),
                    default_args: cfg
                        .quality
                        .default_args
                        .iter()
                        .map(|(k, v)| (k.clone(), v.join(" ")))
                        .collect(),
                },
                logs: LogsDraft {
                    max_lines: cfg
                        .logs
                        .max_lines
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    files: cfg.logs.files.clone().unwrap_or_default(),
                    default_filter: cfg.logs.default_filter.clone().unwrap_or_default(),
                },
                artisan_favorites: cfg.artisan.favorites.clone(),
                make_favorites: cfg.make.favorites.clone(),
            },
            None => Self::default(),
        }
    }

    pub fn to_config(&self) -> LaramuxConfig {
        use crate::config::{ArtisanConfig, DisabledConfig, LogConfig, MakeConfig, QualityConfig};

        LaramuxConfig {
            sail: self.sail,
            disabled: DisabledConfig {
                serve: self.disabled.serve,
                vite: self.disabled.vite,
                queue: self.disabled.queue,
                horizon: self.disabled.horizon,
                reverb: self.disabled.reverb,
            },
            overrides: self
                .overrides
                .iter()
                .filter_map(|(k, v)| v.to_override().map(|o| (k.clone(), o)))
                .collect(),
            custom: self.custom.iter().map(|c| c.to_custom()).collect(),
            quality: QualityConfig {
                disabled_tools: self.quality.disabled_tools.clone(),
                custom_tools: self
                    .quality
                    .custom_tools
                    .iter()
                    .map(|t| t.to_tool())
                    .collect(),
                default_args: self
                    .quality
                    .default_args
                    .iter()
                    .map(|(k, v)| (k.clone(), v.split_whitespace().map(String::from).collect()))
                    .collect(),
            },
            logs: LogConfig {
                max_lines: self.logs.max_lines.parse().ok(),
                files: if self.logs.files.is_empty() {
                    None
                } else {
                    Some(self.logs.files.clone())
                },
                default_filter: if self.logs.default_filter.is_empty() {
                    None
                } else {
                    Some(self.logs.default_filter.clone())
                },
            },
            artisan: ArtisanConfig {
                favorites: self.artisan_favorites.clone(),
            },
            make: MakeConfig {
                favorites: self.make_favorites.clone(),
            },
        }
    }

    // Backward compatibility methods for existing code
    pub fn process_items(&self) -> [(&'static str, bool); 5] {
        self.disabled.items()
    }

    pub fn toggle_item(&mut self, index: usize) {
        self.disabled.toggle(index);
    }

    /// Get override for a process, creating default if none exists
    pub fn get_or_create_override(&mut self, name: &str) -> &mut OverrideDraft {
        if !self.overrides.contains_key(name) {
            self.overrides
                .insert(name.to_string(), OverrideDraft::default());
        }
        self.overrides.get_mut(name).unwrap()
    }

    /// Count of custom processes
    pub fn custom_count(&self) -> usize {
        self.custom.len()
    }
}

// ============================================================================
// Main App State
// ============================================================================

/// The main application state
pub struct App {
    /// Whether Laravel Sail is detected (commands run through Docker)
    pub is_sail: bool,

    /// Currently active tab
    pub active_tab: Tab,

    /// Processes tab state
    pub processes_tab: ProcessesTabState,

    /// Logs tab state
    pub logs_tab: LogsTabState,

    /// Artisan tab state
    pub artisan_tab: ArtisanTabState,

    /// Make tab state
    pub make_tab: MakeTabState,

    /// Quality tab state
    pub quality_tab: QualityTabState,

    /// Config tab state
    pub config_tab: ConfigTabState,

    /// All managed processes
    pub processes: HashMap<ProcessId, Process>,

    /// Order of processes for display
    pub process_order: Vec<ProcessId>,

    /// Laravel log lines (ring buffer)
    pub log_lines: VecDeque<LogLine>,

    /// Maximum number of log lines to keep
    pub max_log_lines: usize,

    /// Working directory (Laravel project root)
    pub working_dir: PathBuf,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Status message to display
    pub status_message: Option<String>,

    /// Process registry for metadata lookup
    pub registry: ProcessRegistry,

    /// Current configuration (if loaded)
    pub config: Option<LaramuxConfig>,

    /// Configuration loading error (if any)
    pub config_error: Option<String>,

    /// System resource statistics
    pub system_stats: SystemStats,
}

impl App {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            is_sail: false,
            active_tab: Tab::default(),
            processes_tab: ProcessesTabState::default(),
            logs_tab: LogsTabState::default(),
            artisan_tab: ArtisanTabState::default(),
            make_tab: MakeTabState::default(),
            quality_tab: QualityTabState::default(),
            config_tab: ConfigTabState::default(),
            processes: HashMap::new(),
            process_order: Vec::new(),
            log_lines: VecDeque::with_capacity(DEFAULT_MAX_LOG_LINES),
            max_log_lines: DEFAULT_MAX_LOG_LINES,
            working_dir,
            should_quit: false,
            status_message: None,
            registry: ProcessRegistry::new(),
            config: None,
            config_error: None,
            system_stats: SystemStats::default(),
        }
    }

    /// Set configuration loading error
    pub fn set_config_error(&mut self, error: String) {
        self.config_error = Some(error);
    }

    /// Set the configuration
    pub fn set_config(&mut self, config: Option<LaramuxConfig>) {
        self.config_tab.config_draft = Some(ConfigDraft::from_config(config.as_ref()));

        // Apply log config
        if let Some(ref cfg) = config {
            // Set max log lines
            self.max_log_lines = cfg.log_max_lines();

            // Apply default log filter
            if let Some(filter) = cfg.default_log_filter() {
                self.logs_tab.filter_level = Some(LogLevel::from_str(filter));
            }
        }

        self.config = config;
    }

    /// Set the process registry
    pub fn set_registry(&mut self, registry: ProcessRegistry) {
        self.registry = registry;
    }

    /// Set the discovered artisan commands
    pub fn set_artisan_commands(&mut self, commands: Vec<FullArtisanCommand>) {
        self.artisan_tab.artisan_commands = commands;
    }

    /// Set the discovered artisan make commands
    pub fn set_artisan_make_commands(&mut self, commands: Vec<FullArtisanCommand>) {
        self.make_tab.make_commands = commands;
    }

    /// Set the discovered quality tools
    pub fn set_quality_tools(&mut self, tools: Vec<QualityTool>) {
        self.quality_tab.quality_tools = tools;
    }

    /// Set the discovered testing tools
    pub fn set_testing_tools(&mut self, tools: Vec<QualityTool>) {
        self.quality_tab.testing_tools = tools;
    }

    /// Register a process configuration
    pub fn register_process(&mut self, config: ProcessConfig) {
        let id = config.id.clone();
        if !self.process_order.contains(&id) {
            self.process_order.push(id.clone());
        }
        self.processes.insert(id, Process::new(config));
    }

    /// Get the currently selected process (uses processes_tab.selected_index)
    pub fn selected_process(&self) -> Option<&Process> {
        self.process_order
            .get(self.processes_tab.selected_index)
            .and_then(|id| self.processes.get(id))
    }

    /// Get the currently selected process mutably
    pub fn selected_process_mut(&mut self) -> Option<&mut Process> {
        self.process_order
            .get(self.processes_tab.selected_index)
            .and_then(|id| self.processes.get_mut(id))
    }

    /// Get the currently selected process id
    pub fn selected_id(&self) -> Option<&ProcessId> {
        self.process_order.get(self.processes_tab.selected_index)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.process_order.is_empty() && self.processes_tab.selected_index > 0 {
            self.processes_tab.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.process_order.is_empty()
            && self.processes_tab.selected_index < self.process_order.len() - 1
        {
            self.processes_tab.selected_index += 1;
        }
    }

    /// Add output to a process
    pub fn add_process_output(&mut self, id: &ProcessId, line: String, is_stderr: bool) {
        if let Some(process) = self.processes.get_mut(id) {
            let output_line = if is_stderr {
                OutputLine::stderr(line)
            } else {
                OutputLine::stdout(line)
            };
            process.add_output(output_line);
        }
    }

    /// Update process status
    pub fn set_process_status(&mut self, id: &ProcessId, status: ProcessStatus) {
        if let Some(process) = self.processes.get_mut(id) {
            process.status = status;
        }
    }

    /// Set process PID
    pub fn set_process_pid(&mut self, id: &ProcessId, pid: Option<u32>) {
        if let Some(process) = self.processes.get_mut(id) {
            process.pid = pid;
        }
    }

    /// Add log lines from Laravel log
    pub fn add_log_lines(&mut self, entries: Vec<crate::log::LogEntry>) {
        for entry in entries {
            // Track available files
            if !self.logs_tab.available_files.contains(&entry.file) {
                self.logs_tab.available_files.push(entry.file.clone());
                self.logs_tab.available_files.sort();
            }

            let level = Self::parse_log_level(&entry.content);
            let log_line = LogLine {
                content: entry.content,
                level,
                file: entry.file,
            };

            if self.log_lines.len() >= self.max_log_lines {
                self.log_lines.pop_front();
            }
            self.log_lines.push_back(log_line);
        }
    }

    /// Parse log level from a Laravel log line
    fn parse_log_level(line: &str) -> LogLevel {
        // Laravel log format: [YYYY-MM-DD HH:MM:SS] environment.LEVEL: message
        // Example: [2024-01-26 10:30:45] local.INFO: Test message
        if let Some(bracket_end) = line.find("] ") {
            let after_bracket = &line[bracket_end + 2..];
            if let Some(colon_pos) = after_bracket.find(':') {
                let env_level = &after_bracket[..colon_pos];
                if let Some(dot_pos) = env_level.rfind('.') {
                    return LogLevel::from_str(&env_level[dot_pos + 1..]);
                }
            }
        }
        LogLevel::Unknown
    }

    /// Get filtered log lines based on search query, filter level, and selected file
    pub fn filtered_logs(&self) -> Vec<&LogLine> {
        self.log_lines
            .iter()
            .filter(|log| {
                // Filter by file
                if let Some(selected_file) = &self.logs_tab.selected_file {
                    if &log.file != selected_file {
                        return false;
                    }
                }

                // Filter by level (Unknown logs always pass the level filter)
                if let Some(min_level) = self.logs_tab.filter_level {
                    if log.level != LogLevel::Unknown {
                        let level_order = |l: &LogLevel| -> u8 {
                            match l {
                                LogLevel::Debug => 0,
                                LogLevel::Info => 1,
                                LogLevel::Notice => 2,
                                LogLevel::Warning => 3,
                                LogLevel::Error => 4,
                                LogLevel::Critical => 5,
                                LogLevel::Alert => 6,
                                LogLevel::Emergency => 7,
                                LogLevel::Unknown => 0,
                            }
                        };
                        if level_order(&log.level) < level_order(&min_level) {
                            return false;
                        }
                    }
                }

                // Filter by search query
                if !self.logs_tab.search_query.is_empty() {
                    let query = self.logs_tab.search_query.to_lowercase();
                    if !log.content.to_lowercase().contains(&query) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Clear all log lines
    pub fn clear_logs(&mut self) {
        self.log_lines.clear();
        self.logs_tab.scroll_offset = 0;
    }

    /// Clear output for the selected process
    pub fn clear_selected_output(&mut self) {
        if let Some(process) = self.selected_process_mut() {
            process.clear_output();
        }
    }

    /// Set a status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Request app quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Scroll selected process output up
    pub fn scroll_output_up(&mut self, amount: usize) {
        if let Some(process) = self.selected_process_mut() {
            process.scroll_offset = process.scroll_offset.saturating_add(amount);
        }
    }

    /// Scroll selected process output down
    pub fn scroll_output_down(&mut self, amount: usize) {
        if let Some(process) = self.selected_process_mut() {
            process.scroll_offset = process.scroll_offset.saturating_sub(amount);
        }
    }

    /// Scroll log pane up
    pub fn scroll_log_up(&mut self, amount: usize) {
        self.logs_tab.scroll_offset = self.logs_tab.scroll_offset.saturating_add(amount);
    }

    /// Scroll log pane down
    pub fn scroll_log_down(&mut self, amount: usize) {
        self.logs_tab.scroll_offset = self.logs_tab.scroll_offset.saturating_sub(amount);
    }

    // Tab navigation
    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.active_tab = self.active_tab.previous();
    }

    pub fn go_to_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }
}
