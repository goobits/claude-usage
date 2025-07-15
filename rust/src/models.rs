use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
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
    pub usage: Option<UsageData>,  // Make usage optional to match Python behavior
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

#[derive(Debug, Clone)]
pub struct CurrentSessionData {
    pub session_id: String,
    pub total_cost: f64,
    pub total_tokens: u32,
    pub start_time: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub file_modified: DateTime<Utc>,
    pub tokens_by_minute: HashMap<String, u32>,
    pub cost_by_minute: HashMap<String, f64>,
    pub models_used: HashSet<String>,
    pub real_burn_rate_tokens: f64,
    pub real_burn_rate_cost: f64,
}

#[derive(Debug, Clone)]
pub enum CostMode {
    Auto,
    Calculate,
    Display,
}

impl From<&str> for CostMode {
    fn from(s: &str) -> Self {
        match s {
            "calculate" => CostMode::Calculate,
            "display" => CostMode::Display,
            _ => CostMode::Auto,
        }
    }
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
            last_activity: data.last_activity.unwrap_or_else(|| "1970-01-01".to_string()),
            models_used: {
                let mut models: Vec<String> = data.models_used.into_iter().collect();
                models.sort();
                models
            },
        }
    }
}

impl TokenCounts {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}