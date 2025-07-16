use claude_usage::parser::FileParser;
use claude_usage::models::CostMode;

mod common;

#[test]
fn test_extract_session_info() {
    let parser = FileParser::new(CostMode::Auto);
    
    // Test with dash prefix
    let (session_id, project_name) = parser.extract_session_info("-vm1-project-test");
    assert_eq!(session_id, "-vm1-project-test");
    assert_eq!(project_name, "test");
    
    // Test without dash prefix
    let (session_id, project_name) = parser.extract_session_info("simple-project");
    assert_eq!(session_id, "simple-project");
    assert_eq!(project_name, "simple-project");
}

#[test]
fn test_create_unique_hash() {
    let parser = FileParser::new(CostMode::Auto);
    
    let entry = claude_usage::models::UsageEntry {
        timestamp: "2024-01-01T12:00:00Z".to_string(),
        message: claude_usage::models::MessageData {
            id: "msg123".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            usage: Some(claude_usage::models::UsageData {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
        },
        cost_usd: Some(0.001),
        request_id: "req456".to_string(),
    };
    
    let hash = parser.create_unique_hash(&entry);
    assert_eq!(hash, Some("msg123:req456".to_string()));
}

#[tokio::test]
async fn test_parse_jsonl_file() -> anyhow::Result<()> {
    let temp_dir = common::setup_test_environment()?;
    let parser = FileParser::new(CostMode::Auto);
    
    let jsonl_path = temp_dir.path().join("projects").join("test-session").join("conversation_test.jsonl");
    let entries = parser.parse_jsonl_file(&jsonl_path)?;
    
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message.id, "msg1");
    assert_eq!(entries[1].message.id, "msg2");
    
    Ok(())
}