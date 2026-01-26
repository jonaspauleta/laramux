#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;

/// The kind of Laravel process being managed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessKind {
    Serve,
    Vite,
    Queue,
    Reverb,
}

impl ProcessKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProcessKind::Serve => "Serve",
            ProcessKind::Vite => "Vite",
            ProcessKind::Queue => "Queue",
            ProcessKind::Reverb => "Reverb",
        }
    }

    pub fn hotkey(&self) -> Option<char> {
        match self {
            ProcessKind::Serve => Some('s'),
            ProcessKind::Vite => Some('v'),
            ProcessKind::Queue => Some('q'),
            ProcessKind::Reverb => Some('b'),
        }
    }

    pub fn all() -> &'static [ProcessKind] {
        &[
            ProcessKind::Serve,
            ProcessKind::Vite,
            ProcessKind::Queue,
            ProcessKind::Reverb,
        ]
    }
}

impl std::fmt::Display for ProcessKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Current status of a managed process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessStatus {
    #[default]
    Stopped,
    Running,
    Restarting,
    Failed,
}

impl ProcessStatus {
    pub fn indicator(&self) -> &'static str {
        match self {
            ProcessStatus::Running => "🟢",
            ProcessStatus::Stopped => "⚫",
            ProcessStatus::Restarting => "🟡",
            ProcessStatus::Failed => "🔴",
        }
    }
}

/// Configuration for spawning a process
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub kind: ProcessKind,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

impl ProcessConfig {
    pub fn new(kind: ProcessKind, command: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            kind,
            command: command.into(),
            args: Vec::new(),
            working_dir,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }
}

/// Maximum number of output lines to keep per process
pub const MAX_OUTPUT_LINES: usize = 1000;

/// A single line of process output
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: String,
    pub is_stderr: bool,
    pub is_error: bool,
}

impl OutputLine {
    pub fn stdout(content: String) -> Self {
        let is_error = Self::detect_error(&content);
        Self {
            content,
            is_stderr: false,
            is_error,
        }
    }

    pub fn stderr(content: String) -> Self {
        Self {
            content,
            is_stderr: true,
            is_error: true,
        }
    }

    fn detect_error(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("error")
            || lower.contains("exception")
            || lower.contains("fatal")
            || lower.contains("failed")
            || content.contains("Stack trace:")
    }
}

/// A managed process with its state and output
#[derive(Debug)]
pub struct Process {
    pub kind: ProcessKind,
    pub status: ProcessStatus,
    pub config: ProcessConfig,
    pub output: VecDeque<OutputLine>,
    pub pid: Option<u32>,
    pub scroll_offset: usize,
}

impl Process {
    pub fn new(config: ProcessConfig) -> Self {
        Self {
            kind: config.kind,
            status: ProcessStatus::Stopped,
            config,
            output: VecDeque::with_capacity(MAX_OUTPUT_LINES),
            pid: None,
            scroll_offset: 0,
        }
    }

    pub fn add_output(&mut self, line: OutputLine) {
        if self.output.len() >= MAX_OUTPUT_LINES {
            self.output.pop_front();
        }
        self.output.push_back(line);
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
        self.scroll_offset = 0;
    }
}
