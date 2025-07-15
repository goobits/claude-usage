use claude_usage::display::DisplayManager;
use claude_usage::models::SessionOutput;
use std::collections::HashSet;

#[test]
fn test_display_manager() {
    let display_manager = DisplayManager::new();
    
    let test_data = vec![
        SessionOutput {
            session_id: "test-session-1".to_string(),
            project_path: "project1".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.15,
            last_activity: "2024-01-01".to_string(),
            models_used: vec!["claude-sonnet-4-20250514".to_string()],
        },
        SessionOutput {
            session_id: "test-session-2".to_string(),
            project_path: "project2".to_string(),
            input_tokens: 200,
            output_tokens: 100,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_cost: 0.30,
            last_activity: "2024-01-02".to_string(),
            models_used: vec!["claude-sonnet-4-20250514".to_string()],
        },
    ];
    
    // Test that display methods don't panic
    display_manager.display_session(&test_data, Some(5), true); // JSON output
    display_manager.display_daily(&test_data, Some(5), true); // JSON output
    display_manager.display_monthly(&test_data, Some(5), true); // JSON output
}