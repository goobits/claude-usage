//! Parquet summary reader
//!
//! This module provides efficient reading of claude-keeper parquet backup files
//! to extract summary information without loading full datasets into memory.

use anyhow::{Context, Result};
use chrono;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::{debug, info, warn};


use crate::live::BaselineSummary;

/// Read a parquet file using claude-keeper library 
fn read_parquet_with_library(parquet_file: &PathBuf) -> Result<String> {
    debug!("Attempting to read parquet file: {}", parquet_file.display());
    
    // Use claude-keeper library to read and convert parquet to JSONL
    // Note: The cfg check is not needed since claude-keeper is a direct dependency
    use claude_keeper::parquet_reader::{ConversationParquetReader, QueryFilter};
    match ConversationParquetReader::new(parquet_file) {
            Ok(reader) => {
                let filter = QueryFilter::new(); // No filters - get all data
                match reader.query(&filter) {
                    Ok(results) => {
                        debug!("Query returned {} objects", results.objects.len());
                        // Convert FlexObjects to JSONL format
                        let mut jsonl_lines = Vec::new();
                        for (i, flex_obj) in results.objects.iter().enumerate() {
                            let json_val = flex_obj.to_json();
                            
                            // Debug: print first object structure
                            if i == 0 {
                                debug!("First FlexObject as JSON (truncated): {}", 
                                    serde_json::to_string(&json_val)
                                        .unwrap_or_default()
                                        .chars()
                                        .take(500)
                                        .collect::<String>());
                                if let serde_json::Value::Object(ref map) = json_val {
                                    debug!("Top-level fields: {:?}", map.keys().collect::<Vec<_>>());
                                    
                                    // Check if there's a metadata or message field
                                    if let Some(metadata) = map.get("metadata") {
                                        debug!("Found metadata field: {}", 
                                            serde_json::to_string(metadata)
                                                .unwrap_or_default()
                                                .chars()
                                                .take(200)
                                                .collect::<String>());
                                    }
                                    if let Some(message) = map.get("message") {
                                        debug!("Found message field: {}", 
                                            serde_json::to_string(message)
                                                .unwrap_or_default()
                                                .chars()
                                                .take(200)
                                                .collect::<String>());
                                    }
                                }
                            }
                            
                            if let Ok(json_str) = serde_json::to_string(&json_val) {
                                jsonl_lines.push(json_str);
                            }
                        }
                        debug!("Converted {} objects to JSONL", jsonl_lines.len());
                        Ok(jsonl_lines.join("\n"))
                    }
                    Err(e) => {
                        warn!(
                            file = %parquet_file.display(),
                            error = %e,
                            "Failed to query parquet file with claude-keeper library"
                        );
                        Ok(String::new()) // Return empty instead of failing
                    }
                }
            }
            Err(e) => {
                warn!(
                    file = %parquet_file.display(),
                    error = %e,
                    "Failed to create parquet reader for file"
                );
                Ok(String::new()) // Return empty instead of failing
            }
        }
}

/// Reads summary information from parquet backup files
pub struct ParquetSummaryReader {
    backup_dir: PathBuf,
}

impl ParquetSummaryReader {
    /// Create a new parquet summary reader
    pub fn new(backup_dir: PathBuf) -> Result<Self> {
        if !backup_dir.exists() {
            return Err(anyhow::anyhow!(
                "Backup directory does not exist: {}",
                backup_dir.display()
            ));
        }

        Ok(Self { backup_dir })
    }

