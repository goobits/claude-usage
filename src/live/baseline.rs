//! Baseline data loading from parquet files
//!
//! This module handles loading summary information from existing parquet backup
//! files created by claude-keeper. This provides the initial state for live mode.

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::config::get_config;
use crate::live::BaselineSummary;
use crate::parquet::reader::ParquetSummaryReader;

/// Load baseline summary from parquet backup files
pub fn load_baseline_summary() -> Result<BaselineSummary> {
    let config = get_config();
    
    // Get claude-keeper backup directory
    let backup_dir = config.paths.claude_home.join("backups");
    
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
        .context("Failed to execute claude-keeper backup")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            exit_code = output.status.code(),
            stderr = %stderr,
            "Claude-keeper backup command failed"
        );
        // Continue with existing baseline rather than failing
        return load_baseline_summary();
    }

    // Reload the baseline data
    load_baseline_summary()
}