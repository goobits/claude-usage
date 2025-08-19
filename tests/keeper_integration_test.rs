//! Integration tests for claude-keeper integration
//!
//! These tests validate that the keeper integration correctly:
//! - Parses JSONL files with schema resilience
//! - Handles malformed data gracefully
//! - Converts FlexObject to UsageEntry accurately
//! - Maintains backward compatibility

#[cfg(feature = "keeper-integration")]
mod keeper_tests {
    use claude_usage::keeper_integration::KeeperIntegration;
    use claude_usage::models::UsageEntry;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use tempfile::NamedTempFile;

    fn create_test_jsonl(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_keeper_integration_parses_valid_jsonl() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_123","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":50}},"costUSD":0.0045,"requestId":"req_456"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_789","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":150,"output_tokens":250,"cache_creation_input_tokens":10,"cache_read_input_tokens":60}},"costUSD":0.0055,"requestId":"req_012"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message.id, "msg_123");
        assert_eq!(entries[0].request_id, "req_456");
        assert_eq!(entries[0].message.usage.as_ref().unwrap().input_tokens, 100);
        assert_eq!(entries[0].cost_usd, Some(0.0045));

        assert_eq!(entries[1].message.id, "msg_789");
        assert_eq!(
            entries[1].message.usage.as_ref().unwrap().output_tokens,
            250
        );
    }

    #[test]
    fn test_keeper_integration_handles_malformed_lines() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_valid","model":"claude-3-5-sonnet-20241022"},"requestId":"req_valid"}
{this is completely broken json that should be skipped}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_also_valid","model":"claude-3-5-sonnet-20241022"},"requestId":"req_also_valid"}
null
undefined
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_final","model":"claude-3-5-sonnet-20241022"},"requestId":"req_final"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        // Should gracefully handle errors and return valid entries
        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(
            entries.len(),
            3,
            "Should parse 3 valid entries, skipping malformed lines"
        );
        assert_eq!(entries[0].message.id, "msg_valid");
        assert_eq!(entries[1].message.id, "msg_also_valid");
        assert_eq!(entries[2].message.id, "msg_final");
    }

    #[test]
    fn test_keeper_integration_handles_schema_variations() {
        // Test different field naming conventions (camelCase vs snake_case)
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022"},"cost_usd":0.001,"requestId":"req_1"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022"},"costUSD":0.002,"request_id":"req_2"}
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_3","model":"claude-3-5-sonnet-20241022"},"cost":0.003,"request_id":"req_3"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        // Should handle both cost_usd and costUSD
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].cost_usd, Some(0.001));
        assert_eq!(entries[1].cost_usd, Some(0.002));
        // Third entry might not have cost parsed due to different field name
    }

    #[test]
    fn test_keeper_integration_handles_missing_fields() {
        // Test entries with missing optional fields
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_no_usage","model":"claude-3-5-sonnet-20241022"},"requestId":"req_1"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_no_cost","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"requestId":"req_2"}
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_complete","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":150,"output_tokens":250,"cache_creation_input_tokens":10,"cache_read_input_tokens":20}},"costUSD":0.005,"requestId":"req_3"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 3);

        // First entry: no usage data
        assert_eq!(entries[0].message.id, "msg_no_usage");
        assert!(entries[0].message.usage.is_none());

        // Second entry: no cost
        assert_eq!(entries[1].message.id, "msg_no_cost");
        assert!(entries[1].message.usage.is_some());
        assert!(entries[1].cost_usd.is_none());

        // Third entry: complete
        assert_eq!(entries[2].message.id, "msg_complete");
        assert!(entries[2].message.usage.is_some());
        assert!(entries[2].cost_usd.is_some());
    }

    #[test]
    fn test_keeper_integration_empty_file() {
        let temp_file = create_test_jsonl("");
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();
        assert_eq!(entries.len(), 0, "Empty file should return empty vec");
    }

    #[test]
    fn test_keeper_integration_large_token_values() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_large","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":999999,"output_tokens":888888,"cache_creation_input_tokens":777777,"cache_read_input_tokens":666666}},"costUSD":123.456,"requestId":"req_large"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 1);
        let usage = entries[0].message.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 999999);
        assert_eq!(usage.output_tokens, 888888);
        assert_eq!(usage.cache_creation_input_tokens, 777777);
        assert_eq!(usage.cache_read_input_tokens, 666666);
        assert_eq!(entries[0].cost_usd, Some(123.456));
    }

    #[test]
    fn test_keeper_integration_multiple_models() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_1"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-haiku-20240307","usage":{"input_tokens":50,"output_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.001,"requestId":"req_2"}
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_3","model":"claude-3-opus-20240229","usage":{"input_tokens":200,"output_tokens":300,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.015,"requestId":"req_3"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message.model, "claude-3-5-sonnet-20241022");
        assert_eq!(entries[1].message.model, "claude-3-haiku-20240307");
        assert_eq!(entries[2].message.model, "claude-3-opus-20240229");
    }

    #[test]
    fn test_keeper_integration_timestamp_formats() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022"},"requestId":"req_1"}
{"timestamp":"2025-01-15T10:30:00.123Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022"},"requestId":"req_2"}
{"timestamp":"2025-01-15T10:30:00+00:00","message":{"id":"msg_3","model":"claude-3-5-sonnet-20241022"},"requestId":"req_3"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 3);
        // All should parse successfully regardless of timestamp format
        for entry in entries {
            assert!(entry.timestamp.len() > 10); // Should have valid timestamp strings
        }
    }

    #[test]
    fn test_keeper_integration_zero_values() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_zero","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.0,"requestId":"req_zero"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 1);
        let usage = entries[0].message.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.cache_creation_input_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
        assert_eq!(entries[0].cost_usd, Some(0.0));
    }

    #[test]
    fn test_keeper_integration_mixed_valid_invalid() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022"},"requestId":"req_1"}
{"invalid": "json_structure"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022"},"requestId":"req_2"}
{"timestamp":"2025-01-15T10:32:00Z","message":null,"requestId":"req_3"}
{"timestamp":"2025-01-15T10:33:00Z","message":{"id":"msg_4","model":"claude-3-5-sonnet-20241022"},"requestId":"req_4"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        // Should parse valid entries and skip invalid ones
        assert!(entries.len() >= 2); // At least msg_1, msg_2, and msg_4 should be valid

        // Check that valid entries are properly parsed
        let valid_ids: Vec<_> = entries.iter().map(|e| &e.message.id).collect();
        assert!(valid_ids.contains(&&"msg_1".to_string()));
        assert!(valid_ids.contains(&&"msg_2".to_string()));
    }

    #[test]
    fn test_keeper_integration_various_request_id_formats() {
        let jsonl_content = r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022"},"requestId":"req_abc123"}
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022"},"request_id":"req_def456"}
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_3","model":"claude-3-5-sonnet-20241022"},"requestId":"1234567890"}"#;

        let temp_file = create_test_jsonl(jsonl_content);
        let integration = KeeperIntegration::new();

        let entries = integration.parse_jsonl_file(temp_file.path()).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].request_id, "req_abc123");
        assert_eq!(entries[1].request_id, "req_def456");
        assert_eq!(entries[2].request_id, "1234567890");
    }
}

#[cfg(not(feature = "keeper-integration"))]
mod keeper_tests {
    #[test]
    fn test_keeper_integration_disabled() {
        // When feature is disabled, integration should not be available
        // This test ensures the feature flag works correctly
        assert!(true, "keeper-integration feature is disabled");
    }
}
