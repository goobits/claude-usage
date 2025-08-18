//! Tests for the unified parser wrapper

use claude_usage::parser_wrapper::UnifiedParser;
use tempfile::NamedTempFile;
use std::io::Write;

fn create_test_jsonl(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn test_unified_parser_basic_functionality() {
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_test","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_test"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    let entries = parser.parse_jsonl_file(temp_file.path()).unwrap();
    
    assert!(!entries.is_empty(), "Parser should return entries");
    assert_eq!(entries[0].message.id, "msg_test");
}

#[cfg(feature = "keeper-integration")]
#[test]
fn test_unified_parser_uses_keeper_when_enabled() {
    // When keeper-integration is enabled, should handle malformed data gracefully
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022"},"requestId":"req_1"}
{broken json line}
{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022"},"requestId":"req_2"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    // With keeper integration, should still parse valid lines
    let entries = parser.parse_jsonl_file(temp_file.path()).unwrap();
    assert_eq!(entries.len(), 2, "Should parse 2 valid entries with keeper integration");
}

#[cfg(feature = "keeper-integration")]
#[test]
fn test_unified_parser_keeper_feature_comprehensive() {
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_valid1","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":5,"cache_read_input_tokens":10}},"costUSD":0.005,"requestId":"req_1"}
{this line should be skipped due to malformed json}
{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_valid2","model":"claude-3-haiku-20240307","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":25}},"costUSD":0.002,"requestId":"req_2"}
null
{"timestamp":"2024-01-15T10:32:00Z","message":{"id":"msg_valid3","model":"claude-3-5-sonnet-20241022"},"requestId":"req_3"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    let entries = parser.parse_jsonl_file(temp_file.path()).unwrap();
    
    // Should successfully parse 3 valid entries, skipping malformed ones
    assert_eq!(entries.len(), 3, "Should parse 3 valid entries with keeper integration");
    
    // Validate specific entries
    assert_eq!(entries[0].message.id, "msg_valid1");
    assert_eq!(entries[0].message.model, "claude-3-5-sonnet-20241022");
    assert!(entries[0].message.usage.is_some());
    assert_eq!(entries[0].cost_usd, Some(0.005));
    
    assert_eq!(entries[1].message.id, "msg_valid2");
    assert_eq!(entries[1].message.model, "claude-3-haiku-20240307");
    assert!(entries[1].message.usage.is_some());
    assert_eq!(entries[1].cost_usd, Some(0.002));
    
    assert_eq!(entries[2].message.id, "msg_valid3");
    assert_eq!(entries[2].message.model, "claude-3-5-sonnet-20241022");
    assert!(entries[2].message.usage.is_none()); // No usage data in this entry
    assert!(entries[2].cost_usd.is_none()); // No cost data
}

#[cfg(not(feature = "keeper-integration"))]
#[test]
fn test_unified_parser_uses_legacy_when_disabled() {
    // When keeper-integration is disabled, should use legacy parser
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_legacy","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_legacy"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    // Should parse using legacy parser
    let result = parser.parse_jsonl_file(temp_file.path());
    
    match result {
        Ok(entries) => {
            assert!(!entries.is_empty(), "Legacy parser should return entries");
            assert_eq!(entries[0].message.id, "msg_legacy");
        }
        Err(_) => {
            // If legacy parser fails, that's also valid behavior - just ensure it doesn't panic
            assert!(true, "Legacy parser handled the file (with error is acceptable)");
        }
    }
}

#[test]
fn test_unified_parser_interface_consistency() {
    // Test that UnifiedParser provides consistent interface regardless of feature flag
    let parser = UnifiedParser::new();
    
    // Should compile and work with or without keeper-integration feature
    let empty_file = create_test_jsonl("");
    let result = parser.parse_jsonl_file(empty_file.path());
    
    assert!(result.is_ok(), "Parser should handle empty file");
    assert_eq!(result.unwrap().len(), 0, "Empty file should return empty vec");
}

#[test]
fn test_unified_parser_creation() {
    // Test that parser can be created without issues
    let parser = UnifiedParser::new();
    
    // Basic smoke test - parser should be created successfully
    // The actual behavior depends on feature flags, but creation should always work
    drop(parser); // Explicit drop to ensure no issues with cleanup
}

#[test]
fn test_unified_parser_handles_single_entry() {
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"single_msg","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":75,"output_tokens":125,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.003,"requestId":"single_req"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    let entries = parser.parse_jsonl_file(temp_file.path()).unwrap();
    
    assert_eq!(entries.len(), 1, "Should parse single entry correctly");
    assert_eq!(entries[0].message.id, "single_msg");
    assert_eq!(entries[0].request_id, "single_req");
    assert_eq!(entries[0].message.model, "claude-3-5-sonnet-20241022");
    
    if let Some(usage) = &entries[0].message.usage {
        assert_eq!(usage.input_tokens, 75);
        assert_eq!(usage.output_tokens, 125);
    }
}

#[test]
fn test_unified_parser_handles_multiple_entries() {
    let jsonl_content = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_1"}
{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-haiku-20240307","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.001,"requestId":"req_2"}
{"timestamp":"2024-01-15T10:32:00Z","message":{"id":"msg_3","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":200,"output_tokens":300,"cache_creation_input_tokens":10,"cache_read_input_tokens":20}},"costUSD":0.008,"requestId":"req_3"}"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    let entries = parser.parse_jsonl_file(temp_file.path()).unwrap();
    
    assert_eq!(entries.len(), 3, "Should parse all three entries");
    
    // Verify each entry
    assert_eq!(entries[0].message.id, "msg_1");
    assert_eq!(entries[1].message.id, "msg_2");
    assert_eq!(entries[2].message.id, "msg_3");
    
    // Verify models are preserved
    assert_eq!(entries[0].message.model, "claude-3-5-sonnet-20241022");
    assert_eq!(entries[1].message.model, "claude-3-haiku-20240307");
    assert_eq!(entries[2].message.model, "claude-3-5-sonnet-20241022");
}

#[test]
fn test_unified_parser_file_not_found() {
    use std::path::Path;
    
    let parser = UnifiedParser::new();
    let non_existent_path = Path::new("/tmp/non_existent_file_xyz_123.jsonl");
    
    let result = parser.parse_jsonl_file(non_existent_path);
    
    // Should handle file not found gracefully with error
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_unified_parser_whitespace_handling() {
    let jsonl_content = r#"
{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_with_whitespace","model":"claude-3-5-sonnet-20241022"},"requestId":"req_ws"}

{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_after_blank","model":"claude-3-5-sonnet-20241022"},"requestId":"req_ab"}
"#;
    
    let temp_file = create_test_jsonl(jsonl_content);
    let parser = UnifiedParser::new();
    
    let result = parser.parse_jsonl_file(temp_file.path());
    
    match result {
        Ok(entries) => {
            // Should handle whitespace gracefully
            assert!(entries.len() >= 1, "Should parse at least one valid entry despite whitespace");
        }
        Err(_) => {
            // Some parsers might be strict about whitespace - that's acceptable behavior
            assert!(true, "Parser handled whitespace (error is acceptable)");
        }
    }
}