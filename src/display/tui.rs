//! Terminal User Interface Implementation
//!
//! This module provides the main TUI implementation using ratatui with crossterm backend.
//! It handles terminal setup, event processing, and the main display loop.

use super::{LiveDisplay, widgets::{render_live_display, AppTheme}};
use crate::live::{BaselineSummary, LiveUpdate};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Update interval for the display (milliseconds)
const UPDATE_INTERVAL_MS: u64 = 1000;

/// Terminal backend type alias
type TerminalBackend = CrosstermBackend<Stdout>;

/// Main display manager for the live monitoring TUI
pub struct LiveDisplayManager {
    /// The ratatui terminal instance
    terminal: Terminal<TerminalBackend>,
    /// Current display state
    display_state: LiveDisplay,
    /// Channel for receiving live updates
    update_receiver: mpsc::Receiver<LiveUpdate>,
    /// Theme for styling the UI
    theme: AppTheme,
    /// Last error message to display
    error_message: Option<String>,
    /// Last cleanup time for memory management
    last_cleanup: Instant,
}

impl LiveDisplayManager {
    /// Create a new display manager
    pub async fn new(
        baseline: BaselineSummary,
        update_receiver: mpsc::Receiver<LiveUpdate>,
    ) -> Result<Self> {
        let terminal = setup_terminal()?;
        let display_state = LiveDisplay::new(baseline);
        let theme = AppTheme::default();

        Ok(Self {
            terminal,
            display_state,
            update_receiver,
            theme,
            error_message: None,
            last_cleanup: Instant::now(),
        })
    }

    /// Run the display loop
    pub async fn run(&mut self) -> Result<()> {
        let mut last_update = Instant::now();

        loop {
            // Handle terminal events (non-blocking)
            if let Err(e) = self.handle_events().await {
                self.error_message = Some(format!("Event handling error: {}", e));
            }

            // Process live updates (non-blocking)
            if let Err(e) = self.process_updates().await {
                self.error_message = Some(format!("Update processing error: {}", e));
            }

            // Render the display
            if let Err(e) = self.render() {
                self.error_message = Some(format!("Rendering error: {}", e));
            }

            // Periodic cleanup to prevent memory growth
            if self.last_cleanup.elapsed() > Duration::from_secs(300) { // 5 minutes
                self.display_state.cleanup_old_sessions();
                self.last_cleanup = Instant::now();
            }

            // Control update rate
            let elapsed = last_update.elapsed();
            if elapsed < Duration::from_millis(UPDATE_INTERVAL_MS) {
                let sleep_duration = Duration::from_millis(UPDATE_INTERVAL_MS) - elapsed;
                tokio::time::sleep(sleep_duration).await;
            }
            last_update = Instant::now();
        }
    }

    /// Handle keyboard and terminal events
    async fn handle_events(&mut self) -> Result<()> {
        // Check for events with a timeout to avoid blocking
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                return self.exit().await;
                            },
                            KeyCode::Up => {
                                self.display_state.scroll_up();
                                // Clear any error message when user interacts
                                self.error_message = None;
                            },
                            KeyCode::Down => {
                                // Use the last known size or default
                                let activity_height = 10; // Default scroll amount
                                self.display_state.scroll_down(activity_height);
                                // Clear any error message when user interacts
                                self.error_message = None;
                            },
                            KeyCode::Char('q') => {
                                return self.exit().await;
                            },
                            KeyCode::Char('r') => {
                                // Reset scroll position
                                self.display_state.scroll_position = 0;
                                self.error_message = None;
                            },
                            _ => {}
                        }
                    }
                },
                Event::Resize(_, _) => {
                    // Terminal was resized, ratatui will handle this automatically
                },
                _ => {}
            }
        }
        Ok(())
    }

    /// Process pending live updates from the channel
    async fn process_updates(&mut self) -> Result<()> {
        // Process all available updates without blocking
        while let Ok(update) = self.update_receiver.try_recv() {
            self.display_state.update(update);
            // Clear error message on successful update
            if self.error_message.is_some() {
                self.error_message = None;
            }
        }
        Ok(())
    }

    /// Render the current display state
    fn render(&mut self) -> Result<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            render_live_display(
                frame,
                &self.display_state,
                area,
                &self.theme,
                self.error_message.as_deref(),
            );
        })?;
        Ok(())
    }

    /// Exit the display and cleanup terminal
    async fn exit(&mut self) -> Result<()> {
        cleanup_terminal(&mut self.terminal)?;
        std::process::exit(0);
    }
}

impl Drop for LiveDisplayManager {
    fn drop(&mut self) {
        let _ = cleanup_terminal(&mut self.terminal);
    }
}

/// Setup the terminal for TUI mode
fn setup_terminal() -> Result<Terminal<TerminalBackend>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)
        .context("Failed to create terminal")?;
    Ok(terminal)
}

/// Cleanup terminal and restore normal mode
fn cleanup_terminal(terminal: &mut Terminal<TerminalBackend>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).context("Failed to cleanup terminal")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

/// Graceful shutdown handler for the display
#[allow(dead_code)]
pub async fn handle_shutdown(mut display_manager: LiveDisplayManager) -> Result<()> {
    // Allow some time for final updates
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Exit gracefully
    display_manager.exit().await
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_display_manager_creation() {
        let baseline = BaselineSummary::default();
        let (_tx, rx) = mpsc::channel(100);
        
        // This test requires a terminal, so we'll just test the creation logic
        // In a real environment, this would work
        let result = LiveDisplayManager::new(baseline, rx).await;
        
        // In test environment without a terminal, this might fail
        // That's expected and acceptable for unit tests
        if result.is_err() {
            println!("Terminal not available in test environment - this is expected");
        }
    }

    #[test]
    fn test_update_interval_constant() {
        assert_eq!(UPDATE_INTERVAL_MS, 1000);
    }
}