use claude_usage::dedup::{DeduplicationEngine, ProcessOptions};
use claude_usage::models::CostMode;

mod common;

#[tokio::test]
async fn test_deduplication_engine() -> anyhow::Result<()> {
    let temp_dir = common::setup_test_environment()?;
    let dedup_engine = DeduplicationEngine::new(CostMode::Auto);
    
    let file_tuples = vec![
        (
            temp_dir.path().join("projects").join("test-session").join("conversation_test.jsonl"),
            temp_dir.path().join("projects").join("test-session"),
        )
    ];
    
    let options = ProcessOptions {
        command: "session".to_string(),
        json_output: false,
        last: None,
        since_date: None,
        until_date: None,
        snapshot: false,
    };
    
    let results = dedup_engine.process_files_with_global_dedup(file_tuples, &options).await?;
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_id, "test-session");
    assert_eq!(results[0].input_tokens, 300); // 100 + 200
    assert_eq!(results[0].output_tokens, 150); // 50 + 100
    
    Ok(())
}