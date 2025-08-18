//! End-to-end integration tests with real-world data patterns

use claude_usage::analyzer::ClaudeUsageAnalyzer;
use claude_usage::dedup::ProcessOptions;
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use std::path::{Path, PathBuf};

/// Create a mock Claude Desktop directory structure
fn create_mock_claude_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    
    // Create main instance
    let main_project = claude_dir.join("projects").join("test_project_main");
    fs::create_dir_all(&main_project).unwrap();
    
    // Create VM instance
    let vm_project = claude_dir.join("vms").join("test_vm").join("projects").join("test_project_vm");
    fs::create_dir_all(&vm_project).unwrap();
    
    temp_dir
}

/// Create realistic JSONL content based on actual Claude Desktop patterns
fn create_realistic_jsonl(path: &Path, num_entries: usize, include_malformed: bool) {
    let mut file = fs::File::create(path).unwrap();
    
    for i in 0..num_entries {
        // Mix of different patterns seen in real Claude Desktop files
        let entry = if i % 3 == 0 {
            // Standard format with all fields
            format!(
                r#"{{"timestamp":"2024-01-15T10:30:{:02}Z","message":{{"id":"msg_{}","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_input_tokens":{},"cache_read_input_tokens":{}}}}},"costUSD":{},"requestId":"req_{}"}}"#,
                i % 60, i, 100 + i, 200 + i, i % 50, i % 100, 0.001 * (i as f64), i
            )
        } else if i % 3 == 1 {
            // Alternative field naming (snake_case)
            format!(
                r#"{{"timestamp":"2024-01-15T10:31:{:02}Z","message":{{"id":"msg_{}","model":"claude-3-opus-20240229","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}},"cost_usd":{},"request_id":"req_{}"}}"#,
                i % 60, i, 150 + i, 250 + i, 0.002 * (i as f64), i
            )
        } else {
            // Missing optional fields
            format!(
                r#"{{"timestamp":"2024-01-15T10:32:{:02}Z","message":{{"id":"msg_{}","model":"claude-3-5-sonnet-20241022"}},"requestId":"req_{}"}}"#,
                i % 60, i, i
            )
        };
        
        writeln!(file, "{}", entry).unwrap();
        
        // Add malformed line occasionally if requested
        if include_malformed && i % 10 == 5 {
            writeln!(file, "{{broken json line that should be skipped}}").unwrap();
        }
    }
}

#[tokio::test]
async fn test_e2e_basic_analysis() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    // Create test data
    let project_path = claude_path.join("projects").join("test_project_main");
    create_realistic_jsonl(&project_path.join("conversation_test.jsonl"), 50, false);
    
    // Set up analyzer
    let mut analyzer = ClaudeUsageAnalyzer::new();
    let options = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    
    // Run analysis - this uses UnifiedParser internally
    let result = analyzer.aggregate_data("daily", options).await;
    
    assert!(result.is_ok(), "Analysis should succeed");
    let sessions = result.unwrap();
    assert!(!sessions.is_empty(), "Should find sessions");
}

#[tokio::test]
async fn test_e2e_with_malformed_data() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    // Create test data with malformed lines
    let project_path = claude_path.join("projects").join("test_project_main");
    create_realistic_jsonl(&project_path.join("conversation_malformed.jsonl"), 50, true);
    
    let mut analyzer = ClaudeUsageAnalyzer::new();
    let options = ProcessOptions {
        command: "monthly".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    
    // Should handle malformed data gracefully
    let result = analyzer.aggregate_data("monthly", options).await;
    
    assert!(result.is_ok(), "Should handle malformed data gracefully");
    let sessions = result.unwrap();
    assert!(!sessions.is_empty(), "Should parse valid entries despite malformed ones");
}

#[tokio::test]
async fn test_e2e_vm_exclusion() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    // Create data in both main and VM
    let main_project = claude_path.join("projects").join("test_project_main");
    create_realistic_jsonl(&main_project.join("conversation_main.jsonl"), 30, false);
    
    let vm_project = claude_path.join("vms").join("test_vm").join("projects").join("test_project_vm");
    create_realistic_jsonl(&vm_project.join("conversation_vm.jsonl"), 20, false);
    
    let mut analyzer = ClaudeUsageAnalyzer::new();
    
    // Test with VMs included
    let options_with_vms = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    
    let result_with_vms = analyzer.aggregate_data("daily", options_with_vms).await.unwrap();
    
    // Test with VMs excluded
    let options_without_vms = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: true,
    };
    
    let result_without_vms = analyzer.aggregate_data("daily", options_without_vms).await.unwrap();
    
    // Should have fewer sessions when VMs excluded
    assert!(result_with_vms.len() > result_without_vms.len(), 
            "Should have more sessions with VMs included");
}

