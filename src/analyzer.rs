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
//! - [`DisplayManager`] - Formats and presents analysis results
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
use crate::reports::DisplayManager;
use crate::models::*;
use crate::parser::FileParser;
use crate::parser_wrapper::UnifiedParser;
use anyhow::Result;
use tracing::warn;

pub struct ClaudeUsageAnalyzer {
    parser: UnifiedParser,
    file_parser: FileParser,
    dedup_engine: DeduplicationEngine,
    display_manager: DisplayManager,
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
            display_manager: DisplayManager::new(),
        }
    }

    pub async fn aggregate_data(
        &self,
        _command: &str,
        options: ProcessOptions,
    ) -> Result<Vec<SessionOutput>> {
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
                    all_jsonl_files.push((jsonl_file, session_dir));
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

        // Pass UnifiedParser to dedup engine
        self.dedup_engine
            .process_files_with_global_dedup(sorted_files, &options, &self.parser)
            .await
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
