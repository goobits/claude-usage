use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::error;

mod analyzer;
mod commands;
mod config;
mod dedup;
mod display;
mod file_discovery;
mod keeper_integration;
mod live;
mod logging;
mod memory;
mod models;
mod parquet;
mod parser;
mod parser_wrapper;
mod pricing;
mod session_utils;
mod timestamp_parser;

use analyzer::ClaudeUsageAnalyzer;
use config::get_config;
use dedup::ProcessOptions;

#[derive(Parser)]
#[command(name = "claude-usage")]
#[command(about = "Fast Rust implementation for Claude usage analysis across multiple VMs")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show daily usage with project breakdown
    Daily {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show last N entries
        #[arg(long)]
        limit: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Exclude VMs directory from analysis
        #[arg(long)]
        exclude_vms: bool,
    },
    /// Show monthly usage aggregation
    Monthly {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show last N entries
        #[arg(long)]
        limit: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Exclude VMs directory from analysis
        #[arg(long)]
        exclude_vms: bool,
    },
    /// Real-time usage monitoring via claude-keeper integration
    Live {
        /// Skip loading baseline data from parquet backups
        #[arg(long)]
        no_baseline: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first (this also validates it)
    get_config();

    // Initialize logging with config
    logging::init_logging();

    // Initialize memory monitoring with config
    memory::init_memory_limit();

    let cli = Cli::parse();

    // Handle command with its specific options
    match cli.command.unwrap_or(Commands::Daily {
        json: false,
        limit: None,
        since: None,
        until: None,
        exclude_vms: false,
    }) {
        Commands::Daily {
            json,
            limit,
            since,
            until,
            exclude_vms,
        } => {
            let (_since_date, _until_date, mut analyzer, options) =
                parse_common_args(json, limit, since, until, "daily", exclude_vms)?;

            match analyzer.run_command("daily", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Monthly {
            json,
            limit,
            since,
            until,
            exclude_vms,
        } => {
            let (_since_date, _until_date, mut analyzer, options) =
                parse_common_args(json, limit, since, until, "monthly", exclude_vms)?;

            match analyzer.run_command("monthly", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Live { no_baseline } => {
            match commands::live::run_live_mode(no_baseline).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!(error = %e, "Live mode failed");
                    eprintln!("Error: {}", e);
                    Err(e)
                }
            }
        }
    }
}

fn parse_common_args(
    json: bool,
    limit: Option<usize>,
    since: Option<String>,
    until: Option<String>,
    command: &str,
    exclude_vms: bool,
) -> Result<(
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    ClaudeUsageAnalyzer,
    ProcessOptions,
)> {
    // Parse date filters
    let since_date = if let Some(since_str) = since {
        match chrono::NaiveDate::parse_from_str(&since_str, "%Y-%m-%d") {
            Ok(date) => Some(
                date.and_hms_opt(0, 0, 0)
                    .context("Failed to create time from date")?
                    .and_utc(),
            ),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Invalid since date format: {}. Use YYYY-MM-DD",
                    since_str
                ));
            }
        }
    } else {
        None
    };

    let until_date = if let Some(until_str) = until {
        match chrono::NaiveDate::parse_from_str(&until_str, "%Y-%m-%d") {
            Ok(date) => Some(
                date.and_hms_opt(23, 59, 59)
                    .context("Failed to create time from date")?
                    .and_utc(),
            ),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Invalid until date format: {}. Use YYYY-MM-DD",
                    until_str
                ));
            }
        }
    } else {
        None
    };

    // Create analyzer
    let analyzer = ClaudeUsageAnalyzer::new();

    // Build options
    let options = ProcessOptions {
        command: command.to_string(),
        json_output: json,
        limit,
        since_date,
        until_date,
        snapshot: false,
        exclude_vms,
    };

    Ok((since_date, until_date, analyzer, options))
}

fn handle_error(e: anyhow::Error, json: bool) -> Result<(), anyhow::Error> {
    if json {
        error!(error = %e, "Command failed");
        println!("{{\"error\": \"{}\"}}", e);
    } else {
        error!(error = %e, "Command failed");
        eprintln!("Error: {}", e);
    }
    Err(e)
}
