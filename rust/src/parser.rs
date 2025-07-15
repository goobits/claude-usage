use crate::models::*;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc, NaiveDateTime, Local};
use std::collections::HashMap;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
// use rayon::prelude::*; // Commented out as not currently used
use glob::glob;

pub struct FileParser {
    cost_mode: CostMode,
}

impl FileParser {
    pub fn new(cost_mode: CostMode) -> Self {
        Self { cost_mode }
    }

    pub fn discover_claude_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        
        // Get home directory
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        
        // Main Claude path
        let main_path = home_dir.join(".claude");
        if main_path.join("projects").exists() {
            paths.push(main_path.clone());
        }
        
        // VM paths
        let vms_dir = main_path.join("vms");
        if vms_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&vms_dir) {
                for entry in entries.flatten() {
                    let vm_path = entry.path();
                    if vm_path.is_dir() && vm_path.join("projects").exists() {
                        paths.push(vm_path);
                    }
                }
            }
        }
        
        Ok(paths)
    }

    pub fn find_jsonl_files(&self, claude_paths: &[PathBuf]) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut file_tuples = Vec::new();
        
        for claude_path in claude_paths {
            let projects_dir = claude_path.join("projects");
            if !projects_dir.exists() {
                continue;
            }
            
            if let Ok(entries) = std::fs::read_dir(&projects_dir) {
                for entry in entries.flatten() {
                    let session_dir = entry.path();
                    if !session_dir.is_dir() {
                        continue;
                    }
                    
                    let pattern = session_dir.join("*.jsonl");
                    if let Ok(jsonl_files) = glob(&pattern.to_string_lossy()) {
                        for jsonl_file in jsonl_files.flatten() {
                            file_tuples.push((jsonl_file, session_dir.clone()));
                        }
                    }
                }
            }
        }
        
        Ok(file_tuples)
    }

    pub fn should_include_file(&self, file_path: &Path, since_date: Option<&DateTime<Utc>>, until_date: Option<&DateTime<Utc>>) -> bool {
        if since_date.is_none() && until_date.is_none() {
            return true;
        }
        
        // Use file modification time as fast pre-filter
        if let Ok(metadata) = metadata(file_path) {
            if let Ok(modified) = metadata.modified() {
                let file_time = DateTime::<Utc>::from(modified);
                
                // Quick exclusion checks
                if let Some(since) = since_date {
                    if file_time < *since {
                        return false;
                    }
                }
                
                if let Some(until) = until_date {
                    // Add one day buffer for until date
                    let until_plus_day = *until + chrono::Duration::days(1);
                    if file_time > until_plus_day {
                        return false;
                    }
                }
            }
        }
        
        // Parse file content for date range if needed
        if let Ok((earliest, latest)) = self.get_file_date_range(file_path) {
            if let (Some(earliest), Some(latest)) = (earliest, latest) {
                if let Some(since) = since_date {
                    if latest.date_naive() < since.date_naive() {
                        return false;
                    }
                }
                if let Some(until) = until_date {
                    if earliest.date_naive() > until.date_naive() {
                        return false;
                    }
                }
            }
        }
        
        true
    }

    fn get_file_date_range(&self, file_path: &Path) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        let mut earliest_date: Option<DateTime<Utc>> = None;
        let mut latest_date: Option<DateTime<Utc>> = None;
        
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(timestamp_str) = data.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(timestamp) = self.parse_timestamp(timestamp_str) {
                        if earliest_date.is_none() || timestamp < earliest_date.unwrap() {
                            earliest_date = Some(timestamp);
                        }
                        if latest_date.is_none() || timestamp > latest_date.unwrap() {
                            latest_date = Some(timestamp);
                        }
                    }
                }
            }
        }
        
        Ok((earliest_date, latest_date))
    }

    pub fn get_earliest_timestamp(&self, file_path: &Path) -> Result<Option<DateTime<Utc>>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(timestamp_str) = data.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(timestamp) = self.parse_timestamp(timestamp_str) {
                        return Ok(Some(timestamp));
                    }
                }
            }
        }
        
        Ok(None)
    }

    pub fn sort_files_by_timestamp(&self, mut file_tuples: Vec<(PathBuf, PathBuf)>) -> Vec<(PathBuf, PathBuf)> {
        // For large datasets, use file modification time as primary sort
        // Only parse content timestamp for smaller datasets
        let use_content_timestamp = file_tuples.len() < 100;
        
        file_tuples.sort_by(|a, b| {
            // Primary sort: file modification time
            let a_mtime = metadata(&a.0).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            let b_mtime = metadata(&b.0).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            
            let primary_cmp = a_mtime.cmp(&b_mtime);
            
            if use_content_timestamp && primary_cmp == std::cmp::Ordering::Equal {
                // Secondary sort: content timestamp
                let a_timestamp = self.get_earliest_timestamp(&a.0).unwrap_or(None);
                let b_timestamp = self.get_earliest_timestamp(&b.0).unwrap_or(None);
                
                match (a_timestamp, b_timestamp) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            } else {
                primary_cmp
            }
        });
        
        file_tuples
    }

    pub fn parse_jsonl_file(&self, file_path: &Path) -> Result<Vec<UsageEntry>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        let mut entries = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Ok(entry) = serde_json::from_str::<UsageEntry>(&line) {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    pub fn parse_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Handle both Z suffix and timezone info
        let timestamp = if timestamp_str.ends_with('Z') {
            timestamp_str.replace('Z', "+00:00")
        } else {
            timestamp_str.to_string()
        };
        
        // Try parsing as ISO 8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(&timestamp) {
            return Ok(dt.with_timezone(&Utc));
        }
        
        // Try parsing as naive datetime and assume UTC
        if let Ok(naive) = NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
        
        anyhow::bail!("Failed to parse timestamp: {}", timestamp_str)
    }

    pub fn extract_session_info(&self, session_dir_name: &str) -> (String, String) {
        let session_id = session_dir_name.to_string();
        
        let project_name = if session_dir_name.starts_with('-') {
            // Remove leading - and split
            let parts: Vec<&str> = session_dir_name[1..].split('-').collect();
            parts.last().unwrap_or(&"unknown").to_string()
        } else {
            session_dir_name.split('/').last().unwrap_or("unknown").to_string()
        };
        
        (session_id, project_name)
    }

    pub fn create_unique_hash(&self, entry: &UsageEntry) -> Option<String> {
        // Match Python behavior: return None if either ID is empty
        if entry.message.id.is_empty() || entry.request_id.is_empty() {
            return None;
        }
        Some(format!("{}:{}", entry.message.id, entry.request_id))
    }

    pub fn load_session_blocks(&self) -> Result<Vec<SessionBlock>> {
        self.load_session_blocks_with_filter(true)
    }
    
    pub fn load_session_blocks_with_filter(&self, filter_recent: bool) -> Result<Vec<SessionBlock>> {
        let paths = self.discover_claude_paths()?;
        let mut blocks = Vec::new();
        let cutoff_time = if filter_recent {
            Some(Utc::now() - chrono::Duration::hours(24))
        } else {
            None
        };
        
        for claude_path in paths {
            let possible_dirs = vec![
                claude_path.join("usage_tracking"),
                claude_path.clone(), // Sometimes stored in root
                claude_path.join("sessions"), // Alternative location
            ];
            
            for usage_dir in possible_dirs {
                if !usage_dir.exists() {
                    continue;
                }
                
                // Find session blocks files
                let patterns = vec![
                    usage_dir.join("session_blocks_*.json"),
                    usage_dir.join("*session_blocks*.json"),
                ];
                
                for pattern in patterns {
                    if let Ok(files) = glob(&pattern.to_string_lossy()) {
                        for file_path in files.flatten() {
                            // Filter by file modification time if requested
                            if let Some(cutoff) = cutoff_time {
                                if let Ok(metadata) = metadata(&file_path) {
                                    if let Ok(modified) = metadata.modified() {
                                        let file_time = DateTime::<Utc>::from(modified);
                                        if file_time < cutoff {
                                            continue;
                                        }
                                    }
                                }
                            }
                            
                            if let Ok(file_blocks) = self.parse_session_blocks_file(&file_path) {
                                // Filter blocks to only recent ones if requested
                                for block in file_blocks {
                                    if let Some(cutoff) = cutoff_time {
                                        if let Ok(start_time) = self.parse_timestamp(&block.start_time) {
                                            if start_time > cutoff {
                                                blocks.push(block);
                                            }
                                        }
                                    } else {
                                        blocks.push(block);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(blocks)
    }

    fn parse_session_blocks_file(&self, file_path: &Path) -> Result<Vec<SessionBlock>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        let data: serde_json::Value = serde_json::from_reader(reader)?;
        
        let blocks = if data.is_array() {
            serde_json::from_value::<Vec<SessionBlock>>(data)?
        } else if let Some(blocks) = data.get("blocks") {
            serde_json::from_value::<Vec<SessionBlock>>(blocks.clone())?
        } else if let Some(sessions) = data.get("sessions") {
            serde_json::from_value::<Vec<SessionBlock>>(sessions.clone())?
        } else {
            Vec::new()
        };
        
        Ok(blocks)
    }
}