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
use crate::live::baseline::{load_baseline_summary, refresh_baseline, should_refresh_baseline};
use crate::live::watcher::KeeperWatcher;
use crate::models::{SessionData, UsageEntry};

/// Format token count with appropriate units (K, M)
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Main orchestrator for live mode operations
pub struct LiveOrchestrator {
    config: LiveConfig,
    baseline: BaselineSummary,
    sessions: HashMap<String, SessionData>,
    no_baseline: bool,
}

impl LiveOrchestrator {
    /// Create a new live orchestrator
    pub async fn new(no_baseline: bool) -> Result<Self> {
        let config = LiveConfig::default(); // Use default for now
        
        let baseline = if no_baseline {
            info!("Skipping baseline loading (--no-baseline specified)");
            BaselineSummary::default()
        } else {
            // Check if we need to refresh baseline
            match should_refresh_baseline() {
                true => {
                    println!("📦 Creating baseline from conversation history...");
                    println!("⏳ Running auto-backup (this may take 10-30 seconds)...");
                    info!("Refreshing baseline data (missing or stale)...");
                    
                    refresh_baseline().await.unwrap_or_else(|e| {
                        println!("⚠️  Auto-backup encountered an issue, using existing data");
                        println!("💡 Live monitoring will still work for new conversations");
                        warn!(error = %e, "Auto-backup failed, using existing baseline");
                        load_baseline_summary().unwrap_or_default()
                    })
                }
                false => {
                    println!("📚 Loading existing baseline data...");
                    load_baseline_summary().unwrap_or_else(|e| {
                        println!("⚠️  Unable to load baseline, creating fresh backup...");
                        warn!(error = %e, "Failed to load baseline, trying auto-backup");
                        // Fallback: try refresh if load fails
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                refresh_baseline().await.unwrap_or_else(|e| {
                                    println!("❌ Backup creation failed - starting with empty baseline");
                                    println!("💡 You'll see all new usage starting from now");
                                    warn!(error = %e, "Fallback auto-backup also failed, using empty baseline");
                                    BaselineSummary::default()
                                })
                            })
                        })
                    })
                }
            }
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
        // Show baseline summary to user
        if !self.no_baseline && (self.baseline.total_cost > 0.0 || self.baseline.total_tokens > 0) {
            println!("📈 Baseline loaded successfully:");
            println!("   💰 Total cost: ${:.2}", self.baseline.total_cost);
            println!("   🎯 Total tokens: {}", format_tokens(self.baseline.total_tokens));
            println!("   📅 Sessions today: {}", self.baseline.sessions_today);
        } else if !self.no_baseline {
            println!("🆕 Starting fresh - no previous usage data found");
            println!("💡 New conversations will appear as you use Claude");
        }
        println!();
        
        info!(
            baseline_cost = self.baseline.total_cost,
            baseline_tokens = self.baseline.total_tokens,
            sessions_today = self.baseline.sessions_today,
            "Starting live mode orchestrator"
        );

        // Start claude-keeper watcher
        println!("🔗 Connecting to claude-keeper for live updates...");
        let mut watcher = KeeperWatcher::new(&self.config)?;
        
        // Flag to track first successful connection
        let mut first_connection = true;
        
        // Main processing loop
        loop {
            // Get next usage entry from claude-keeper
            match watcher.next_entry().await {
                Ok(Some(entry)) => {
                    // Show success message on first entry
                    if first_connection {
                        println!("✅ Connected! Now monitoring live Claude usage...");
                        println!("💡 Use new Claude conversations to see real-time updates");
                        println!();
                        first_connection = false;
                    }
                    
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
                        println!("⚠️  Connection lost, attempting to reconnect...");
                        warn!("Attempting to restart claude-keeper watcher");
                        watcher = KeeperWatcher::new(&self.config)?;
                        continue;
                    } else {
                        println!("❌ Connection failed permanently after multiple attempts");
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

    /// Get the baseline summary
    pub fn get_baseline(&self) -> BaselineSummary {
        self.baseline.clone()
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