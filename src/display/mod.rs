//! Display Module for Terminal UI and Live Mode
//!
//! This module provides the terminal user interface (TUI) components for real-time
//! monitoring of Claude usage. It includes responsive layouts, live updates, and
//! interactive features for monitoring usage across sessions and projects.
//!
//! ## Core Components
//!
//! - [`LiveDisplay`] - Core display state with ring buffer for recent entries
//! - [`DisplayManager`] - Terminal UI manager using ratatui with crossterm backend
//! - [`RunningTotals`] - Running totals for cost, tokens, and sessions
//! - [`SessionActivity`] - Recent activity tracking with timestamps
//!
//! ## TUI Layout
//!
//! The terminal interface provides a professional, responsive layout:
//!
//! ```text
//! ┌─ Claude Usage Live ─────────────────────────┐
//! │ Total: $45.23 | Tokens: 1.2M | Sessions: 15 │
//! ├──────────────────────────────────────────────┤
//! │ Current Session                              │
//! │ ├─ Cost: $2.10                              │
//! │ ├─ Duration: 5m 23s                         │
//! │ └─ Tokens: In 10K / Out 15K                 │
//! ├──────────────────────────────────────────────┤
//! │ Recent Activity (↑/↓ to scroll)             │
//! │ [12:05:23] Project A: +500 tokens ($0.05)   │
//! │ [12:04:15] Project B: +1200 tokens ($0.12)  │
//! │ [12:03:45] Project A: +300 tokens ($0.03)   │
//! └─────────────────────── Ctrl+C to exit ──────┘
//! ```
//!
//! ## Features
//!
//! - **Real-time Updates**: Processes live updates via async channels from orchestrator
//! - **Ring Buffer**: Maintains exactly 100 recent entries with FIFO behavior
//! - **Keyboard Navigation**: ↑/↓ arrows for scrolling, Ctrl+C to exit
//! - **Responsive Design**: Handles terminal resize gracefully
//! - **Memory Efficient**: No unbounded growth, fixed-size buffers
//!
//! ## Integration
//!
//! The display manager integrates with the live orchestrator via async channels:
//!
//! ```rust
//! use claude_usage::display::run_display;
//! use claude_usage::live::{BaselineSummary, LiveUpdate};
//!
//! let baseline = BaselineSummary::default();
//! let (tx, rx) = tokio::sync::mpsc::channel(100);
//!
//! // Run display in async context
//! run_display(baseline, rx).await?;
//! ```

pub mod tui;
pub mod state;
pub mod widgets;

pub use tui::*;
pub use state::*;
pub use widgets::*;

use crate::live::{BaselineSummary, LiveUpdate};
use anyhow::Result;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// Main entry point for running the live display
///
/// This function sets up the terminal UI and starts the display loop,
/// processing live updates from the provided channel.
///
/// # Arguments
///
/// * `baseline` - Summary of existing usage data from parquet files
/// * `update_receiver` - Channel for receiving real-time updates
///
/// # Returns
///
/// Returns `Ok(())` when the display exits normally, or an error if
/// terminal setup or update processing fails.
pub async fn run_display(
    baseline: BaselineSummary,
    update_receiver: mpsc::Receiver<LiveUpdate>
) -> Result<()> {
    let mut display_manager = DisplayManager::new(baseline, update_receiver).await?;
    display_manager.run().await
}

/// Running totals maintained across all updates
#[derive(Debug, Clone)]
pub struct RunningTotals {
    /// Total cost including baseline and live updates
    pub total_cost: f64,
    /// Total tokens including baseline and live updates
    pub total_tokens: u64,
    /// Total number of sessions
    pub total_sessions: u32,
}

impl RunningTotals {
    /// Create new running totals from baseline
    pub fn from_baseline(baseline: &BaselineSummary) -> Self {
        Self {
            total_cost: baseline.total_cost,
            total_tokens: baseline.total_tokens,
            total_sessions: baseline.sessions_today,
        }
    }

    /// Update totals with a new live update
    pub fn update(&mut self, update: &LiveUpdate) {
        if let Some(cost) = update.entry.cost_usd {
            self.total_cost += cost;
        }

        if let Some(ref usage) = update.entry.message.usage {
            self.total_tokens += (usage.input_tokens + usage.output_tokens +
                usage.cache_creation_input_tokens + usage.cache_read_input_tokens) as u64;
        }
    }
}

/// Recent activity entry for the activity log
#[derive(Debug, Clone)]
pub struct SessionActivity {
    /// Timestamp when this activity occurred
    pub timestamp: SystemTime,
    /// Human-readable time string (e.g., "12:05:23")
    pub time_str: String,
    /// Project path or name
    pub project: String,
    /// Number of tokens in this activity
    pub tokens: u32,
    /// Cost for this activity
    pub cost: f64,
    /// Session ID this activity belongs to
    pub session_id: String,
}

impl SessionActivity {
    /// Create new session activity from a live update
    pub fn from_update(update: &LiveUpdate) -> Self {
        let tokens = if let Some(ref usage) = update.entry.message.usage {
            usage.input_tokens + usage.output_tokens +
                usage.cache_creation_input_tokens + usage.cache_read_input_tokens
        } else {
            0
        };

        let cost = update.entry.cost_usd.unwrap_or(0.0);

        // Extract project name from path (take last component)
        let project = update.session_stats.project_path
            .split('/')
            .last()
            .unwrap_or(&update.session_stats.project_path)
            .to_string();

        // Format timestamp as HH:MM:SS
        let time_str = {
            use std::time::UNIX_EPOCH;
            let duration = update.timestamp.duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0));
            let secs = duration.as_secs();
            let hours = (secs / 3600) % 24;
            let minutes = (secs / 60) % 60;
            let seconds = secs % 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        };

        Self {
            timestamp: update.timestamp,
            time_str,
            project,
            tokens,
            cost,
            session_id: update.session_stats.session_id.clone(),
        }
    }
}