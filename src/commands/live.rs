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
    info!(no_baseline, "Starting live mode");

    // Create communication channel for updates
    let (tx, mut rx) = mpsc::channel::<LiveUpdate>(100);

    // Create and start the orchestrator
    let mut orchestrator = LiveOrchestrator::new(no_baseline)?;
    
    // Start the orchestrator in a background task
    let mut orchestrator_handle = tokio::spawn(async move {
        if let Err(e) = orchestrator.run(tx).await {
            error!(error = %e, "Live orchestrator failed");
        }
    });

    // Main event loop - receive and process updates
    loop {
        tokio::select! {
            // Handle incoming updates
            update = rx.recv() => {
                match update {
                    Some(update) => {
                        // For now, just log the update
                        // Diana will implement the display logic
                        info!(
                            session_id = %update.entry.message.id,
                            tokens = update.session_stats.total_tokens(),
                            cost = update.session_stats.total_cost,
                            "Received live update"
                        );
                    }
                    None => {
                        // Channel closed, orchestrator finished
                        break;
                    }
                }
            }
            
            // Handle orchestrator completion
            result = &mut orchestrator_handle => {
                match result {
                    Ok(_) => {
                        info!("Live mode orchestrator completed successfully");
                    }
                    Err(e) => {
                        error!(error = %e, "Live mode orchestrator task failed");
                    }
                }
                break;
            }
        }
    }

    info!("Live mode completed");
    Ok(())
}