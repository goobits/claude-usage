//! Parquet summary reader
//!
//! This module provides efficient reading of claude-keeper parquet backup files
//! to extract summary information without loading full datasets into memory.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
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

        // For now, we'll implement a basic stub that returns mock data
        // In a full implementation, this would use a parquet library like arrow-rs
        // to efficiently read and aggregate the data
        
        let total_files = parquet_files.len();
        debug!(file_count = total_files, "Found parquet backup files");

        // Get the most recent file modification time as last backup time
        let last_backup = parquet_files
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .filter_map(|metadata| metadata.modified().ok())
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // For now, return a basic summary based on file count
        // In a real implementation, we would parse the parquet files to get actual usage data
        let summary = BaselineSummary {
            total_cost: 0.0,  // Would be calculated from parquet data
            total_tokens: 0,   // Would be calculated from parquet data
            sessions_today: 0, // Would be calculated from parquet data
            last_backup,
        };

        info!(
            file_count = total_files,
            last_backup = ?last_backup,
            "Loaded baseline summary from parquet files"
        );

        Ok(summary)
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