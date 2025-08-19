//! Configuration system demonstration
//!
//! This example shows how the configuration system works:
//! 1. Loading defaults
//! 2. Environment variable overrides
//! 3. Configuration file loading
//! 4. Validation

use claude_usage::config::{get_config, Config};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Claude Usage Configuration Demo");
    println!("===============================\n");

    // 1. Show default configuration
    println!("1. Default Configuration:");
    let default_config = Config::default();
    println!("   Batch Size: {}", default_config.processing.batch_size);
    println!("   Memory Limit: {}MB", default_config.memory.max_memory_mb);
    println!("   Log Level: {}", default_config.logging.level);
    println!("   Dedup Enabled: {}\n", default_config.dedup.enabled);

    // 2. Show environment variable overrides
    println!("2. Environment Variable Override:");
    env::set_var("CLAUDE_USAGE_BATCH_SIZE", "25");
    env::set_var("LOG_LEVEL", "DEBUG");
    env::set_var("CLAUDE_USAGE_MAX_MEMORY_MB", "1024");

    let mut config_with_env = Config::default();
    config_with_env.apply_env_overrides()?;
    println!(
        "   Batch Size (from env): {}",
        config_with_env.processing.batch_size
    );
    println!(
        "   Memory Limit (from env): {}MB",
        config_with_env.memory.max_memory_mb
    );
    println!(
        "   Log Level (from env): {}\n",
        config_with_env.logging.level
    );

    // 3. Show configuration file example
    println!("3. Configuration File Format (TOML):");
    let sample_toml = toml::to_string_pretty(&default_config)?;
    println!("   First few lines of TOML config:");
    for line in sample_toml.lines().take(15) {
        println!("   {}", line);
    }
    println!("   ... (truncated)\n");

    // 4. Show validation
    println!("4. Configuration Validation:");
    let mut invalid_config = Config::default();
    invalid_config.processing.batch_size = 0; // Invalid

    match invalid_config.validate() {
        Ok(_) => println!("   ✓ Configuration is valid"),
        Err(e) => println!("   ✗ Configuration error: {}", e),
    }

    let valid_config = Config::default();
    match valid_config.validate() {
        Ok(_) => println!("   ✓ Default configuration is valid"),
        Err(e) => println!("   ✗ Validation error: {}", e),
    }

    println!("\n5. Runtime Configuration Access:");
    println!("   Use get_config() to access configuration throughout the application");
    println!(
        "   Example: get_config().processing.batch_size = {}",
        default_config.processing.batch_size
    );

    // Cleanup environment
    env::remove_var("CLAUDE_USAGE_BATCH_SIZE");
    env::remove_var("LOG_LEVEL");
    env::remove_var("CLAUDE_USAGE_MAX_MEMORY_MB");

    println!("\n✅ Configuration system demonstration complete!");

    Ok(())
}
