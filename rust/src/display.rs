use crate::models::*;
use colored::*;
use crate::utils::format_with_commas;
use std::collections::HashMap;
use serde_json;

pub struct DisplayManager;

impl DisplayManager {
    pub fn new() -> Self {
        Self
    }

    pub fn display_daily(&self, data: &[SessionOutput], limit: Option<usize>, json_output: bool) {
        let daily_data = self.process_daily_with_projects(data, limit);
        
        if json_output {
            let output = serde_json::json!({"daily": daily_data});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return;
        }
        
        println!("\n{}", "=".repeat(80));
        println!("Claude Code Usage Report - Daily with Project Breakdown (All Instances)");
        println!("{}", "=".repeat(80));
        
        let total_cost: f64 = daily_data.iter().map(|d| d.total_cost).sum();
        let total_sessions: u32 = daily_data.iter().map(|d| d.total_sessions).sum();
        
        println!("\n📊 {} days • {} sessions • ${:.2} total\n", 
                 daily_data.len(), total_sessions, total_cost);
        
        for day in &daily_data {
            println!("📅 {} — ${:.2} ({} sessions)", 
                     day.date, day.total_cost, day.total_sessions);
            
            // Show top 8 projects
            for project in day.projects.iter().take(8) {
                let percentage = if day.total_cost > 0.0 {
                    (project.total_cost / day.total_cost * 100.0)
                } else {
                    0.0
                };
                println!("   {}: ${:.2} ({:.0}%, {} sessions)", 
                         project.project, project.total_cost, percentage, project.sessions);
            }
            
            if day.projects.len() > 8 {
                let remaining_cost: f64 = day.projects.iter().skip(8).map(|p| p.total_cost).sum();
                println!("   ... {} more: ${:.2}", day.projects.len() - 8, remaining_cost);
            }
            
            println!(); // Empty line
        }
    }

    pub fn display_monthly(&self, data: &[SessionOutput], limit: Option<usize>, json_output: bool) {
        let daily_data = self.process_daily_with_projects(data, None);
        let monthly_data = self.process_monthly_data(&daily_data, limit);
        
        if json_output {
            let output = serde_json::json!({"monthly": monthly_data});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return;
        }
        
        println!("\n{}", "=".repeat(80));
        println!("Claude Code Usage Report - Monthly (All Instances)");
        println!("{}", "=".repeat(80));
        
        let total_cost: f64 = monthly_data.iter().map(|m| m.total_cost).sum();
        let total_sessions: u32 = monthly_data.iter().map(|m| m.total_sessions).sum();
        
        println!("\n📊 Total Usage Summary:");
        println!("   Records: {}", monthly_data.len());
        println!("   Total Cost: ${:.2}", total_cost);
        println!("   Total Sessions: {}", total_sessions);
        println!();
        
        let display_limit = limit.unwrap_or(10);
        let recent_data: Vec<_> = monthly_data.iter().rev().take(display_limit).collect();
        println!("📅 Recent monthly usage (last {}):", recent_data.len());
        for month in recent_data.iter().rev() {
            println!("   {}: ${:.2} ({} sessions)", 
                     month.month, month.total_cost, month.total_sessions);
        }
    }

    pub fn display_session(&self, data: &[SessionOutput], limit: Option<usize>, json_output: bool) {
        let mut sorted_data = data.to_vec();
        sorted_data.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        
        if json_output {
            let output = serde_json::json!({"session": sorted_data});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return;
        }
        
        println!("\n{}", "=".repeat(80));
        println!("Claude Code Usage Report - Session (All Instances)");
        println!("{}", "=".repeat(80));
        
        let total_cost: f64 = sorted_data.iter().map(|s| s.total_cost).sum();
        let total_tokens: u32 = sorted_data.iter().map(|s| {
            s.input_tokens + s.output_tokens + s.cache_creation_tokens + s.cache_read_tokens
        }).sum();
        
        println!("\n📊 Total Usage Summary:");
        println!("   Records: {}", sorted_data.len());
        println!("   Total Cost: ${:.2}", total_cost);
        println!("   Total Tokens: {}", format_with_commas(total_tokens));
        println!();
        
        let display_limit = limit.unwrap_or(10);
        let recent_data: Vec<_> = sorted_data.iter().take(display_limit).collect();
        println!("📅 Recent session usage (last {}):", recent_data.len());
        for session in recent_data {
            let session_name = self.format_session_name(session);
            let tokens = session.input_tokens + session.output_tokens + 
                        session.cache_creation_tokens + session.cache_read_tokens;
            println!("   {} | {}: ${:.2} ({} tokens)", 
                     session.last_activity, session_name, session.total_cost, format_with_commas(tokens));
        }
    }

