//! Display State Management
//!
//! This module manages the state for the live display TUI, including the ring buffer
//! for recent activities, current session tracking, and running totals.

use crate::live::{BaselineSummary, LiveUpdate};
use crate::models::SessionData;
use super::{RunningTotals, SessionActivity};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

/// Maximum number of recent entries to keep in the ring buffer
const MAX_RECENT_ENTRIES: usize = 100;

/// Core display state for the live monitoring TUI
#[derive(Debug)]
pub struct LiveDisplay {
    /// Baseline summary from parquet files
    pub baseline: BaselineSummary,
    /// Ring buffer of recent activities (max 100, FIFO)
    pub recent_entries: VecDeque<SessionActivity>,
    /// Current active session, if any
    pub current_session: Option<SessionData>,
    /// Running totals including baseline and live updates
    pub running_totals: RunningTotals,
    /// Current scroll position for recent activities
    pub scroll_position: usize,
    /// Track sessions and their start times for duration calculation
    session_start_times: HashMap<String, SystemTime>,
    /// Last update timestamp for calculating session duration
    last_update_time: SystemTime,
}

impl LiveDisplay {
    /// Create new LiveDisplay from baseline summary
    pub fn new(baseline: BaselineSummary) -> Self {
        let running_totals = RunningTotals::from_baseline(&baseline);
        
        Self {
            baseline,
            recent_entries: VecDeque::with_capacity(MAX_RECENT_ENTRIES),
            current_session: None,
            running_totals,
            scroll_position: 0,
            session_start_times: HashMap::new(),
            last_update_time: SystemTime::now(),
        }
    }

    /// Update display state with a new live update
    pub fn update(&mut self, update: LiveUpdate) {
        self.last_update_time = update.timestamp;

        // Update running totals
        self.running_totals.update(&update);

        // Track session start time
        let session_id = update.session_stats.session_id.clone();
        self.session_start_times
            .entry(session_id.clone())
            .or_insert(update.timestamp);

        // Update current session
        self.current_session = Some(update.session_stats.clone());

        // Add to recent activities
        let activity = SessionActivity::from_update(&update);
        self.add_recent_activity(activity);
    }

    /// Add a new activity to the ring buffer
    fn add_recent_activity(&mut self, activity: SessionActivity) {
        self.recent_entries.push_front(activity);
        
        // Maintain ring buffer size
        if self.recent_entries.len() > MAX_RECENT_ENTRIES {
            self.recent_entries.pop_back();
        }

        // Reset scroll position to show newest entries
        self.scroll_position = 0;
    }

    /// Get current session duration if there's an active session
    pub fn get_current_session_duration(&self) -> Option<Duration> {
        if let Some(ref session) = self.current_session {
            if let Some(&start_time) = self.session_start_times.get(&session.session_id) {
                return self.last_update_time.duration_since(start_time).ok();
            }
        }
        None
    }

    /// Scroll up in the recent activities list
    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    /// Scroll down in the recent activities list
    pub fn scroll_down(&mut self, visible_lines: usize) {
        let max_scroll = if self.recent_entries.len() > visible_lines {
            self.recent_entries.len() - visible_lines
        } else {
            0
        };
        
        if self.scroll_position < max_scroll {
            self.scroll_position += 1;
        }
    }

    /// Get visible recent activities based on scroll position and available space
    pub fn get_visible_activities(&self, visible_lines: usize) -> Vec<&SessionActivity> {
        self.recent_entries
            .iter()
            .skip(self.scroll_position)
            .take(visible_lines)
            .collect()
    }

    /// Format current session info for display
    pub fn format_current_session(&self) -> Option<String> {
        if let Some(ref session) = self.current_session {
            let duration = self.get_current_session_duration()
                .map(|d| format!("{}m {}s", d.as_secs() / 60, d.as_secs() % 60))
                .unwrap_or_else(|| "0s".to_string());

            let project_name = session.project_path
                .split('/')
                .last()
                .unwrap_or(&session.project_path);

            Some(format!(
                "Project: {} | Duration: {} | Cost: ${:.2} | Tokens: In {}K / Out {}K",
                project_name,
                duration,
                session.total_cost,
                session.input_tokens / 1000,
                session.output_tokens / 1000
            ))
        } else {
            None
        }
    }

