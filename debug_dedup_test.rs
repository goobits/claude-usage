use claude_usage::analyzer::ClaudeUsageAnalyzer;
use claude_usage::dedup::ProcessOptions;
use std::env;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let temp_dir = TempDir::new().unwrap();
    
    // Set CLAUDE_HOME to isolate test environment from real Claude installation
    env::set_var("CLAUDE_HOME", temp_dir.path());
    println!("🏠 Set CLAUDE_HOME to: {}", temp_dir.path().display());
    
    let claude_path = temp_dir.path().join(".claude");

    // Create ONLY a single project directory (no VMs to avoid multiple instances)
    let project_path = claude_path.join("projects").join("test_project_dedup");
    fs::create_dir_all(&project_path).unwrap();

    let jsonl_path = project_path.join("conversation_dedup.jsonl");
    let mut file = fs::File::create(&jsonl_path).unwrap();

    // Create duplicate entries with same messageId and requestId
    println!("Creating test data...");
    for i in 0..3 {
        let line = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_duplicate","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200}},"costUSD":0.003,"requestId":"req_duplicate"}"#;
        println!("Entry {}: {}", i + 1, line);
        writeln!(file, "{}", line).unwrap();
    }

    // Add unique entry
    let unique_line = r#"{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_unique","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":150,"output_tokens":250}},"costUSD":0.004,"requestId":"req_unique"}"#;
    println!("Unique entry: {}", unique_line);
    writeln!(file, "{}", unique_line).unwrap();

    drop(file); // Close file

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

    println!("\nRunning analysis...");
    let result = analyzer.aggregate_data("daily", options).await;

    match result {
        Ok(sessions) => {
            println!("\nAnalysis results:");
            let total_cost: f64 = sessions.iter().map(|s| s.total_cost).sum();
            println!("Total cost: {}", total_cost);
            println!("Expected cost after deduplication: 0.007 (1 x 0.003 + 0.004)");
            println!("Actual cost: {}", total_cost);
            
            if total_cost < 0.013 {
                println!("✅ PASS: Deduplication worked correctly");
            } else {
                println!("❌ FAIL: Deduplication did not work - cost too high");
            }
            
            for session in &sessions {
                println!("Session: {} - Cost: {}", session.session_id, session.total_cost);
            }
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
        }
    }
}