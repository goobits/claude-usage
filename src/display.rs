//! Output Formatting and Display Management
//!
//! This module handles all output formatting for Claude usage analysis results.
//! It provides both human-readable terminal output with colors and structured JSON
//! output for programmatic consumption.
//!
//! ## Core Functionality
//!
//! ### Report Types
//! - **Daily Reports**: Day-by-day usage breakdown with project-level details
//! - **Monthly Reports**: Month-by-month usage summaries with totals
//! - **JSON Output**: Machine-readable structured data for API consumption
//! - **Terminal Output**: Human-friendly colored output with progress indicators
//!
//! ### Display Features
//! - **Color-coded Output**: Uses different colors for costs, dates, and metrics
//! - **Project Breakdown**: Shows usage per project within each time period
//! - **Percentage Calculations**: Displays relative usage between projects
//! - **Summary Statistics**: Provides totals and counts across all data
//!
//! ## Key Types
//!
//! - [`DisplayManager`] - Main interface for all display operations
//!
//! ## Output Formats
//!
//! ### Daily Reports
//! Daily reports show usage for each day with project-level breakdown:
//! - Date and total cost per day
//! - Individual project costs and percentages
//! - Session counts per project
//! - Configurable display limits (default: 30 days)
//!
//! ### Monthly Reports
//! Monthly reports provide higher-level summaries:
//! - Month-by-month cost totals
//! - Session counts per month
//! - Configurable display limits (default: 10 months)
//! - Reverse chronological ordering (most recent first)
//!
//! ### JSON Output
//! When `json_output` is enabled, all reports are formatted as structured JSON:
//! ```json
//! {
//!   "daily": [
//!     {
//!       "date": "2025-01-15",
//!       "projects": [
//!         {
//!           "project": "my-project",
//!           "sessions": 3,
//!           "totalCost": 1.25,
//!           "totalTokens": 15000
//!         }
//!       ],
//!       "totalCost": 1.25,
//!       "totalSessions": 3
//!     }
//!   ]
//! }
//! ```
//!
//! ## Data Processing
//!
//! ### Daily Aggregation
//! - Processes session data to create daily summaries
//! - Handles overlapping sessions across day boundaries
//! - Ensures accurate session counting (sessions counted once per day)
//! - Generates reports for last N days, including days with no activity
//!
//! ### Monthly Aggregation
//! - Groups daily data into monthly buckets
//! - Tracks unique sessions per month
//! - Applies display limits for recent months
//! - Sorts chronologically for easy trend analysis
//!
//! ## Usage Example
//!
//! ```rust
//! use claude_usage::display::DisplayManager;
//!
//! let display_manager = DisplayManager::new();
//! let sessions = vec![/* session data */];
//!
//! // Display daily report
//! display_manager.display_daily(&sessions, Some(7), false);
//!
//! // Display monthly report
//! display_manager.display_monthly(&sessions, Some(6), false);
//! ```
//!
//! ## Integration Points
//!
//! The display manager integrates with:
//! - [`crate::models`] for data structure definitions
//! - [`crate::analyzer::ClaudeUsageAnalyzer`] for receiving processed data
//! - Terminal color libraries for enhanced visual output

use crate::models::*;
use colored::Colorize;
use std::collections::{HashMap, HashSet};

