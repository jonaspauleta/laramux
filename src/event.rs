#![allow(dead_code)]

use crossterm::event::KeyEvent;

use crate::process::types::ProcessKind;

/// Events that can occur in the application
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input from the user
    Input(KeyEvent),

    /// Output line from a managed process
    ProcessOutput {
        kind: ProcessKind,
        line: String,
        is_stderr: bool,
    },

    /// A managed process has exited
    ProcessExited {
        kind: ProcessKind,
        exit_code: Option<i32>,
    },

    /// New content from laravel.log
    LogUpdate(Vec<String>),

    /// Terminal resize event
    Resize(u16, u16),

    /// Tick event for periodic updates
    Tick,
}
