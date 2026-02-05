#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use crate::config::RestartPolicy;

/// The kind of built-in Laravel process being managed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessKind {
    Serve,
    Vite,
    Queue,
    Horizon,
    Reverb,
}

impl ProcessKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProcessKind::Serve => "Serve",
            ProcessKind::Vite => "Vite",
            ProcessKind::Queue => "Queue",
            ProcessKind::Horizon => "Horizon",
            ProcessKind::Reverb => "Reverb",
        }
    }

    pub fn hotkey(&self) -> Option<char> {
        match self {
            ProcessKind::Serve => Some('s'),
            ProcessKind::Vite => Some('v'),
            ProcessKind::Queue => Some('q'),
            ProcessKind::Horizon => Some('h'),
            ProcessKind::Reverb => Some('b'),
        }
    }

    pub fn config_name(&self) -> &'static str {
        match self {
            ProcessKind::Serve => "serve",
            ProcessKind::Vite => "vite",
            ProcessKind::Queue => "queue",
            ProcessKind::Horizon => "horizon",
            ProcessKind::Reverb => "reverb",
        }
    }

    pub fn all() -> &'static [ProcessKind] {
        &[
            ProcessKind::Serve,
            ProcessKind::Vite,
            ProcessKind::Queue,
            ProcessKind::Horizon,
            ProcessKind::Reverb,
        ]
    }
}

impl std::fmt::Display for ProcessKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Unified identifier for both built-in and custom processes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessId {
    Builtin(ProcessKind),
    Custom(String),
}

impl ProcessId {
    /// Create a ProcessId from a built-in ProcessKind
    pub fn builtin(kind: ProcessKind) -> Self {
        ProcessId::Builtin(kind)
    }

    /// Create a ProcessId for a custom process
    pub fn custom(name: impl Into<String>) -> Self {
        ProcessId::Custom(name.into())
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessId::Builtin(kind) => write!(f, "{}", kind.display_name()),
            ProcessId::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl From<ProcessKind> for ProcessId {
    fn from(kind: ProcessKind) -> Self {
        ProcessId::Builtin(kind)
    }
}

/// Metadata for a process (used by registry)
#[derive(Debug, Clone)]
pub struct ProcessMetadata {
    pub display_name: String,
    pub hotkey: Option<char>,
}

/// Registry for process metadata, handles both built-in and custom processes
#[derive(Debug, Default)]
pub struct ProcessRegistry {
    custom_metadata: HashMap<String, ProcessMetadata>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            custom_metadata: HashMap::new(),
        }
    }

    /// Register a custom process
    pub fn register_custom(&mut self, name: String, display_name: String, hotkey: Option<char>) {
        self.custom_metadata.insert(
            name,
            ProcessMetadata {
                display_name,
                hotkey,
            },
        );
    }

    /// Get display name for a process
    pub fn display_name(&self, id: &ProcessId) -> String {
        match id {
            ProcessId::Builtin(kind) => kind.display_name().to_string(),
            ProcessId::Custom(name) => self
                .custom_metadata
                .get(name)
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| name.clone()),
        }
    }

    /// Get hotkey for a process
    pub fn hotkey(&self, id: &ProcessId) -> Option<char> {
        match id {
            ProcessId::Builtin(kind) => kind.hotkey(),
            ProcessId::Custom(name) => self.custom_metadata.get(name).and_then(|m| m.hotkey),
        }
    }

    /// Find a process by hotkey
    pub fn find_by_hotkey(&self, hotkey: char, process_order: &[ProcessId]) -> Option<ProcessId> {
        for id in process_order {
            if self.hotkey(id) == Some(hotkey) {
                return Some(id.clone());
            }
        }
        None
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
    Supervised,
}

impl ProcessStatus {
    pub fn indicator(&self) -> &'static str {
        match self {
            ProcessStatus::Running => "🟢",
            ProcessStatus::Stopped => "⚫",
            ProcessStatus::Restarting => "🟡",
            ProcessStatus::Failed => "🔴",
            ProcessStatus::Supervised => "🔵",
        }
    }
}

/// Configuration for spawning a process
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub id: ProcessId,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
    pub supervised: bool,
    pub supervisor_program: Option<String>,
}

impl ProcessConfig {
    pub fn new(id: impl Into<ProcessId>, command: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            id: id.into(),
            command: command.into(),
            args: Vec::new(),
            working_dir,
            env: HashMap::new(),
            restart_policy: RestartPolicy::default(),
            supervised: false,
            supervisor_program: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    pub fn with_supervised(mut self, program_name: String) -> Self {
        self.supervised = true;
        self.supervisor_program = Some(program_name);
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
    pub id: ProcessId,
    pub status: ProcessStatus,
    pub config: ProcessConfig,
    pub output: VecDeque<OutputLine>,
    pub pid: Option<u32>,
    pub scroll_offset: usize,
}

impl Process {
    pub fn new(config: ProcessConfig) -> Self {
        Self {
            id: config.id.clone(),
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

    pub fn is_supervised(&self) -> bool {
        self.config.supervised
    }
}