pub struct DisplayManager;

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayManager {
    pub fn new() -> Self {
        Self
    }

    pub fn display_daily(&self, data: &[SessionOutput], limit: Option<usize>, json_output: bool) {
        let daily_data = self.process_daily_with_projects(data, limit);

        if json_output {
            let output = serde_json::json!({"daily": daily_data});
            match serde_json::to_string_pretty(&output) {
                Ok(json_str) => println!("{}", json_str),
                Err(e) => {
                    eprintln!("Error serializing daily data to JSON: {}", e);
                    return;
                }
            }
            return;
        }

        println!("\n{}", "=".repeat(80).bright_cyan());
        println!(
            "{}",
            "Claude Code Usage Report - Daily with Project Breakdown (All Instances)"
                .bright_white()
                .bold()
        );
        println!("{}", "=".repeat(80).bright_cyan());

        let total_cost: f64 = daily_data.iter().map(|d| d.total_cost).sum();
        let total_sessions: u32 = daily_data.iter().map(|d| d.total_sessions).sum();

        println!(
            "\n{} {} days • {} sessions • {} total\n",
            "📊".bright_yellow(),
            daily_data.len().to_string().bright_white().bold(),
            total_sessions.to_string().bright_white().bold(),
            format!("${:.2}", total_cost).bright_green().bold()
        );

        for day in &daily_data {
            println!(
                "{} {} — {} ({} sessions)",
                "📅".bright_blue(),
                day.date.bright_white().bold(),
                format!("${:.2}", day.total_cost).bright_green().bold(),
                format!("{}", day.total_sessions).bright_white()
            );

            // Show all projects
            for project in &day.projects {
                let percentage = if day.total_cost > 0.0 {
                    project.total_cost / day.total_cost * 100.0
                } else {
                    0.0
                };
                println!(
                    "   {}: {} ({}%, {} sessions)",
                    project.project.bright_cyan(),
                    format!("${:.2}", project.total_cost).bright_green(),
                    format!("{:.0}", percentage).bright_yellow(),
                    format!("{}", project.sessions).bright_white()
                );
            }

            println!(); // Empty line
        }
    }

    pub fn display_monthly(&self, data: &[SessionOutput], limit: Option<usize>, json_output: bool) {
        let monthly_data = self.process_monthly_data(data, limit);

        if json_output {
            let output = serde_json::json!({"monthly": monthly_data});
            match serde_json::to_string_pretty(&output) {
                Ok(json_str) => println!("{}", json_str),
                Err(e) => {
                    eprintln!("Error serializing monthly data to JSON: {}", e);
                    return;
                }
            }
            return;
        }

        println!("\n{}", "=".repeat(80).bright_cyan());
        println!(
            "{}",
            "Claude Code Usage Report - Monthly (All Instances)"
                .bright_white()
                .bold()
        );
        println!("{}", "=".repeat(80).bright_cyan());

        let total_cost: f64 = monthly_data.iter().map(|m| m.total_cost).sum();
        let total_sessions: u32 = monthly_data.iter().map(|m| m.total_sessions).sum();

        println!("\n{} Total Usage Summary:", "📊".bright_yellow());
        println!(
            "   Records: {}",
            monthly_data.len().to_string().bright_white().bold()
        );
        println!(
            "   Total Cost: {}",
            format!("${:.2}", total_cost).bright_green().bold()
        );
        println!(
            "   Total Sessions: {}",
            total_sessions.to_string().bright_white().bold()
        );
        println!();

        let display_limit = limit.unwrap_or(10);
        let recent_data: Vec<_> = monthly_data.iter().rev().take(display_limit).collect();
        println!(
            "{} Recent monthly usage (last {}):",
            "📅".bright_blue(),
            recent_data.len().to_string().bright_white().bold()
        );
        for month in recent_data.iter().rev() {
            println!(
                "   {}: {} ({} sessions)",
                month.month.bright_white().bold(),
                format!("${:.2}", month.total_cost).bright_green(),
                format!("{}", month.total_sessions).bright_white()
            );
        }
    }

    fn process_daily_with_projects(
        &self,
        session_data: &[SessionOutput],
        limit: Option<usize>,
    ) -> Vec<DailyData> {
        let display_limit = limit.unwrap_or(30);

        // Create a map to store daily aggregated data
        let mut daily_aggregates: HashMap<String, HashMap<String, DailyProject>> = HashMap::new();

        // Track which sessions have been counted for each date
        let mut counted_sessions_per_day: HashMap<String, HashSet<String>> = HashMap::new();

        // Process each session's daily usage breakdown
        for session in session_data {
            for (date, daily_usage) in &session.daily_usage {
                let date_projects = daily_aggregates.entry(date.clone()).or_default();

                let project = date_projects
                    .entry(session.project_path.clone())
                    .or_insert_with(|| DailyProject {
                        project: session.project_path.clone(),
                        sessions: 0,
                        total_cost: 0.0,
                        total_tokens: 0,
                    });

                // Add tokens and cost for this day
                project.total_cost += daily_usage.cost;
                project.total_tokens += daily_usage.input_tokens
                    + daily_usage.output_tokens
                    + daily_usage.cache_creation_tokens
                    + daily_usage.cache_read_tokens;
            }

            // Count the session only once per day it was active
            for date in session.daily_usage.keys() {
                let counted_this_day = counted_sessions_per_day.entry(date.clone()).or_default();
                if counted_this_day.insert(session.session_id.clone()) {
                    // This session hasn't been counted for this day yet
                    if let Some(date_projects) = daily_aggregates.get_mut(date) {
                        if let Some(project) = date_projects.get_mut(&session.project_path) {
                            project.sessions += 1;
                        }
                    }
                }
            }
        }

        // Generate the last N days, even if they have no data
        let mut result = Vec::new();

        // Get today's date
        let today = chrono::Local::now().date_naive();

        // Generate the last display_limit days
        for i in 0..display_limit {
            let target_date = today - chrono::Duration::days(i as i64);
            let date_str = target_date.format("%Y-%m-%d").to_string();

            if let Some(date_projects) = daily_aggregates.get(&date_str) {
                // Process projects for this date
                let mut projects: Vec<DailyProject> = date_projects.values().cloned().collect();
                projects.sort_by(|a, b| a.project.cmp(&b.project));

                let day_total: f64 = projects.iter().map(|p| p.total_cost).sum();
                let day_sessions: u32 = projects.iter().map(|p| p.sessions).sum();

                result.push(DailyData {
                    date: date_str,
                    projects,
                    total_cost: day_total,
                    total_sessions: day_sessions,
                });
            } else {
                // No data for this date, create empty entry
                result.push(DailyData {
                    date: date_str,
                    projects: Vec::new(),
                    total_cost: 0.0,
                    total_sessions: 0,
                });
            }
        }

        // Don't truncate - show exactly the number of days requested

        result
    }

    fn process_monthly_data(
        &self,
        session_data: &[SessionOutput],
        limit: Option<usize>,
    ) -> Vec<MonthlyData> {
        let mut monthly_aggregates: HashMap<String, (f64, HashSet<String>)> = HashMap::new();

        // Process each session
        for session in session_data {
            // For each day the session was active
            for (date, daily_usage) in &session.daily_usage {
                // Extract month from date (YYYY-MM-DD -> YYYY-MM)
                let month = if date.len() >= 7 {
                    date[..7].to_string()
                } else {
                    "unknown".to_string()
                };

                let (cost, sessions) = monthly_aggregates
                    .entry(month)
                    .or_insert_with(|| (0.0, HashSet::new()));

                // Add cost for this day
                *cost += daily_usage.cost;

                // Track unique session for this month
                sessions.insert(session.session_id.clone());
            }
        }

        // Convert to MonthlyData
        let mut result: Vec<MonthlyData> = monthly_aggregates
            .into_iter()
            .map(|(month, (total_cost, sessions))| MonthlyData {
                month,
                total_cost,
                total_sessions: sessions.len() as u32,
            })
            .collect();

        result.sort_by(|a, b| a.month.cmp(&b.month));

        // Apply limit - show most recent months
        let display_limit = limit.unwrap_or(10);
        if result.len() > display_limit {
            let skip_count = result.len() - display_limit;
            result = result.into_iter().skip(skip_count).collect();
        }

        result
    }
}
