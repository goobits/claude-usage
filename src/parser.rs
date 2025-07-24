use crate::models::*;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc, NaiveDateTime};
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
// use rayon::prelude::*; // Commented out as not currently used
use glob::glob;

pub struct FileParser {}

// Trait for custom JSONL processing
pub trait JsonlProcessor {
    type Output;
    
    fn process_entry(&mut self, entry: UsageEntry, line_number: usize) -> Result<()>;
    fn finalize(self) -> Result<Self::Output>;
}

// ProcessedEntry represents a parsed JSONL entry with extracted metadata
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProcessedEntry {
    pub entry: UsageEntry,
    pub timestamp: DateTime<Utc>,
    pub date: String, // YYYY-MM-DD format
    pub line_number: usize,
    pub total_tokens: u32,
}

#[allow(dead_code)]
impl ProcessedEntry {
    pub fn new(entry: UsageEntry, parser: &FileParser, line_number: usize) -> Result<Self> {
        let timestamp = parser.parse_timestamp(&entry.timestamp)?;
        let date = timestamp.format("%Y-%m-%d").to_string();
        let total_tokens = Self::calculate_total_tokens(&entry);
        
        Ok(Self {
            entry,
            timestamp,
            date,
            line_number,
            total_tokens,
        })
    }
    
    fn calculate_total_tokens(entry: &UsageEntry) -> u32 {
        if let Some(usage) = &entry.message.usage {
            usage.input_tokens + 
            usage.output_tokens + 
            usage.cache_creation_input_tokens + 
            usage.cache_read_input_tokens
        } else {
            0
        }
    }
    
    pub fn input_tokens(&self) -> u32 {
        self.entry.message.usage.as_ref()
            .map(|u| u.input_tokens)
            .unwrap_or(0)
    }
    
    pub fn output_tokens(&self) -> u32 {
        self.entry.message.usage.as_ref()
            .map(|u| u.output_tokens)
            .unwrap_or(0)
    }
    
    pub fn cache_tokens(&self) -> u32 {
        self.entry.message.usage.as_ref()
            .map(|u| u.cache_creation_input_tokens + u.cache_read_input_tokens)
            .unwrap_or(0)
    }
    
    pub fn has_usage(&self) -> bool {
        self.entry.message.usage.is_some()
    }
}


