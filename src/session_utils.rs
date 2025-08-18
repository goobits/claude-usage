use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::models::*;
use crate::keeper_integration::KeeperIntegration;

/// Handles session-related utilities including session ID extraction and session blocks parsing
pub struct SessionUtils;

impl SessionUtils {
    /// Extract session information from a session directory name
    /// Returns (session_id, project_name)
    pub fn extract_session_info(session_dir_name: &str) -> (String, String) {
        let session_id = session_dir_name.to_string();
        
        let project_name = if session_dir_name.starts_with('-') {
            // Remove only the leading dash, keep the full path
            session_dir_name[1..].to_string()
        } else {
            session_dir_name.to_string()
        };
        
        (session_id, project_name)
    }

    /// Create a unique hash for deduplication from a usage entry
    /// Uses messageId:requestId format
    pub fn create_unique_hash(entry: &UsageEntry) -> Option<String> {
        let message_id = &entry.message.id;
        let request_id = &entry.request_id;
        
        if message_id.is_empty() || request_id.is_empty() {
            return None;
        }
        
        Some(format!("{}:{}", message_id, request_id))
    }

    /// Parse a session blocks file and return the session blocks
    pub fn parse_session_blocks_file(file_path: &Path, keeper: &KeeperIntegration) -> Result<Vec<SessionBlock>> {
        
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        // For JSON files, we need to accumulate lines
        let mut content = String::new();
        for line in reader.lines() {
            content.push_str(&line?);
            content.push('\n');
        }
        
        keeper.parse_session_blocks(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_info_with_dash() {
        let (session_id, project_name) = SessionUtils::extract_session_info("-some-project-path");
        assert_eq!(session_id, "-some-project-path");
        assert_eq!(project_name, "some-project-path");
    }

    #[test]
    fn test_extract_session_info_without_dash() {
        let (session_id, project_name) = SessionUtils::extract_session_info("uuid-session-id");
        assert_eq!(session_id, "uuid-session-id");
        assert_eq!(project_name, "uuid-session-id");
    }

    #[test]
    fn test_create_unique_hash() {
        let entry = UsageEntry {
            message: MessageData {
                id: "msg123".to_string(),
                usage: None,
                model: "claude-3".to_string(),
            },
            request_id: "req456".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            cost_usd: None,
        };
        
        let hash = SessionUtils::create_unique_hash(&entry);
        assert_eq!(hash, Some("msg123:req456".to_string()));
    }

    #[test]
    fn test_create_unique_hash_empty_ids() {
        let entry = UsageEntry {
            message: MessageData {
                id: "".to_string(),
                usage: None,
                model: "claude-3".to_string(),
            },
            request_id: "req456".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            cost_usd: None,
        };
        
        let hash = SessionUtils::create_unique_hash(&entry);
        assert_eq!(hash, None);
    }
}