//! Live mode module for real-time Claude usage monitoring
//!
//! This module provides real-time monitoring capabilities by integrating with
//! claude-keeper to stream usage updates as they occur.

use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::models::{UsageEntry, SessionData};

pub mod orchestrator;
pub mod baseline;
pub mod watcher;

/// Live mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveConfig {
    /// Maximum time to wait for claude-keeper subprocess to start (seconds)
    pub startup_timeout_secs: u64,
    /// Maximum number of restart attempts for claude-keeper subprocess
    pub max_restart_attempts: u32,
    /// Buffer size for the update channel
    pub update_channel_buffer: usize,
    /// Path to claude-keeper executable
    pub claude_keeper_path: String,
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            startup_timeout_secs: 30,
            max_restart_attempts: 3,
            update_channel_buffer: 100,
            claude_keeper_path: "claude-keeper".to_string(),
        }
    }
}

/// Summary data from baseline parquet files
#[derive(Debug, Clone)]
pub struct BaselineSummary {
    /// Total cost from baseline data
    pub total_cost: f64,
    /// Total tokens from baseline data
    pub total_tokens: u64,
    /// Number of sessions today from baseline data
    pub sessions_today: u32,
    /// Timestamp of last backup
    pub last_backup: SystemTime,
}

impl Default for BaselineSummary {
    fn default() -> Self {
        Self {
            total_cost: 0.0,
            total_tokens: 0,
            sessions_today: 0,
            last_backup: SystemTime::UNIX_EPOCH,
        }
    }
}

/// Real-time update from claude-keeper watch mode
#[derive(Debug, Clone)]
pub struct LiveUpdate {
    /// The usage entry from claude-keeper
    pub entry: UsageEntry,
    /// Current session statistics
    pub session_stats: SessionData,
    /// Timestamp when this update was received
    pub timestamp: SystemTime,
}

/// Session statistics for live updates
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub project_path: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub total_cost: f64,
}

impl SessionStats {
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
}

impl From<SessionData> for SessionStats {
    fn from(data: SessionData) -> Self {
        Self {
            session_id: data.session_id,
            project_path: data.project_path,
            input_tokens: data.input_tokens,
            output_tokens: data.output_tokens,
            cache_creation_tokens: data.cache_creation_tokens,
            cache_read_tokens: data.cache_read_tokens,
            total_cost: data.total_cost,
        }
    }
}