    /// Read summary data from parquet files
    pub fn read_summary(&self) -> Result<BaselineSummary> {
        info!(
            backup_dir = %self.backup_dir.display(),
            "Reading parquet backup summary"
        );

        // Find parquet files in the backup directory
        let parquet_files = self.find_parquet_files()?;
        
        if parquet_files.is_empty() {
            warn!(
                backup_dir = %self.backup_dir.display(),
                "No parquet files found in backup directory"
            );
            return Ok(BaselineSummary::default());
        }

        let total_files = parquet_files.len();
        debug!(file_count = total_files, "Found parquet backup files");

        // Get the most recent file modification time as last backup time
        let last_backup = parquet_files
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .filter_map(|metadata| metadata.modified().ok())
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Initialize aggregation variables
        let mut total_cost = 0.0;
        let mut total_tokens = 0u64;
        let mut sessions_today = 0u32;

        // Get today's date for session counting
        let today = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() / 86400; // Days since epoch

        // Process each parquet file
        for parquet_file in &parquet_files {
            debug!(file = %parquet_file.display(), "Processing parquet file");
            
            let stats_result = futures::executor::block_on(
                self.read_parquet_file_stats_async(parquet_file));
            
            match stats_result {
                Ok(stats) => {
                    total_cost += stats.total_cost;
                    total_tokens += stats.total_tokens;
                    
                    // Count sessions from today
                    for session_time in stats.session_times {
                        let session_day = session_time
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs() / 86400;
                        
                        if session_day == today {
                            sessions_today += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        file = %parquet_file.display(),
                        error = %e,
                        "Failed to read parquet file stats, skipping"
                    );
                }
            }
        }

        let summary = BaselineSummary {
            total_cost,
            total_tokens,
            sessions_today,
            last_backup,
        };

        info!(
            file_count = total_files,
            last_backup = ?last_backup,
            "Loaded baseline summary from parquet files"
        );

        Ok(summary)
    }

    /// Read statistics from a single parquet file using QueryEngine
    async fn read_parquet_file_stats_async(&self, parquet_file: &PathBuf) -> Result<ParquetFileStats> {
        debug!(
            file = %parquet_file.display(),
            "Querying parquet file using QueryEngine - TEMPORARILY DISABLED to avoid infinite loop"
        );

        // TEMPORARY FIX: Skip QueryEngine to avoid infinite loop during testing
        // TODO: Fix the QueryEngine infinite loop issue in claude-keeper integration
        warn!(
            file = %parquet_file.display(),
            "QueryEngine temporarily disabled - using placeholder values"
        );
        
        Ok(ParquetFileStats {
            total_cost: 0.0,
            total_tokens: 0,
            session_times: Vec::new(),
        })
    }

    /// Find all parquet files in the backup directory (recursively)
    fn find_parquet_files(&self) -> Result<Vec<PathBuf>> {
        let mut parquet_files = Vec::new();
        self.find_parquet_files_recursive(&self.backup_dir, &mut parquet_files)?;
        
        // Sort files by name for consistent ordering
        parquet_files.sort();
        
        Ok(parquet_files)
    }

    /// Recursively find parquet files in a directory
    fn find_parquet_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively search subdirectories
                self.find_parquet_files_recursive(&path, files)?;
            } else if path.is_file() && 
               path.extension()
                   .and_then(|ext| ext.to_str())
                   .map(|ext| ext.eq_ignore_ascii_case("parquet"))
                   .unwrap_or(false)
            {
                files.push(path);
            }
        }
        Ok(())
    }

    /// Get statistics about the backup files
    #[allow(dead_code)]
    pub fn get_backup_stats(&self) -> Result<BackupStats> {
        let parquet_files = self.find_parquet_files()?;
        
        let mut total_size = 0;
        let mut latest_modified = SystemTime::UNIX_EPOCH;
        
        for file in &parquet_files {
            if let Ok(metadata) = fs::metadata(file) {
                total_size += metadata.len();
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_modified {
                        latest_modified = modified;
                    }
                }
            }
        }

        Ok(BackupStats {
            file_count: parquet_files.len(),
            total_size_bytes: total_size,
            latest_modified,
        })
    }

    /// Read detailed session data for daily/monthly analysis
    pub fn read_detailed_sessions(&self) -> Result<Vec<crate::models::SessionOutput>> {
        use crate::models::{SessionData, SessionOutput, DailyUsage};
        use crate::timestamp_parser::TimestampParser;
        use std::collections::{HashMap, HashSet};
        
        info!(
            backup_dir = %self.backup_dir.display(),
            "Reading detailed session data from parquet backups"
        );

        let parquet_files = self.find_parquet_files()?;
        
        if parquet_files.is_empty() {
            warn!("No parquet files found in backup directory");
            return Ok(Vec::new());
        }

        let total_files = parquet_files.len();
        debug!(file_count = total_files, "Processing parquet files for detailed sessions");

        // Map to aggregate sessions across all files
        let mut sessions_map: HashMap<String, SessionData> = HashMap::new();
        
        // Set for deduplication using messageId:requestId (like ccusage)
        let mut seen_messages: HashSet<String> = HashSet::new();
        
        // Debug counters
        let mut total_messages_seen = 0;
        let mut deduplicated_count = 0;
        let mut no_dedup_key_count = 0;
        let mut messages_with_usage = 0;

        // Process each parquet file
        for parquet_file in &parquet_files {
            debug!(file = %parquet_file.display(), "Reading messages from parquet file");
            
            // Use claude-keeper library directly to read parquet data
            let parquet_data = match read_parquet_with_library(parquet_file) {
                Ok(data) => {
                    debug!(file = %parquet_file.display(), "Successfully read parquet data, length: {}", data.len());
                    data
                },
                Err(e) => {
                    warn!(
                        file = %parquet_file.display(),
                        error = %e,
                        "Failed to read parquet file with library, skipping"
                    );
                    continue;
                }
            };
            
            if parquet_data.is_empty() {
                debug!(file = %parquet_file.display(), "Parquet file returned empty data, skipping");
                continue;
            }
            
            let stdout = parquet_data;
            
            // Parse the JSON output - supports both JSON array and JSONL formats
            let messages: Vec<Value> = match serde_json::from_str(&stdout) {
                Ok(Value::Array(arr)) => arr,
                Ok(Value::Object(obj)) if obj.contains_key("messages") => {
                    obj.get("messages")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                }
                _ => {
                    // Try parsing as JSONL (newline-delimited JSON)
                    let mut jsonl_messages = Vec::new();
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            match serde_json::from_str::<Value>(trimmed) {
                                Ok(msg) => jsonl_messages.push(msg),
                                Err(_) => continue, // Skip invalid JSON lines
                            }
                        }
                    }
                    
                    if jsonl_messages.is_empty() {
                        warn!(
                            file = %parquet_file.display(),
                            "No valid JSON found in parquet file - file may be empty or contain no conversation data"
                        );
                        continue;
                    }
                    
                    jsonl_messages
                }
            };

            // Process each message
            for msg in messages {
                total_messages_seen += 1;
                
                // Extract message ID and request ID for deduplication
                let message_id = msg.get("message")
                    .and_then(|m| m.get("id"))
                    .or_else(|| msg.get("messageId"))
                    .and_then(|v| v.as_str());
                
                let request_id = msg.get("requestId")
                    .and_then(|v| v.as_str());
                
                // Create deduplication key like ccusage: messageId:requestId
                if let (Some(mid), Some(rid)) = (message_id, request_id) {
                    let dedup_key = format!("{}:{}", mid, rid);
                    if seen_messages.contains(&dedup_key) {
                        // Skip duplicate message
                        deduplicated_count += 1;
                        continue;
                    }
                    seen_messages.insert(dedup_key);
                } else {
                    no_dedup_key_count += 1;
                }
                
                // Extract key fields
                let session_id = msg.get("session_id")
                    .or_else(|| msg.get("sessionId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let project_name = msg.get("project_name")
                    .or_else(|| msg.get("projectName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let timestamp_str = msg.get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Get usage data - check message field first (where it actually is)
                let usage = msg.get("message")
                    .and_then(|m| m.get("usage"))
                    .or_else(|| msg.get("usage"));
                
                // Skip if no usage data (like ccusage does)
                if usage.is_none() {
                    continue;
                }

                let input_tokens = usage
                    .and_then(|u| u.get("input_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let output_tokens = usage
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                
                // Validate like ccusage - require both input and output tokens
                if input_tokens == 0 && output_tokens == 0 {
                    continue;
                }
                
                messages_with_usage += 1;

                let cache_creation_tokens = usage
                    .and_then(|u| u.get("cache_creation_input_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let cache_read_tokens = usage
                    .and_then(|u| u.get("cache_read_input_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let model = msg.get("message")
                    .and_then(|m| m.get("model"))
                    .or_else(|| msg.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("claude-3-sonnet");

                // Calculate cost - prefer costUSD field but fallback to LiteLLM pricing
                let cost = if let Some(cost_val) = msg.get("costUSD")
                    .or_else(|| msg.get("cost_usd")) {
                    cost_val.as_f64().unwrap_or(0.0)
                } else {
                    // Use hardcoded pricing as fallback since LiteLLM pricing is async
                    // In the future, we could pre-fetch pricing data to avoid this
                    crate::pricing::calculate_cost_simple(
                        model,
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens
                    )
                };

                // Parse date for daily aggregation
                let date_str = if let Ok(ts) = TimestampParser::parse(timestamp_str) {
                    ts.format("%Y-%m-%d").to_string()
                } else {
                    chrono::Utc::now().format("%Y-%m-%d").to_string()
                };

                // Get or create session
                let session = sessions_map.entry(session_id.clone())
                    .or_insert_with(|| SessionData::new(session_id.clone(), project_name.clone()));

                // Update session totals
                session.input_tokens += input_tokens;
                session.output_tokens += output_tokens;
                session.cache_creation_tokens += cache_creation_tokens;
                session.cache_read_tokens += cache_read_tokens;
                session.total_cost += cost;
                session.last_activity = Some(timestamp_str.to_string());
                session.models_used.insert(model.to_string());

                // Update daily usage
                let daily = session.daily_usage.entry(date_str.clone())
                    .or_insert_with(|| DailyUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_creation_tokens: 0,
                        cache_read_tokens: 0,
                        cost: 0.0,
                    });
                
                daily.input_tokens += input_tokens;
                daily.output_tokens += output_tokens;
                daily.cache_creation_tokens += cache_creation_tokens;
                daily.cache_read_tokens += cache_read_tokens;
                daily.cost += cost;
            }
        }

        // Convert to SessionOutput format
        let mut sessions: Vec<SessionOutput> = sessions_map
            .into_iter()
            .map(|(_, session_data)| SessionOutput {
                session_id: session_data.session_id,
                project_path: session_data.project_path,
                input_tokens: session_data.input_tokens,
                output_tokens: session_data.output_tokens,
                cache_creation_tokens: session_data.cache_creation_tokens,
                cache_read_tokens: session_data.cache_read_tokens,
                total_cost: session_data.total_cost,
                last_activity: session_data.last_activity.unwrap_or_else(|| "".to_string()),
                models_used: session_data.models_used.into_iter().collect(),
                daily_usage: session_data.daily_usage,
            })
            .collect();

        // Sort by last activity (most recent first)
        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        info!(
            session_count = sessions.len(),
            total_messages = total_messages_seen,
            deduplicated = deduplicated_count,
            no_dedup_key = no_dedup_key_count,
            with_usage = messages_with_usage,
            "Loaded detailed session data from parquet files"
        );

        Ok(sessions)
    }
}

/// Statistics about backup files
#[allow(dead_code)]
pub struct BackupStats {
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub latest_modified: SystemTime,
}

/// Statistics extracted from a single parquet file
#[derive(Default)]
struct ParquetFileStats {
    total_cost: f64,
    total_tokens: u64,
    session_times: Vec<SystemTime>,
}