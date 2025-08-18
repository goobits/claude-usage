//! Tests for streaming parser memory safety

use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_streaming_parser_memory_safety() {
    use claude_usage::keeper_integration::KeeperIntegration;
    
    let integration = KeeperIntegration::new();
    
    // Create a large test file (simulate 50MB)
    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 0..500_000 {
        writeln!(
            temp_file,
            r#"{{"timestamp":"2024-01-15T10:30:00Z","message":{{"id":"msg_{}","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"requestId":"req_{}"}}"#,
            i, i
        ).unwrap();
    }
    temp_file.flush().unwrap();
    
    let file_size = temp_file.as_file().metadata().unwrap().len();
    println!("Test file size: {} MB", file_size / 1_000_000);
    
    // Parse should complete without OOM
    let before_mem = claude_usage::memory::get_memory_usage_mb();
    let result = integration.parse_jsonl_file(temp_file.path());
    let after_mem = claude_usage::memory::get_memory_usage_mb();
    
    assert!(result.is_ok(), "Should parse large file successfully");
    
    let memory_increase = after_mem.saturating_sub(before_mem);
    println!("Memory increase: {} MB", memory_increase);
    
    // Memory increase should be much less than file size
    assert!(
        memory_increase < (file_size / 1_000_000 / 2) as usize,
        "Memory usage should be less than half of file size"
    );
}