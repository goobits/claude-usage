//! Performance benchmarks for keeper-based parsing
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_usage::keeper_integration::KeeperIntegration;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

/// Generate test JSONL data with specified number of lines
fn generate_test_jsonl(num_lines: usize, include_errors: bool) -> String {
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

fn create_temp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn benchmark_keeper_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("keeper_parser");
    
    for size in [10, 100, 1000, 10000].iter() {
        let jsonl_content = generate_test_jsonl(*size, false);
        let temp_file = create_temp_file(&jsonl_content);
        
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

fn benchmark_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    // Test with 10% malformed lines
    let jsonl_with_errors = generate_test_jsonl(1000, true);
    let temp_file = create_temp_file(&jsonl_with_errors);
    
    group.bench_function("keeper_with_errors", |b| {
        let integration = KeeperIntegration::new();
        b.iter(|| {
            integration.parse_jsonl_file(black_box(temp_file.path()))
        });
    });
    
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    // Test memory behavior with large files
    for size in [10000, 50000].iter() {
        let jsonl_content = generate_test_jsonl(*size, false);
        let temp_file = create_temp_file(&jsonl_content);
        
        group.bench_with_input(
            BenchmarkId::new("large_file", size),
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

criterion_group!(benches, benchmark_keeper_parser, benchmark_error_handling, benchmark_memory_usage);
criterion_main!(benches);