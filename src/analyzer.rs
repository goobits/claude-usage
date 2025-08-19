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

use crate::dedup::{DeduplicationEngine, ProcessOptions};
use crate::reports::ReportDisplayManager;
use crate::models::*;
use crate::parser::FileParser;
use crate::parser_wrapper::UnifiedParser;
use anyhow::Result;
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::warn;

pub struct ClaudeUsageAnalyzer {
    parser: UnifiedParser,
    file_parser: FileParser,
    dedup_engine: DeduplicationEngine,
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
            parser: UnifiedParser::new(),
            file_parser: FileParser::new(),
            dedup_engine: DeduplicationEngine::new(),
            display_manager: ReportDisplayManager::new(),
        }
    }

    pub async fn aggregate_data(
        &self,
        _command: &str,
        options: ProcessOptions,
    ) -> Result<Vec<SessionOutput>> {
        // Check and refresh baseline for daily/monthly commands
        use crate::live::baseline::{should_refresh_baseline, refresh_baseline, load_baseline_summary};
        
        // Only use baseline for daily/monthly commands (not for session command)
        let use_baseline = matches!(_command, "daily" | "monthly");
        
        let baseline = if use_baseline {
            // Check if we need to refresh the backup
            if should_refresh_baseline() {
                // Run backup if needed (this is async)
                refresh_baseline().await.unwrap_or_default()
            } else {
                // Load existing baseline
                load_baseline_summary().unwrap_or_default()
            }
        } else {
            crate::live::BaselineSummary::default()
        };

        // Discover Claude paths - use file_parser for discovery
        let paths = self
            .file_parser
            .discover_claude_paths(options.exclude_vms)?;

        if !options.json_output {
            println!(
                "🔍 Discovered {} Claude instance{}",
                paths.len(),
                if paths.len() == 1 { "" } else { "s" }
            );
        }

        // Find all JSONL files - use file_parser for discovery
        let mut all_jsonl_files = Vec::new();
        let mut files_filtered = 0;

        for claude_path in &paths {
            let file_tuples = self.file_parser.find_jsonl_files(std::slice::from_ref(claude_path))?;

            for (jsonl_file, session_dir) in file_tuples {
                // Pre-filter files by date range - use file_parser for filtering
                if self.file_parser.should_include_file(
                    &jsonl_file,
                    options.since_date.as_ref(),
                    options.until_date.as_ref(),
                ) {
                    // Also check if file is newer than baseline
                    let should_process = if use_baseline && baseline.last_backup != SystemTime::UNIX_EPOCH {
                        // Check if file was modified after the baseline backup
                        jsonl_file.metadata()
                            .and_then(|m| m.modified())
                            .map(|modified| modified > baseline.last_backup)
                            .unwrap_or(true) // If we can't check, process it
                    } else {
                        true // No baseline, process all files
                    };
                    
                    if should_process {
                        all_jsonl_files.push((jsonl_file, session_dir));
                    } else {
                        files_filtered += 1;
                    }
                } else {
                    files_filtered += 1;
                }
            }
        }

        if !options.json_output {
            if files_filtered > 0 {
                println!(
                    "📁 Found {} JSONL files (filtered out {} by date)",
                    all_jsonl_files.len(),
                    files_filtered
                );
            } else {
                println!(
                    "📁 Found {} JSONL files across all instances",
                    all_jsonl_files.len()
                );
            }
        }

        // Sort files by timestamp - use file_parser for sorting
        let sorted_files = self.file_parser.sort_files_by_timestamp(all_jsonl_files);

        // Process files with dedup
        let mut fresh_sessions = self.dedup_engine
            .process_files_with_global_dedup(sorted_files, &options, &self.parser)
            .await?;
        
        // Prepend baseline summary if we have one
        if use_baseline && baseline.total_cost > 0.0 {
            // Create a summary session from baseline
            let baseline_session = SessionOutput {
                session_id: "baseline".to_string(),
                project_path: "All Historical Projects".to_string(),
                input_tokens: (baseline.total_tokens / 2) as u32, // Approximate split
                output_tokens: (baseline.total_tokens / 2) as u32,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_cost: baseline.total_cost,
                last_activity: "Baseline (Cached)".to_string(),
                models_used: vec!["various".to_string()],
                daily_usage: HashMap::new(),
            };
            
            // Prepend baseline to results
            fresh_sessions.insert(0, baseline_session);
        }
        
        Ok(fresh_sessions)
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
