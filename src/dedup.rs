use crate::models::{*, DailyUsage};
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
    global_hashes: Arc<DashSet<String>>,
    hash_timestamps: Arc<dashmap::DashMap<String, DateTime<Utc>>>,
    dedup_window_hours: i64,
    dedup_cleanup_threshold: usize,
}

impl DeduplicationEngine {
    pub fn new() -> Self {
        Self {
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
        let parser = FileParser::new();
        let mut sessions_by_dir: HashMap<PathBuf, SessionData> = HashMap::new();
        
        let need_timestamps = matches!(options.command.as_str(), "daily" | "session" | "monthly");
        let mut total_entries_processed = 0;
        let mut total_entries_skipped = 0;
        let mut session_count = 0;
        
        // Early exit optimization for --limit N queries (only for session command)
        let should_stop_early = options.limit.is_some() && options.command == "session";
        
        // Process files in parallel chunks for better performance
        let chunk_size = 10; // Process 10 files at a time
        let mut _processed_files = 0;
        
        for chunk in sorted_file_tuples.chunks(chunk_size) {
            // Early exit optimization
            if should_stop_early && session_count >= options.limit.unwrap_or(0) {
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
                // Check if entry has usage data (match Python behavior)
                let Some(usage) = &entry.message.usage else {
                    continue;  // Skip entries without usage data
                };
                if usage.input_tokens == 0 && usage.output_tokens == 0 {
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
                
                // Extract session info with more context
                let session_dir_name = session_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                
                // Extract meaningful project path, stripping only the home/.claude/ base
                let full_path = session_dir.to_string_lossy();
                let project_name = if let Some(claude_pos) = full_path.find("/.claude/") {
                    let after_claude = &full_path[claude_pos + 9..]; // "/.claude/" is 9 chars
                    
                    if after_claude.starts_with("projects/") {
                        // For main projects, check if it's a simple project name or has path structure
                        let project_part = &after_claude[9..]; // Skip "projects/"
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
                    } else if after_claude.starts_with("vms/") {
                        // For VMs, extract vm_name from "vms/vm_name/projects/-workspace" -> "vms/vm_name"
                        let vm_part = &after_claude[4..]; // Skip "vms/"
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
                
                let (session_id, _) = parser.extract_session_info(session_dir_name);
                
                // Get or create session data (use full path like Python)
                let session_data = sessions_by_dir.entry(session_dir.clone())
                    .or_insert_with(|| SessionData::new(session_id, project_name));
                
                // Get the date for this entry (use UTC for consistent bucketing)
                let entry_date = if let Ok(timestamp) = parser.parse_timestamp(&entry.timestamp) {
                    timestamp.format("%Y-%m-%d").to_string()
                } else {
                    "unknown".to_string()
                };
                
                // Update daily usage for this specific date
                let daily = session_data.daily_usage.entry(entry_date.clone()).or_insert_with(|| {
                    DailyUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_creation_tokens: 0,
                        cache_read_tokens: 0,
                        cost: 0.0,
                    }
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
                if need_timestamps {
                    if session_data.last_activity.is_none() || 
                       session_data.last_activity.as_ref().unwrap() < &entry_date {
                        session_data.last_activity = Some(entry_date);
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
        if let Some(usage) = &entry.message.usage {
            PricingManager::calculate_cost_from_tokens(usage, &entry.message.model).await
        } else {
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