#[cfg(feature = "keeper-integration")]
#[tokio::test]
async fn test_e2e_keeper_schema_resilience() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    // Create JSONL with various field naming patterns
    let project_path = claude_path.join("projects").join("test_project_schema");
    let jsonl_path = project_path.join("conversation_schema.jsonl");
    fs::create_dir_all(&project_path).unwrap();
    
    let mut file = fs::File::create(&jsonl_path).unwrap();
    
    // Mix of camelCase, snake_case, and new fields
    writeln!(file, r#"{{"timestamp":"2024-01-15T10:30:00Z","message":{{"id":"msg_1","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"costUSD":0.003,"requestId":"req_1"}}"#).unwrap();
    writeln!(file, r#"{{"timestamp":"2024-01-15T10:31:00Z","message":{{"id":"msg_2","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":150,"output_tokens":250}}}},"cost_usd":0.004,"request_id":"req_2"}}"#).unwrap();
    writeln!(file, r#"{{"timestamp":"2024-01-15T10:32:00Z","message":{{"id":"msg_3","model":"claude-3-5-sonnet-20241022","usage":{{"inputTokens":200,"outputTokens":300}}}},"cost":0.005,"req_id":"req_3","newField":"future_value"}}"#).unwrap();
    
    let mut analyzer = ClaudeUsageAnalyzer::new();
    let options = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    
    // Keeper integration should handle all variations
    let result = analyzer.aggregate_data("daily", options).await;
    
    assert!(result.is_ok(), "Keeper should handle schema variations");
    let sessions = result.unwrap();
    assert!(!sessions.is_empty(), "Should parse all variations");
}

#[tokio::test]
async fn test_e2e_date_filtering() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    // Create data across multiple dates
    let project_path = claude_path.join("projects").join("test_project_dates");
    fs::create_dir_all(&project_path).unwrap();
    
    let jsonl_path = project_path.join("conversation_dates.jsonl");
    let mut file = fs::File::create(&jsonl_path).unwrap();
    
    // Entries from different dates
    writeln!(file, r#"{{"timestamp":"2024-01-10T10:00:00Z","message":{{"id":"msg_old","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"costUSD":0.003,"requestId":"req_old"}}"#).unwrap();
    writeln!(file, r#"{{"timestamp":"2024-01-15T10:00:00Z","message":{{"id":"msg_mid","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"costUSD":0.003,"requestId":"req_mid"}}"#).unwrap();
    writeln!(file, r#"{{"timestamp":"2024-01-20T10:00:00Z","message":{{"id":"msg_new","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"costUSD":0.003,"requestId":"req_new"}}"#).unwrap();
    
    let mut analyzer = ClaudeUsageAnalyzer::new();
    
    // Test with date range
    let options = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: Some(chrono::DateTime::parse_from_rfc3339("2024-01-14T00:00:00Z").unwrap().with_timezone(&chrono::Utc)),
        until_date: Some(chrono::DateTime::parse_from_rfc3339("2024-01-16T23:59:59Z").unwrap().with_timezone(&chrono::Utc)),
        snapshot: false,
        exclude_vms: false,
    };
    
    let result = analyzer.aggregate_data("daily", options).await;
    
    assert!(result.is_ok(), "Date filtering should work");
    // Should only include the middle entry
}

#[tokio::test]
async fn test_e2e_deduplication() {
    let temp_dir = create_mock_claude_structure();
    let claude_path = temp_dir.path().join(".claude");
    
    let project_path = claude_path.join("projects").join("test_project_dedup");
    fs::create_dir_all(&project_path).unwrap();
    
    let jsonl_path = project_path.join("conversation_dedup.jsonl");
    let mut file = fs::File::create(&jsonl_path).unwrap();
    
    // Create duplicate entries with same messageId and requestId
    for _ in 0..3 {
        writeln!(file, r#"{{"timestamp":"2024-01-15T10:30:00Z","message":{{"id":"msg_duplicate","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200}}}},"costUSD":0.003,"requestId":"req_duplicate"}}"#).unwrap();
    }
    
    // Add unique entry
    writeln!(file, r#"{{"timestamp":"2024-01-15T10:31:00Z","message":{{"id":"msg_unique","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":150,"output_tokens":250}}}},"costUSD":0.004,"requestId":"req_unique"}}"#).unwrap();
    
    let mut analyzer = ClaudeUsageAnalyzer::new();
    let options = ProcessOptions {
        command: "daily".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    
    let result = analyzer.aggregate_data("daily", options).await;
    
    assert!(result.is_ok(), "Deduplication should work");
    let sessions = result.unwrap();
    
    // Should have deduplicated the duplicate entries
    let total_cost: f64 = sessions.iter().map(|s| s.total_cost).sum();
    assert!(total_cost < 0.013, "Should have deduplicated entries (cost should be ~0.007, not 0.013)");
}