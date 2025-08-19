//! Real-time Session Monitoring
//!
//! This module provides real-time monitoring capabilities for active Claude Code sessions.
//! It displays live usage statistics, token consumption, cost tracking, and session progress
//! with an interactive terminal interface.
//!
//! ## Core Functionality
//!
//! ### Live Monitoring Features
//! - **Real-time Updates**: Refreshes every 3 seconds with current session data
//! - **Token Tracking**: Shows current token usage against configurable limits
//! - **Cost Monitoring**: Displays session costs with budget tracking
//! - **Progress Visualization**: ASCII progress bars for tokens, budget, and time
//! - **Burn Rate Analysis**: Calculates token and cost consumption rates
//! - **Session Detection**: Automatically finds and tracks active sessions
//!
//! ### Display Modes
//! - **Live Mode**: Continuous monitoring with terminal updates
//! - **Snapshot Mode**: Single-point-in-time report
//! - **JSON Output**: Machine-readable session status for automation
//! - **Terminal Interface**: Color-coded visual display with progress indicators
//!
//! ## Key Types
//!
//! - [`LiveMonitor`] - Main monitoring interface with caching and display logic
//!
//! ## Monitoring Display
//!
//! ### Visual Elements
//! The monitor displays several key metrics:
//!
//! ```text
//! [ CLAUDE USAGE MONITOR ]
//!
//! ⚡ Tokens:  🟢 ████████████░░░░░░░░ 65000 / 880000
//! 💲 Budget:  🟢 ███████░░░░░░░░░░░░░ $3.25 / $15.00
//! ♻️  Reset:   🟡 ██████████████░░░░░░ 2h 15m
//!
//! 🔥 125.5 tok/min | 💰 $2.40/hour
//!
//! 🕐 14:23 | 🏁 16:45 | ♻️  18:00
//!
//! ⛵ Smooth sailing...
//! ```
//!
//! ### Status Indicators
//! - 🟢 Green: Usage below 70% of limit
//! - 🟡 Yellow: Usage between 70-90% of limit  
//! - 🔴 Red: Usage above 90% of limit
//! - 📝 No active session when idle
//!
//! ### Metrics Tracked
//! - **Token Usage**: Current tokens vs. 880K limit (Claude Code's Max20 limit)
//! - **Budget Tracking**: Estimated costs vs. budget (~$1.50 per 1000 tokens)
//! - **Session Progress**: Time elapsed vs. session reset time
//! - **Burn Rates**: Tokens per minute and dollars per hour
//! - **Time Predictions**: Estimated depletion time and session reset
//!
//! ## Configuration
//!
//! ### Default Limits
//! - **Token Limit**: 880,000 tokens (Max20 configuration)
//! - **Budget Limit**: ~$1.50 per 1000 tokens
//! - **Refresh Rate**: 3 seconds
//! - **Cache Duration**: 30 seconds for session block data
//!
//! ## Session Detection
//!
//! The monitor automatically discovers active sessions by:
//! - Scanning Claude instance directories
//! - Reading session block files
//! - Finding sessions with end times in the future
//! - Caching results for performance
//!
//! ## Usage Examples
//!
//! ### Live Monitoring
//! ```rust
//! use claude_usage::monitor::LiveMonitor;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut monitor = LiveMonitor::new();
//!
//! // Start live monitoring (blocks until Ctrl+C)
//! monitor.run_live_monitor(false, false, false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Snapshot Mode
//! ```rust
//! use claude_usage::monitor::LiveMonitor;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut monitor = LiveMonitor::new();
//!
//! // Get single snapshot
//! monitor.run_live_monitor(false, true, false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### JSON Output
//! ```rust
//! use claude_usage::monitor::LiveMonitor;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut monitor = LiveMonitor::new();
//!
//! // Get JSON snapshot
//! monitor.run_live_monitor(true, true, false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration Points
//!
//! The monitor integrates with:
//! - [`crate::parser::FileParser`] for session block discovery and parsing
//! - [`crate::models::SessionBlock`] for session timing and token data
//! - Terminal control libraries for cursor management and screen clearing
//! - Tokio async runtime for non-blocking updates and signal handling

use crate::models::*;
use crate::parser::FileParser;
use anyhow::Result;
use chrono::Utc;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

pub struct LiveMonitor {
    file_parser: FileParser,
    cached_blocks: Option<Vec<SessionBlock>>,
    cache_time: Option<std::time::Instant>,
}

