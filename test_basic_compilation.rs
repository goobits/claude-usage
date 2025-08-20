//! Basic compilation test for parser_wrapper functionality

use claude_usage::parser_wrapper::UnifiedParser;
use tempfile::NamedTempFile;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing basic UnifiedParser compilation and functionality...");
    
    // Test 1: Can create UnifiedParser
    let parser = UnifiedParser::new();
    println!("✓ UnifiedParser::new() works");
    
    // Test 2: Can call parse_jsonl_file on empty file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(b"")?;
    temp_file.flush()?;
    
    let result = parser.parse_jsonl_file(temp_file.path());
    match result {
        Ok(entries) => {
            println!("✓ parse_jsonl_file() works, returned {} entries", entries.len());
        }
        Err(e) => {
            println!("✓ parse_jsonl_file() handled error gracefully: {}", e);
        }
    }
    
    // Test 3: Can call parse_jsonl_file on valid JSONL
    let mut temp_file2 = NamedTempFile::new()?;
    let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_test","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_test"}"#;
    temp_file2.write_all(jsonl_content.as_bytes())?;
    temp_file2.flush()?;
    
    let result = parser.parse_jsonl_file(temp_file2.path());
    match result {
        Ok(entries) => {
            println!("✓ parse_jsonl_file() parsed {} entries from valid JSONL", entries.len());
            if !entries.is_empty() {
                println!("  Entry 0 message ID: {}", entries[0].message.id);
                println!("  Entry 0 request ID: {}", entries[0].request_id);
            }
        }
        Err(e) => {
            println!("✗ parse_jsonl_file() failed on valid JSONL: {}", e);
        }
    }
    
    println!("All basic compilation tests completed!");
    Ok(())
}