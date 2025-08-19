use crate::config::get_config;
use crate::keeper_integration::KeeperIntegration;
use anyhow::Result;
use chrono::{DateTime, Utc};
use glob::glob;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Handles file system traversal and discovery of Claude usage data files
pub struct FileDiscovery {
    keeper_integration: KeeperIntegration,
}

impl Default for FileDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDiscovery {
    pub fn new() -> Self {
        Self {
            keeper_integration: KeeperIntegration::new(),
        }
    }

    /// Discover all Claude installation paths (main + VMs)
    pub fn discover_claude_paths(&self, exclude_vms: bool) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let config = get_config();

        // Get Claude home directory from config (respects CLAUDE_HOME env var)
        let claude_home = &config.paths.claude_home;
        
        // Main Claude path
        let main_path = claude_home.clone();
        if main_path.join("projects").exists() {
            paths.push(main_path.clone());
        }

        // VM paths (only if not excluded)
        if !exclude_vms {
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
        }

        Ok(paths)
    }

    /// Find all JSONL files in the given Claude paths
    pub fn find_jsonl_files(&self, claude_paths: &[PathBuf]) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut file_tuples = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for claude_path in claude_paths {
            let projects_dir = claude_path.join("projects");
            if !projects_dir.exists() {
                continue;
            }

            // Find session directories (format: -base64-encoded-path)
            // Files can be named either conversation_*.jsonl or *.jsonl (UUID format)
            let patterns = vec![
                projects_dir.join("*").join("conversation_*.jsonl"),
                projects_dir.join("*").join("*.jsonl"),
            ];

            for pattern in patterns {
                if let Ok(paths) = glob(&pattern.to_string_lossy()) {
                    for entry in paths.flatten() {
                        // Deduplicate files that match multiple patterns
                        if seen_files.insert(entry.clone()) {
                            if let Some(session_dir) = entry.parent() {
                                file_tuples.push((entry.clone(), session_dir.to_path_buf()));
                            }
                        }
                    }
                }
            }
        }

        Ok(file_tuples)
    }

    /// Check if a file should be included based on date filtering
    pub fn should_include_file(
        &self,
        file_path: &Path,
        since_date: Option<&DateTime<Utc>>,
        until_date: Option<&DateTime<Utc>>,
    ) -> bool {
        if since_date.is_none() && until_date.is_none() {
            return true;
        }

        // Check file lifespan overlap with search date range
        if let Ok(metadata) = metadata(file_path) {
            let mut file_start_time = None;
            let mut file_end_time = None;

            // Get creation time (birth time) as the start of file lifespan
            if let Ok(created) = metadata.created() {
                file_start_time = Some(DateTime::<Utc>::from(created));
            }

            // Get modification time as the end of file lifespan
            if let Ok(modified) = metadata.modified() {
                file_end_time = Some(DateTime::<Utc>::from(modified));
            }

            // If we don't have creation time, use modification time as both start and end
            if file_start_time.is_none() && file_end_time.is_some() {
                file_start_time = file_end_time;
            }

            // Check for overlap between file lifespan and search range
            if let (Some(file_start), Some(file_end)) = (file_start_time, file_end_time) {
                // File lifespan: [file_start, file_end]
                // Search range: [since_date, until_date]

                // For overlap to occur:
                // 1. File must have been created before or during the search range ends
                // 2. File must have been modified after or during the search range starts

                if let Some(until) = until_date {
                    let until_plus_day = *until + chrono::Duration::days(1);
                    // File was created after the search range ended
                    if file_start > until_plus_day {
                        return false;
                    }
                }

                if let Some(since) = since_date {
                    // File was last modified before the search range started
                    if file_end < *since {
                        return false;
                    }
                }

                // If we reach here, the file lifespan overlaps with the search range
                // No need to check file content
                return true;
            }
        }

        // Fallback: Parse file content for date range if metadata is unavailable
        if let Ok((earliest, latest)) = self.get_file_date_range(file_path) {
            if let (Some(earliest), Some(latest)) = (earliest, latest) {
                // Check overlap using content timestamps
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

    /// Get the earliest and latest timestamps from a file's content
    fn get_file_date_range(
        &self,
        file_path: &Path,
    ) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut earliest_timestamp: Option<DateTime<Utc>> = None;
        let mut latest_timestamp: Option<DateTime<Utc>> = None;

        // Read first and last non-empty lines efficiently
        let mut first_line = None;
        let mut last_line = None;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if first_line.is_none() {
                first_line = Some(line.to_string());
            }
            last_line = Some(line.to_string());
        }

        // Parse timestamps from first and last entries
        if let Some(line) = first_line {
            if let Some(entry) = self.keeper_integration.parse_single_line(&line) {
                if let Ok(timestamp) =
                    crate::timestamp_parser::TimestampParser::parse(&entry.timestamp)
                {
                    earliest_timestamp = Some(timestamp);
                }
            }
        }

        if let Some(line) = last_line {
            if let Some(entry) = self.keeper_integration.parse_single_line(&line) {
                if let Ok(timestamp) =
                    crate::timestamp_parser::TimestampParser::parse(&entry.timestamp)
                {
                    latest_timestamp = Some(timestamp);
                }
            }
        }

        Ok((earliest_timestamp, latest_timestamp))
    }

    /// Get the earliest timestamp from a file
    pub fn get_earliest_timestamp(&self, file_path: &Path) -> Result<Option<DateTime<Utc>>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(entry) = self.keeper_integration.parse_single_line(line) {
                if let Ok(timestamp) =
                    crate::timestamp_parser::TimestampParser::parse(&entry.timestamp)
                {
                    return Ok(Some(timestamp));
                }
            }
        }

        Ok(None)
    }

    /// Sort files by timestamp (modification time + content timestamp for smaller datasets)
    pub fn sort_files_by_timestamp(
        &self,
        mut file_tuples: Vec<(PathBuf, PathBuf)>,
    ) -> Vec<(PathBuf, PathBuf)> {
        // For large datasets, use file modification time as primary sort
        // Only parse content timestamp for smaller datasets
        let use_content_timestamp = file_tuples.len() < 100;

        file_tuples.sort_by(|a, b| {
            // Primary sort: file modification time
            let a_mtime = metadata(&a.0)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            let b_mtime = metadata(&b.0)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);

            let primary_cmp = a_mtime.cmp(&b_mtime);

            if use_content_timestamp && primary_cmp == std::cmp::Ordering::Equal {
                // Secondary sort: content timestamp
                let a_timestamp = self.get_earliest_timestamp(&a.0).unwrap_or(None);
                let b_timestamp = self.get_earliest_timestamp(&b.0).unwrap_or(None);

                match (a_timestamp, b_timestamp) {
                    (Some(a_ts), Some(b_ts)) => a_ts.cmp(&b_ts),
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

    /// Find session blocks files
    #[allow(dead_code)]
    pub fn find_session_blocks_files(&self, claude_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut block_files = Vec::new();

        for claude_path in claude_paths {
            let usage_dir = claude_path.join("usage_tracking");
            if !usage_dir.exists() {
                continue;
            }

            // Find session block files
            let pattern = usage_dir.join("session_blocks_*.json");
            if let Ok(paths) = glob(&pattern.to_string_lossy()) {
                for entry in paths.flatten() {
                    block_files.push(entry);
                }
            }
        }

        // Sort by modification time (newest first)
        block_files.sort_by(|a, b| {
            let a_mtime = metadata(a)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            let b_mtime = metadata(b)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            b_mtime.cmp(&a_mtime) // Reverse order (newest first)
        });

        Ok(block_files)
    }
}
