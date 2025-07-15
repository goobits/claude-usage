use crate::models::*;
use crate::parser::FileParser;
use crate::pricing::PricingManager;
use dashmap::DashSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rayon::prelude::*;

pub struct DeduplicationEngine {
    cost_mode: CostMode,
    global_hashes: Arc<DashSet<String>>,
    hash_timestamps: Arc<dashmap::DashMap<String, DateTime<Utc>>>,
    dedup_window_hours: i64,
    dedup_cleanup_threshold: usize,
}

impl DeduplicationEngine {
    pub fn new(cost_mode: CostMode) -> Self {
        Self {
            cost_mode,
            global_hashes: Arc::new(DashSet::new()),
            hash_timestamps: Arc::new(dashmap::DashMap::new()),
            dedup_window_hours: 24,
            dedup_cleanup_threshold: 10000,
        }
    }

    pub async fn process_files_with_global_dedup(
        &self,
        sorted_file_tuples: Vec<(PathBuf, PathBuf)>,
        options: &ProcessOptions,
    ) -> Result<Vec<SessionOutput>> {
        let parser = FileParser::new(self.cost_mode.clone());
        let mut sessions_by_dir: HashMap<String, SessionData> = HashMap::new();
        
        let need_timestamps = matches!(options.command.as_str(), "daily" | "session" | "monthly");
        let mut total_entries_processed = 0;
        let mut total_entries_skipped = 0;
        let mut session_count = 0;
        
        // Early exit optimization for --last N queries
        let should_stop_early = options.last.is_some() && options.command == "session";
        
        // Process files in parallel chunks for better performance
        let chunk_size = 10; // Process 10 files at a time
        let mut _processed_files = 0;
        
        for chunk in sorted_file_tuples.chunks(chunk_size) {
            // Early exit optimization
            if should_stop_early && session_count >= options.last.unwrap_or(0) {
                break;
            }
            
            // Process chunk in parallel
            let chunk_results: Vec<_> = chunk.par_iter()
                .map(|(jsonl_file, session_dir)| {
                    let entries = parser.parse_jsonl_file(jsonl_file)?;
                    Ok::<_, anyhow::Error>((entries, session_dir.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            // Process results sequentially to maintain deduplication correctness
            for (entries, session_dir) in chunk_results {
                let mut has_session_data = false;
                _processed_files += 1;
            
            for entry in entries {
                // Check if entry has usage data
                if entry.message.usage.input_tokens == 0 && entry.message.usage.output_tokens == 0 {
                    continue;
                }
                
                total_entries_processed += 1;
                
                // Create unique hash for deduplication
                let unique_hash = parser.create_unique_hash(&entry);
                
                // Get current entry timestamp
                let current_timestamp = parser.parse_timestamp(&entry.timestamp).ok();
                
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
                    continue;
                }
                
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
                        let cutoff_time = current_time - chrono::Duration::hours(self.dedup_window_hours * 2);
                        let old_hashes: Vec<String> = self.hash_timestamps
                            .iter()
                            .filter_map(|item| {
                                if *item.value() < cutoff_time {
                                    Some(item.key().clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        
                        for old_hash in old_hashes {
                            self.hash_timestamps.remove(&old_hash);
                            self.global_hashes.remove(&old_hash);
                        }
                    }
                }
                
                // Calculate cost based on mode
                let entry_cost = self.calculate_entry_cost(&entry).await;
                
                // Extract session info
                let session_dir_name = session_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let (session_id, project_name) = parser.extract_session_info(session_dir_name);
                
                // Get or create session data
                let session_data = sessions_by_dir.entry(session_dir_name.to_string())
                    .or_insert_with(|| SessionData::new(session_id, project_name));
                
                // Aggregate usage data
                session_data.input_tokens += entry.message.usage.input_tokens;
                session_data.output_tokens += entry.message.usage.output_tokens;
                session_data.cache_creation_tokens += entry.message.usage.cache_creation_input_tokens;
                session_data.cache_read_tokens += entry.message.usage.cache_read_input_tokens;
                session_data.total_cost += entry_cost;
                session_data.models_used.insert(entry.message.model.clone());
                
                // Update last activity if needed
                if need_timestamps {
                    if let Ok(timestamp) = parser.parse_timestamp(&entry.timestamp) {
                        let activity_date = timestamp.format("%Y-%m-%d").to_string();
                        if session_data.last_activity.is_none() || 
                           session_data.last_activity.as_ref().unwrap() < &activity_date {
                            session_data.last_activity = Some(activity_date);
                        }
                    }
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
        
        // Apply last limit if specified
        if let Some(last) = options.last {
            result.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
            result.truncate(last);
        }
        
        Ok(result)
    }

    async fn calculate_entry_cost(&self, entry: &UsageEntry) -> f64 {
        match self.cost_mode {
            CostMode::Display => {
                entry.cost_usd.unwrap_or(0.0)
            }
            CostMode::Calculate => {
                PricingManager::calculate_cost_from_tokens(&entry.message.usage, &entry.message.model).await
            }
            CostMode::Auto => {
                if let Some(cost) = entry.cost_usd {
                    cost
                } else {
                    PricingManager::calculate_cost_from_tokens(&entry.message.usage, &entry.message.model).await
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub command: String,
    pub json_output: bool,
    pub last: Option<usize>,
    pub since_date: Option<DateTime<Utc>>,
    pub until_date: Option<DateTime<Utc>>,
    pub snapshot: bool,
}