//! Production configuration system
//!
//! Provides centralized configuration management with:
//! - Environment variable support
//! - Config file loading (optional)
//! - Runtime defaults
//! - Validation and type safety

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::sync::OnceLock;
#[cfg(test)]
use std::sync::Mutex;
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

    /// Live mode configuration
    pub live: LiveConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveConfig {
    pub startup_timeout_secs: u64,
    pub max_restart_attempts: u32,
    pub update_channel_buffer: usize,
    pub claude_keeper_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logging: LoggingConfig {
                level: "WARN".to_string(),
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
            live: LiveConfig {
                startup_timeout_secs: 30,
                max_restart_attempts: 3,
                update_channel_buffer: 100,
                claude_keeper_path: "claude-keeper".to_string(),
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

    /// Expand ~ in path strings
    fn expand_path(path_str: &str) -> PathBuf {
        if path_str.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                if path_str == "~" {
                    return home;
                } else if path_str.starts_with("~/") {
                    return home.join(&path_str[2..]);
                }
            }
        }
        PathBuf::from(path_str)
    }

    /// Load configuration from TOML file
    #[cfg(feature = "basic")]
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        // Expand ~ in path strings
        config.expand_paths();

        Ok(config)
    }
    
    #[cfg(not(feature = "basic"))]
    pub fn load_from_file(_path: &Path) -> Result<Self> {
        // Return default config when TOML support is not compiled in
        Ok(Self::default())
    }

    /// Expand ~ in all path fields
    fn expand_paths(&mut self) {
        // Convert paths to strings, expand, then back to PathBuf
        if let Some(claude_home_str) = self.paths.claude_home.to_str() {
            self.paths.claude_home = Self::expand_path(claude_home_str);
        }
        if let Some(vms_dir_str) = self.paths.vms_directory.to_str() {
            self.paths.vms_directory = Self::expand_path(vms_dir_str);
        }
        if let Some(log_dir_str) = self.paths.log_directory.to_str() {
            self.paths.log_directory = Self::expand_path(log_dir_str);
        }
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
            self.processing.batch_size = val.parse().context("Invalid CLAUDE_USAGE_BATCH_SIZE")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_PARALLEL_CHUNKS") {
            self.processing.parallel_chunks = val
                .parse()
                .context("Invalid CLAUDE_USAGE_PARALLEL_CHUNKS")?;
        }

        // Memory overrides
        if let Ok(val) = env::var("CLAUDE_USAGE_MAX_MEMORY_MB") {
            self.memory.max_memory_mb =
                val.parse().context("Invalid CLAUDE_USAGE_MAX_MEMORY_MB")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_BUFFER_SIZE_KB") {
            self.memory.buffer_size_kb =
                val.parse().context("Invalid CLAUDE_USAGE_BUFFER_SIZE_KB")?;
        }

        // Dedup overrides
        if let Ok(val) = env::var("CLAUDE_USAGE_DEDUP_WINDOW_HOURS") {
            self.dedup.window_hours = val
                .parse()
                .context("Invalid CLAUDE_USAGE_DEDUP_WINDOW_HOURS")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_DEDUP_ENABLED") {
            self.dedup.enabled = val.parse().context("Invalid CLAUDE_USAGE_DEDUP_ENABLED")?;
        }

        // Path overrides (with ~ expansion)
        if let Ok(val) = env::var("CLAUDE_HOME") {
            self.paths.claude_home = Self::expand_path(&val);
        }
        if let Ok(val) = env::var("CLAUDE_VMS_DIR") {
            self.paths.vms_directory = Self::expand_path(&val);
        }
        if let Ok(val) = env::var("CLAUDE_LOG_DIR") {
            self.paths.log_directory = Self::expand_path(&val);
        }

        // Live mode overrides
        if let Ok(val) = env::var("CLAUDE_KEEPER_PATH") {
            self.live.claude_keeper_path = val;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_LIVE_TIMEOUT") {
            self.live.startup_timeout_secs = val
                .parse()
                .context("Invalid CLAUDE_USAGE_LIVE_TIMEOUT")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_LIVE_MAX_RESTARTS") {
            self.live.max_restart_attempts = val
                .parse()
                .context("Invalid CLAUDE_USAGE_LIVE_MAX_RESTARTS")?;
        }
        if let Ok(val) = env::var("CLAUDE_USAGE_LIVE_BUFFER_SIZE") {
            self.live.update_channel_buffer = val
                .parse()
                .context("Invalid CLAUDE_USAGE_LIVE_BUFFER_SIZE")?;
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
    #[cfg(feature = "basic")]
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        info!(path = %path.display(), "Configuration saved to file");

        Ok(())
    }
    
    #[allow(dead_code)]
    #[cfg(not(feature = "basic"))]
    pub fn save_to_file(&self, _path: &Path) -> Result<()> {
        anyhow::bail!("TOML configuration saving not available. Rebuild with --features basic")
    }
}

/// Global configuration instance
#[cfg(not(test))]
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Global configuration instance for tests (mutable)
#[cfg(test)]
static CONFIG: Mutex<Option<&'static Config>> = Mutex::new(None);

/// Get the global configuration instance
#[cfg(not(test))]
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| Config::load().expect("Failed to load configuration"))
}

/// Get the global configuration instance for tests
#[cfg(test)]
pub fn get_config() -> &'static Config {
    let mut guard = CONFIG.lock().unwrap();
    if let Some(config) = *guard {
        config
    } else {
        // Load configuration and leak it to get a static reference
        let config = Config::load().expect("Failed to load configuration");
        let config_ref: &'static Config = Box::leak(Box::new(config));
        *guard = Some(config_ref);
        config_ref
    }
}

/// Reset the global configuration for testing
#[cfg(test)]
pub fn reset_config_for_test() {
    let mut guard = CONFIG.lock().unwrap();
    // Note: This intentionally leaks memory in tests for simplicity
    // The leaked config will be cleaned up when the test process exits
    *guard = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.logging.level, "WARN");
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

    #[test]
    fn test_config_reset_functionality() {
        // Test that reset_config_for_test works correctly
        reset_config_for_test();
        
        // Get config should work after reset
        let config = get_config();
        assert_eq!(config.logging.level, "WARN");
        
        // Reset again to ensure it's safe to call multiple times
        reset_config_for_test();
        
        let config2 = get_config();
        assert_eq!(config2.logging.level, "WARN");
        
        // Test that the function is thread-safe (no undefined behavior)
        // This test mainly ensures the code compiles and runs without panicking
    }
}
