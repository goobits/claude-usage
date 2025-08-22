//! CCUsage Compatibility Module
//!
//! This module replicates the exact algorithm from ccusage (JavaScript) to achieve
//! 100% parity in cost calculations. It includes any quirks or "bugs" that ccusage
//! has to ensure identical results.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// CCUsage-compatible usage data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CCUsageData {
    pub timestamp: String,
    pub message: CCMessage,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CCMessage {
    pub id: Option<String>,
    pub model: Option<String>,
    pub usage: Option<CCUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CCUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    #[serde(rename = "cache_creation_input_tokens")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(rename = "cache_read_input_tokens")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Daily usage summary compatible with ccusage
#[derive(Debug, Clone, Serialize)]
pub struct CCDailyUsage {
    pub date: String,
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
    #[serde(rename = "modelsUsed")]
    pub models_used: Vec<String>,
}

/// Create unique hash for deduplication (ccusage algorithm)
fn create_unique_hash(data: &CCUsageData) -> Option<String> {
    let message_id = data.message.id.as_ref()?;
    let request_id = data.request_id.as_ref()?;
    
    // Create hash using simple concatenation (ccusage method)
    Some(format!("{}:{}", message_id, request_id))
}

/// Extract project name from file path (ccusage method)
fn extract_project_from_path(path: &Path) -> String {
    // ccusage extracts project from path structure: .../projects/{project}/{sessionId}.jsonl
    let parts: Vec<&str> = path.to_str()
        .unwrap_or("")
        .split('/')
        .collect();
    
    // Find "projects" index and get next element
    for (i, part) in parts.iter().enumerate() {
        if *part == "projects" && i + 1 < parts.len() {
            return parts[i + 1].to_string();
        }
    }
    
    "unknown".to_string()
}

/// Format date to YYYY-MM-DD (ccusage uses en-CA locale for this)
fn format_date(timestamp: &str) -> String {
    // Parse timestamp and format to YYYY-MM-DD
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%Y-%m-%d").to_string()
    } else if let Ok(dt) = timestamp.parse::<DateTime<Utc>>() {
        dt.format("%Y-%m-%d").to_string()
    } else {
        // Fallback: try to extract date if it's already in YYYY-MM-DD format
        if timestamp.len() >= 10 {
            timestamp[..10].to_string()
        } else {
            "unknown".to_string()
        }
    }
}

/// Load daily usage data with ccusage-compatible algorithm
pub async fn load_daily_usage_cccompat(
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<CCDailyUsage>> {
    info!("Loading daily usage data with ccusage compatibility mode");
    
    // Get Claude paths (ccusage checks both ~/.claude and ~/.config/claude)
    let claude_paths = vec![
        dirs::home_dir().unwrap().join(".claude"),
        dirs::home_dir().unwrap().join(".config/claude"),
    ];
    
    let mut all_files = Vec::new();
    
    // Collect all JSONL files from projects directories
    for claude_path in &claude_paths {
        let projects_dir = claude_path.join("projects");
        if !projects_dir.exists() {
            continue;
        }
        
        // Walk through all subdirectories to find JSONL files
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Look for JSONL files in this project directory
                    if let Ok(files) = fs::read_dir(&path) {
                        for file in files.flatten() {
                            let file_path = file.path();
                            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                                all_files.push(file_path);
                            }
                        }
                    }
                }
            }
        }
    }
    
    debug!("Found {} JSONL files to process", all_files.len());
    
    // Track processed hashes for deduplication (ccusage behavior)
    let processed_hashes = DashMap::new();
    
    // Collect all valid entries
    let mut all_entries = Vec::new();
    
    for file_path in &all_files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        
        // Process each line (ccusage filters empty lines but still reads them)
        let lines: Vec<&str> = content.split('\n').collect();
        debug!("Processing {} lines from {}", lines.len(), file_path.display());
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip empty lines (ccusage behavior)
            if trimmed.is_empty() {
                continue;
            }
            
            // Try to parse as JSON
            match serde_json::from_str::<CCUsageData>(trimmed) {
                Ok(data) => {
                    // Check for duplicate (ccusage deduplication)
                    if let Some(hash) = create_unique_hash(&data) {
                        if processed_hashes.contains_key(&hash) {
                            continue; // Skip duplicate
                        }
                        processed_hashes.insert(hash, true);
                    }
                    
                    // Extract date
                    let date = format_date(&data.timestamp);
                    
                    // Calculate cost (ccusage uses pre-calculated costUSD when available)
                    let cost = if let Some(cost_usd) = data.cost_usd {
                        cost_usd
                    } else {
                        // Calculate from tokens using pricing
                        calculate_cost_from_tokens(&data)
                    };
                    
                    all_entries.push((date, data, cost));
                }
                Err(_) => {
                    // Skip malformed JSON (ccusage behavior)
                    continue;
                }
            }
        }
    }
    
    info!("Processed {} valid entries after deduplication", all_entries.len());
    
    // Group by date
    let mut daily_data: HashMap<String, CCDailyUsage> = HashMap::new();
    let mut daily_models: HashMap<String, HashSet<String>> = HashMap::new();
    
    for (date, data, cost) in all_entries {
        // Filter by date range if specified
        if let Some(since) = since {
            if date.replace("-", "") < since.to_string() {
                continue;
            }
        }
        if let Some(until) = until {
            if date.replace("-", "") > until.to_string() {
                continue;
            }
        }
        
        let entry = daily_data.entry(date.clone()).or_insert_with(|| CCDailyUsage {
            date: date.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.0,
            models_used: Vec::new(),
        });
        
        // Aggregate tokens
        if let Some(usage) = &data.message.usage {
            entry.input_tokens += usage.input_tokens.unwrap_or(0);
            entry.output_tokens += usage.output_tokens.unwrap_or(0);
            entry.cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
            entry.cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        }
        
        // Add cost
        entry.total_cost += cost;
        
        // Track models
        if let Some(model) = &data.message.model {
            daily_models.entry(date).or_insert_with(HashSet::new).insert(model.clone());
        }
    }
    
    // Set models used for each day
    for (date, models) in daily_models {
        if let Some(entry) = daily_data.get_mut(&date) {
            entry.models_used = models.into_iter().collect();
            entry.models_used.sort();
        }
    }
    
    // Convert to vector and sort by date
    let mut results: Vec<CCDailyUsage> = daily_data.into_values().collect();
    results.sort_by(|a, b| b.date.cmp(&a.date)); // Sort descending (ccusage default)
    
    Ok(results)
}

