use clap::{Parser, Subcommand};
use chrono;
use anyhow::Result;
use std::process;

mod models;
mod parser;
mod dedup;
mod analyzer;
mod display;
mod monitor;
mod pricing;
mod utils;

use analyzer::ClaudeUsageAnalyzer;
use models::CostMode;
use dedup::ProcessOptions;

#[derive(Parser)]
#[command(name = "claude-usage")]
#[command(about = "Fast Rust implementation for Claude usage analysis across multiple VMs")]
#[command(version = "1.0.0")]
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
        last: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Cost calculation mode
        #[arg(long, value_enum, default_value = "auto")]
        mode: CostModeArg,
    },
    /// Show monthly usage aggregation
    Monthly {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show last N entries
        #[arg(long)]
        last: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Cost calculation mode
        #[arg(long, value_enum, default_value = "auto")]
        mode: CostModeArg,
    },
    /// Show recent sessions
    Session {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show last N entries
        #[arg(long)]
        last: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Cost calculation mode
        #[arg(long, value_enum, default_value = "auto")]
        mode: CostModeArg,
    },
    /// Show session blocks
    Blocks {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show last N entries
        #[arg(long)]
        last: Option<usize>,
        /// Start date filter (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Cost calculation mode
        #[arg(long, value_enum, default_value = "auto")]
        mode: CostModeArg,
    },
    /// Show live monitoring
    Live {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show live data snapshot (single view, no monitoring loop)
        #[arg(long)]
        snapshot: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum CostModeArg {
    /// Use costUSD if available, otherwise calculate
    Auto,
    /// Always calculate from tokens
    Calculate,
    /// Always use costUSD
    Display,
}

impl From<CostModeArg> for CostMode {
    fn from(mode: CostModeArg) -> Self {
        match mode {
            CostModeArg::Auto => CostMode::Auto,
            CostModeArg::Calculate => CostMode::Calculate,
            CostModeArg::Display => CostMode::Display,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Handle command with its specific options
    match cli.command.unwrap_or(Commands::Daily {
        json: false,
        last: None,
        since: None,
        until: None,
        mode: CostModeArg::Auto,
    }) {
        Commands::Daily { json, last, since, until, mode } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, last, since, until, mode, "daily");
            
            match analyzer.run_command("daily", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Monthly { json, last, since, until, mode } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, last, since, until, mode, "monthly");
            
            match analyzer.run_command("monthly", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Session { json, last, since, until, mode } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, last, since, until, mode, "session");
            
            match analyzer.run_command("session", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Blocks { json, last, since, until, mode } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, last, since, until, mode, "blocks");
            
            match analyzer.run_command("blocks", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Live { json, snapshot } => {
            if json && !snapshot {
                eprintln!("Error: Live monitoring does not support --json output");
                process::exit(1);
            }
            
            let mut analyzer = ClaudeUsageAnalyzer::new(CostMode::Auto);
            let options = ProcessOptions {
                command: "live".to_string(),
                json_output: json,
                last: None,
                since_date: None,
                until_date: None,
                snapshot,
            };
            
            match analyzer.run_command("live", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
    }
}

fn parse_common_args(
    json: bool,
    last: Option<usize>,
    since: Option<String>,
    until: Option<String>,
    mode: CostModeArg,
    command: &str,
) -> (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, ClaudeUsageAnalyzer, ProcessOptions) {
    // Parse date filters
    let since_date = if let Some(since_str) = since {
        match chrono::NaiveDate::parse_from_str(&since_str, "%Y-%m-%d") {
            Ok(date) => Some(date.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            Err(_) => {
                if !json {
                    eprintln!("❌ Invalid since date format: {}. Use YYYY-MM-DD", since_str);
                }
                process::exit(1);
            }
        }
    } else {
        None
    };
    
    let until_date = if let Some(until_str) = until {
        match chrono::NaiveDate::parse_from_str(&until_str, "%Y-%m-%d") {
            Ok(date) => Some(date.and_hms_opt(23, 59, 59).unwrap().and_utc()),
            Err(_) => {
                if !json {
                    eprintln!("❌ Invalid until date format: {}. Use YYYY-MM-DD", until_str);
                }
                process::exit(1);
            }
        }
    } else {
        None
    };
    
    // Create analyzer
    let analyzer = ClaudeUsageAnalyzer::new(mode.into());
    
    // Build options
    let options = ProcessOptions {
        command: command.to_string(),
        json_output: json,
        last,
        since_date,
        until_date,
        snapshot: false,
    };
    
    (since_date, until_date, analyzer, options)
}

fn handle_error(e: anyhow::Error, json: bool) -> Result<(), anyhow::Error> {
    if json {
        println!("{{\"error\": \"{}\"}}", e);
    } else {
        eprintln!("Error: {}", e);
    }
    process::exit(1);
}