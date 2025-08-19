use std::env;
use std::fs;
use tempfile::tempdir;

#[cfg(test)]
mod config_tests {
    use super::*;
    use claude_usage::config::Config;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();

        // Test logging defaults
        assert_eq!(config.logging.level, "WARN");
        assert_eq!(config.logging.format, "pretty");
        assert_eq!(config.logging.output, "console");

        // Test processing defaults
        assert_eq!(config.processing.batch_size, 10);
        assert_eq!(config.processing.parallel_chunks, 4);
        assert_eq!(config.processing.max_retries, 3);
        assert_eq!(config.processing.progress_interval_mb, 10);

        // Test memory defaults
        assert_eq!(config.memory.max_memory_mb, 512);
        assert_eq!(config.memory.buffer_size_kb, 8);
        assert_eq!(config.memory.warning_threshold_pct, 90);

        // Test dedup defaults
        assert_eq!(config.dedup.window_hours, 24);
        assert_eq!(config.dedup.cleanup_threshold, 10000);
        assert_eq!(config.dedup.enabled, true);

        // Test output defaults
        assert_eq!(config.output.json_pretty, false);
        assert_eq!(config.output.include_metadata, false);
        assert_eq!(config.output.timestamp_format, "%Y-%m-%d %H:%M:%S");
    }

    #[test]
    fn test_env_variable_override() {
        // Set environment variables
        env::set_var("CLAUDE_USAGE_BATCH_SIZE", "20");
        env::set_var("CLAUDE_USAGE_MAX_MEMORY_MB", "1024");
        env::set_var("LOG_LEVEL", "DEBUG");
        env::set_var("CLAUDE_USAGE_DEDUP_ENABLED", "false");

        let mut config = Config::default();
        config
            .apply_env_overrides()
            .expect("Failed to apply env overrides");

        assert_eq!(config.processing.batch_size, 20);
        assert_eq!(config.memory.max_memory_mb, 1024);
        assert_eq!(config.logging.level, "DEBUG");
        assert_eq!(config.dedup.enabled, false);

        // Cleanup
        env::remove_var("CLAUDE_USAGE_BATCH_SIZE");
        env::remove_var("CLAUDE_USAGE_MAX_MEMORY_MB");
        env::remove_var("LOG_LEVEL");
        env::remove_var("CLAUDE_USAGE_DEDUP_ENABLED");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Test valid config
        assert!(config.validate().is_ok());

        // Test invalid batch size
        config.processing.batch_size = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid parallel chunks
        config = Config::default();
        config.processing.parallel_chunks = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid buffer size
        config = Config::default();
        config.memory.buffer_size_kb = 0;
        assert!(config.validate().is_err());

        config.memory.buffer_size_kb = 2000; // Too large
        assert!(config.validate().is_err());

        // Reset and test invalid dedup window
        config = Config::default();
        config.dedup.window_hours = -1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_file_loading() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test-config.toml");

        // Create test config file
        let test_config = r#"
[logging]
level = "DEBUG"
format = "json"
output = "file"

[processing]
batch_size = 5
parallel_chunks = 2
max_retries = 5
progress_interval_mb = 20

[memory]
max_memory_mb = 256
buffer_size_kb = 4
warning_threshold_pct = 80

[dedup]
window_hours = 48
cleanup_threshold = 5000
enabled = false

[output]
json_pretty = true
include_metadata = true
timestamp_format = "%Y/%m/%d %H:%M"

[paths]
claude_home = "/custom/claude"
vms_directory = "/custom/vms"
log_directory = "/custom/logs"

[live]
startup_timeout_secs = 60
max_restart_attempts = 5
update_channel_buffer = 200
claude_keeper_path = "/custom/claude-keeper"
        "#;

        fs::write(&config_path, test_config).expect("Failed to write test config");

        // Load config from file
        let config = Config::load_from_file(&config_path).expect("Failed to load config");

        // Verify loaded values
        assert_eq!(config.logging.level, "DEBUG");
        assert_eq!(config.logging.format, "json");
        assert_eq!(config.processing.batch_size, 5);
        assert_eq!(config.memory.max_memory_mb, 256);
        assert_eq!(config.dedup.window_hours, 48);
        assert_eq!(config.dedup.enabled, false);
        assert_eq!(config.output.json_pretty, true);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();

        // Test TOML serialization
        let toml_string = toml::to_string_pretty(&config).expect("Failed to serialize to TOML");
        assert!(toml_string.contains("[logging]"));
        assert!(toml_string.contains("[processing]"));
        assert!(toml_string.contains("[memory]"));
        assert!(toml_string.contains("[dedup]"));
        assert!(toml_string.contains("[output]"));
        assert!(toml_string.contains("[paths]"));

        // Test round-trip
        let deserialized: Config =
            toml::from_str(&toml_string).expect("Failed to deserialize TOML");
        assert_eq!(config.logging.level, deserialized.logging.level);
        assert_eq!(
            config.processing.batch_size,
            deserialized.processing.batch_size
        );
        assert_eq!(
            config.memory.max_memory_mb,
            deserialized.memory.max_memory_mb
        );
    }
}
