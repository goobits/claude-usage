//! Parquet summary reader
//!
//! This module provides efficient reading of claude-keeper parquet backup files
//! to extract summary information without loading full datasets into memory.

use anyhow::{Context, Result};
use chrono;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::{debug, info, warn};

use crate::live::BaselineSummary;

/// Reads summary information from parquet backup files
pub struct ParquetSummaryReader {
    backup_dir: PathBuf,
}

impl ParquetSummaryReader {
    /// Create a new parquet summary reader
    pub fn new(backup_dir: PathBuf) -> Result<Self> {
        if !backup_dir.exists() {
            return Err(anyhow::anyhow!(
                "Backup directory does not exist: {}",
                backup_dir.display()
            ));
        }

        Ok(Self { backup_dir })
    }

    /// Read summary data from parquet files
    pub fn read_summary(&self) -> Result<BaselineSummary> {
        info!(
            backup_dir = %self.backup_dir.display(),
            "Reading parquet backup summary"
        );

        // Find parquet files in the backup directory
        let parquet_files = self.find_parquet_files()?;
        
        if parquet_files.is_empty() {
            warn!(
                backup_dir = %self.backup_dir.display(),
                "No parquet files found in backup directory"
            );
            return Ok(BaselineSummary::default());
        }

        let total_files = parquet_files.len();
        debug!(file_count = total_files, "Found parquet backup files");

        // Get the most recent file modification time as last backup time
        let last_backup = parquet_files
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .filter_map(|metadata| metadata.modified().ok())
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Initialize aggregation variables
        let mut total_cost = 0.0;
        let mut total_tokens = 0u64;
        let mut sessions_today = 0u32;

        // Get today's date for session counting
        let today = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() / 86400; // Days since epoch

        // Process each parquet file
        for parquet_file in &parquet_files {
            debug!(file = %parquet_file.display(), "Processing parquet file");
            
            match self.read_parquet_file_stats(parquet_file) {
                Ok(stats) => {
                    total_cost += stats.total_cost;
                    total_tokens += stats.total_tokens;
                    
                    // Count sessions from today
                    for session_time in stats.session_times {
                        let session_day = session_time
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs() / 86400;
                        
                        if session_day == today {
                            sessions_today += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        file = %parquet_file.display(),
                        error = %e,
                        "Failed to read parquet file stats, skipping"
                    );
                }
            }
        }

        let summary = BaselineSummary {
            total_cost,
            total_tokens,
            sessions_today,
            last_backup,
        };

        info!(
            file_count = total_files,
            last_backup = ?last_backup,
            "Loaded baseline summary from parquet files"
        );

        Ok(summary)
    }

    /// Read statistics from a single parquet file using claude-keeper
    fn read_parquet_file_stats(&self, parquet_file: &PathBuf) -> Result<ParquetFileStats> {
        debug!(
            file = %parquet_file.display(),
            "Calling claude-keeper read for parquet file"
        );

        let output = Command::new("claude-keeper")
            .args(&[
                "read",
                parquet_file.to_str().unwrap_or(""),
                "--format",
                "json",
                "--stats"
            ])
            .output()
            .context("Failed to execute claude-keeper read command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "claude-keeper read failed with exit code {:?}: {}",
                output.status.code(),
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout)
            .context("Failed to parse claude-keeper JSON output")?;

        // Extract data from JSON response
        let total_cost = json.get("total_cost_usd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_input_tokens = json.get("total_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let total_output_tokens = json.get("total_output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let total_tokens = total_input_tokens + total_output_tokens;

        // Extract session times for today counting
        let mut session_times = Vec::new();
        if let Some(sessions) = json.get("sessions").and_then(|v| v.as_array()) {
            for session in sessions {
                if let Some(timestamp_str) = session.get("timestamp").and_then(|v| v.as_str()) {
                    // Try to parse ISO 8601 timestamp
                    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                        session_times.push(UNIX_EPOCH + Duration::from_secs(timestamp.timestamp() as u64));
                    }
                }
            }
        }

        Ok(ParquetFileStats {
            total_cost,
            total_tokens,
            session_times,
        })
    }

    /// Find all parquet files in the backup directory
    fn find_parquet_files(&self) -> Result<Vec<PathBuf>> {
        let mut parquet_files = Vec::new();

        for entry in fs::read_dir(&self.backup_dir)
            .with_context(|| format!("Failed to read backup directory: {}", self.backup_dir.display()))?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            if path.is_file() && 
               path.extension()
                   .and_then(|ext| ext.to_str())
                   .map(|ext| ext.eq_ignore_ascii_case("parquet"))
                   .unwrap_or(false)
            {
                parquet_files.push(path);
            }
        }

        // Sort files by name for consistent ordering
        parquet_files.sort();
        
        Ok(parquet_files)
    }

    /// Get statistics about the backup files
    #[allow(dead_code)]
    pub fn get_backup_stats(&self) -> Result<BackupStats> {
        let parquet_files = self.find_parquet_files()?;
        
        let mut total_size = 0;
        let mut latest_modified = SystemTime::UNIX_EPOCH;
        
        for file in &parquet_files {
            if let Ok(metadata) = fs::metadata(file) {
                total_size += metadata.len();
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_modified {
                        latest_modified = modified;
                    }
                }
            }
        }

        Ok(BackupStats {
            file_count: parquet_files.len(),
            total_size_bytes: total_size,
            latest_modified,
        })
    }
}

/// Statistics about backup files
#[allow(dead_code)]
pub struct BackupStats {
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub latest_modified: SystemTime,
}

/// Statistics extracted from a single parquet file
struct ParquetFileStats {
    total_cost: f64,
    total_tokens: u64,
    session_times: Vec<SystemTime>,
}