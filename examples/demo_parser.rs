// Demo script showing how to use the JSONL parser with UnifiedParser
// Run with: cargo run --example demo_parser
// Note: This example demonstrates parsing usage data from JSONL files

use anyhow::Result;
use claude_usage::parser::{FileParser, ProcessedEntry};
use claude_usage::parser_wrapper::UnifiedParser;

fn main() -> Result<()> {
    let parser = FileParser::new();
    let unified_parser = UnifiedParser::new();

    println!("=== Claude Usage JSONL Parser Demo ===");

    // Discover Claude instances
    match parser.discover_claude_paths(false) {
        Ok(paths) => {
            println!("Found {} Claude instances", paths.len());
            
            // Find JSONL files
            match parser.find_jsonl_files(&paths) {
                Ok(files) => {
                    println!("Found {} JSONL files total", files.len());
                    
                    // Process first few files as examples
                    for (i, (file_path, session_dir)) in files.iter().take(3).enumerate() {
                        println!("\n--- Example {}: {} ---", i + 1, file_path.display());
                        
                        // Parse the file
                        match unified_parser.parse_jsonl_file(file_path) {
                            Ok(entries) => {
                                println!("Parsed {} entries", entries.len());
                                
                                // Show details for first few entries
                                for (j, entry) in entries.iter().take(2).enumerate() {
                                    println!("  Entry {}: model={}, request_id={}", 
                                        j + 1, entry.message.model, entry.request_id);
                                    
                                    if let Some(usage) = &entry.message.usage {
                                        println!("    Tokens: input={}, output={}", 
                                            usage.input_tokens, usage.output_tokens);
                                    }
                                    
                                    // Show processed metadata using FileParser utilities
                                    if let Ok(processed) = ProcessedEntry::new(entry.clone(), &parser, j + 1) {
                                        println!("    Date: {}, Total tokens: {}", 
                                            processed.date, processed.total_tokens);
                                    }
                                }
                                
                                if entries.len() > 2 {
                                    println!("    ... and {} more entries", entries.len() - 2);
                                }
                            }
                            Err(e) => {
                                println!("  Error parsing file: {}", e);
                            }
                        }
                    }
                }
                Err(e) => println!("Error finding JSONL files: {}", e),
            }
        }
        Err(e) => println!("Error discovering Claude paths: {}", e),
    }

    Ok(())
}