    pub fn display_blocks(&self, blocks: &[SessionBlock], limit: Option<usize>, json_output: bool) {
        if json_output {
            let output = serde_json::json!({"blocks": blocks});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return;
        }
        
        println!("\n{}", "=".repeat(80));
        println!("Claude Code Usage Report - Blocks (All Instances)");
        println!("{}", "=".repeat(80));
        
        let total_cost: f64 = blocks.iter().map(|b| b.cost_usd).sum();
        let total_tokens: u32 = blocks.iter().map(|b| b.token_counts.total()).sum();
        
        println!("\n📊 Total Usage Summary:");
        println!("   Records: {}", blocks.len());
        println!("   Total Cost: ${:.2}", if total_cost == 0.0 { 0.0 } else { total_cost });
        println!("   Total Tokens: {}", format_with_commas(total_tokens));
        println!();
        
        let display_limit = limit.unwrap_or(10);
        let recent_data: Vec<_> = blocks.iter().rev().take(display_limit).collect();
        println!("📅 Recent blocks usage (last {}):", recent_data.len());
        for block in recent_data.iter().rev() {
            // Parse and format start time
            let start_time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&block.start_time) {
                dt.format("%m/%d/%Y, %I:%M:%S %p").to_string()
            } else {
                block.start_time.clone()
            };
            
            let tokens = block.token_counts.total();
            println!("   {}: ${:.2} ({} tokens)", start_time, block.cost_usd, format_with_commas(tokens));
        }
    }

    fn process_daily_with_projects(&self, session_data: &[SessionOutput], limit: Option<usize>) -> Vec<DailyData> {
        let mut date_groups: HashMap<String, Vec<&SessionOutput>> = HashMap::new();
        
        for session in session_data {
            let date = if session.last_activity == "1970-01-01" || session.last_activity == "unknown" {
                "unknown".to_string()
            } else {
                session.last_activity.clone()
            };
            date_groups.entry(date).or_insert_with(Vec::new).push(session);
        }
        
        let mut result = Vec::new();
        
        for (date, sessions) in date_groups {
            let mut project_groups: HashMap<String, DailyProject> = HashMap::new();
            
            for session in &sessions {
                // Extract project name from sessionId
                let project_name = if session.session_id.starts_with('-') {
                    let parts: Vec<&str> = session.session_id[1..].split('-').collect();
                    parts.last().unwrap_or(&"unknown").to_string()
                } else {
                    session.project_path.split('/').last().unwrap_or("unknown").to_string()
                };
                
                let project = project_groups.entry(project_name.clone()).or_insert_with(|| {
                    DailyProject {
                        project: project_name,
                        sessions: 0,
                        total_cost: 0.0,
                        total_tokens: 0,
                    }
                });
                
                project.sessions += 1;
                project.total_cost += session.total_cost;
                project.total_tokens += session.input_tokens + session.output_tokens + 
                                       session.cache_creation_tokens + session.cache_read_tokens;
            }
            
            let day_total = sessions.iter().map(|s| s.total_cost).sum();
            let mut projects: Vec<DailyProject> = project_groups.into_values().collect();
            projects.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());
            
            result.push(DailyData {
                date,
                projects,
                total_cost: day_total,
                total_sessions: sessions.len() as u32,
            });
        }
        
        result.sort_by(|a, b| b.date.cmp(&a.date));
        
        // Apply limit
        let display_limit = limit.unwrap_or(30);
        result.truncate(display_limit);
        
        result
    }

    fn process_monthly_data(&self, daily_data: &[DailyData], limit: Option<usize>) -> Vec<MonthlyData> {
        let mut monthly_groups: HashMap<String, MonthlyData> = HashMap::new();
        
        for day in daily_data {
            let month = if day.date.len() >= 7 {
                day.date[..7].to_string() // YYYY-MM
            } else {
                "unknown".to_string()
            };
            
            let monthly = monthly_groups.entry(month.clone()).or_insert_with(|| {
                MonthlyData {
                    month,
                    total_cost: 0.0,
                    total_sessions: 0,
                }
            });
            
            monthly.total_cost += day.total_cost;
            monthly.total_sessions += day.total_sessions;
        }
        
        let mut result: Vec<MonthlyData> = monthly_groups.into_values().collect();
        result.sort_by(|a, b| a.month.cmp(&b.month));
        
        // Apply limit - show most recent months
        let display_limit = limit.unwrap_or(10);
        if result.len() > display_limit {
            let skip_count = result.len() - display_limit;
            result = result.into_iter().skip(skip_count).collect();
        }
        
        result
    }

    fn format_session_name(&self, session: &SessionOutput) -> String {
        if session.session_id.starts_with('-') {
            let parts: Vec<&str> = session.session_id[1..].split('-').collect();
            parts.last().unwrap_or(&"unknown").to_string()
        } else if session.project_path != "Unknown Project" {
            session.project_path.split('/').last().unwrap_or("unknown").to_string()
        } else {
            session.session_id.clone()
        }
    }
}