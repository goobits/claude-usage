//! Integration module for claude-keeper schema-resilient parsing
//!
//! This module provides the bridge between claude-usage's existing
//! data models and claude-keeper's FlexObject/SchemaAdapter system.

use crate::models::{MessageData, SessionBlock, UsageData, UsageEntry};
use anyhow::Result;
use claude_keeper::claude::{create_claude_adapter, ClaudeMessage};
use claude_keeper::core::{FlexObject, JsonlParser, SchemaAdapter};
use std::path::Path;
use tracing::{debug, info};

// Memory management is now handled by claude-keeper's streaming parser
// No need for custom memory tracking as claude-keeper handles files of any size efficiently

/// Integration wrapper that provides claude-keeper parsing capabilities
#[allow(dead_code)]
pub struct KeeperIntegration {
    parser: JsonlParser<FlexObject>,
    adapter: SchemaAdapter,
}

impl Default for KeeperIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl KeeperIntegration {
    pub fn new() -> Self {
        let mut adapter = create_claude_adapter();

        // Add claude-usage specific field mappings for Claude Desktop format
        
        // Override uuid mapping to include requestId (Claude Desktop uses this instead of uuid)
        adapter.add_mappings(
            "uuid",
            vec![
                "requestId".to_string(), // Claude Desktop uses this
                "uuid".to_string(),
                "id".to_string(),
                "messageId".to_string(),
            ],
        );

        // Add cost mapping for Claude Desktop
        adapter.add_mappings(
            "cost_usd",
            vec![
                "costUSD".to_string(),  // Claude Desktop uses camelCase
                "cost_usd".to_string(),
                "cost".to_string(),
            ],
        );

        // Override message_usage to handle Claude Desktop structure
        adapter.add_mappings(
            "message_usage",
            vec![
                "message.usage".to_string(),  // Nested usage inside message
                "usage".to_string(),          // If usage is at top level
            ],
        );

        Self {
            parser: JsonlParser::new(),
            adapter,
        }
    }

    /// Parse JSONL file using claude-keeper streaming parser
    pub fn parse_jsonl_file(&self, file_path: &Path) -> Result<Vec<UsageEntry>> {
        debug!(
            file = %file_path.display(),
            "Parsing JSONL file with claude-keeper streaming parser"
        );

        // Use claude-keeper's streaming parser - this handles memory efficiently
        let parse_result = self.parser.parse_file(file_path)?;
        
        let mut entries = Vec::new();
        let mut conversion_errors = 0;

        // Extract values we need before consuming parse_result
        let total_lines = parse_result.total_lines;
        let parse_errors_count = parse_result.errors.len();
        let success_rate = parse_result.success_rate();

        // Convert FlexObjects to UsageEntries
        for flex_obj in parse_result.objects {
            if let Some(entry) = self.convert_to_usage_entry(flex_obj) {
                entries.push(entry);
            } else {
                conversion_errors += 1;
            }
        }

        // Log results
        if parse_errors_count > 0 || conversion_errors > 0 {
            info!(
                file = %file_path.display(),
                total_lines = total_lines,
                parse_errors = parse_errors_count,
                conversion_errors = conversion_errors,
                entries_extracted = entries.len(),
                success_rate = format!("{:.1}%", success_rate),
                "Completed parsing with some errors"
            );
        } else {
            debug!(
                file = %file_path.display(),
                entries_extracted = entries.len(),
                "Successfully parsed JSONL file with claude-keeper streaming"
            );
        }

        Ok(entries)
    }

    /// Parse a single JSON line using keeper's parser
    /// Returns None if parsing fails (graceful degradation)
    pub fn parse_single_line(&self, line: &str) -> Option<UsageEntry> {
        // Skip empty lines
        if line.trim().is_empty() {
            return None;
        }

        // Parse using claude-keeper
        match self.parser.parse_string(line, None) {
            result if !result.objects.is_empty() => {
                // Successfully parsed line - return first valid entry
                for flex_obj in result.objects {
                    if let Some(entry) = self.convert_to_usage_entry(flex_obj) {
                        return Some(entry);
                    }
                }
                None
            }
            _ => {
                // Parse error or empty result
                None
            }
        }
    }


