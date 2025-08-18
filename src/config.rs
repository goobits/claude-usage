//! Production configuration system
//!
//! Provides centralized configuration management with:
//! - Environment variable support
//! - Config file loading (optional)
//! - Runtime defaults
//! - Validation and type safety

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::OnceLock;
use tracing::{info, warn};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Processing configuration
    pub processing: ProcessingConfig,
    
    /// Memory configuration
    pub memory: MemoryConfig,
    
    /// Deduplication configuration
    pub dedup: DedupConfig,
    
    /// Output configuration
    pub output: OutputConfig,
    
    /// Paths configuration
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub batch_size: usize,
    pub parallel_chunks: usize,
    pub max_retries: usize,
    pub progress_interval_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_memory_mb: usize,
    pub buffer_size_kb: usize,
    pub warning_threshold_pct: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupConfig {
    pub window_hours: i64,
    pub cleanup_threshold: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub json_pretty: bool,
    pub include_metadata: bool,
    pub timestamp_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub claude_home: PathBuf,
    pub vms_directory: PathBuf,
    pub log_directory: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logging: LoggingConfig {
                level: "ERROR".to_string(),
                format: "pretty".to_string(),
                output: "console".to_string(),
            },
            processing: ProcessingConfig {
                batch_size: 10,
                parallel_chunks: 4,
                max_retries: 3,
                progress_interval_mb: 10,
            },
            memory: MemoryConfig {
                max_memory_mb: 512,
                buffer_size_kb: 8,
                warning_threshold_pct: 90,
            },
            dedup: DedupConfig {
                window_hours: 24,
                cleanup_threshold: 10000,
                enabled: true,
            },
            output: OutputConfig {
                json_pretty: false,
                include_metadata: false,
                timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
            },
            paths: PathsConfig {
                claude_home: dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".claude"),
                vms_directory: dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".claude")
                    .join("vms"),
                log_directory: PathBuf::from("logs"),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment, file, and defaults
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        
        // Try to load from config file if it exists
        let config_paths = [
            PathBuf::from("claude-usage.toml"),
            PathBuf::from(".claude-usage.toml"),
            dirs::config_dir()
                .map(|d| d.join("claude-usage").join("config.toml"))
                .unwrap_or_default(),
        ];
        
        for path in &config_paths {
            if path.exists() {
                info!(config_file = %path.display(), "Loading configuration from file");
                config = Self::load_from_file(path)?;
                break;
            }
        }
        
        // Override with environment variables
        config.apply_env_overrides()?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from TOML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        Ok(config)
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        // Logging overrides
        if let Ok(val) = env::var("LOG_LEVEL") {
            self.logging.level = val;
        }
        if let Ok(val) = env::var("LOG_FORMAT") {
            self.logging.format = val;
        }
        if let Ok(val) = env::var("LOG_OUTPUT") {
            self.logging.output = val;
        }
        
        // Processing overrides
        if let Ok(val) = env::var("CLAUDE_USAGE_BATCH_SIZE") {
            self.processing.batch_size = val.parse()
                .context("Invalid CLAUDE_USAGE_BATCH_SIZE")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_PARALLEL_CHUNKS") {
            self.processing.parallel_chunks = val.parse()
                .context("Invalid CLAUDE_USAGE_PARALLEL_CHUNKS")?;
        }
        
        // Memory overrides
        if let Ok(val) = env::var("CLAUDE_USAGE_MAX_MEMORY_MB") {
            self.memory.max_memory_mb = val.parse()
                .context("Invalid CLAUDE_USAGE_MAX_MEMORY_MB")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_BUFFER_SIZE_KB") {
            self.memory.buffer_size_kb = val.parse()
                .context("Invalid CLAUDE_USAGE_BUFFER_SIZE_KB")?;
        }
        
        // Dedup overrides
        if let Ok(val) = env::var("CLAUDE_USAGE_DEDUP_WINDOW_HOURS") {
            self.dedup.window_hours = val.parse()
                .context("Invalid CLAUDE_USAGE_DEDUP_WINDOW_HOURS")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_DEDUP_ENABLED") {
            self.dedup.enabled = val.parse()
                .context("Invalid CLAUDE_USAGE_DEDUP_ENABLED")?;
        }
        
        // Path overrides
        if let Ok(val) = env::var("CLAUDE_HOME") {
            self.paths.claude_home = PathBuf::from(val);
        }
        if let Ok(val) = env::var("CLAUDE_VMS_DIR") {
            self.paths.vms_directory = PathBuf::from(val);
        }
        if let Ok(val) = env::var("CLAUDE_LOG_DIR") {
            self.paths.log_directory = PathBuf::from(val);
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate memory settings
        if self.memory.max_memory_mb < 64 {
            warn!(
                max_memory_mb = self.memory.max_memory_mb,
                "Memory limit is very low, may cause performance issues"
            );
        }
        
        if self.memory.buffer_size_kb < 1 || self.memory.buffer_size_kb > 1024 {
            return Err(anyhow::anyhow!(
                "Buffer size must be between 1KB and 1024KB, got {}KB",
                self.memory.buffer_size_kb
            ));
        }
        
        // Validate processing settings
        if self.processing.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }
        
        if self.processing.parallel_chunks == 0 {
            return Err(anyhow::anyhow!("Parallel chunks must be greater than 0"));
        }
        
        // Validate dedup settings
        if self.dedup.window_hours < 0 {
            return Err(anyhow::anyhow!("Dedup window hours cannot be negative"));
        }
        
        // Validate paths exist (create if needed)
        if !self.paths.log_directory.exists() {
            fs::create_dir_all(&self.paths.log_directory)
                .context("Failed to create log directory")?;
        }
        
        Ok(())
    }
    
    /// Save current configuration to file
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        info!(path = %path.display(), "Configuration saved to file");
        
        Ok(())
    }
}

/// Global configuration instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Get the global configuration instance
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        Config::load().expect("Failed to load configuration")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.logging.level, "ERROR");
        assert_eq!(config.processing.batch_size, 10);
        assert_eq!(config.memory.max_memory_mb, 512);
    }
    
    #[test]
    fn test_env_override() {
        env::set_var("CLAUDE_USAGE_BATCH_SIZE", "20");
        let mut config = Config::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.processing.batch_size, 20);
        env::remove_var("CLAUDE_USAGE_BATCH_SIZE");
    }
    
    #[test]
    fn test_validation() {
        let mut config = Config::default();
        config.processing.batch_size = 0;
        assert!(config.validate().is_err());
    }
}