    /// Format running totals for display
    pub fn format_totals(&self) -> String {
        format!(
            "Total: ${:.2} | Tokens: {:.1}M | Sessions: {}",
            self.running_totals.total_cost,
            self.running_totals.total_tokens as f64 / 1_000_000.0,
            self.running_totals.total_sessions
        )
    }

    /// Get scroll indicator text
    pub fn get_scroll_indicator(&self, visible_lines: usize) -> String {
        if self.recent_entries.len() <= visible_lines {
            "".to_string()
        } else {
            let total_pages = (self.recent_entries.len() + visible_lines - 1) / visible_lines;
            let current_page = (self.scroll_position / visible_lines) + 1;
            format!(" ({}/{})", current_page, total_pages)
        }
    }

    /// Check if there are activities to scroll through
    pub fn can_scroll(&self, visible_lines: usize) -> bool {
        self.recent_entries.len() > visible_lines
    }

    /// Clean up old session start times to prevent memory growth
    pub fn cleanup_old_sessions(&mut self) {
        let cutoff_time = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
        
        self.session_start_times.retain(|_, &mut start_time| {
            start_time > cutoff_time
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MessageData, UsageData, UsageEntry};

    fn create_test_update(session_id: &str, project: &str, tokens: u32, cost: f64) -> LiveUpdate {
        LiveUpdate {
            entry: UsageEntry {
                timestamp: "2025-01-01T12:00:00Z".to_string(),
                message: MessageData {
                    id: "msg1".to_string(),
                    model: "claude-3-5-sonnet-20241022".to_string(),
                    usage: Some(UsageData {
                        input_tokens: tokens,
                        output_tokens: 0,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                },
                cost_usd: Some(cost),
                request_id: "req1".to_string(),
            },
            session_stats: {
                let mut data = SessionData::new(session_id.to_string(), project.to_string());
                data.input_tokens = tokens;
                data.total_cost = cost;
                data
            },
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn test_ring_buffer_behavior() {
        let baseline = BaselineSummary::default();
        let mut display = LiveDisplay::new(baseline);

        // Add entries beyond the buffer limit
        for i in 0..150 {
            let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
            display.update(update);
        }

        // Should maintain exactly MAX_RECENT_ENTRIES
        assert_eq!(display.recent_entries.len(), MAX_RECENT_ENTRIES);
        
        // Most recent entry should be first
        assert_eq!(display.recent_entries[0].session_id, "session_149");
        assert_eq!(display.recent_entries[MAX_RECENT_ENTRIES - 1].session_id, "session_50");
    }

    #[test]
    fn test_scroll_behavior() {
        let baseline = BaselineSummary::default();
        let mut display = LiveDisplay::new(baseline);

        // Add some entries
        for i in 0..10 {
            let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
            display.update(update);
        }

        // Test scrolling
        assert_eq!(display.scroll_position, 0);
        
        display.scroll_down(5); // 5 visible lines
        assert_eq!(display.scroll_position, 1);
        
        display.scroll_up();
        assert_eq!(display.scroll_position, 0);
        
        // Can't scroll up past 0
        display.scroll_up();
        assert_eq!(display.scroll_position, 0);
    }

    #[test]
    fn test_running_totals_update() {
        let baseline = BaselineSummary {
            total_cost: 10.0,
            total_tokens: 5000,
            sessions_today: 2,
            last_backup: SystemTime::UNIX_EPOCH,
        };
        
        let mut display = LiveDisplay::new(baseline);
        
        let update = create_test_update("session1", "project", 1000, 0.5);
        display.update(update);
        
        assert_eq!(display.running_totals.total_cost, 10.5);
        assert_eq!(display.running_totals.total_tokens, 6000);
    }
}