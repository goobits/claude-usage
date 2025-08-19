//! Tests for the display module
//!
//! These tests verify the TUI components work correctly and handle
//! various scenarios like resizing, scrolling, and data updates.

use claude_usage::display::{LiveDisplay, RunningTotals, SessionActivity};
use claude_usage::live::{BaselineSummary, LiveUpdate, SessionStats};
use claude_usage::models::{MessageData, UsageData, UsageEntry};
use std::time::SystemTime;

fn create_test_baseline() -> BaselineSummary {
    BaselineSummary {
        total_cost: 10.5,
        total_tokens: 50000,
        sessions_today: 5,
        last_backup: SystemTime::UNIX_EPOCH,
    }
}

fn create_test_update(session_id: &str, project: &str, tokens: u32, cost: f64) -> LiveUpdate {
    LiveUpdate {
        entry: UsageEntry {
            timestamp: "2025-01-01T12:00:00Z".to_string(),
            message: MessageData {
                id: "msg1".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                usage: Some(UsageData {
                    input_tokens: tokens,
                    output_tokens: tokens / 2,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }),
            },
            cost_usd: Some(cost),
            request_id: "req1".to_string(),
        },
        session_stats: SessionStats {
            session_id: session_id.to_string(),
            project_path: project.to_string(),
            input_tokens: tokens,
            output_tokens: tokens / 2,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: cost,
        },
        timestamp: SystemTime::now(),
    }
}

#[test]
fn test_live_display_initialization() {
    let baseline = create_test_baseline();
    let display = LiveDisplay::new(baseline.clone());

    assert_eq!(display.running_totals.total_cost, baseline.total_cost);
    assert_eq!(display.running_totals.total_tokens, baseline.total_tokens);
    assert_eq!(display.running_totals.total_sessions, baseline.sessions_today);
    assert!(display.recent_entries.is_empty());
    assert!(display.current_session.is_none());
    assert_eq!(display.scroll_position, 0);
}

#[test]
fn test_live_display_update() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    let update = create_test_update("session1", "/path/to/project", 1000, 0.15);
    let initial_cost = display.running_totals.total_cost;
    let initial_tokens = display.running_totals.total_tokens;

    display.update(update);

    // Check running totals were updated
    assert_eq!(display.running_totals.total_cost, initial_cost + 0.15);
    assert_eq!(display.running_totals.total_tokens, initial_tokens + 1500); // 1000 + 500

    // Check current session was set
    assert!(display.current_session.is_some());
    let session = display.current_session.as_ref().unwrap();
    assert_eq!(session.session_id, "session1");
    assert_eq!(session.project_path, "/path/to/project");

    // Check recent activity was added
    assert_eq!(display.recent_entries.len(), 1);
    let activity = &display.recent_entries[0];
    assert_eq!(activity.session_id, "session1");
    assert_eq!(activity.tokens, 1500);
    assert_eq!(activity.cost, 0.15);
}

#[test]
fn test_ring_buffer_overflow() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    // Add more than 100 entries
    for i in 0..150 {
        let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
        display.update(update);
    }

    // Should maintain exactly 100 entries
    assert_eq!(display.recent_entries.len(), 100);
    
    // Most recent should be first
    assert_eq!(display.recent_entries[0].session_id, "session_149");
    assert_eq!(display.recent_entries[99].session_id, "session_50");
}

#[test]
fn test_scroll_functionality() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    // Add some entries
    for i in 0..20 {
        let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
        display.update(update);
    }

    // Test scrolling down
    assert_eq!(display.scroll_position, 0);
    display.scroll_down(10); // 10 visible lines
    assert_eq!(display.scroll_position, 1);

    // Test scrolling up
    display.scroll_up();
    assert_eq!(display.scroll_position, 0);

    // Can't scroll up past 0
    display.scroll_up();
    assert_eq!(display.scroll_position, 0);

    // Test max scroll
    for _ in 0..20 {
        display.scroll_down(10);
    }
    assert_eq!(display.scroll_position, 10); // 20 entries - 10 visible = 10 max scroll
}

