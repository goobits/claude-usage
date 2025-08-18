use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_usage::parser::FileParser;
use claude_usage::parser_wrapper::UnifiedParser;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

use claude_usage::keeper_integration::KeeperIntegration;

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

/// Generate test JSONL data with specified number of lines for performance testing
fn generate_performance_test_jsonl(num_lines: usize, include_errors: bool) -> String {
    let mut lines = Vec::new();
    
    for i in 0..num_lines {
        if include_errors && i % 10 == 5 {
            // Insert malformed line every 10th entry
            lines.push("{broken json}".to_string());
        } else {
            lines.push(format!(
                r#"{{"timestamp":"2024-01-15T10:30:{}Z","message":{{"id":"msg_{}","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_input_tokens":{},"cache_read_input_tokens":{}}}}},"costUSD":{},"requestId":"req_{}"}}"#,
                format!("{:02}", i % 60),
                i,
                100 + i,
                200 + i,
                i % 50,
                i % 100,
                0.001 * (i as f64),
                i
            ));
        }
    }
    
    lines.join("\n")
}

fn create_performance_temp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn benchmark_legacy_parser_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("legacy_parser_scaling");
    
    for size in [10, 100, 1000, 10000].iter() {
        let jsonl_content = generate_performance_test_jsonl(*size, false);
        let temp_file = create_performance_temp_file(&jsonl_content);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                let parser = FileParser::new();
                b.iter(|| {
                    parser.parse_jsonl_file(black_box(temp_file.path()))
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_keeper_parser_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("keeper_parser_scaling");
    
    for size in [10, 100, 1000, 10000].iter() {
        let jsonl_content = generate_performance_test_jsonl(*size, false);
        let temp_file = create_performance_temp_file(&jsonl_content);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                let integration = KeeperIntegration::new();
                b.iter(|| {
                    integration.parse_jsonl_file(black_box(temp_file.path()))
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_error_handling_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling_performance");
    
    // Test with 10% malformed lines
    let jsonl_with_errors = generate_performance_test_jsonl(1000, true);
    let temp_file = create_performance_temp_file(&jsonl_with_errors);
    
    group.bench_function("legacy_with_errors", |b| {
        let parser = FileParser::new();
        b.iter(|| {
            parser.parse_jsonl_file(black_box(temp_file.path()))
        });
    });
    
    group.bench_function("keeper_with_errors", |b| {
        let integration = KeeperIntegration::new();
        b.iter(|| {
            integration.parse_jsonl_file(black_box(temp_file.path()))
        });
    });
    
    group.finish();
}

fn benchmark_unified_parser_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("unified_parser_performance");
    
    for size in [100, 1000, 5000].iter() {
        let jsonl_content = generate_performance_test_jsonl(*size, false);
        let temp_file = create_performance_temp_file(&jsonl_content);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                let parser = UnifiedParser::new();
                b.iter(|| {
                    parser.parse_jsonl_file(black_box(temp_file.path()))
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_usage_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage_performance");
    
    // Large file to test memory efficiency
    let large_jsonl = generate_performance_test_jsonl(50000, false);
    let temp_file = create_performance_temp_file(&large_jsonl);
    
    group.bench_function("legacy_large_file", |b| {
        let parser = FileParser::new();
        b.iter(|| {
            parser.parse_jsonl_file(black_box(temp_file.path()))
        });
    });
    
    group.bench_function("keeper_large_file", |b| {
        let integration = KeeperIntegration::new();
        b.iter(|| {
            integration.parse_jsonl_file(black_box(temp_file.path()))
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_jsonl_parsing,
    benchmark_timestamp_parsing,
    benchmark_session_info_extraction,
    benchmark_legacy_parser_scaling,
    benchmark_keeper_parser_scaling,
    benchmark_error_handling_performance,
    benchmark_unified_parser_performance,
    benchmark_memory_usage_performance
);

criterion_main!(benches);