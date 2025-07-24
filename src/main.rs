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

use analyzer::ClaudeUsageAnalyzer;
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
    /// Show live monitoring
    Live {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show live data snapshot (single view, no monitoring loop)
        #[arg(long)]
        snapshot: bool,
        /// Exclude VMs directory from analysis
        #[arg(long)]
        exclude_vms: bool,
    },
}


#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Handle command with its specific options
    match cli.command.unwrap_or(Commands::Daily {
        json: false,
        limit: None,
        since: None,
        until: None,
        exclude_vms: false,
    }) {
        Commands::Daily { json, limit, since, until, exclude_vms } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, limit, since, until, "daily", exclude_vms);
            
            match analyzer.run_command("daily", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Monthly { json, limit, since, until, exclude_vms } => {
            let (_since_date, _until_date, mut analyzer, options) = 
                parse_common_args(json, limit, since, until, "monthly", exclude_vms);
            
            match analyzer.run_command("monthly", options).await {
                Ok(_) => Ok(()),
                Err(e) => handle_error(e, json),
            }
        }
        Commands::Live { json, snapshot, exclude_vms } => {
            if json && !snapshot {
                eprintln!("Error: Live monitoring does not support --json output");
                process::exit(1);
            }
            
            let mut analyzer = ClaudeUsageAnalyzer::new();
            let options = ProcessOptions {
                command: "live".to_string(),
                json_output: json,
                limit: None,
                since_date: None,
                until_date: None,
                snapshot,
                exclude_vms,
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
    limit: Option<usize>,
    since: Option<String>,
    until: Option<String>,
    command: &str,
    exclude_vms: bool,
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