//! Parser that uses keeper-based parsing for schema resilience

use anyhow::Result;
use crate::models::UsageEntry;
use crate::keeper_integration::KeeperIntegration;
use std::path::Path;

/// Unified parser interface using keeper integration
pub struct UnifiedParser {
    keeper: KeeperIntegration,
}

impl Default for UnifiedParser {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedParser {
    pub fn new() -> Self {
        Self {
            keeper: KeeperIntegration::new(),
        }
    }
    
    pub fn parse_jsonl_file(&self, file_path: &Path) -> Result<Vec<UsageEntry>> {
        self.keeper.parse_jsonl_file(file_path)
    }
}