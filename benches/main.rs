use criterion::{black_box, criterion_group, criterion_main, Criterion};
use claude_usage::parser::FileParser;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

fn create_large_jsonl_file(dir: &std::path::Path, entries: usize) -> anyhow::Result<PathBuf> {
    let session_dir = dir.join("projects").join("benchmark-session");
    fs::create_dir_all(&session_dir)?;
    
    let jsonl_path = session_dir.join("conversation_benchmark.jsonl");
    let mut content = String::new();
    
    for i in 0..entries {
        content.push_str(&format!(
            r#"{{"timestamp": "2024-01-01T12:{:02}:00Z", "message": {{"id": "msg{}", "model": "claude-sonnet-4-20250514", "usage": {{"input_tokens": {}, "output_tokens": {}, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}}}}, "requestId": "req{}", "costUSD": {}}}
"#,
            i % 60, // minutes
            i,
            100 + (i % 100), // varying input tokens
            50 + (i % 50),   // varying output tokens
            i,
            (i as f64) * 0.001
        ));
    }
    
    fs::write(&jsonl_path, content)?;
    Ok(jsonl_path)
}

fn benchmark_jsonl_parsing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let jsonl_path = create_large_jsonl_file(temp_dir.path(), 1000).unwrap();
    
    let parser = FileParser::new();
    
    c.bench_function("parse_jsonl_1000_entries", |b| {
        b.iter(|| {
            let entries = parser.parse_jsonl_file(black_box(&jsonl_path)).unwrap();
            black_box(entries)
        })
    });
}

fn benchmark_timestamp_parsing(c: &mut Criterion) {
    let parser = FileParser::new();
    
    c.bench_function("parse_timestamp", |b| {
        b.iter(|| {
            let timestamp = parser.parse_timestamp(black_box("2024-01-01T12:00:00Z")).unwrap();
            black_box(timestamp)
        })
    });
}

fn benchmark_session_info_extraction(c: &mut Criterion) {
    let parser = FileParser::new();
    
    c.bench_function("extract_session_info", |b| {
        b.iter(|| {
            let (session_id, project_name) = parser.extract_session_info(black_box("-vm1-project-test"));
            black_box((session_id, project_name))
        })
    });
}

criterion_group!(
    benches,
    benchmark_jsonl_parsing,
    benchmark_timestamp_parsing,
    benchmark_session_info_extraction
);
criterion_main!(benches);