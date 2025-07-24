// Demo script showing how to use the pluggable JSONL parser
// Run with: cargo run --example demo_parser

use claude_usage::parser::{
    FileParser, JsonlProcessor, ProcessedEntry, ProcessedEntryCollector,
    CountProcessor, FilterProcessor, ValidEntryProcessor
};
use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    let parser = FileParser::new();
    
    // Example 1: Count entries in a file
    println!("=== Example 1: Counting entries ===");
    if let Ok(paths) = parser.discover_claude_paths() {
        if let Ok(files) = parser.find_jsonl_files(&paths) {
            if let Some((file_path, _)) = files.first() {
                let count = parser.process_jsonl_file(file_path, CountProcessor::new())?;
                println!("Total entries in file: {}", count);
            }
        }
    }
    
    // Example 2: Filter entries by model
    println!("\n=== Example 2: Filter by model ===");
    let demo_file = Path::new("test.jsonl");
    if demo_file.exists() {
        let filter = FilterProcessor::new(|entry| {
            entry.message.model.contains("claude-3-5-sonnet")
        });
        let filtered_entries = parser.process_jsonl_file(demo_file, filter)?;
        println!("Found {} Claude 3.5 Sonnet entries", filtered_entries.len());
    }
    
    // Example 3: Stream processing with parsed timestamps
    println!("\n=== Example 3: Stream processing ===");
    if demo_file.exists() {
        let mut total_tokens = 0u64;
        let mut entry_count = 0;
        
        let processor = ValidEntryProcessor::new(|processed: ProcessedEntry| {
            println!("Entry {} on {}: {} tokens from {}", 
                processed.line_number,
                processed.date,
                processed.total_tokens,
                processed.entry.message.model
            );
            total_tokens += processed.total_tokens as u64;
            entry_count += 1;
            Ok(())
        });
        
        parser.process_jsonl_file(demo_file, processor)?;
        println!("Processed {} entries with {} total tokens", entry_count, total_tokens);
    }
    
    // Example 4: Collect ProcessedEntry objects with all metadata
    println!("\n=== Example 4: Processed entries with metadata ===");
    if demo_file.exists() {
        let processed_entries = parser.process_jsonl_file(
            demo_file, 
            ProcessedEntryCollector::new()
        )?;
        
        for entry in processed_entries.iter().take(5) {
            println!("Line {}: {} @ {} - Input: {}, Output: {}, Cache: {}", 
                entry.line_number,
                entry.entry.message.model,
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.input_tokens(),
                entry.output_tokens(),
                entry.cache_tokens()
            );
        }
    }
    
    // Example 5: Custom processor for aggregating by date
    println!("\n=== Example 5: Custom date aggregation ===");
    if demo_file.exists() {
        use std::collections::HashMap;
        
        struct DateAggregator {
            daily_tokens: HashMap<String, u32>,
            parser: FileParser,
        }
        
        impl DateAggregator {
            fn new() -> Self {
                Self {
                    daily_tokens: HashMap::new(),
                    parser: FileParser::new(),
                }
            }
        }
        
        impl JsonlProcessor for DateAggregator {
            type Output = HashMap<String, u32>;
            
            fn process_entry(&mut self, entry: claude_usage::models::UsageEntry, line_number: usize) -> Result<()> {
                if let Ok(processed) = ProcessedEntry::new(entry, &self.parser, line_number) {
                    *self.daily_tokens.entry(processed.date).or_insert(0) += processed.total_tokens;
                }
                Ok(())
            }
            
            fn finalize(self) -> Result<Self::Output> {
                Ok(self.daily_tokens)
            }
        }
        
        let daily_totals = parser.process_jsonl_file(demo_file, DateAggregator::new())?;
        for (date, tokens) in daily_totals {
            println!("{}: {} tokens", date, tokens);
        }
    }
    
    Ok(())
}