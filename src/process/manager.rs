use std::collections::HashMap;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::{LaraMuxError, Result};
use crate::event::Event;
use crate::process::types::{ProcessConfig, ProcessKind};

/// Manages spawning, killing, and restarting processes
pub struct ProcessManager {
    children: HashMap<ProcessKind, Child>,
    configs: HashMap<ProcessKind, ProcessConfig>,
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
        self.configs.insert(config.kind, config);
    }

    /// Spawn a process
    pub async fn spawn(&mut self, kind: ProcessKind) -> Result<()> {
        let config = self
            .configs
            .get(&kind)
            .ok_or_else(|| LaraMuxError::ProcessNotFound(kind.to_string()))?
            .clone();

        // Kill existing process if running
        self.kill(kind).await?;

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| LaraMuxError::SpawnFailed {
            name: kind.to_string(),
            reason: e.to_string(),
        })?;

        let pid = child.id();

        // Spawn stdout reader task
        if let Some(stdout) = child.stdout.take() {
            let tx = self.event_tx.clone();
            let token = self.cancel_token.clone();
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
                                        kind,
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
                                        kind,
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


        self.children.insert(kind, child);

        // Send initial status via event
        let _ = self
            .event_tx
            .send(Event::ProcessOutput {
                kind,
                line: format!("Started {} (PID: {:?})", kind, pid),
                is_stderr: false,
            })
            .await;

        Ok(())
    }

    /// Spawn all registered processes
    pub async fn spawn_all(&mut self) -> Result<()> {
        let kinds: Vec<ProcessKind> = self.configs.keys().copied().collect();
        for kind in kinds {
            self.spawn(kind).await?;
        }
        Ok(())
    }

    /// Kill a process gracefully (SIGTERM, wait, then SIGKILL)
    pub async fn kill(&mut self, kind: ProcessKind) -> Result<()> {
        if let Some(mut child) = self.children.remove(&kind) {
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
            let timeout = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                child.wait(),
            );

            match timeout.await {
                Ok(Ok(status)) => {
                    let _ = self
                        .event_tx
                        .send(Event::ProcessExited {
                            kind,
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
                            kind,
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
        let kinds: Vec<ProcessKind> = self.children.keys().copied().collect();
        for kind in kinds {
            self.kill(kind).await?;
        }
        Ok(())
    }

    /// Restart a process
    pub async fn restart(&mut self, kind: ProcessKind) -> Result<()> {
        self.kill(kind).await?;
        // Small delay before restarting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        self.spawn(kind).await
    }

    /// Restart all processes
    pub async fn restart_all(&mut self) -> Result<()> {
        let kinds: Vec<ProcessKind> = self.configs.keys().copied().collect();
        for kind in kinds {
            self.restart(kind).await?;
        }
        Ok(())
    }

    /// Check if a process is running
    pub fn is_running(&self, kind: ProcessKind) -> bool {
        self.children.contains_key(&kind)
    }

    /// Get process PID
    pub fn get_pid(&self, kind: ProcessKind) -> Option<u32> {
        self.children.get(&kind).and_then(|c| c.id())
    }
}
