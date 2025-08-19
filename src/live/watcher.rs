//! Claude-keeper subprocess integration
//!
//! This module manages the claude-keeper subprocess in watch mode and handles
//! the JSON streaming of usage updates.

use anyhow::{Context, Result};
use serde_json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

use crate::live::LiveConfig;
use crate::models::UsageEntry;

/// Manages claude-keeper subprocess for live usage monitoring
pub struct KeeperWatcher {
    process: Option<Child>,
    restart_count: u32,
    max_restarts: u32,
    config: LiveConfig,
}

impl KeeperWatcher {
    /// Create a new keeper watcher and start the subprocess
    pub fn new(config: &LiveConfig) -> Result<Self> {
        let mut watcher = Self {
            process: None,
            restart_count: 0,
            max_restarts: config.max_restart_attempts,
            config: config.clone(),
        };

        watcher.start_process()?;
        Ok(watcher)
    }

    /// Start the claude-keeper watch process
    fn start_process(&mut self) -> Result<()> {
        info!(
            executable = %self.config.claude_keeper_path,
            "Starting claude-keeper watch process"
        );

        let mut cmd = Command::new(&self.config.claude_keeper_path);
        cmd.args(&["watch", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let child = cmd.spawn()
            .with_context(|| format!("Failed to start claude-keeper process: {}", self.config.claude_keeper_path))?;

        self.process = Some(child);
        
        debug!("Claude-keeper watch process started successfully");
        Ok(())
    }

    /// Get the next usage entry from claude-keeper
    pub async fn next_entry(&mut self) -> Result<Option<UsageEntry>> {
        let process = self.process.as_mut()
            .context("No claude-keeper process running")?;

        let stdout = process.stdout.as_mut()
            .context("No stdout available from claude-keeper process")?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        loop {
            // Read the next line from stdout
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF reached, process finished
                    info!("Claude-keeper process finished (EOF)");
                    return Ok(None);
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        line.clear();
                        continue;
                    }

                    debug!(line = %trimmed, "Received line from claude-keeper");

                    // Try to parse as JSON
                    match serde_json::from_str::<UsageEntry>(trimmed) {
                        Ok(entry) => {
                            line.clear();
                            return Ok(Some(entry));
                        }
                        Err(e) => {
                            // Log parse error but continue processing
                            warn!(
                                error = %e,
                                line = %trimmed,
                                "Failed to parse JSON from claude-keeper"
                            );
                            line.clear();
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to read from claude-keeper stdout");
                    return Err(e.into());
                }
            }
        }
    }

    /// Check if the watcher should attempt to restart
    pub fn should_restart(&self) -> bool {
        self.restart_count < self.max_restarts
    }

    /// Restart the claude-keeper process
    #[allow(dead_code)]
    pub async fn restart(&mut self) -> Result<()> {
        if !self.should_restart() {
            return Err(anyhow::anyhow!(
                "Maximum restart attempts ({}) exceeded",
                self.max_restarts
            ));
        }

        warn!(
            attempt = self.restart_count + 1,
            max_attempts = self.max_restarts,
            "Restarting claude-keeper process"
        );

        // Kill existing process if it's still running
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }

        self.restart_count += 1;
        self.start_process()
    }

    /// Check if the process is still running
    #[allow(dead_code)]
    pub fn is_running(&mut self) -> bool {
        if let Some(process) = &mut self.process {
            match process.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Process is still running
                Err(_) => false,      // Error checking status, assume not running
            }
        } else {
            false
        }
    }
}

impl Drop for KeeperWatcher {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // For Drop implementation, we use the synchronous kill
            let _ = process.start_kill();
        }
    }
}