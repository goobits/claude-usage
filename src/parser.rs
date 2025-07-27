use crate::models::*;
use crate::file_discovery::FileDiscovery;
use crate::timestamp_parser::TimestampParser;
use crate::session_utils::SessionUtils;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub struct FileParser {
    file_discovery: FileDiscovery,
}

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
        Self {
            file_discovery: FileDiscovery::new(),
        }
    }

    pub fn discover_claude_paths(&self, exclude_vms: bool) -> Result<Vec<PathBuf>> {
        self.file_discovery.discover_claude_paths(exclude_vms)
    }

    pub fn find_jsonl_files(&self, claude_paths: &[PathBuf]) -> Result<Vec<(PathBuf, PathBuf)>> {
        self.file_discovery.find_jsonl_files(claude_paths)
    }

    pub fn should_include_file(&self, file_path: &Path, since_date: Option<&DateTime<Utc>>, until_date: Option<&DateTime<Utc>>) -> bool {
        self.file_discovery.should_include_file(file_path, since_date, until_date)
    }

    #[allow(dead_code)]
    pub fn get_earliest_timestamp(&self, file_path: &Path) -> Result<Option<DateTime<Utc>>> {
        self.file_discovery.get_earliest_timestamp(file_path)
    }

    pub fn sort_files_by_timestamp(&self, file_tuples: Vec<(PathBuf, PathBuf)>) -> Vec<(PathBuf, PathBuf)> {
        self.file_discovery.sort_files_by_timestamp(file_tuples)
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
        TimestampParser::parse(timestamp_str)
    }

    pub fn extract_session_info(&self, session_dir_name: &str) -> (String, String) {
        SessionUtils::extract_session_info(session_dir_name)
    }

    pub fn create_unique_hash(&self, entry: &UsageEntry) -> Option<String> {
        SessionUtils::create_unique_hash(entry)
    }

    pub fn find_session_blocks_files(&self, claude_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        self.file_discovery.find_session_blocks_files(claude_paths)
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
        SessionUtils::parse_session_blocks_file(file_path)
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

