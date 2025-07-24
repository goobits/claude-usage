// Debug script to check file discovery
// Run with: cargo run --example debug_files

use claude_usage::parser::FileParser;
use anyhow::Result;

fn main() -> Result<()> {
    let parser = FileParser::new();
    
    println!("🔍 Debugging file discovery...\n");
    
    // Step 1: Discover Claude paths
    let claude_paths = parser.discover_claude_paths()?;
    println!("Claude instances found: {}", claude_paths.len());
    for (i, path) in claude_paths.iter().enumerate() {
        println!("  {}: {}", i + 1, path.display());
    }
    
    // Step 2: Find JSONL files
    println!("\nSearching for JSONL files...");
    let file_tuples = parser.find_jsonl_files(&claude_paths)?;
    println!("Total JSONL files found: {}", file_tuples.len());
    
    if file_tuples.is_empty() {
        println!("\n⚠️  No JSONL files found!");
        
        // Let's check manually
        use std::fs;
        println!("\nManual check of ~/.claude/projects:");
        if let Some(home) = dirs::home_dir() {
            let claude_projects = home.join(".claude").join("projects");
            if claude_projects.exists() {
                if let Ok(entries) = fs::read_dir(&claude_projects) {
                    let mut count = 0;
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            println!("  Dir: {}", entry.file_name().to_string_lossy());
                            
                            // Check for conversation files
                            if let Ok(files) = fs::read_dir(entry.path()) {
                                for file in files.flatten() {
                                    let name = file.file_name();
                                    let name_str = name.to_string_lossy();
                                    if name_str.starts_with("conversation_") && name_str.ends_with(".jsonl") {
                                        count += 1;
                                        if count <= 5 {
                                            println!("    -> {}", name_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if count > 5 {
                        println!("    ... and {} more files", count - 5);
                    }
                }
            }
        }
    } else {
        // Show first few files
        println!("\nFirst 5 JSONL files:");
        for (i, (file_path, session_dir)) in file_tuples.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, file_path.display());
            if let Some(session_name) = session_dir.file_name() {
                println!("     Session: {}", session_name.to_string_lossy());
            }
        }
        
        // Check file dates
        println!("\nChecking file modification times:");
        use std::fs;
        for (file_path, _) in file_tuples.iter().take(3) {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    let datetime = chrono::DateTime::<chrono::Utc>::from(modified);
                    println!("  {} - Modified: {}", 
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        datetime.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }
    }
    
    Ok(())
}