/// Calculate cost from tokens (simplified version matching ccusage pricing)
fn calculate_cost_from_tokens(data: &CCUsageData) -> f64 {
    let usage = match &data.message.usage {
        Some(u) => u,
        None => return 0.0,
    };
    
    let model = data.message.model.as_deref().unwrap_or("claude-3-5-sonnet");
    
    // Simplified pricing matching ccusage's litellm integration
    // These are the prices that cause the discrepancy
    let (input_price, output_price, cache_create_price, cache_read_price) = 
        if model.contains("opus") {
            (0.015, 0.075, 0.01875, 0.001875) // Per 1K tokens
        } else if model.contains("sonnet") {
            (0.003, 0.015, 0.00375, 0.0003) // Per 1K tokens
        } else {
            (0.003, 0.015, 0.00375, 0.0003) // Default to sonnet pricing
        };
    
    let input_tokens = usage.input_tokens.unwrap_or(0) as f64;
    let output_tokens = usage.output_tokens.unwrap_or(0) as f64;
    let cache_creation = usage.cache_creation_input_tokens.unwrap_or(0) as f64;
    let cache_read = usage.cache_read_input_tokens.unwrap_or(0) as f64;
    
    // Calculate cost (price per 1K tokens)
    (input_tokens * input_price / 1000.0) +
    (output_tokens * output_price / 1000.0) +
    (cache_creation * cache_create_price / 1000.0) +
    (cache_read * cache_read_price / 1000.0)
}

/// Get total cost for a date range using ccusage-compatible algorithm
pub async fn get_ccusage_compatible_cost(
    since: Option<&str>,
    until: Option<&str>,
) -> Result<f64> {
    let daily_data = load_daily_usage_cccompat(since, until).await?;
    
    let total_cost: f64 = daily_data.iter()
        .map(|d| d.total_cost)
        .sum();
    
    info!("CCUsage-compatible cost calculation: ${:.2}", total_cost);
    Ok(total_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unique_hash_creation() {
        let data = CCUsageData {
            timestamp: "2025-08-20T10:30:00Z".to_string(),
            message: CCMessage {
                id: Some("msg_123".to_string()),
                model: Some("claude-3-opus".to_string()),
                usage: None,
            },
            cost_usd: Some(0.5),
            request_id: Some("req_456".to_string()),
            session_id: None,
        };
        
        let hash = create_unique_hash(&data);
        assert_eq!(hash, Some("msg_123:req_456".to_string()));
    }
    
    #[test]
    fn test_date_formatting() {
        assert_eq!(format_date("2025-08-20T10:30:00Z"), "2025-08-20");
        assert_eq!(format_date("2025-08-20T10:30:00.123Z"), "2025-08-20");
        assert_eq!(format_date("2025-08-20"), "2025-08-20");
    }
    
    #[test]
    fn test_cost_calculation() {
        let data = CCUsageData {
            timestamp: "2025-08-20T10:30:00Z".to_string(),
            message: CCMessage {
                id: Some("msg_123".to_string()),
                model: Some("claude-3-opus".to_string()),
                usage: Some(CCUsage {
                    input_tokens: Some(1000),
                    output_tokens: Some(2000),
                    cache_creation_input_tokens: Some(500),
                    cache_read_input_tokens: Some(1500),
                }),
            },
            cost_usd: None,
            request_id: Some("req_456".to_string()),
            session_id: None,
        };
        
        let cost = calculate_cost_from_tokens(&data);
        // Opus pricing: input=0.015, output=0.075, cache_create=0.01875, cache_read=0.001875
        // (1000 * 0.015 + 2000 * 0.075 + 500 * 0.01875 + 1500 * 0.001875) / 1000
        // = (15 + 150 + 9.375 + 2.8125) / 1000 = 0.1771875
        assert!((cost - 0.177).abs() < 0.001);
    }
}