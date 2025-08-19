//! Deduplication Engine
//!
//! This module provides intelligent deduplication capabilities to prevent double-counting
//! of Claude usage data across multiple analysis runs and overlapping data sources.
//! The engine uses time-windowed hashing with automatic cleanup for optimal performance.
//!
//! ## Core Functionality
//!
//! ### Deduplication Strategy
//! - **Unique Hash Generation**: Creates deterministic hashes from usage entry content
//! - **Time-Windowed Deduplication**: Only considers duplicates within a configurable time window
//! - **Global Tracking**: Maintains global state across all processed files and sessions
//! - **Automatic Cleanup**: Periodically removes old hashes to prevent memory growth
//!
//! ### Processing Pipeline
//! The deduplication engine coordinates the complete data processing pipeline:
//!
//! 1. **File Processing**: Handles files in parallel chunks with configurable batch sizes
//! 2. **Entry Validation**: Filters out entries without valid usage data
//! 3. **Duplicate Detection**: Uses content-based hashing with time-window validation
//! 4. **Cost Calculation**: Integrates with pricing manager for accurate cost computation
//! 5. **Session Aggregation**: Groups entries by session and project with daily breakdowns
//! 6. **Project Path Extraction**: Intelligently extracts meaningful project names from paths
//!
//! ## Key Types
//!
//! - [`DeduplicationEngine`] - Main deduplication coordinator
//! - [`ProcessOptions`] - Configuration for processing operations
//!
//! ## Configuration
//!
//! The engine respects configuration settings from [`crate::config`]:
//! - `dedup.window_hours` - Time window for considering duplicates (default: 24 hours)
//! - `dedup.cleanup_threshold` - Number of hashes before triggering cleanup (default: 10,000)
//! - `processing.batch_size` - Number of files to process in parallel (default: 10)
//!
//! ## Performance Optimizations
//!
//! ### Memory Management
//! - **Streaming Processing**: Processes files without loading entire dataset into memory
//! - **Periodic Cleanup**: Automatically removes old hash entries to prevent memory growth
//! - **Efficient Data Structures**: Uses DashMap for concurrent access with minimal locking
//!
//! ### Parallel Processing
//! - **Chunked Processing**: Processes files in parallel chunks for optimal throughput
//! - **Early Exit**: Stops processing when limits are reached (for session queries)
//! - **Rayon Integration**: Leverages work-stealing for efficient parallel execution
//!
//! ### Intelligent Filtering
//! - **Date Range Filtering**: Pre-filters files by modification time before parsing
//! - **Usage Data Validation**: Skips entries without meaningful token usage
//! - **Duplicate Skip**: Fast hash-based duplicate detection with time constraints
//!
//! ## Project Path Extraction
//!
//! The engine includes sophisticated logic for extracting meaningful project names:
//! - Handles standard project structures: `projects/project-name`
//! - Supports VM-based projects: `vms/vm-name/projects/...`
//! - Processes encoded paths with dash-separated components
//! - Provides fallback handling for unexpected directory structures
//!
//! ## Usage Example
//!
//! ```rust
//! use claude_usage::dedup::{DeduplicationEngine, ProcessOptions};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let engine = DeduplicationEngine::new();
//! let options = ProcessOptions {
//!     command: "daily".to_string(),
//!     json_output: false,
//!     limit: None,
//!     since_date: None,
//!     until_date: None,
//!     snapshot: false,
//!     exclude_vms: false,
//! };
//!
//! // Process files with deduplication
//! let sessions = engine.process_files_with_global_dedup(
//!     file_tuples,
//!     &options,
//!     &parser
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::config::get_config;
use crate::memory;
use crate::models::{DailyUsage, *};
use crate::parser::FileParser;
use crate::parser_wrapper::UnifiedParser;
use crate::pricing::PricingManager;
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashSet;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct DeduplicationEngine {
    global_hashes: Arc<DashSet<String>>,
    hash_timestamps: Arc<dashmap::DashMap<String, DateTime<Utc>>>,
    dedup_window_hours: i64,
    dedup_cleanup_threshold: usize,
}

impl Default for DeduplicationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DeduplicationEngine {
    pub fn new() -> Self {
        let config = get_config();

        Self {
            global_hashes: Arc::new(DashSet::new()),
            hash_timestamps: Arc::new(dashmap::DashMap::new()),
            dedup_window_hours: config.dedup.window_hours,
            dedup_cleanup_threshold: config.dedup.cleanup_threshold,
        }
    }