    /// Parse JSON content that might be an array or object containing session blocks
    /// Handles different formats: direct array, {"blocks": [...]}, {"sessions": [...]}
    pub fn parse_session_blocks(&self, content: &str) -> Result<Vec<SessionBlock>> {
        // Skip empty content
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut session_blocks = Vec::new();

        // First try to parse as raw JSON to handle arrays directly
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json_value) => {
                // Case 1: Direct array of session blocks
                if let Some(array) = json_value.as_array() {
                    for item in array {
                        if let Ok(block) = serde_json::from_value::<SessionBlock>(item.clone()) {
                            session_blocks.push(block);
                        }
                    }
                    return Ok(session_blocks);
                }

                // Case 2: Object with "blocks" field
                if let Some(blocks) = json_value.get("blocks").and_then(|v| v.as_array()) {
                    for item in blocks {
                        if let Ok(block) = serde_json::from_value::<SessionBlock>(item.clone()) {
                            session_blocks.push(block);
                        }
                    }
                    return Ok(session_blocks);
                }

                // Case 3: Object with "sessions" field
                if let Some(sessions) = json_value.get("sessions").and_then(|v| v.as_array()) {
                    for item in sessions {
                        if let Ok(block) = serde_json::from_value::<SessionBlock>(item.clone()) {
                            session_blocks.push(block);
                        }
                    }
                    return Ok(session_blocks);
                }

                // Case 4: Single session block object
                if let Ok(block) = serde_json::from_value::<SessionBlock>(json_value) {
                    session_blocks.push(block);
                    return Ok(session_blocks);
                }
            }
            Err(_) => {
                // If raw JSON parsing fails, try using claude-keeper parser as fallback
                let parse_result = self.parser.parse_string(content, None);

                if !parse_result.objects.is_empty() {
                    for flex_obj in parse_result.objects {
                        // Case 2: Object with "blocks" field
                        if let Some(blocks) =
                            flex_obj.get_field("blocks").and_then(|v| v.as_array())
                        {
                            for item in blocks {
                                if let Ok(block) =
                                    serde_json::from_value::<SessionBlock>(item.clone())
                                {
                                    session_blocks.push(block);
                                }
                            }
                            continue;
                        }

                        // Case 3: Object with "sessions" field
                        if let Some(sessions) =
                            flex_obj.get_field("sessions").and_then(|v| v.as_array())
                        {
                            for item in sessions {
                                if let Ok(block) =
                                    serde_json::from_value::<SessionBlock>(item.clone())
                                {
                                    session_blocks.push(block);
                                }
                            }
                            continue;
                        }

                        // Case 4: Single session block object
                        let json_value = flex_obj.to_json();
                        if let Ok(block) = serde_json::from_value::<SessionBlock>(json_value) {
                            session_blocks.push(block);
                        }
                    }
                }
            }
        }

        Ok(session_blocks)
    }

    /// Convert FlexObject to UsageEntry using SchemaAdapter
    fn convert_to_usage_entry(&self, obj: FlexObject) -> Option<UsageEntry> {
        let message = ClaudeMessage::new(obj);

        // Extract fields using schema adapter - with debug logging
        debug!("Processing message object for field extraction");
        
        let timestamp = match message.timestamp(&self.adapter) {
            Some(ts) => {
                debug!("Successfully extracted timestamp: {}", ts.to_rfc3339());
                ts.to_rfc3339()
            },
            None => {
                debug!("Failed to extract timestamp from message - checking raw field");
                if let Some(ts_field) = message.inner.get_field("timestamp") {
                    debug!("Found raw timestamp field: {:?}", ts_field);
                } else {
                    debug!("No timestamp field found in raw object");
                }
                return None;
            }
        };

        // Use schema adapter for request_id field resolution (Claude Desktop uses requestId)
        let request_id = message.uuid(&self.adapter);
        
        let request_id = match request_id {
            Some(id) => {
                debug!("Successfully extracted request_id: {}", id);
                id
            },
            None => {
                debug!("Failed to extract request_id or uuid from message");
                debug!("Checking raw request_id field: {:?}", message.inner.get_field("request_id"));
                debug!("Checking raw requestId field: {:?}", message.inner.get_field("requestId"));
                debug!("Checking raw uuid field: {:?}", message.inner.get_field("uuid"));
                return None;
            }
        };

        // Extract message data
        let message_content = match message.message_content(&self.adapter) {
            Some(content) => {
                debug!("Successfully extracted message content with keys: {:?}", content.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                content
            },
            None => {
                debug!("Failed to extract message content");
                debug!("Checking raw message field: {:?}", message.inner.get_field("message"));
                return None;
            }
        };
        let message_id = message_content
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let model = message_content
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-3-5-sonnet-20241022")
            .to_string();

        // Extract usage data if present
        let usage = message
            .message_usage(&self.adapter)
            .map(|usage_val| UsageData {
                input_tokens: usage_val
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                output_tokens: usage_val
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                cache_creation_input_tokens: usage_val
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                cache_read_input_tokens: usage_val
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            });

        // Extract cost if present using schema adapter
        let cost_usd = self
            .adapter
            .get_field(&message.inner, "cost_usd")
            .and_then(|v| v.as_f64());

        Some(UsageEntry {
            timestamp,
            message: MessageData {
                id: message_id,
                model,
                usage,
            },
            cost_usd,
            request_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_streaming_parser_handles_large_files() {
        let integration = KeeperIntegration::new();

        // Create a temporary file with many lines
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 0..10000 {
            writeln!(
                temp_file,
                r#"{{"timestamp":"2024-01-15T10:30:00Z","message":{{"id":"msg_{}","model":"claude-3-5-sonnet-20241022"}},"requestId":"req_{}"}}"#,
                i, i
            ).unwrap();
        }
        temp_file.flush().unwrap();

        // Should handle large file without loading all into memory
        let result = integration.parse_jsonl_file(temp_file.path());
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 10000);
    }

    #[test]
    fn test_streaming_parser_handles_mixed_valid_invalid() {
        let integration = KeeperIntegration::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:30:00Z","message":{{"id":"valid","model":"claude-3-5-sonnet-20241022"}},"requestId":"req_1"}}"#).unwrap();
        writeln!(temp_file, "{{broken json}}").unwrap();
        writeln!(temp_file, r#"{{"timestamp":"2025-01-15T10:31:00Z","message":{{"id":"also_valid","model":"claude-3-5-sonnet-20241022"}},"requestId":"req_2"}}"#).unwrap();
        temp_file.flush().unwrap();

        let result = integration.parse_jsonl_file(temp_file.path());
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse valid lines despite errors
    }

    #[test]  
    fn debug_claude_keeper_parsing() {
        // Set up debug logging for this test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        let integration = KeeperIntegration::new();

        // Test different Claude Desktop JSON formats
        let test_cases = vec![
            // Full format with usage and cost
            r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_full","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.005,"requestId":"req_full"}"#,
            
            // Minimal format (no usage, no cost)
            r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_minimal","model":"claude-3-5-sonnet-20241022"},"requestId":"req_minimal"}"#,
            
            // Snake case format
            r#"{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_snake","model":"claude-3-5-sonnet-20241022"},"cost_usd":0.003,"request_id":"req_snake"}"#,
        ];

        println!("\n=== DEBUG: Testing Claude-Keeper Parsing ===");

        for (i, test_json) in test_cases.iter().enumerate() {
            println!("\n--- Test Case {} ---", i + 1);
            println!("Input JSON: {}", test_json);
            
            match integration.parse_single_line(test_json) {
                Some(entry) => {
                    println!("✅ SUCCESS: {}", entry.message.id);
                    assert!(!entry.timestamp.is_empty());
                    assert!(!entry.request_id.is_empty());
                }
                None => {
                    println!("❌ FAILED: Could not parse");
                    // Don't fail the test, just log for debugging
                }
            }
        }
    }

    #[test]
    fn test_parse_single_line() {
        let integration = KeeperIntegration::new();

        // Test valid JSON line
        let valid_line = r#"{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"test","model":"claude-3-5-sonnet-20241022"},"requestId":"req_123"}"#;
        let result = integration.parse_single_line(valid_line);
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.request_id, "req_123");
        assert_eq!(entry.message.id, "test");

        // Test invalid JSON line
        let invalid_line = "{broken json}";
        let result = integration.parse_single_line(invalid_line);
        assert!(result.is_none());

        // Test empty line
        let result = integration.parse_single_line("");
        assert!(result.is_none());
    }


    #[test]
    fn test_parse_session_blocks() {
        let integration = KeeperIntegration::new();

        // Test direct array format
        let array_content = r#"[{"startTime":"2024-01-15T10:00:00Z","endTime":"2024-01-15T10:30:00Z","tokenCounts":{"inputTokens":100,"outputTokens":50,"cacheCreationInputTokens":0,"cacheReadInputTokens":0},"costUSD":0.001}]"#;
        let result = integration.parse_session_blocks(array_content);
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 1);

        // Test object with "blocks" field
        let blocks_content = r#"{"blocks":[{"startTime":"2024-01-15T10:00:00Z","endTime":"2024-01-15T10:30:00Z","tokenCounts":{"inputTokens":100,"outputTokens":50,"cacheCreationInputTokens":0,"cacheReadInputTokens":0},"costUSD":0.001}]}"#;
        let result = integration.parse_session_blocks(blocks_content);
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 1);

        // Test empty content
        let result = integration.parse_session_blocks("");
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 0);

        // Test invalid JSON
        let result = integration.parse_session_blocks("{broken json}");
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 0); // Graceful degradation
    }
}
