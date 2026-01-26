#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::process::types::{Process, ProcessConfig, ProcessKind, ProcessStatus};

/// Maximum number of log lines to display
pub const MAX_LOG_LINES: usize = 100;

/// A line from the Laravel log file
#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
    pub level: LogLevel,
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
}

/// The main application state
pub struct App {
    /// All managed processes
    pub processes: HashMap<ProcessKind, Process>,

    /// Order of processes for display
    pub process_order: Vec<ProcessKind>,

    /// Currently selected process index in the sidebar
    pub selected_index: usize,

    /// Laravel log lines (ring buffer)
    pub log_lines: VecDeque<LogLine>,

    /// Log pane scroll offset
    pub log_scroll_offset: usize,

    /// Working directory (Laravel project root)
    pub working_dir: PathBuf,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Status message to display
    pub status_message: Option<String>,
}

impl App {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            processes: HashMap::new(),
            process_order: Vec::new(),
            selected_index: 0,
            log_lines: VecDeque::with_capacity(MAX_LOG_LINES),
            log_scroll_offset: 0,
            working_dir,
            should_quit: false,
            status_message: None,
        }
    }

    /// Register a process configuration
    pub fn register_process(&mut self, config: ProcessConfig) {
        let kind = config.kind;
        if !self.process_order.contains(&kind) {
            self.process_order.push(kind);
        }
        self.processes.insert(kind, Process::new(config));
    }

    /// Get the currently selected process
    pub fn selected_process(&self) -> Option<&Process> {
        self.process_order
            .get(self.selected_index)
            .and_then(|kind| self.processes.get(kind))
    }

    /// Get the currently selected process mutably
    pub fn selected_process_mut(&mut self) -> Option<&mut Process> {
        self.process_order
            .get(self.selected_index)
            .and_then(|kind| self.processes.get_mut(kind))
    }

    /// Get the currently selected process kind
    pub fn selected_kind(&self) -> Option<ProcessKind> {
        self.process_order.get(self.selected_index).copied()
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.process_order.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.process_order.is_empty() && self.selected_index < self.process_order.len() - 1 {
            self.selected_index += 1;
        }
    }

    /// Add output to a process
    pub fn add_process_output(&mut self, kind: ProcessKind, line: String, is_stderr: bool) {
        if let Some(process) = self.processes.get_mut(&kind) {
            let output_line = if is_stderr {
                crate::process::types::OutputLine::stderr(line)
            } else {
                crate::process::types::OutputLine::stdout(line)
            };
            process.add_output(output_line);
        }
    }

    /// Update process status
    pub fn set_process_status(&mut self, kind: ProcessKind, status: ProcessStatus) {
        if let Some(process) = self.processes.get_mut(&kind) {
            process.status = status;
        }
    }

    /// Set process PID
    pub fn set_process_pid(&mut self, kind: ProcessKind, pid: Option<u32>) {
        if let Some(process) = self.processes.get_mut(&kind) {
            process.pid = pid;
        }
    }

    /// Add log lines from Laravel log
    pub fn add_log_lines(&mut self, lines: Vec<String>) {
        for line in lines {
            let level = Self::parse_log_level(&line);
            let log_line = LogLine { content: line, level };

            if self.log_lines.len() >= MAX_LOG_LINES {
                self.log_lines.pop_front();
            }
            self.log_lines.push_back(log_line);
        }
    }

    /// Parse log level from a Laravel log line
    fn parse_log_level(line: &str) -> LogLevel {
        // Laravel log format: [YYYY-MM-DD HH:MM:SS] environment.LEVEL: message
        if let Some(start) = line.find("].") {
            if let Some(end) = line[start..].find(':') {
                let level_part = &line[start + 2..start + end];
                if let Some(dot_pos) = level_part.rfind('.') {
                    return LogLevel::from_str(&level_part[dot_pos + 1..]);
                }
            }
        }
        LogLevel::Unknown
    }

    /// Clear all log lines
    pub fn clear_logs(&mut self) {
        self.log_lines.clear();
        self.log_scroll_offset = 0;
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
        self.log_scroll_offset = self.log_scroll_offset.saturating_add(amount);
    }

    /// Scroll log pane down
    pub fn scroll_log_down(&mut self, amount: usize) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
    }
}
