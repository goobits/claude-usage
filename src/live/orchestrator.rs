//! Live mode orchestrator
//!
//! The orchestrator coordinates all live mode operations including:
//! - Loading baseline data from parquet files
//! - Managing claude-keeper subprocess
//! - Processing incoming usage updates
//! - Maintaining session state

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::live::{BaselineSummary, LiveConfig, LiveUpdate};
use crate::live::baseline::load_baseline_summary;
use crate::live::watcher::KeeperWatcher;
use crate::models::{SessionData, UsageEntry};

/// Main orchestrator for live mode operations
pub struct LiveOrchestrator {
    config: LiveConfig,
    baseline: BaselineSummary,
    sessions: HashMap<String, SessionData>,
    no_baseline: bool,
}

impl LiveOrchestrator {
    /// Create a new live orchestrator
    pub fn new(no_baseline: bool) -> Result<Self> {
        let config = LiveConfig::default(); // Use default for now
        
        let baseline = if no_baseline {
            info!("Skipping baseline loading (--no-baseline specified)");
            BaselineSummary::default()
        } else {
            load_baseline_summary().unwrap_or_else(|e| {
                warn!(error = %e, "Failed to load baseline, using empty baseline");
                BaselineSummary::default()
            })
        };

        Ok(Self {
            config,
            baseline,
            sessions: HashMap::new(),
            no_baseline,
        })
    }

    /// Run the live orchestrator
    pub async fn run(&mut self, tx: mpsc::Sender<LiveUpdate>) -> Result<()> {
        info!(
            baseline_cost = self.baseline.total_cost,
            baseline_tokens = self.baseline.total_tokens,
            sessions_today = self.baseline.sessions_today,
            "Starting live mode orchestrator"
        );

        // Start claude-keeper watcher
        let mut watcher = KeeperWatcher::new(&self.config)?;
        
        // Main processing loop
        loop {
            // Get next usage entry from claude-keeper
            match watcher.next_entry().await {
                Ok(Some(entry)) => {
                    if let Err(e) = self.process_entry(entry, &tx).await {
                        error!(error = %e, "Failed to process usage entry");
                        // Continue processing other entries
                    }
                }
                Ok(None) => {
                    // No more entries, keeper process finished
                    info!("Claude-keeper watcher finished");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "Error from claude-keeper watcher");
                    
                    // Try to restart watcher
                    if watcher.should_restart() {
                        warn!("Attempting to restart claude-keeper watcher");
                        watcher = KeeperWatcher::new(&self.config)?;
                        continue;
                    } else {
                        return Err(e).context("Claude-keeper watcher failed and cannot restart");
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a single usage entry
    async fn process_entry(
        &mut self,
        entry: UsageEntry,
        tx: &mpsc::Sender<LiveUpdate>,
    ) -> Result<()> {
        debug!(
            request_id = %entry.request_id,
            model = %entry.message.model,
            "Processing usage entry"
        );

        // Extract session information from the entry
        let session_id = entry.message.id.clone();
        
        // For now, use a simple project path extraction
        // In the future, this could be enhanced to use real project detection
        let project_path = "unknown".to_string();

        // Update or create session data
        let session_data = self.sessions.entry(session_id.clone())
            .or_insert_with(|| SessionData::new(session_id.clone(), project_path));

        // Update session with new usage data
        if let Some(usage) = &entry.message.usage {
            session_data.input_tokens += usage.input_tokens;
            session_data.output_tokens += usage.output_tokens;
            session_data.cache_creation_tokens += usage.cache_creation_input_tokens;
            session_data.cache_read_tokens += usage.cache_read_input_tokens;
            
            if let Some(cost) = entry.cost_usd {
                session_data.total_cost += cost;
            }
            
            session_data.models_used.insert(entry.message.model.clone());
            session_data.last_activity = Some(entry.timestamp.clone());
        }

        // Create live update
        let update = LiveUpdate {
            entry,
            session_stats: session_data.clone(),
            timestamp: SystemTime::now(),
        };

        // Send update through channel
        if let Err(e) = tx.send(update).await {
            warn!(error = %e, "Failed to send live update, channel may be closed");
        }

        Ok(())
    }

    /// Get current session summary
    #[allow(dead_code)]
    pub fn get_session_summary(&self) -> (usize, f64, u64) {
        let total_sessions = self.sessions.len();
        let total_cost = self.baseline.total_cost + 
            self.sessions.values().map(|s| s.total_cost).sum::<f64>();
        let total_tokens = self.baseline.total_tokens +
            self.sessions.values().map(|s| s.total_tokens() as u64).sum::<u64>();
        
        (total_sessions, total_cost, total_tokens)
    }
}