#[test]
fn test_visible_activities() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    // Add 15 entries
    for i in 0..15 {
        let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
        display.update(update);
    }

    // Get visible activities (10 lines visible)
    let visible = display.get_visible_activities(10);
    assert_eq!(visible.len(), 10);
    assert_eq!(visible[0].session_id, "session_14"); // Most recent first
    assert_eq!(visible[9].session_id, "session_5");

    // Scroll down and check again
    display.scroll_down(10);
    let visible = display.get_visible_activities(10);
    assert_eq!(visible.len(), 5); // Only 5 remaining entries
    assert_eq!(visible[0].session_id, "session_4");
    assert_eq!(visible[4].session_id, "session_0");
}

#[test]
fn test_format_totals() {
    let baseline = BaselineSummary {
        total_cost: 45.23,
        total_tokens: 1_200_000,
        sessions_today: 15,
        last_backup: SystemTime::UNIX_EPOCH,
    };
    
    let display = LiveDisplay::new(baseline);
    let formatted = display.format_totals();
    
    assert!(formatted.contains("$45.23"));
    assert!(formatted.contains("1.2M"));
    assert!(formatted.contains("15"));
}

#[test]
fn test_format_current_session() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    // No current session
    assert!(display.format_current_session().is_none());

    // Add an update to set current session
    let update = create_test_update("session1", "/path/to/my-project", 10000, 2.10);
    display.update(update);

    let formatted = display.format_current_session().unwrap();
    assert!(formatted.contains("my-project"));
    assert!(formatted.contains("$2.10"));
    assert!(formatted.contains("10K")); // Input tokens
    assert!(formatted.contains("5K"));  // Output tokens (half of input)
}

#[test]
fn test_session_activity_creation() {
    let update = create_test_update("session1", "/path/to/project", 1500, 0.25);
    let activity = SessionActivity::from_update(&update);

    assert_eq!(activity.session_id, "session1");
    assert_eq!(activity.project, "project"); // Should extract last path component
    assert_eq!(activity.tokens, 2250); // 1500 + 750 (half for output)
    assert_eq!(activity.cost, 0.25);
}

#[test]
fn test_can_scroll() {
    let baseline = create_test_baseline();
    let mut display = LiveDisplay::new(baseline);

    // No entries - can't scroll
    assert!(!display.can_scroll(10));

    // Add some entries but less than visible lines
    for i in 0..5 {
        let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
        display.update(update);
    }
    assert!(!display.can_scroll(10)); // 5 entries, 10 visible lines

    // Add more entries than visible lines
    for i in 5..15 {
        let update = create_test_update(&format!("session_{}", i), "project", 100, 0.01);
        display.update(update);
    }
    assert!(display.can_scroll(10)); // 15 entries, 10 visible lines
}

#[test]
fn test_running_totals_from_baseline() {
    let baseline = BaselineSummary {
        total_cost: 123.45,
        total_tokens: 987654,
        sessions_today: 42,
        last_backup: SystemTime::UNIX_EPOCH,
    };

    let totals = RunningTotals::from_baseline(&baseline);
    assert_eq!(totals.total_cost, 123.45);
    assert_eq!(totals.total_tokens, 987654);
    assert_eq!(totals.total_sessions, 42);
}

#[test]
fn test_running_totals_update() {
    let baseline = create_test_baseline();
    let mut totals = RunningTotals::from_baseline(&baseline);
    let initial_cost = totals.total_cost;
    let initial_tokens = totals.total_tokens;

    let update = create_test_update("session1", "project", 2000, 0.30);
    totals.update(&update);

    assert_eq!(totals.total_cost, initial_cost + 0.30);
    assert_eq!(totals.total_tokens, initial_tokens + 3000); // 2000 + 1000 (output)
}