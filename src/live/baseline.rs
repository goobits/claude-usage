//! Baseline data loading from parquet files
//!
//! This module handles loading summary information from existing parquet backup
//! files created by claude-keeper. This provides the initial state for live mode.

use anyhow::{Context, Result};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use crate::config::get_config;
use crate::live::BaselineSummary;
use crate::parquet::reader::ParquetSummaryReader;

/// Load baseline summary from parquet backup files
pub fn load_baseline_summary() -> Result<BaselineSummary> {
    let _config = get_config();
    
    // Get claude-keeper backup directory (uses ~/.claude-backup by default)
    let backup_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claude-backup");
    
    if !backup_dir.exists() {
        info!(
            backup_dir = %backup_dir.display(),
            "No backup directory found, using empty baseline"
        );
        return Ok(BaselineSummary::default());
    }

    debug!(
        backup_dir = %backup_dir.display(),
        "Loading baseline from parquet backups"
    );

    // Use the parquet reader to get summary data
    let reader = ParquetSummaryReader::new(backup_dir)?;
    let summary = reader.read_summary()?;

    info!(
        total_cost = summary.total_cost,
        total_tokens = summary.total_tokens,
        sessions_today = summary.sessions_today,
        "Loaded baseline summary from parquet files"
    );

    Ok(summary)
}

/// Trigger a backup via claude-keeper subprocess and reload baseline
pub async fn refresh_baseline() -> Result<BaselineSummary> {
    info!("Refreshing baseline data via claude-keeper backup");
    
    // Get standard Claude paths
    let claude_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claude");
    
    let backup_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claude-backup");
    
    // Execute claude-keeper backup command
    info!("Running claude-keeper backup from {} to {}", claude_dir.display(), backup_dir.display());
    
    let output = tokio::process::Command::new("claude-keeper")
        .args(&["backup", claude_dir.to_str().unwrap(), "--out", backup_dir.to_str().unwrap(), "--quiet"])
        .output()
        .await
        .context("Failed to execute claude-keeper backup")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("claude-keeper backup failed: {}", stderr);
        return Err(anyhow::anyhow!("Backup failed: {}", stderr));
    }
    
    info!("Successfully completed claude-keeper backup");
    println!("✅ Auto-backup completed successfully");
    
    // Reload the baseline data
    load_baseline_summary()
}

/// Check if baseline should be refreshed (missing or stale)
pub fn should_refresh_baseline() -> bool {
    let _config = get_config();
    let backup_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claude-backup");
    
    // If backup directory doesn't exist, we definitely need to refresh
    if !backup_dir.exists() {
        debug!("Backup directory doesn't exist, baseline refresh needed");
        return true;
    }
    
    // Check for recent parquet files (within last 5 minutes)
    let stale_threshold = Duration::from_secs(5 * 60); // 5 minutes
    let now = SystemTime::now();
    
    match std::fs::read_dir(&backup_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && 
                   path.extension()
                       .and_then(|ext| ext.to_str())
                       .map(|ext| ext.eq_ignore_ascii_case("parquet"))
                       .unwrap_or(false)
                {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age <= stale_threshold {
                                    debug!(
                                        file = %path.display(),
                                        age_secs = age.as_secs(),
                                        "Found recent parquet file, no refresh needed"
                                    );
                                    return false; // Found recent file, no refresh needed
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to read backup directory, assuming refresh needed");
            return true;
        }
    }
    
    debug!("No recent parquet files found, baseline refresh needed");
    true
}

/// Get enhanced analytics using claude-keeper's SQL query engine
pub async fn get_sql_analytics() -> Result<serde_json::Value> {
    info!("Running SQL analytics using claude-keeper query engine");
    
    let backup_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claude-backup");
    
    if !backup_dir.exists() {
        warn!("No backup directory found for SQL analytics");
        return Ok(serde_json::json!({
            "error": "No backup data available",
            "suggestion": "Run claude-keeper backup first"
        }));
    }
    
    // Run SQL queries using claude-keeper
    let queries = vec![
        ("message_type_distribution", 
         "SELECT message_type, COUNT(*) as count FROM conversations GROUP BY message_type"),
        ("daily_activity_last_7_days", 
         "SELECT DATE_TRUNC('day', timestamp) as date, COUNT(*) as messages FROM conversations WHERE timestamp > NOW() - INTERVAL '7 days' GROUP BY DATE_TRUNC('day', timestamp) ORDER BY date DESC"),
        ("programming_languages",
         "SELECT COUNT(CASE WHEN tool_usage LIKE '%rust%' THEN 1 END) as rust_mentions, COUNT(CASE WHEN tool_usage LIKE '%python%' THEN 1 END) as python_mentions, COUNT(CASE WHEN tool_usage LIKE '%sql%' THEN 1 END) as sql_mentions FROM conversations"),
        ("top_sessions",
         "SELECT session_id, COUNT(*) as messages, MIN(timestamp) as start_time, MAX(timestamp) as end_time FROM conversations GROUP BY session_id ORDER BY messages DESC LIMIT 5")
    ];
    
    let mut results = serde_json::Map::new();
    
    for (query_name, sql) in queries {
        debug!("Running SQL query: {}", query_name);
        
        let output = tokio::process::Command::new("claude-keeper")
            .args(&["query", sql])
            .current_dir(&backup_dir)
            .output()
            .await
            .context(format!("Failed to execute SQL query: {}", query_name))?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse the table output or JSON (claude-keeper returns table format by default)
            results.insert(query_name.to_string(), serde_json::Value::String(stdout.to_string()));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("SQL query {} failed: {}", query_name, stderr);
            results.insert(query_name.to_string(), serde_json::Value::String(format!("Error: {}", stderr)));
        }
    }
    
    Ok(serde_json::Value::Object(results))
}