use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::event::Event;

/// Watch Laravel log file for changes
pub struct LogWatcher {
    log_path: PathBuf,
    event_tx: mpsc::Sender<Event>,
    cancel_token: CancellationToken,
}

impl LogWatcher {
    pub fn new(
        log_path: PathBuf,
        event_tx: mpsc::Sender<Event>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            log_path,
            event_tx,
            cancel_token,
        }
    }

    /// Start watching the log file
    pub async fn watch(self) -> Result<()> {
        let log_path = self.log_path.clone();
        let event_tx = self.event_tx.clone();
        let cancel_token = self.cancel_token.clone();

        // Create channel for file system events
        let (fs_tx, mut fs_rx) = mpsc::channel::<notify::Result<notify::Event>>(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = fs_tx.blocking_send(res);
            },
            Config::default(),
        )?;

        // Watch the log directory (parent of log file)
        if let Some(parent) = log_path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

        // Track file position for incremental reading
        let mut last_pos = if log_path.exists() {
            std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                Some(event) = fs_rx.recv() => {
                    if let Ok(event) = event {
                        // Check if this event is for our log file
                        let is_our_file = event.paths.iter().any(|p| {
                            p.file_name() == log_path.file_name()
                        });

                        if is_our_file && matches!(event.kind, notify::EventKind::Modify(_) | notify::EventKind::Create(_)) {
                            // Read new content
                            if let Ok(new_lines) = read_new_lines(&log_path, &mut last_pos) {
                                if !new_lines.is_empty() {
                                    let _ = event_tx.send(Event::LogUpdate(new_lines)).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Read new lines from file starting at given position
fn read_new_lines(path: &PathBuf, last_pos: &mut u64) -> std::io::Result<Vec<String>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let current_size = metadata.len();

    // File was truncated (rotated), start from beginning
    if current_size < *last_pos {
        *last_pos = 0;
    }

    // No new content
    if current_size == *last_pos {
        return Ok(Vec::new());
    }

    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(*last_pos))?;

    let mut lines = Vec::new();
    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim_end();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
        line.clear();
    }

    *last_pos = current_size;
    Ok(lines)
}

/// Find Laravel log file in project
pub fn find_log_file(working_dir: &std::path::Path) -> Option<PathBuf> {
    let log_path = working_dir.join("storage/logs/laravel.log");
    if log_path.exists() {
        Some(log_path)
    } else {
        // Try to find any .log file in storage/logs
        let logs_dir = working_dir.join("storage/logs");
        if logs_dir.exists() {
            std::fs::read_dir(&logs_dir)
                .ok()?
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "log")
                        .unwrap_or(false)
                })
                .map(|e| e.path())
        } else {
            None
        }
    }
}
