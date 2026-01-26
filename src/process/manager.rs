use std::collections::HashMap;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::{LaraMuxError, Result};
use crate::event::Event;
use crate::process::types::{ProcessConfig, ProcessId};

/// Manages spawning, killing, and restarting processes
pub struct ProcessManager {
    children: HashMap<ProcessId, Child>,
    configs: HashMap<ProcessId, ProcessConfig>,
    event_tx: mpsc::Sender<Event>,
    cancel_token: CancellationToken,
}

impl ProcessManager {
    pub fn new(event_tx: mpsc::Sender<Event>, cancel_token: CancellationToken) -> Self {
        Self {
            children: HashMap::new(),
            configs: HashMap::new(),
            event_tx,
            cancel_token,
        }
    }

    /// Register a process configuration
    pub fn register(&mut self, config: ProcessConfig) {
        self.configs.insert(config.id.clone(), config);
    }

    /// Spawn a process
    pub async fn spawn(&mut self, id: &ProcessId) -> Result<()> {
        let config = self
            .configs
            .get(id)
            .ok_or_else(|| LaraMuxError::ProcessNotFound(id.to_string()))?
            .clone();

        // Kill existing process if running
        self.kill(id).await?;

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            // Force color output even when not connected to a TTY
            .env("FORCE_COLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .env("COLORTERM", "truecolor");

        let mut child = cmd.spawn().map_err(|e| LaraMuxError::SpawnFailed {
            name: id.to_string(),
            reason: e.to_string(),
        })?;

        let pid = child.id();

        // Spawn stdout reader task
        if let Some(stdout) = child.stdout.take() {
            let tx = self.event_tx.clone();
            let token = self.cancel_token.clone();
            let process_id = id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        result = lines.next_line() => {
                            match result {
                                Ok(Some(line)) => {
                                    let _ = tx.send(Event::ProcessOutput {
                                        id: process_id.clone(),
                                        line,
                                        is_stderr: false,
                                    }).await;
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    }
                }
            });
        }

        // Spawn stderr reader task
        if let Some(stderr) = child.stderr.take() {
            let tx = self.event_tx.clone();
            let token = self.cancel_token.clone();
            let process_id = id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        result = lines.next_line() => {
                            match result {
                                Ok(Some(line)) => {
                                    let _ = tx.send(Event::ProcessOutput {
                                        id: process_id.clone(),
                                        line,
                                        is_stderr: true,
                                    }).await;
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    }
                }
            });
        }

        self.children.insert(id.clone(), child);

        // Send initial status via event
        let _ = self
            .event_tx
            .send(Event::ProcessOutput {
                id: id.clone(),
                line: format!("Started {} (PID: {:?})", id, pid),
                is_stderr: false,
            })
            .await;

        Ok(())
    }

    /// Spawn all registered processes
    pub async fn spawn_all(&mut self) -> Result<()> {
        let ids: Vec<ProcessId> = self.configs.keys().cloned().collect();
        for id in ids {
            self.spawn(&id).await?;
        }
        Ok(())
    }

    /// Kill a process gracefully (SIGTERM, wait, then SIGKILL)
    pub async fn kill(&mut self, id: &ProcessId) -> Result<()> {
        if let Some(mut child) = self.children.remove(id) {
            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                if let Some(pid) = child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
            }

            // Wait for process to exit with timeout
            let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(5), child.wait());

            match timeout.await {
                Ok(Ok(status)) => {
                    let _ = self
                        .event_tx
                        .send(Event::ProcessExited {
                            id: id.clone(),
                            exit_code: status.code(),
                        })
                        .await;
                }
                _ => {
                    // Force kill if timeout
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let _ = self
                        .event_tx
                        .send(Event::ProcessExited {
                            id: id.clone(),
                            exit_code: None,
                        })
                        .await;
                }
            }
        }
        Ok(())
    }

    /// Kill all processes
    pub async fn kill_all(&mut self) -> Result<()> {
        let ids: Vec<ProcessId> = self.children.keys().cloned().collect();
        for id in ids {
            self.kill(&id).await?;
        }
        Ok(())
    }

    /// Restart a process
    pub async fn restart(&mut self, id: &ProcessId) -> Result<()> {
        self.kill(id).await?;
        // Small delay before restarting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        self.spawn(id).await
    }

    /// Restart all processes
    pub async fn restart_all(&mut self) -> Result<()> {
        let ids: Vec<ProcessId> = self.configs.keys().cloned().collect();
        for id in ids {
            self.restart(&id).await?;
        }
        Ok(())
    }

    /// Check if a process is running
    pub fn is_running(&self, id: &ProcessId) -> bool {
        self.children.contains_key(id)
    }

    /// Get process PID
    pub fn get_pid(&self, id: &ProcessId) -> Option<u32> {
        self.children.get(id).and_then(|c| c.id())
    }
}