impl Default for LiveMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveMonitor {
    pub fn new() -> Self {
        Self {
            file_parser: FileParser::new(),
            cached_blocks: None,
            cache_time: None,
        }
    }

    pub async fn run_live_monitor(
        &mut self,
        json_output: bool,
        snapshot: bool,
        exclude_vms: bool,
    ) -> Result<()> {
        const TOKEN_LIMIT: u32 = 880000; // Max20 limit
        const BUDGET_LIMIT: f64 = TOKEN_LIMIT as f64 * 0.0015; // ~$1.50 per 1000 tokens

        // Store exclude_vms for use in other methods
        self.file_parser = FileParser::new(); // We'll pass exclude_vms to discover_claude_paths directly

        if json_output || snapshot {
            // Snapshot mode for JSON or when --snapshot is used
            self.display_snapshot(TOKEN_LIMIT, BUDGET_LIMIT, json_output, exclude_vms)
                .await?;
            return Ok(());
        }

        // Set up signal handling
        let mut interval = time::interval(Duration::from_secs(3));

        // Hide cursor
        self.hide_cursor();

        // Handle Ctrl+C gracefully
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        loop {
            tokio::select! {
                _ = &mut ctrl_c => {
                    self.show_cursor();
                    println!("\n\n\x1b[96mMonitoring stopped.\x1b[0m");
                    break;
                }
                _ = interval.tick() => {
                    self.display_live_data(TOKEN_LIMIT, BUDGET_LIMIT, exclude_vms).await?;
                }
            }
        }

        Ok(())
    }

    async fn display_live_data(
        &mut self,
        token_limit: u32,
        budget_limit: f64,
        exclude_vms: bool,
    ) -> Result<()> {
        self.clear_screen();

        let active_block = self.find_active_session_block(exclude_vms).await?;
        let current_time = chrono::Local::now().format("%H:%M").to_string();

        // Print header
        println!("\x1b[1m[ CLAUDE USAGE MONITOR ]\x1b[0m");
        println!();

        if let Some(block) = active_block {
            self.display_active_session(
                &block,
                token_limit,
                budget_limit,
                &current_time,
                exclude_vms,
            )
            .await?;
        } else {
            self.display_inactive_session(token_limit, budget_limit, &current_time, exclude_vms)
                .await?;
        }

        Ok(())
    }

    async fn display_snapshot(
        &mut self,
        token_limit: u32,
        budget_limit: f64,
        json_output: bool,
        exclude_vms: bool,
    ) -> Result<()> {
        let active_block = self.find_active_session_block(exclude_vms).await?;
        let current_time = chrono::Local::now().format("%H:%M").to_string();

        if json_output {
            let snapshot_data = if let Some(block) = active_block {
                self.create_snapshot_data(&block, token_limit, budget_limit)
                    .await?
            } else {
                serde_json::json!({
                    "status": "inactive",
                    "message": "No active session"
                })
            };

            println!("{}", serde_json::to_string_pretty(&snapshot_data)?);
        } else {
            println!("\x1b[1m[ CLAUDE USAGE MONITOR ]\x1b[0m");
            println!();

            if let Some(block) = active_block {
                self.display_active_session(
                    &block,
                    token_limit,
                    budget_limit,
                    &current_time,
                    exclude_vms,
                )
                .await?;
                println!("\n[Snapshot mode - aggregated from active sessions across {} Claude instances]", 
                         self.file_parser.discover_claude_paths(exclude_vms)?.len());
            } else {
                self.display_inactive_session(
                    token_limit,
                    budget_limit,
                    &current_time,
                    exclude_vms,
                )
                .await?;
                println!(
                    "\n[Snapshot mode - scanned {} Claude instances]",
                    self.file_parser.discover_claude_paths(exclude_vms)?.len()
                );
            }
        }

        Ok(())
    }

