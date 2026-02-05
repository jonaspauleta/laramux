use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::RestartPolicy;
use crate::error::{LaraMuxError, Result};
use crate::event::Event;
use crate::process::types::{ProcessConfig, ProcessId};

/// Maximum backoff delay for restarts (60 seconds)
const MAX_RESTART_BACKOFF_SECS: u64 = 60;

/// Track restart state for a process
#[derive(Debug, Clone, Default)]
pub struct RestartState {
    /// Consecutive failures count
    pub consecutive_failures: u32,
    /// Last restart time
    pub last_restart: Option<Instant>,
}

impl RestartState {
    /// Calculate the backoff delay based on consecutive failures (exponential: 2^failures, max 60s)
    pub fn backoff_delay(&self) -> Duration {
        let secs = 2u64
            .saturating_pow(self.consecutive_failures)
            .min(MAX_RESTART_BACKOFF_SECS);
        Duration::from_secs(secs)
    }

    /// Reset state on successful start
    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.last_restart = Some(Instant::now());
    }

    /// Record a failure
    pub fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_restart = Some(Instant::now());
    }
}

/// Manages spawning, killing, and restarting processes
pub struct ProcessManager {
    children: HashMap<ProcessId, Child>,
    configs: HashMap<ProcessId, ProcessConfig>,
    restart_states: HashMap<ProcessId, RestartState>,
    event_tx: mpsc::Sender<Event>,
    cancel_token: CancellationToken,
}

impl ProcessManager {
    pub fn new(event_tx: mpsc::Sender<Event>, cancel_token: CancellationToken) -> Self {
        Self {
            children: HashMap::new(),
            configs: HashMap::new(),
            restart_states: HashMap::new(),
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
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            // Force color output even when not connected to a TTY
            .env("FORCE_COLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .env("COLORTERM", "truecolor");

        // Apply configured environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn().map_err(|e| {
            // Provide more helpful error messages
            let reason = if e.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Command '{}' not found. Make sure it is installed and in your PATH.",
                    config.command
                )
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!(
                    "Permission denied when trying to execute '{}'. Check file permissions.",
                    config.command
                )
            } else {
                e.to_string()
            };
            LaraMuxError::SpawnFailed {
                name: id.to_string(),
                reason,
            }
        })?;

        // Reset restart state on successful spawn
        self.restart_states.entry(id.clone()).or_default().reset();

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
        let initial_msg = if config.supervised {
            format!(
                "Tailing supervisor logs for {} (PID: {:?})",
                config.supervisor_program.as_deref().unwrap_or("unknown"),
                pid
            )
        } else {
            format!("Started {} (PID: {:?})", id, pid)
        };
        let _ = self
            .event_tx
            .send(Event::ProcessOutput {
                id: id.clone(),
                line: initial_msg,
                is_stderr: false,
            })
            .await;

        Ok(())
    }

    /// Spawn all registered processes
    /// Returns a list of (process_id, error_message) for any that failed to spawn
    pub async fn spawn_all(&mut self) -> Result<Vec<(ProcessId, String)>> {
        let ids: Vec<ProcessId> = self.configs.keys().cloned().collect();
        let mut errors = Vec::new();
        for id in ids {
            if let Err(e) = self.spawn(&id).await {
                errors.push((id.clone(), e.to_string()));
                // Send error message as process output so it's visible in the UI
                let _ = self
                    .event_tx
                    .send(Event::ProcessOutput {
                        id: id.clone(),
                        line: format!("ERROR: Failed to start process: {}", e),
                        is_stderr: true,
                    })
                    .await;
            }
        }
        Ok(errors)
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

    /// Kill all processes in parallel for fast shutdown
    pub async fn kill_all(&mut self) -> Result<()> {
        use futures::future::join_all;

        // Extract all children for parallel killing
        let children: Vec<(ProcessId, Child)> = self.children.drain().collect();
        if children.is_empty() {
            return Ok(());
        }

        let event_tx = self.event_tx.clone();

        // Kill all processes in parallel
        let futures: Vec<_> = children
            .into_iter()
            .map(|(id, child)| {
                let tx = event_tx.clone();
                async move {
                    kill_child(child, &id, &tx).await;
                }
            })
            .collect();

        join_all(futures).await;
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

    /// Check if a process should be auto-restarted based on its exit code and restart policy
    pub fn should_restart(&self, id: &ProcessId, exit_code: Option<i32>) -> bool {
        let Some(config) = self.configs.get(id) else {
            return false;
        };

        // Supervised processes are managed by supervisor — never auto-restart
        if config.supervised {
            return false;
        }

        match config.restart_policy {
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure => {
                // Restart only if exit code is non-zero
                exit_code.map(|c| c != 0).unwrap_or(true)
            }
            RestartPolicy::Always => true,
        }
    }

    /// Check if a process is supervised (managed by Docker supervisor)
    pub fn is_supervised(&self, id: &ProcessId) -> bool {
        self.configs.get(id).map(|c| c.supervised).unwrap_or(false)
    }

    /// Get the restart policy for a process
    #[allow(dead_code)]
    pub fn get_restart_policy(&self, id: &ProcessId) -> RestartPolicy {
        self.configs
            .get(id)
            .map(|c| c.restart_policy)
            .unwrap_or_default()
    }

    /// Get the restart state for a process (for backoff calculation)
    #[allow(dead_code)]
    pub fn get_restart_state(&self, id: &ProcessId) -> Option<&RestartState> {
        self.restart_states.get(id)
    }

    /// Record a failure for a process (for backoff calculation)
    pub fn record_failure(&mut self, id: &ProcessId) {
        self.restart_states
            .entry(id.clone())
            .or_default()
            .record_failure();
    }

    /// Get the backoff delay for restarting a process
    pub fn get_backoff_delay(&self, id: &ProcessId) -> Duration {
        self.restart_states
            .get(id)
            .map(|s| s.backoff_delay())
            .unwrap_or(Duration::from_secs(1))
    }
}

/// Helper to kill a single child process with timeout
async fn kill_child(mut child: Child, id: &ProcessId, event_tx: &mpsc::Sender<Event>) {
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

    // Wait for process to exit with shorter timeout for quit (1 second)
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(1), child.wait());

    match timeout.await {
        Ok(Ok(status)) => {
            let _ = event_tx
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
            let _ = event_tx
                .send(Event::ProcessExited {
                    id: id.clone(),
                    exit_code: None,
                })
                .await;
        }
    }
}
