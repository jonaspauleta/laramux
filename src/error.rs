#![allow(dead_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LaraMuxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Failed to spawn process '{name}': {reason}")]
    SpawnFailed { name: String, reason: String },

    #[error("Process '{0}' not found")]
    ProcessNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration validation error: {0}")]
    ConfigValidation(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("File watch error: {0}")]
    Watch(#[from] notify::Error),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Terminal error: {0}")]
    Terminal(String),
}

pub type Result<T> = std::result::Result<T, LaraMuxError>;
