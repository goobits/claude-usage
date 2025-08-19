//! Usage Analysis Engine
//!
//! This module provides the main analysis engine that orchestrates the entire Claude usage
//! analysis pipeline. It serves as the primary entry point for all analysis operations,
//! coordinating between parsing, deduplication, aggregation, and reporting.
//!
//! ## Core Functionality
//!
//! The [`ClaudeUsageAnalyzer`] acts as the central coordinator, managing:
//!
//! ### Data Discovery and Parsing
//! - Discovers Claude instances across local projects and virtual machines
//! - Identifies and filters JSONL files based on date ranges
//! - Coordinates parallel parsing with configurable batch sizes
//! - Handles regular analysis modes
//!
//! ### Data Processing Pipeline
//! 1. **Discovery**: Finds Claude installation directories and JSONL files
//! 2. **Filtering**: Applies date range filters to reduce processing overhead
//! 3. **Parsing**: Processes files in parallel using the unified parser
//! 4. **Deduplication**: Removes duplicate entries using time-windowed deduplication
//! 5. **Aggregation**: Groups usage data by sessions and projects
//! 6. **Reporting**: Formats output for display or JSON export
//!
//! ### Command Processing
//! - **daily**: Generates daily usage reports with project breakdowns
//! - **monthly**: Creates monthly usage summaries
//!
//! ## Key Types
//!
//! - [`ClaudeUsageAnalyzer`] - Main analysis engine and coordinator
//!
//! ## Architecture Integration
//!
//! The analyzer integrates with all major system components:
//!
//! - [`UnifiedParser`] - Handles JSONL file parsing with schema flexibility
//! - [`FileParser`] - Provides file discovery and basic parsing utilities
//! - [`DeduplicationEngine`] - Prevents double-counting of usage data
//! - [`ReportDisplayManager`] - Formats and presents analysis results
//! ## Usage Example
//!
//! ```rust
//! use claude_usage::{ClaudeUsageAnalyzer, ProcessOptions};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut analyzer = ClaudeUsageAnalyzer::new();
//!
//! let options = ProcessOptions {
//!     command: "daily".to_string(),
//!     json_output: false,
//!     limit: Some(30),
//!     since_date: None,
//!     until_date: None,
//!     snapshot: false,
//!     exclude_vms: false,
//! };
//!
//! // Run analysis command
//! analyzer.run_command("daily", options).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Parallel Processing**: Files are processed in parallel chunks for optimal performance
//! - **Memory Efficiency**: Streaming processing minimizes memory usage
//! - **Intelligent Caching**: Deduplication engine maintains time-windowed caches
//! - **Early Exit Optimization**: Can stop processing early when limits are reached

use crate::dedup::ProcessOptions;
use crate::reports::ReportDisplayManager;
use crate::models::*;
use anyhow::Result;
use tracing::warn;

pub struct ClaudeUsageAnalyzer {
    display_manager: ReportDisplayManager,
}

impl Default for ClaudeUsageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeUsageAnalyzer {
    pub fn new() -> Self {
        Self {
            display_manager: ReportDisplayManager::new(),
        }
    }

    pub async fn aggregate_data(
        &self,
        _command: &str,
        options: ProcessOptions,
    ) -> Result<Vec<SessionOutput>> {
        // Check and refresh baseline for daily/monthly commands
        use crate::live::baseline::{should_refresh_baseline, refresh_baseline};
        use crate::parquet::reader::ParquetSummaryReader;
        use crate::config::get_config;
        
        // Only use Parquet data for daily/monthly commands
        let use_parquet = matches!(_command, "daily" | "monthly");
        
        if use_parquet {
            // Check if we need to refresh the backup
            if should_refresh_baseline() {
                // Run backup if needed (this is async)
                refresh_baseline().await.unwrap_or_default();
            }

            // Get backup directory from config
            let _config = get_config();
            // Use ~/.claude-backup/ as the default backup location (claude-keeper default)
            let backup_dir = dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".claude-backup");
            
            // Use ParquetSummaryReader to get detailed session data
            let reader = ParquetSummaryReader::new(backup_dir)?;
            let sessions = reader.read_detailed_sessions()?;

            if !options.json_output {
                println!(
                    "📊 Processed {} sessions from backup data",
                    sessions.len()
                );
            }

            // Apply date filtering if needed
            let mut filtered_sessions = sessions;
            if options.since_date.is_some() || options.until_date.is_some() {
                filtered_sessions = filtered_sessions.into_iter()
                    .filter(|session| {
                        // Parse session's last activity date
                        use chrono::NaiveDate;
                        
                        if let Some(date_str) = session.last_activity.split('T').next() {
                            if let Ok(session_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                                let session_datetime = session_date.and_hms_opt(0, 0, 0)
                                    .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc));
                                
                                if let Some(session_dt) = session_datetime {
                                    // Check since date
                                    if let Some(ref since) = options.since_date {
                                        if session_dt < *since {
                                            return false;
                                        }
                                    }
                                    
                                    // Check until date
                                    if let Some(ref until) = options.until_date {
                                        if session_dt > *until {
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                        true
                    })
                    .collect();
            }

            // Apply limit if specified
            if let Some(limit) = options.limit {
                filtered_sessions.truncate(limit);
            }

            Ok(filtered_sessions)
        } else {
            // For non-daily/monthly commands, return empty for now
            // This path could be extended later if needed
            Ok(Vec::new())
        }
    }

    pub async fn run_command(&mut self, command: &str, options: ProcessOptions) -> Result<()> {
        let data = self.aggregate_data(command, options.clone()).await?;

        if data.is_empty() {
            warn!("No Claude usage data found across all instances");
            if options.json_output {
                println!("[]");
            } else {
                println!("No Claude usage data found across all instances.");
            }
            return Ok(());
        }

        match command {
            "daily" => self.display_manager.display_daily(
                &data,
                options.limit,
                options.json_output,
            ),
            "monthly" => self.display_manager.display_monthly(
                &data,
                options.limit,
                options.json_output,
            ),
            _ => {
                anyhow::bail!("Unknown command: {}", command);
            }
        }

        Ok(())
    }
}
