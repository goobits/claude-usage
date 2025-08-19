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

/// Trigger a backup via claude-keeper and reload baseline
#[allow(dead_code)]
pub async fn refresh_baseline() -> Result<BaselineSummary> {
    info!("Refreshing baseline data via claude-keeper backup");
    
    // Trigger claude-keeper backup
    let output = tokio::process::Command::new("claude-keeper")
        .args(&["backup", "--quiet"])
        .output()
        .await
        .context("Failed to execute claude-keeper backup. Make sure claude-keeper is installed and accessible.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            exit_code = output.status.code(),
            stderr = %stderr,
            "Claude-keeper backup command failed"
        );
        
        // Provide user-friendly error context
        if stderr.contains("command not found") || stderr.contains("not found") {
            return Err(anyhow::anyhow!(
                "claude-keeper not found. Please install claude-keeper first:\n\
                 Visit https://github.com/mufeedvh/claude-keeper for installation instructions"
            ));
        } else if stderr.contains("permission") {
            return Err(anyhow::anyhow!(
                "Permission denied running claude-keeper.\n\
                 Make sure claude-keeper is executable and you have proper permissions"
            ));
        }
        
        // Continue with existing baseline rather than failing
        println!("⚠️  Backup command failed, trying to load existing data...");
        return load_baseline_summary();
    }

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