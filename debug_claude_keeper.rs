use claude_usage::keeper_integration::KeeperIntegration;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing_subscriber;

fn main() {
    // Set up debug logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let integration = KeeperIntegration::new();

    // Test cases with different Claude Desktop JSON formats
    let test_cases = vec![
        // Standard Claude Desktop format with all fields
        r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_test","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_test"}"#,
        
        // Minimal Claude Desktop format (no usage, no cost)
        r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_minimal","model":"claude-3-5-sonnet-20241022"},"requestId":"req_minimal"}"#,
        
        // Format with snake_case fields (if any exist)
        r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_snake","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":50,"output_tokens":75,"cache_creation_input_tokens":5,"cache_read_input_tokens":10}},"cost_usd":0.003,"request_id":"req_snake"}"#,
    ];

    println!("=== Debugging Claude-Keeper Integration ===\n");

    for (i, test_json) in test_cases.iter().enumerate() {
        println!("--- Test Case {} ---", i + 1);
        println!("JSON: {}", test_json);
        
        match integration.parse_single_line(test_json) {
            Some(entry) => {
                println!("✅ SUCCESS: Parsed successfully");
                println!("   Timestamp: {}", entry.timestamp);
                println!("   Message ID: {}", entry.message.id);
                println!("   Model: {}", entry.message.model);
                println!("   Request ID: {}", entry.request_id);
                println!("   Cost: {:?}", entry.cost_usd);
                if let Some(usage) = &entry.message.usage {
                    println!("   Usage: input={}, output={}, cache_create={}, cache_read={}", 
                             usage.input_tokens, usage.output_tokens, 
                             usage.cache_creation_input_tokens, usage.cache_read_input_tokens);
                } else {
                    println!("   Usage: None");
                }
            }
            None => {
                println!("❌ FAILED: Could not parse");
            }
        }
        println!();
    }

    // Test with a file containing multiple lines
    println!("--- File Parsing Test ---");
    let mut temp_file = NamedTempFile::new().unwrap();
    for test_json in &test_cases {
        writeln!(temp_file, "{}", test_json).unwrap();
    }
    writeln!(temp_file, "{{\"malformed\": json}}").unwrap(); // Add one bad line
    temp_file.flush().unwrap();

    match integration.parse_jsonl_file(temp_file.path()) {
        Ok(entries) => {
            println!("✅ File parsing succeeded: {} entries parsed", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                println!("  Entry {}: {} -> {}", i + 1, entry.message.id, entry.request_id);
            }
        }
        Err(e) => {
            println!("❌ File parsing failed: {}", e);
        }
    }
}