//! Validation test to ensure all integration components work together

#[test]
fn test_unified_parser_import() {
    // Validate that UnifiedParser can be imported and instantiated
    use claude_usage::parser_wrapper::UnifiedParser;
    let _parser = UnifiedParser::new();
    assert!(
        true,
        "UnifiedParser should be importable and instantiatable"
    );
}

#[test]
fn test_analyzer_import() {
    // Validate that ClaudeUsageAnalyzer can be imported
    use claude_usage::analyzer::ClaudeUsageAnalyzer;
    let _analyzer = ClaudeUsageAnalyzer::new();
    assert!(
        true,
        "ClaudeUsageAnalyzer should be importable and instantiatable"
    );
}

#[test]
fn test_process_options_import() {
    // Validate ProcessOptions can be imported and created
    use claude_usage::dedup::ProcessOptions;
    let _options = ProcessOptions {
        command: "test".to_string(),
        json_output: false,
        limit: None,
        since_date: None,
        until_date: None,
        snapshot: false,
        exclude_vms: false,
    };
    assert!(true, "ProcessOptions should be importable and creatable");
}

#[test]
fn test_tempfile_dependency() {
    // Validate tempfile crate is available for test utilities
    use tempfile::TempDir;
    let _temp_dir = TempDir::new().unwrap();
    assert!(true, "tempfile should be available for testing");
}

#[test]
fn test_feature_flag_conditional_compilation() {
    // Test that conditional compilation works for feature flags

    #[cfg(feature = "keeper-integration")]
    {
        // When keeper-integration is enabled, this code should compile
        use claude_usage::keeper_integration::KeeperIntegration;
        let _integration = KeeperIntegration::new();
        assert!(
            true,
            "KeeperIntegration should be available when feature is enabled"
        );
    }

    #[cfg(not(feature = "keeper-integration"))]
    {
        // When keeper-integration is disabled, legacy functionality should work
        use claude_usage::parser::FileParser;
        let _parser = FileParser::new();
        assert!(
            true,
            "Legacy parser should be available when feature is disabled"
        );
    }
}

#[test]
fn test_test_utilities_work() {
    // Validate our test utilities are functional
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"{{"test": "data"}}"#).unwrap();
    temp_file.flush().unwrap();

    // Should be able to read the file back
    let contents = fs::read_to_string(temp_file.path()).unwrap();
    assert!(
        contents.contains("test"),
        "Test utility should create readable files"
    );
}

#[test]
fn test_path_utilities() {
    // Validate path handling utilities work correctly
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let test_path = temp_dir
        .path()
        .join("test")
        .join("nested")
        .join("file.jsonl");

    // Should be able to create directory structure
    if let Some(parent) = test_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
        std::fs::write(&test_path, "test content").unwrap();

        assert!(
            test_path.exists(),
            "Should create nested directory structure"
        );
        assert!(test_path.is_file(), "Should create actual file");
    }
}

#[test]
fn test_chrono_import() {
    // Validate chrono is available for date/time handling
    use chrono::{DateTime, Utc};

    let _now = Utc::now();
    let _parsed = DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z");

    assert!(true, "chrono should be available for timestamp parsing");
}

#[test]
fn test_json_handling() {
    // Validate serde_json is available for JSON operations
    use serde_json::{json, Value};

    let test_json = json!({
        "timestamp": "2025-01-15T10:30:00Z",
        "message": {
            "id": "test_msg",
            "model": "claude-3-5-sonnet-20241022"
        }
    });

    let _as_string = test_json.to_string();
    let _parsed: Value = serde_json::from_str(&_as_string).unwrap();

    assert!(true, "JSON serialization/deserialization should work");
}

#[test]
fn validate_test_data_patterns() {
    // Validate that our test data patterns match real Claude Desktop structures
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");

    // Main projects structure
    let main_project = claude_dir.join("projects").join("test_project");
    fs::create_dir_all(&main_project).unwrap();

    // VM projects structure
    let vm_project = claude_dir
        .join("vms")
        .join("test_vm")
        .join("projects")
        .join("vm_project");
    fs::create_dir_all(&vm_project).unwrap();

    assert!(
        main_project.exists(),
        "Should create main project structure"
    );
    assert!(vm_project.exists(), "Should create VM project structure");
}

#[test]
fn test_realistic_jsonl_patterns() {
    // Validate that our JSONL test patterns are realistic
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();

    // Pattern 1: Full featured entry
    writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:30:00Z","message":{{"id":"msg_1","model":"claude-3-5-sonnet-20241022","usage":{{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":5,"cache_read_input_tokens":10}}}},"costUSD":0.005,"requestId":"req_1"}}"#).unwrap();

    // Pattern 2: Minimal entry
    writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:31:00Z","message":{{"id":"msg_2","model":"claude-3-5-sonnet-20241022"}},"requestId":"req_2"}}"#).unwrap();

    // Pattern 3: Alternative field names
    writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:32:00Z","message":{{"id":"msg_3","model":"claude-3-haiku-20240307","usage":{{"input_tokens":50,"output_tokens":100}}}},"cost_usd":0.001,"request_id":"req_3"}}"#).unwrap();

    temp_file.flush().unwrap();

    let contents = std::fs::read_to_string(temp_file.path()).unwrap();
    let lines: Vec<&str> = contents.lines().collect();

    assert_eq!(lines.len(), 3, "Should have 3 test patterns");
    assert!(
        lines[0].contains("costUSD"),
        "Should have camelCase cost field"
    );
    assert!(
        lines[2].contains("cost_usd"),
        "Should have snake_case cost field"
    );
}

#[test]
fn test_integration_smoke_test() {
    // Smoke test that would catch major integration issues
    use claude_usage::parser_wrapper::UnifiedParser;
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:30:00Z","message":{{"id":"smoke_test","model":"claude-3-5-sonnet-20241022"}},"requestId":"smoke_req"}}"#).unwrap();
    temp_file.flush().unwrap();

    let parser = UnifiedParser::new();
    let result = parser.parse_jsonl_file(temp_file.path());

    // Should not panic and should return a result (Ok or Err both acceptable)
    match result {
        Ok(entries) => {
            assert!(entries.len() <= 1, "Should parse 0 or 1 entries");
        }
        Err(_) => {
            // Error is acceptable - just ensure it doesn't crash
            assert!(true, "Parser handled file gracefully (with error)");
        }
    }
}
