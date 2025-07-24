// Script to check entries for a specific date
// Run with: cargo run --example check_date

use claude_usage::parser::{FileParser, JsonlProcessor, ProcessedEntry};
use claude_usage::models::UsageEntry;
use anyhow::Result;
use std::collections::HashMap;

struct DateChecker {
    target_date: String,
    entries_found: Vec<ProcessedEntry>,
    parser: FileParser,
    file_count: usize,
    total_files_checked: usize,
}

impl DateChecker {
    fn new(target_date: &str) -> Self {
        Self {
            target_date: target_date.to_string(),
            entries_found: Vec::new(),
            parser: FileParser::new(),
            file_count: 0,
            total_files_checked: 0,
        }
    }
}

impl JsonlProcessor for DateChecker {
    type Output = (Vec<ProcessedEntry>, usize, usize);
    
    fn process_entry(&mut self, entry: UsageEntry, line_number: usize) -> Result<()> {
        if let Ok(processed) = ProcessedEntry::new(entry, &self.parser, line_number) {
            if processed.date == self.target_date {
                self.entries_found.push(processed);
            }
        }
        Ok(())
    }
    
    fn finalize(self) -> Result<Self::Output> {
        Ok((self.entries_found, self.file_count, self.total_files_checked))
    }
}

fn main() -> Result<()> {
    let target_date = "2025-07-09";
    println!("🔍 Checking for entries on {}", target_date);
    println!("{}", "=".repeat(80));
    
    let parser = FileParser::new();
    
    // Discover all Claude instances
    let claude_paths = parser.discover_claude_paths()?;
    println!("Found {} Claude instances", claude_paths.len());
    
    // Find all JSONL files
    let file_tuples = parser.find_jsonl_files(&claude_paths)?;
    println!("Found {} JSONL files to check\n", file_tuples.len());
    
    let mut all_entries = Vec::new();
    let mut files_with_entries = 0;
    let mut total_files_checked = 0;
    
    // Check each file
    for (file_path, session_dir) in &file_tuples {
        total_files_checked += 1;
        
        // Use DateChecker to find entries for the target date
        let mut checker = DateChecker::new(target_date);
        checker.total_files_checked = total_files_checked;
        
        let (entries, _, _) = parser.process_jsonl_file(file_path, checker)?;
        
        if !entries.is_empty() {
            files_with_entries += 1;
            println!("📁 Found {} entries in: {}", 
                entries.len(), 
                file_path.display()
            );
            
            // Extract session info
            if let Some(session_name) = session_dir.file_name() {
                println!("   Session: {}", session_name.to_string_lossy());
            }
            
            // Show entry details
            for entry in &entries {
                // Note: In the example we'll use stored cost_usd for simplicity
                // The main tool uses async PricingManager::calculate_cost_from_tokens
                let cost = entry.entry.cost_usd.unwrap_or(0.0);
                
                println!("   - Line {}: {} @ {} - {} tokens (${:.4})",
                    entry.line_number,
                    entry.entry.message.model,
                    entry.timestamp.format("%H:%M:%S UTC"),
                    entry.total_tokens,
                    cost
                );
                
                if entry.has_usage() {
                    if let Some(usage) = &entry.entry.message.usage {
                        println!("     Input: {}, Output: {}, Cache: {} + {}",
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.cache_creation_input_tokens,
                            usage.cache_read_input_tokens
                        );
                    }
                }
            }
            println!();
            
            all_entries.extend(entries);
        }
        
        // Progress indicator every 100 files
        if total_files_checked % 100 == 0 {
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
    }
    
    if total_files_checked >= 100 {
        println!(); // New line after progress dots
    }
    
    // Summary
    println!("\n{}", "=".repeat(80));
    println!("📊 Summary for {}:", target_date);
    println!("   Files checked: {}", total_files_checked);
    println!("   Files with entries: {}", files_with_entries);
    println!("   Total entries found: {}", all_entries.len());
    
    if !all_entries.is_empty() {
        // Calculate totals
        let total_tokens: u32 = all_entries.iter().map(|e| e.total_tokens).sum();
        
        // Note: The actual tool uses PricingManager::calculate_cost_from_tokens
        // which fetches pricing from LiteLLM API and calculates based on model
        // For this example, we'll just show that stored cost_usd is 0
        let total_cost: f64 = all_entries.iter()
            .map(|e| e.entry.cost_usd.unwrap_or(0.0))
            .sum();
        
        println!("   Total tokens: {}", total_tokens);
        println!("   Total cost: ${:.4}", total_cost);
        
        if total_cost == 0.0 && total_tokens > 0 {
            println!("\n   Note: JSONL entries store cost_usd as 0. The main tool calculates");
            println!("   costs dynamically using PricingManager with LiteLLM pricing data.");
        }
        
        // Group by model
        let mut model_stats: HashMap<String, (u32, u32)> = HashMap::new();
        for entry in &all_entries {
            let stats = model_stats.entry(entry.entry.message.model.clone())
                .or_insert((0, 0));
            stats.0 += 1; // count
            stats.1 += entry.total_tokens; // tokens
        }
        
        println!("\n📈 By Model:");
        for (model, (count, tokens)) in model_stats {
            println!("   {}: {} entries, {} tokens", model, count, tokens);
        }
        
        // Show time range
        if let (Some(first), Some(last)) = (all_entries.first(), all_entries.last()) {
            println!("\n⏰ Time range:");
            println!("   First: {}", first.timestamp.format("%H:%M:%S UTC"));
            println!("   Last: {}", last.timestamp.format("%H:%M:%S UTC"));
        }
    } else {
        println!("\n❌ No entries found for {}!", target_date);
        println!("   This explains why the daily report shows $0.00 with 0 sessions.");
        
        // Let's check nearby dates to see if there's data
        println!("\n🔍 Checking for entries on nearby dates...");
        
        let nearby_dates = vec![
            "2025-07-11",
            "2025-07-13",
            "2025-07-10",
            "2025-07-14",
        ];
        
        for check_date in nearby_dates {
            let mut count = 0;
            for (file_path, _) in &file_tuples {
                let checker = DateChecker::new(check_date);
                let (entries, _, _) = parser.process_jsonl_file(file_path, checker)?;
                count += entries.len();
            }
            if count > 0 {
                println!("   {} - Found {} entries", check_date, count);
            }
        }
    }
    
    Ok(())
}