    pub async fn process_files_with_global_dedup(
        &self,
        sorted_file_tuples: Vec<(PathBuf, PathBuf)>,
        options: &ProcessOptions,
        parser: &UnifiedParser,
    ) -> Result<Vec<SessionOutput>> {
        let file_parser = FileParser::new();
        let mut sessions_by_dir: HashMap<PathBuf, SessionData> = HashMap::new();

        let need_timestamps = matches!(options.command.as_str(), "daily" | "session" | "monthly");
        let mut total_entries_processed = 0;
        let mut total_entries_skipped = 0;
        let mut session_count = 0;

        // Early exit optimization for --limit N queries (only for session command)
        let should_stop_early = options.limit.is_some() && options.command == "session";

        // Process files in parallel chunks for better performance
        let base_chunk_size = get_config().processing.batch_size;
        let adaptive_chunk_size = memory::get_adaptive_batch_size(base_chunk_size);
        let mut _processed_files = 0;

        // Log adaptive sizing decision
        tracing::debug!(
            base_chunk_size = base_chunk_size,
            adaptive_chunk_size = adaptive_chunk_size,
            memory_pressure = ?memory::get_pressure_level(),
            "Using adaptive chunk size for parallel processing"
        );

        for chunk in sorted_file_tuples.chunks(adaptive_chunk_size) {
            // Early exit optimization
            if should_stop_early && session_count >= options.limit.unwrap_or(0) {
                break;
            }

            // Check memory pressure before processing chunk
            if memory::check_memory_pressure() {
                tracing::warn!(
                    chunk_files = chunk.len(),
                    memory_stats = ?memory::get_memory_stats(),
                    "Memory pressure detected before processing chunk"
                );

                // Try to trigger GC if needed
                memory::try_gc_if_needed()?;

                // If critical pressure, consider processing smaller chunks
                if memory::should_spill_to_disk() {
                    tracing::warn!(
                        chunk_files = chunk.len(),
                        "Critical memory pressure - consider reducing chunk size"
                    );
                }
            }

            // Process chunk in parallel - USE UnifiedParser
            let chunk_results: Vec<_> = chunk
                .par_iter()
                .map(|(jsonl_file, session_dir)| {
                    // Track memory for each file being processed
                    let file_size = std::fs::metadata(jsonl_file)
                        .map(|m| m.len() as usize)
                        .unwrap_or(0);
                    memory::track_allocation(file_size);

                    let entries = parser.parse_jsonl_file(jsonl_file)?;

                    // Clean up file memory tracking
                    memory::track_deallocation(file_size);

                    Ok::<_, anyhow::Error>((entries, session_dir.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            // Process results sequentially to maintain deduplication correctness
            for (entries, session_dir) in chunk_results {
                let mut has_session_data = false;
                _processed_files += 1;

                for entry in entries {
                    // Check if entry has usage data (match Python behavior)
                    let Some(usage) = &entry.message.usage else {
                        continue; // Skip entries without usage data
                    };
                    if usage.input_tokens == 0 && usage.output_tokens == 0 {
                        continue;
                    }

                    total_entries_processed += 1;

                    // Create unique hash for deduplication - use file_parser for utility
                    let unique_hash = file_parser.create_unique_hash(&entry);

                    // Get current entry timestamp - use file_parser for utility
                    let current_timestamp = file_parser.parse_timestamp(&entry.timestamp).ok();

                    // Optimized deduplication: only check within time window
                    let mut skip_duplicate = false;
                    if let Some(hash) = &unique_hash {
                        if self.global_hashes.contains(hash) {
                            if let Some(hash_time) = self.hash_timestamps.get(hash) {
                                if let Some(current_time) = current_timestamp {
                                    let time_diff = (current_time - *hash_time).num_hours().abs();
                                    if time_diff <= self.dedup_window_hours {
                                        skip_duplicate = true;
                                    }
                                } else {
                                    skip_duplicate = true;
                                }
                            } else {
                                skip_duplicate = true;
                            }
                        }
                    }

                    if skip_duplicate {
                        total_entries_skipped += 1;
                        tracing::debug!(
                            message_id = %entry.message.id,
                            request_id = %entry.request_id,
                            "Skipping duplicate entry"
                        );
                        continue;
                    }
                    
                    tracing::debug!(
                        message_id = %entry.message.id,
                        request_id = %entry.request_id,
                        cost_usd = ?entry.cost_usd,
                        "Processing entry for cost calculation"
                    );

                    // Mark as processed globally
                    if let Some(hash) = &unique_hash {
                        self.global_hashes.insert(hash.clone());
                        if let Some(timestamp) = current_timestamp {
                            self.hash_timestamps.insert(hash.clone(), timestamp);
                        }
                    }

                    // Periodic cleanup of old dedup hashes
                    if self.hash_timestamps.len() > self.dedup_cleanup_threshold {
                        if let Some(current_time) = current_timestamp {
                            let cutoff_time =
                                current_time - chrono::Duration::hours(self.dedup_window_hours * 2);

                            // Use retain() for efficient in-place cleanup without allocating a vector
                            self.hash_timestamps.retain(|key, timestamp| {
                                if *timestamp < cutoff_time {
                                    // Also remove from global_hashes when removing from timestamps
                                    self.global_hashes.remove(key);
                                    false // Remove this entry from hash_timestamps
                                } else {
                                    true // Keep this entry
                                }
                            });
                        }
                    }

                    // Calculate cost based on mode
                    let entry_cost = self.calculate_entry_cost(&entry).await;

                    // Extract session info with more context
                    let session_dir_name = session_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    // Extract meaningful project path, stripping only the home/.claude/ base
                    let full_path = session_dir.to_string_lossy();
                    let project_name = if let Some(claude_pos) = full_path.find("/.claude/") {
                        let after_claude = &full_path[claude_pos + 9..]; // "/.claude/" is 9 chars

                        if let Some(project_part) = after_claude.strip_prefix("projects/") {
                            // For main projects, check if it's a simple project name or has path structure
                            // Skip "projects/"
                            if project_part.starts_with('-') {
                                // Handle cases like "projects/-home-miko-projects-system-weather" -> "projects/system-weather"
                                if let Some(last_dash) = project_part.rfind('-') {
                                    let suffix = &project_part[last_dash + 1..];
                                    if !suffix.is_empty() && suffix != "projects" {
                                        format!("projects/{}", suffix)
                                    } else {
                                        "projects".to_string()
                                    }
                                } else {
                                    "projects".to_string()
                                }
                            } else {
                                format!("projects/{}", project_part)
                            }
                        } else if let Some(vm_part) = after_claude.strip_prefix("vms/") {
                            // For VMs, extract vm_name from "vms/vm_name/projects/-workspace" -> "vms/vm_name"
                            // Skip "vms/"
                            if let Some(slash_pos) = vm_part.find('/') {
                                let vm_name = &vm_part[..slash_pos];
                                format!("vms/{}", vm_name)
                            } else {
                                after_claude.to_string()
                            }
                        } else {
                            session_dir_name.to_string()
                        }
                    } else {
                        // Fallback: just use the directory name
                        session_dir_name.to_string()
                    };

                    let (session_id, _) = file_parser.extract_session_info(session_dir_name);

                    // Get or create session data (use full path like Python)
                    let session_data = sessions_by_dir
                        .entry(session_dir.clone())
                        .or_insert_with(|| SessionData::new(session_id, project_name));

                    // Get the date for this entry (use UTC for consistent bucketing)
                    let entry_date =
                        if let Ok(timestamp) = file_parser.parse_timestamp(&entry.timestamp) {
                            timestamp.format("%Y-%m-%d").to_string()
                        } else {
                            "unknown".to_string()
                        };

                    // Update daily usage for this specific date
                    let daily = session_data
                        .daily_usage
                        .entry(entry_date.clone())
                        .or_insert_with(|| DailyUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_creation_tokens: 0,
                            cache_read_tokens: 0,
                            cost: 0.0,
                        });

                    // Aggregate usage data for this day
                    if let Some(usage) = &entry.message.usage {
                        daily.input_tokens += usage.input_tokens;
                        daily.output_tokens += usage.output_tokens;
                        daily.cache_creation_tokens += usage.cache_creation_input_tokens;
                        daily.cache_read_tokens += usage.cache_read_input_tokens;

                        // Also update totals
                        session_data.input_tokens += usage.input_tokens;
                        session_data.output_tokens += usage.output_tokens;
                        session_data.cache_creation_tokens += usage.cache_creation_input_tokens;
                        session_data.cache_read_tokens += usage.cache_read_input_tokens;
                    }
                    daily.cost += entry_cost;
                    session_data.total_cost += entry_cost;
                    session_data.models_used.insert(entry.message.model.clone());

                    // Update last activity if needed
                    if need_timestamps
                        && (session_data.last_activity.is_none()
                            || session_data.last_activity.as_ref().unwrap() < &entry_date)
                    {
                        session_data.last_activity = Some(entry_date);
                    }

                    has_session_data = true;
                }

                if has_session_data {
                    session_count += 1;
                }
            }
        }

        if !options.json_output {
            let status_msg = format!(
                "📊 Processed {} entries, skipped {} duplicates",
                total_entries_processed, total_entries_skipped
            );
            println!("{}", status_msg);
        }

        // Convert to output format
        let mut result: Vec<SessionOutput> = sessions_by_dir
            .into_values()
            .filter(|session| session.total_cost > 0.0 || session.total_tokens() > 0)
            .map(|session| session.into())
            .collect();

        // Apply limit if specified (only for session command, not daily/monthly)
        if let Some(limit) = options.limit {
            if options.command == "session" {
                result.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
                result.truncate(limit);
            }
        }

        Ok(result)
    }

    async fn calculate_entry_cost(&self, entry: &UsageEntry) -> f64 {
        // First check if entry has pre-calculated cost from JSON
        if let Some(cost) = entry.cost_usd {
            tracing::debug!(
                message_id = %entry.message.id,
                request_id = %entry.request_id,
                cost_usd = cost,
                "Using pre-calculated cost from JSON"
            );
            return cost;
        }
        
        // Fall back to token-based calculation
        if let Some(usage) = &entry.message.usage {
            let calculated_cost = PricingManager::calculate_cost_from_tokens(usage, &entry.message.model).await;
            tracing::debug!(
                message_id = %entry.message.id,
                request_id = %entry.request_id,
                input_tokens = usage.input_tokens,
                output_tokens = usage.output_tokens,
                model = %entry.message.model,
                calculated_cost = calculated_cost,
                "Using token-based cost calculation"
            );
            calculated_cost
        } else {
            tracing::debug!(
                message_id = %entry.message.id,
                request_id = %entry.request_id,
                "No usage data or cost information"
            );
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub command: String,
    pub json_output: bool,
    pub limit: Option<usize>,
    pub since_date: Option<DateTime<Utc>>,
    pub until_date: Option<DateTime<Utc>>,
    pub snapshot: bool,
    pub exclude_vms: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_retain_optimization_works() {
        let dedup_engine = DeduplicationEngine::new();

        // Add some test entries with different timestamps
        let now = Utc::now();
        let old_time = now - chrono::Duration::hours(24);
        let very_old_time = now - chrono::Duration::hours(72);

        // Insert some test hashes with timestamps
        dedup_engine
            .hash_timestamps
            .insert("hash1".to_string(), now);
        dedup_engine
            .hash_timestamps
            .insert("hash2".to_string(), old_time);
        dedup_engine
            .hash_timestamps
            .insert("hash3".to_string(), very_old_time);

        dedup_engine.global_hashes.insert("hash1".to_string());
        dedup_engine.global_hashes.insert("hash2".to_string());
        dedup_engine.global_hashes.insert("hash3".to_string());

        // Simulate cleanup with cutoff time between old_time and very_old_time
        let cutoff_time = now - chrono::Duration::hours(48);

        // Use the retain method like in our optimization
        dedup_engine.hash_timestamps.retain(|key, timestamp| {
            if *timestamp < cutoff_time {
                dedup_engine.global_hashes.remove(key);
                false
            } else {
                true
            }
        });

        // Verify that only very_old_time entries were removed
        assert!(dedup_engine.hash_timestamps.contains_key("hash1"));
        assert!(dedup_engine.hash_timestamps.contains_key("hash2"));
        assert!(!dedup_engine.hash_timestamps.contains_key("hash3"));

        assert!(dedup_engine.global_hashes.contains("hash1"));
        assert!(dedup_engine.global_hashes.contains("hash2"));
        assert!(!dedup_engine.global_hashes.contains("hash3"));
    }
}