impl FileParser {
    pub fn new() -> Self {
        Self {}
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
            
            // Find session directories (format: -base64-encoded-path)
            // Files can be named either conversation_*.jsonl or *.jsonl (UUID format)
            let patterns = vec![
                projects_dir.join("*").join("conversation_*.jsonl"),
                projects_dir.join("*").join("*.jsonl"),
            ];
            
            for pattern in patterns {
                if let Ok(paths) = glob(&pattern.to_string_lossy()) {
                    for entry in paths.flatten() {
                        if let Some(session_dir) = entry.parent() {
                            file_tuples.push((entry.clone(), session_dir.to_path_buf()));
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

    fn get_file_date_range(&self, file_path: &Path) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
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
            if let Ok(entry) = serde_json::from_str::<UsageEntry>(&line) {
                if let Ok(timestamp) = self.parse_timestamp(&entry.timestamp) {
                    earliest_timestamp = Some(timestamp);
                }
            }
        }
        
        if let Some(line) = last_line {
            if let Ok(entry) = serde_json::from_str::<UsageEntry>(&line) {
                if let Ok(timestamp) = self.parse_timestamp(&entry.timestamp) {
                    latest_timestamp = Some(timestamp);
                }
            }
        }
        
        Ok((earliest_timestamp, latest_timestamp))
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
            
            if let Ok(entry) = serde_json::from_str::<UsageEntry>(&line) {
                if let Ok(timestamp) = self.parse_timestamp(&entry.timestamp) {
                    return Ok(Some(timestamp));
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

    pub fn parse_jsonl_file(&self, file_path: &Path) -> Result<Vec<UsageEntry>> {
        // Use the default collector processor
        let processor = CollectorProcessor::new();
        self.process_jsonl_file(file_path, processor)
    }
    
    // Generic method that accepts any processor
    pub fn process_jsonl_file<P: JsonlProcessor>(&self, file_path: &Path, mut processor: P) -> Result<P::Output> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut line_number = 0;
        
        for line in reader.lines() {
            line_number += 1;
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Ok(entry) = serde_json::from_str::<UsageEntry>(&line) {
                processor.process_entry(entry, line_number)?;
            }
        }
        
        processor.finalize()
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
            // Remove only the leading dash, keep the full path
            session_dir_name[1..].to_string()
        } else {
            session_dir_name.to_string()
        };
        
        (session_id, project_name)
    }

    pub fn create_unique_hash(&self, entry: &UsageEntry) -> Option<String> {
        let message_id = &entry.message.id;
        let request_id = &entry.request_id;
        
        if message_id.is_empty() || request_id.is_empty() {
            return None;
        }
        
        Some(format!("{}:{}", message_id, request_id))
    }

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
            let a_mtime = metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            let b_mtime = metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            b_mtime.cmp(&a_mtime) // Reverse order (newest first)
        });
        
        Ok(block_files)
    }

    pub fn get_latest_session_blocks(&self, claude_paths: &[PathBuf]) -> Result<Vec<SessionBlock>> {
        let block_files = self.find_session_blocks_files(claude_paths)?;
        
        if let Some(latest_file) = block_files.first() {
            self.parse_session_blocks_file(latest_file)
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_session_blocks_file(&self, file_path: &Path) -> Result<Vec<SessionBlock>> {
        let content = std::fs::read_to_string(file_path)?;
        let data: serde_json::Value = serde_json::from_str(&content)?;
        
        // Handle both direct array format and wrapped format
        let blocks = if data.is_array() {
            serde_json::from_value::<Vec<SessionBlock>>(data)?
        } else if let Some(blocks) = data.get("blocks") {
            serde_json::from_value::<Vec<SessionBlock>>(blocks.clone())?
        } else if let Some(sessions) = data.get("sessions") {
            serde_json::from_value::<Vec<SessionBlock>>(sessions.clone())?
        } else {
            // Try to parse the whole object as a single block
            vec![serde_json::from_value::<SessionBlock>(data)?]
        };
        
        Ok(blocks)
    }
}

// Default processor that collects all entries into a Vec
pub struct CollectorProcessor {
    entries: Vec<UsageEntry>,
}

impl CollectorProcessor {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
}

impl JsonlProcessor for CollectorProcessor {
    type Output = Vec<UsageEntry>;
    
    fn process_entry(&mut self, entry: UsageEntry, _line_number: usize) -> Result<()> {
        self.entries.push(entry);
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(self.entries)
    }
}

// Processor that counts entries
#[allow(dead_code)]
pub struct CountProcessor {
    count: usize,
}

#[allow(dead_code)]
impl CountProcessor {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[allow(dead_code)]
impl JsonlProcessor for CountProcessor {
    type Output = usize;
    
    fn process_entry(&mut self, _entry: UsageEntry, _line_number: usize) -> Result<()> {
        self.count += 1;
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(self.count)
    }
}

// Processor that filters entries based on a predicate
#[allow(dead_code)]
pub struct FilterProcessor<F>
where
    F: Fn(&UsageEntry) -> bool,
{
    predicate: F,
    entries: Vec<UsageEntry>,
}

#[allow(dead_code)]
impl<F> FilterProcessor<F>
where
    F: Fn(&UsageEntry) -> bool,
{
    pub fn new(predicate: F) -> Self {
        Self {
            predicate,
            entries: Vec::new(),
        }
    }
}

#[allow(dead_code)]
impl<F> JsonlProcessor for FilterProcessor<F>
where
    F: Fn(&UsageEntry) -> bool,
{
    type Output = Vec<UsageEntry>;
    
    fn process_entry(&mut self, entry: UsageEntry, _line_number: usize) -> Result<()> {
        if (self.predicate)(&entry) {
            self.entries.push(entry);
        }
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(self.entries)
    }
}

// Processor that streams entries through a callback
#[allow(dead_code)]
pub struct StreamProcessor<F>
where
    F: FnMut(UsageEntry, usize) -> Result<()>,
{
    callback: F,
}

#[allow(dead_code)]
impl<F> StreamProcessor<F>
where
    F: FnMut(UsageEntry, usize) -> Result<()>,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

#[allow(dead_code)]
impl<F> JsonlProcessor for StreamProcessor<F>
where
    F: FnMut(UsageEntry, usize) -> Result<()>,
{
    type Output = ();
    
    fn process_entry(&mut self, entry: UsageEntry, line_number: usize) -> Result<()> {
        (self.callback)(entry, line_number)
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(())
    }
}

// Processor that collects ProcessedEntry objects
#[allow(dead_code)]
pub struct ProcessedEntryCollector {
    entries: Vec<ProcessedEntry>,
    parser: FileParser,
}

#[allow(dead_code)]
impl ProcessedEntryCollector {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            parser: FileParser::new(),
        }
    }
}

#[allow(dead_code)]
impl JsonlProcessor for ProcessedEntryCollector {
    type Output = Vec<ProcessedEntry>;
    
    fn process_entry(&mut self, entry: UsageEntry, line_number: usize) -> Result<()> {
        if let Ok(processed) = ProcessedEntry::new(entry, &self.parser, line_number) {
            self.entries.push(processed);
        }
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(self.entries)
    }
}

// Processor that only processes valid entries (with usage data) through a callback
#[allow(dead_code)]
pub struct ValidEntryProcessor<F>
where
    F: FnMut(ProcessedEntry) -> Result<()>,
{
    callback: F,
    parser: FileParser,
}

#[allow(dead_code)]
impl<F> ValidEntryProcessor<F>
where
    F: FnMut(ProcessedEntry) -> Result<()>,
{
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            parser: FileParser::new(),
        }
    }
}

#[allow(dead_code)]
impl<F> JsonlProcessor for ValidEntryProcessor<F>
where
    F: FnMut(ProcessedEntry) -> Result<()>,
{
    type Output = ();
    
    fn process_entry(&mut self, entry: UsageEntry, line_number: usize) -> Result<()> {
        if entry.message.usage.is_some() {
            if let Ok(processed) = ProcessedEntry::new(entry, &self.parser, line_number) {
                (self.callback)(processed)?;
            }
        }
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok(())
    }
}