    async fn display_active_session(
        &self,
        block: &SessionBlock,
        token_limit: u32,
        budget_limit: f64,
        current_time: &str,
        _exclude_vms: bool,
    ) -> Result<()> {
        let start_time = self.file_parser.parse_timestamp(&block.start_time)?;
        let end_time = self.file_parser.parse_timestamp(&block.end_time)?;
        let now = Utc::now();

        let total_tokens = block.token_counts.total();
        let cost_used = block.cost_usd;

        // Calculate session progress
        let total_session_minutes = (end_time - start_time).num_seconds() as f64 / 60.0;
        let elapsed_minutes = (now - start_time).num_seconds().max(0) as f64 / 60.0;
        let remaining_minutes = (end_time - now).num_seconds().max(0) as f64 / 60.0;

        // Progress percentages
        let token_percentage = (total_tokens as f64 / token_limit as f64) * 100.0;
        let token_status = if token_percentage < 70.0 {
            "🟢"
        } else if token_percentage < 90.0 {
            "🟡"
        } else {
            "🔴"
        };

        let budget_percentage = (cost_used / budget_limit) * 100.0;
        let budget_status = if budget_percentage < 70.0 {
            "🟢"
        } else if budget_percentage < 90.0 {
            "🟡"
        } else {
            "🔴"
        };

        let reset_percentage = if total_session_minutes > 0.0 {
            (elapsed_minutes / total_session_minutes) * 100.0
        } else {
            0.0
        };

        // Calculate burn rates
        let burn_rate = if elapsed_minutes > 0.0 {
            total_tokens as f64 / elapsed_minutes
        } else {
            0.0
        };

        let cost_burn_rate = if elapsed_minutes > 0.0 {
            (cost_used / elapsed_minutes) * 60.0 // per hour
        } else {
            0.0
        };

        // Time displays
        let reset_time = end_time.format("%H:%M").to_string();

        // Predict when tokens will run out
        let predicted_end_str = if burn_rate > 0.0 && total_tokens < token_limit {
            let tokens_left = token_limit - total_tokens;
            let minutes_to_depletion = tokens_left as f64 / burn_rate;
            let predicted_end = now + chrono::Duration::minutes(minutes_to_depletion as i64);
            predicted_end.format("%H:%M").to_string()
        } else if total_tokens >= token_limit {
            "LIMIT HIT".to_string()
        } else {
            reset_time.clone()
        };

        // Status message
        let status_message = if total_tokens > token_limit {
            format!(
                "🚨 Session tokens exceeded limit! ({} > {})",
                total_tokens, token_limit
            )
        } else if budget_percentage > 90.0 {
            "💸 High session cost!".to_string()
        } else if token_percentage > 90.0 {
            "🔥 High session usage!".to_string()
        } else {
            "⛵ Smooth sailing...".to_string()
        };

        // Display the monitor
        println!(
            "⚡ Tokens:  {} {} / {}",
            self.create_progress_bar(token_percentage, 20, token_status),
            total_tokens,
            token_limit
        );
        println!(
            "💲 Budget:  {} ${:.2} / ${:.2}",
            self.create_progress_bar(budget_percentage, 20, budget_status),
            cost_used,
            budget_limit
        );
        println!(
            "♻️  Reset:   {} {}",
            self.create_progress_bar(reset_percentage, 20, "🕐"),
            self.format_time(remaining_minutes)
        );
        println!();

        let burn_rate_str = if burn_rate > 0.0 {
            format!("{:.1} tok/min", burn_rate)
        } else {
            "0.0 tok/min".to_string()
        };

        let cost_rate_str = if cost_burn_rate > 0.0 {
            format!("${:.2}/hour", cost_burn_rate)
        } else {
            "$0.00/hour".to_string()
        };

        println!("🔥 {} | 💰 {}", burn_rate_str, cost_rate_str);
        println!();
        println!(
            "🕐 {} | 🏁 {} | ♻️  {}",
            current_time, predicted_end_str, reset_time
        );
        println!();
        println!("{}", status_message);

        Ok(())
    }

    async fn display_inactive_session(
        &self,
        token_limit: u32,
        budget_limit: f64,
        current_time: &str,
        _exclude_vms: bool,
    ) -> Result<()> {
        println!(
            "⚡ Tokens:  {} 0 / {}",
            self.create_progress_bar(0.0, 20, "🟢"),
            token_limit
        );
        println!(
            "💲 Budget:  {} $0.00 / ${:.2}",
            self.create_progress_bar(0.0, 20, "🟢"),
            budget_limit
        );
        println!(
            "♻️  Reset:   {} 0m",
            self.create_progress_bar(0.0, 20, "🕐")
        );
        println!();
        println!("🔥 0.0 tok/min | 💰 $0.00/hour");
        println!();
        println!("🕐 {} | 🏁 No session | ♻️  Next reset", current_time);
        println!();
        println!("📝 No active session");

        Ok(())
    }

