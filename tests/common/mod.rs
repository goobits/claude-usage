use std::fs;
use std::path::Path;
use tempfile::TempDir;
use anyhow::Result;

pub fn create_test_jsonl(dir: &Path, filename: &str, content: &str) -> Result<()> {
    let file_path = dir.join(filename);
    fs::write(&file_path, content)?;
    Ok(())
}

