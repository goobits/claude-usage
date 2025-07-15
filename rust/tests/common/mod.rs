use std::fs;
use std::path::Path;
use tempfile::TempDir;
use anyhow::Result;

pub fn create_test_jsonl(dir: &Path, filename: &str, content: &str) -> Result<()> {
    let file_path = dir.join(filename);
    fs::write(&file_path, content)?;
    Ok(())
}

pub fn create_test_session_dir(temp_dir: &TempDir) -> Result<()> {
    let session_dir = temp_dir.path().join("projects").join("test-session");
    fs::create_dir_all(&session_dir)?;
    
    let jsonl_content = r#"{"timestamp": "2024-01-01T12:00:00Z", "message": {"id": "msg1", "model": "claude-sonnet-4-20250514", "usage": {"input_tokens": 100, "output_tokens": 50, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}}, "requestId": "req1", "costUSD": 0.001}
{"timestamp": "2024-01-01T12:01:00Z", "message": {"id": "msg2", "model": "claude-sonnet-4-20250514", "usage": {"input_tokens": 200, "output_tokens": 100, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}}, "requestId": "req2", "costUSD": 0.002}
"#;
    
    create_test_jsonl(&session_dir, "conversation_test.jsonl", jsonl_content)?;
    Ok(())
}

pub fn setup_test_environment() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    create_test_session_dir(&temp_dir)?;
    Ok(temp_dir)
}