    async fn create_snapshot_data(
        &self,
        block: &SessionBlock,
        token_limit: u32,
        budget_limit: f64,
    ) -> Result<serde_json::Value> {
        let start_time = self.file_parser.parse_timestamp(&block.start_time)?;
        let end_time = self.file_parser.parse_timestamp(&block.end_time)?;
        let now = Utc::now();

        let total_tokens = block.token_counts.total();
        let cost_used = block.cost_usd;

        let elapsed_minutes = (now - start_time).num_seconds().max(0) as f64 / 60.0;
        let remaining_minutes = (end_time - now).num_seconds().max(0) as f64 / 60.0;

        let burn_rate = if elapsed_minutes > 0.0 {
            total_tokens as f64 / elapsed_minutes
        } else {
            0.0
        };

        let cost_burn_rate = if elapsed_minutes > 0.0 {
            (cost_used / elapsed_minutes) * 60.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "status": "active",
            "tokens": {
                "current": total_tokens,
                "limit": token_limit,
                "percentage": (total_tokens as f64 / token_limit as f64) * 100.0
            },
            "cost": {
                "current": cost_used,
                "limit": budget_limit
            },
            "timing": {
                "elapsed_minutes": elapsed_minutes,
                "remaining_minutes": remaining_minutes,
                "current_time": chrono::Local::now().format("%H:%M").to_string()
            },
            "burn_rates": {
                "tokens_per_minute": burn_rate,
                "cost_per_hour": cost_burn_rate
            },
            "session_count": 1
        }))
    }

    async fn find_active_session_block(
        &mut self,
        exclude_vms: bool,
    ) -> Result<Option<SessionBlock>> {
        let current_time = std::time::Instant::now();

        // Use cache if available and recent (30 seconds)
        if let (Some(blocks), Some(cache_time)) = (&self.cached_blocks, &self.cache_time) {
            if current_time.duration_since(*cache_time).as_secs() < 30 {
                let now = Utc::now();
                for block in blocks {
                    if let Ok(end_time) = self.file_parser.parse_timestamp(&block.end_time) {
                        if end_time > now {
                            return Ok(Some(block.clone()));
                        }
                    }
                }
                return Ok(None);
            }
        }

        // Load fresh session blocks
        let claude_paths = self.file_parser.discover_claude_paths(exclude_vms)?;
        let blocks = self.file_parser.get_latest_session_blocks(&claude_paths)?;
        let now = Utc::now();

        // Find active block
        for block in &blocks {
            if let Ok(end_time) = self.file_parser.parse_timestamp(&block.end_time) {
                if end_time > now {
                    // Update cache
                    self.cached_blocks = Some(blocks.clone());
                    self.cache_time = Some(current_time);
                    return Ok(Some(block.clone()));
                }
            }
        }

        // No active session blocks found
        self.cached_blocks = Some(blocks.clone());
        self.cache_time = Some(current_time);
        Ok(None)
    }

    fn create_progress_bar(&self, percentage: f64, width: usize, status_color: &str) -> String {
        let pct = percentage.clamp(0.0, 100.0);

        if pct >= 100.0 {
            let filled = "█".repeat(width);
            return format!("{} {}", status_color, filled);
        }

        let filled = (width as f64 * pct / 100.0) as usize;
        let cursor = if pct < 100.0 { 1 } else { 0 };
        let empty = width.saturating_sub(filled).saturating_sub(cursor);

        let filled_bar = "█".repeat(filled);
        let cursor_char = if cursor > 0 { "▓" } else { "" };
        let empty_bar = "░".repeat(empty);

        format!(
            "{} {}{}{}",
            status_color, filled_bar, cursor_char, empty_bar
        )
    }

    fn format_time(&self, minutes: f64) -> String {
        if minutes < 60.0 {
            format!("{}m", minutes as i32)
        } else {
            let hours = (minutes / 60.0) as i32;
            let mins = (minutes % 60.0) as i32;
            if mins == 0 {
                format!("{}h", hours)
            } else {
                format!("{}h {}m", hours, mins)
            }
        }
    }

    fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H");
        io::stdout()
            .flush()
            .expect("Failed to flush stdout for screen clear");
    }

    fn hide_cursor(&self) {
        print!("\x1b[?25l");
        io::stdout()
            .flush()
            .expect("Failed to flush stdout for cursor hide");
    }

    fn show_cursor(&self) {
        print!("\x1b[?25h");
        io::stdout()
            .flush()
            .expect("Failed to flush stdout for cursor show");
    }
}
