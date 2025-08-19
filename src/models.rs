//! Core Data Models
//!
//! This module defines the primary data structures used throughout the Claude usage analysis
//! system. These models represent the complete data pipeline from raw usage entries to
//! aggregated reports.
//!
//! ## Data Flow
//!
//! The data flows through these models in the following sequence:
//!
//! 1. **Raw Data**: [`UsageEntry`] - Individual entries parsed from JSONL files
//! 2. **Aggregation**: [`SessionData`] - Usage data grouped by session and project
//! 3. **Output**: [`SessionOutput`] - Serializable format for reports and JSON output
//! 4. **Reports**: [`DailyData`], [`MonthlyData`] - Time-based aggregated views
//!
//! ## Core Types
//!
//! ### Usage Entry Structure
//! - [`UsageEntry`] - Top-level wrapper for a single usage record
//! - [`MessageData`] - Information about the specific Claude message/interaction
//! - [`UsageData`] - Token consumption details (input, output, cache operations)
//!
//! ### Session Management
//! - [`SessionData`] - Internal session tracking with daily breakdowns
//! - [`SessionOutput`] - External-facing session summary for reports
//! - [`DailyUsage`] - Per-day usage summary within a session
//!
//! ### Report Generation
//! - [`DailyData`] - Daily usage report with project breakdown
//! - [`DailyProject`] - Project-specific usage within a day
//! - [`MonthlyData`] - Monthly usage summary
//!
//! ### Session Blocks
//! - [`SessionBlock`] - Time-bounded session information from Claude's session tracking
//! - [`TokenCounts`] - Structured token usage counts for session blocks
//!
//! ### Pricing Data
//! - [`PricingData`] - Cost per token for different token types and models
//!
//! ## Features
//!
//! - **Serde Integration**: All public types support serialization/deserialization
//! - **Optional Fields**: Handles missing data gracefully (e.g., cost information)
//! - **Token Calculation**: Automatic total token computation
//! - **Type Safety**: Strong typing prevents common data manipulation errors

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub timestamp: String,
    pub message: MessageData,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub id: String,
    pub model: String,
    pub usage: Option<UsageData>, // Make usage optional to match Python behavior
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    #[serde(rename = "input_tokens")]
    pub input_tokens: u32,
    #[serde(rename = "output_tokens")]
    pub output_tokens: u32,
    #[serde(rename = "cache_creation_input_tokens")]
    pub cache_creation_input_tokens: u32,
    #[serde(rename = "cache_read_input_tokens")]
    pub cache_read_input_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct DailyUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub project_path: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub total_cost: f64,
    pub last_activity: Option<String>,
    pub models_used: HashSet<String>,
    pub daily_usage: HashMap<String, DailyUsage>, // Track usage per day
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionOutput {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "inputTokens")]
    pub input_tokens: u32,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u32,
    #[serde(rename = "cacheCreationTokens")]
    pub cache_creation_tokens: u32,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: u32,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    #[serde(rename = "modelsUsed")]
    pub models_used: Vec<String>,
    #[serde(skip)]
    pub daily_usage: HashMap<String, DailyUsage>, // Daily breakdown for internal use
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyProject {
    pub project: String,
    pub sessions: u32,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyData {
    pub date: String,
    pub projects: Vec<DailyProject>,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "totalSessions")]
    pub total_sessions: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthlyData {
    pub month: String,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "totalSessions")]
    pub total_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBlock {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    #[serde(rename = "tokenCounts")]
    pub token_counts: TokenCounts,
    #[serde(rename = "costUSD")]
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    #[serde(rename = "inputTokens")]
    pub input_tokens: u32,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u32,
    #[serde(rename = "cacheCreationInputTokens")]
    pub cache_creation_input_tokens: u32,
    #[serde(rename = "cacheReadInputTokens")]
    pub cache_read_input_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    pub input_cost_per_token: Option<f64>,
    pub output_cost_per_token: Option<f64>,
    pub cache_creation_input_token_cost: Option<f64>,
    pub cache_read_input_token_cost: Option<f64>,
}

impl SessionData {
    pub fn new(session_id: String, project_path: String) -> Self {
        Self {
            session_id,
            project_path,
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.0,
            last_activity: None,
            models_used: HashSet::new(),
            daily_usage: HashMap::new(),
        }
    }

    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
}

impl From<SessionData> for SessionOutput {
    fn from(data: SessionData) -> Self {
        Self {
            session_id: data.session_id,
            project_path: data.project_path,
            input_tokens: data.input_tokens,
            output_tokens: data.output_tokens,
            cache_creation_tokens: data.cache_creation_tokens,
            cache_read_tokens: data.cache_read_tokens,
            total_cost: data.total_cost,
            last_activity: data
                .last_activity
                .unwrap_or_else(|| "1970-01-01".to_string()),
            models_used: {
                let mut models: Vec<String> = data.models_used.into_iter().collect();
                models.sort();
                models
            },
            daily_usage: data.daily_usage,
        }
    }
}

impl TokenCounts {
    pub fn total(&self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }
}
