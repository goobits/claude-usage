//! Live mode command implementation
//!
//! This module implements the live mode functionality that provides real-time
//! monitoring of Claude usage through integration with claude-keeper.

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::live::orchestrator::LiveOrchestrator;
use crate::live::LiveUpdate;

/// Run live mode with optional baseline
pub async fn run_live_mode(no_baseline: bool) -> Result<()> {
    // Welcome message for users
    println!("🚀 Starting Claude Usage Live Monitor");
    println!();
    
    if no_baseline {
        println!("⚠️  Running without baseline data (--no-baseline specified)");
        println!("💡 This means you'll only see new usage from this point forward");
    } else {
        println!("📊 Preparing live monitoring with baseline data...");
        println!("🔄 This may take a moment while we load your conversation history");
    }
    println!();

    info!(no_baseline, "Starting live mode");

    // Create communication channel for updates
    let (tx, rx) = mpsc::channel::<LiveUpdate>(100);

    // Create the orchestrator
    let mut orchestrator = LiveOrchestrator::new(no_baseline).await?;
    
    // Extract baseline before moving orchestrator into spawn task
    let baseline = orchestrator.get_baseline();
    
    // Start the orchestrator in a background task
    tokio::spawn(async move {
        if let Err(e) = orchestrator.run(tx).await {
            error!(error = %e, "Live orchestrator failed");
        }
    });

    // Success message before starting display
    println!("✅ Live monitoring ready! Starting real-time dashboard...");
    println!("💡 Use Ctrl+C to exit");
    println!();

    // Run the display with baseline and receiver
    crate::display::run_display(baseline, rx).await?;

    println!("👋 Live monitoring stopped. Thank you for using Claude Usage!");
    info!("Live mode completed");
    Ok(())
}