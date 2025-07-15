use crate::models::*;
use crate::parser::FileParser;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

pub struct LiveMonitor {
    cost_mode: CostMode,
    parser: FileParser,
    cached_blocks: Option<Vec<SessionBlock>>,
    cache_time: Option<std::time::Instant>,
}

impl LiveMonitor {
    pub fn new(cost_mode: CostMode) -> Self {
        Self {
            cost_mode: cost_mode.clone(),
            parser: FileParser::new(cost_mode),
            cached_blocks: None,
            cache_time: None,
        }
    }

    pub async fn run_live_monitor(&mut self, json_output: bool, snapshot: bool) -> Result<()> {
        const TOKEN_LIMIT: u32 = 880000; // Max20 limit
        const BUDGET_LIMIT: f64 = TOKEN_LIMIT as f64 * 0.000015; // ~$1.50 per 1000 tokens
        
        if json_output || snapshot {
            // Snapshot mode for JSON or when --snapshot is used
            self.display_snapshot(TOKEN_LIMIT, BUDGET_LIMIT, json_output).await?;
            return Ok(());
        }
        
        // Set up signal handling
        let mut interval = time::interval(Duration::from_secs(3));
        
        // Hide cursor
        self.hide_cursor();
        
        // Handle Ctrl+C gracefully
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);
        
        loop {
            tokio::select! {
                _ = &mut ctrl_c => {
                    self.show_cursor();
                    println!("\n\n\x1b[96mMonitoring stopped.\x1b[0m");
                    break;
                }
                _ = interval.tick() => {
                    self.display_live_data(TOKEN_LIMIT, BUDGET_LIMIT).await?;
                }
            }
        }
        
        Ok(())
    }

    async fn display_live_data(&mut self, token_limit: u32, budget_limit: f64) -> Result<()> {
        self.clear_screen();
        
        let active_block = self.find_active_session_block().await?;
        let current_time = chrono::Local::now().format("%H:%M").to_string();
        
        // Print header
        println!("\x1b[1m[ CLAUDE USAGE MONITOR ]\x1b[0m");
        println!();
        
        if let Some(block) = active_block {
            self.display_active_session(&block, token_limit, budget_limit, &current_time).await?;
        } else {
            self.display_inactive_session(token_limit, budget_limit, &current_time).await?;
        }
        
        Ok(())
    }

    async fn display_snapshot(&mut self, token_limit: u32, budget_limit: f64, json_output: bool) -> Result<()> {
        let active_block = self.find_active_session_block().await?;
        let current_time = chrono::Local::now().format("%H:%M").to_string();
        
        if json_output {
            let snapshot_data = if let Some(block) = active_block {
                self.create_snapshot_data(&block, token_limit, budget_limit).await?
            } else {
                serde_json::json!({
                    "status": "inactive",
                    "message": "No active session"
                })
            };
            
            println!("{}", serde_json::to_string_pretty(&snapshot_data)?);
        } else {
            println!("\x1b[1m[ CLAUDE USAGE MONITOR ]\x1b[0m");
            println!();
            
            if let Some(block) = active_block {
                self.display_active_session(&block, token_limit, budget_limit, &current_time).await?;
                println!("\n[Snapshot mode - aggregated from active sessions across {} Claude instances]", 
                         self.parser.discover_claude_paths()?.len());
            } else {
                self.display_inactive_session(token_limit, budget_limit, &current_time).await?;
                println!("\n[Snapshot mode - scanned {} Claude instances]", 
                         self.parser.discover_claude_paths()?.len());
            }
        }
        
        Ok(())
    }

    async fn display_active_session(&self, block: &SessionBlock, token_limit: u32, budget_limit: f64, current_time: &str) -> Result<()> {
        let start_time = self.parser.parse_timestamp(&block.start_time)?;
        let end_time = self.parser.parse_timestamp(&block.end_time)?;
        let now = Utc::now();
        
        let total_tokens = block.token_counts.total();
        let cost_used = block.cost_usd;
        
        // Calculate session progress
        let total_session_minutes = (end_time - start_time).num_minutes() as f64;
        let elapsed_minutes = (now - start_time).num_minutes().max(0) as f64;
        let remaining_minutes = (end_time - now).num_minutes().max(0) as f64;
        
        // Progress percentages
        let token_percentage = (total_tokens as f64 / token_limit as f64) * 100.0;
        let token_status = if token_percentage < 70.0 { "🟢" } else if token_percentage < 90.0 { "🟡" } else { "🔴" };
        
        let budget_percentage = (cost_used / budget_limit) * 100.0;
        let budget_status = if budget_percentage < 70.0 { "🟢" } else if budget_percentage < 90.0 { "🟡" } else { "🔴" };
        
        let reset_percentage = if total_session_minutes > 0.0 {
            (elapsed_minutes / total_session_minutes) * 100.0
        } else {
            0.0
        };
        
        // Calculate burn rates
        let burn_rate = if elapsed_minutes > 0.0 {
            total_tokens as f64 / elapsed_minutes
        } else {
            0.0
        };
        
        let cost_burn_rate = if elapsed_minutes > 0.0 {
            (cost_used / elapsed_minutes) * 60.0 // per hour
        } else {
            0.0
        };
        
        // Time displays
        let reset_time = end_time.format("%H:%M").to_string();
        
        // Predict when tokens will run out
        let predicted_end_str = if burn_rate > 0.0 && total_tokens < token_limit {
            let tokens_left = token_limit - total_tokens;
            let minutes_to_depletion = tokens_left as f64 / burn_rate;
            let predicted_end = now + chrono::Duration::minutes(minutes_to_depletion as i64);
            predicted_end.format("%H:%M").to_string()
        } else if total_tokens >= token_limit {
            "LIMIT HIT".to_string()
        } else {
            reset_time.clone()
        };
        
        // Status message
        let status_message = if total_tokens > token_limit {
            format!("🚨 Session tokens exceeded limit! ({} > {})", total_tokens, token_limit)
        } else if budget_percentage > 90.0 {
            "💸 High session cost!".to_string()
        } else if token_percentage > 90.0 {
            "🔥 High session usage!".to_string()
        } else {
            "⛵ Smooth sailing...".to_string()
        };
        
        // Display the monitor
        println!("⚡ Tokens:  {} {} / {}", 
                 self.create_progress_bar(token_percentage, 20, token_status), 
                 total_tokens, token_limit);
        println!("💲 Budget:  {} ${:.2} / ${:.2}", 
                 self.create_progress_bar(budget_percentage, 20, budget_status), 
                 cost_used, budget_limit);
        println!("♻️  Reset:   {} {}", 
                 self.create_progress_bar(reset_percentage, 20, "🕐"), 
                 self.format_time(remaining_minutes));
        println!();
        
        let burn_rate_str = if burn_rate > 0.0 {
            format!("{:.1} tok/min", burn_rate)
        } else {
            "0.0 tok/min".to_string()
        };
        
        let cost_rate_str = if cost_burn_rate > 0.0 {
            format!("${:.2}/hour", cost_burn_rate)
        } else {
            "$0.00/hour".to_string()
        };
        
        println!("🔥 {} | 💰 {}", burn_rate_str, cost_rate_str);
        println!();
        println!("🕐 {} | 🏁 {} | ♻️  {}", current_time, predicted_end_str, reset_time);
        println!();
        println!("{}", status_message);
        
        Ok(())
    }

    async fn display_inactive_session(&self, token_limit: u32, budget_limit: f64, current_time: &str) -> Result<()> {
        println!("⚡ Tokens:  {} 0 / {}", 
                 self.create_progress_bar(0.0, 20, "🟢"), token_limit);
        println!("💲 Budget:  {} $0.00 / ${:.2}", 
                 self.create_progress_bar(0.0, 20, "🟢"), budget_limit);
        println!("♻️  Reset:   {} 0m", 
                 self.create_progress_bar(0.0, 20, "🕐"));
        println!();
        println!("🔥 0.0 tok/min | 💰 $0.00/hour");
        println!();
        println!("🕐 {} | 🏁 No session | ♻️  Next reset", current_time);
        println!();
        println!("📝 No active session");
        
        Ok(())
    }

    async fn create_snapshot_data(&self, block: &SessionBlock, token_limit: u32, budget_limit: f64) -> Result<serde_json::Value> {
        let start_time = self.parser.parse_timestamp(&block.start_time)?;
        let end_time = self.parser.parse_timestamp(&block.end_time)?;
        let now = Utc::now();
        
        let total_tokens = block.token_counts.total();
        let cost_used = block.cost_usd;
        
        let elapsed_minutes = (now - start_time).num_minutes().max(0) as f64;
        let remaining_minutes = (end_time - now).num_minutes().max(0) as f64;
        
        let burn_rate = if elapsed_minutes > 0.0 {
            total_tokens as f64 / elapsed_minutes
        } else {
            0.0
        };
        
        let cost_burn_rate = if elapsed_minutes > 0.0 {
            (cost_used / elapsed_minutes) * 60.0
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "status": "active",
            "tokens": {
                "current": total_tokens,
                "limit": token_limit,
                "percentage": (total_tokens as f64 / token_limit as f64) * 100.0
            },
            "cost": {
                "current": cost_used,
                "limit": budget_limit
            },
            "timing": {
                "elapsed_minutes": elapsed_minutes,
                "remaining_minutes": remaining_minutes,
                "current_time": chrono::Local::now().format("%H:%M").to_string()
            },
            "burn_rates": {
                "tokens_per_minute": burn_rate,
                "cost_per_hour": cost_burn_rate
            },
            "session_count": 1
        }))
    }

    async fn find_active_session_block(&mut self) -> Result<Option<SessionBlock>> {
        let current_time = std::time::Instant::now();
        
        // Use cache if available and recent (30 seconds)
        if let (Some(blocks), Some(cache_time)) = (&self.cached_blocks, &self.cache_time) {
            if current_time.duration_since(*cache_time).as_secs() < 30 {
                let now = Utc::now();
                for block in blocks {
                    if let Ok(end_time) = self.parser.parse_timestamp(&block.end_time) {
                        if end_time > now {
                            return Ok(Some(block.clone()));
                        }
                    }
                }
                return Ok(None);
            }
        }
        
        // Load fresh session blocks
        let blocks = self.parser.load_session_blocks()?;
        let now = Utc::now();
        
        // Find active block
        for block in &blocks {
            if let Ok(end_time) = self.parser.parse_timestamp(&block.end_time) {
                if end_time > now {
                    // Update cache
                    self.cached_blocks = Some(blocks.clone());
                    self.cache_time = Some(current_time);
                    return Ok(Some(block.clone()));
                }
            }
        }
        
        // No active session blocks found - implement fallback to current session data
        if let Ok(current_session_data) = self.get_current_session_data().await {
            if let Some(synthetic_block) = current_session_data {
                // Update cache with synthetic block
                let mut updated_blocks = blocks.clone();
                updated_blocks.push(synthetic_block.clone());
                self.cached_blocks = Some(updated_blocks);
                self.cache_time = Some(current_time);
                return Ok(Some(synthetic_block));
            }
        }
        
        // No active session blocks or current session data found
        self.cached_blocks = Some(blocks.clone());
        self.cache_time = Some(current_time);
        Ok(None)
    }

    fn create_progress_bar(&self, percentage: f64, width: usize, status_color: &str) -> String {
        let pct = percentage.max(0.0).min(100.0);
        
        if pct >= 100.0 {
            let filled = "█".repeat(width);
            return format!("{} {}", status_color, filled);
        }
        
        let filled = (width as f64 * pct / 100.0) as usize;
        let cursor = if pct < 100.0 { 1 } else { 0 };
        let empty = width.saturating_sub(filled).saturating_sub(cursor);
        
        let filled_bar = "█".repeat(filled);
        let cursor_char = if cursor > 0 { "▓" } else { "" };
        let empty_bar = "░".repeat(empty);
        
        format!("{} {}{}{}", status_color, filled_bar, cursor_char, empty_bar)
    }

    fn format_time(&self, minutes: f64) -> String {
        if minutes < 60.0 {
            format!("{}m", minutes as i32)
        } else {
            let hours = (minutes / 60.0) as i32;
            let mins = (minutes % 60.0) as i32;
            if mins == 0 {
                format!("{}h", hours)
            } else {
                format!("{}h {}m", hours, mins)
            }
        }
    }

    fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H");
        io::stdout().flush().unwrap();
    }

    fn hide_cursor(&self) {
        print!("\x1b[?25l");
        io::stdout().flush().unwrap();
    }

    fn show_cursor(&self) {
        print!("\x1b[?25h");
        io::stdout().flush().unwrap();
    }

    async fn get_current_session_data(&self) -> Result<Option<SessionBlock>> {
        
        let paths = self.parser.discover_claude_paths()?;
        let now = Utc::now();
        let cutoff_time = now - chrono::Duration::minutes(10); // Look at last 10 minutes
        let activity_cutoff = now - chrono::Duration::minutes(2); // Recent activity within 2 minutes
        
        let mut total_input_tokens = 0;
        let mut total_output_tokens = 0;
        let mut total_cache_creation_tokens = 0;
        let mut total_cache_read_tokens = 0;
        let mut total_cost = 0.0;
        let mut earliest_activity = None;
        let mut latest_activity = None;
        let mut has_recent_activity = false;
        
        for claude_path in paths {
            let file_tuples = self.parser.find_jsonl_files(&[claude_path])?;
            
            for (jsonl_file, _session_dir) in file_tuples {
                // Check if file was modified recently
                if let Ok(metadata) = std::fs::metadata(&jsonl_file) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time = DateTime::<Utc>::from(modified);
                        if modified_time < cutoff_time {
                            continue;
                        }
                    }
                }
                
                // Parse entries from this file
                let entries = self.parser.parse_jsonl_file(&jsonl_file)?;
                
                for entry in entries {
                    // Skip entries with no usage data
                    let Some(usage) = &entry.message.usage else {
                        continue;  // Skip entries without usage data
                    };
                    if usage.input_tokens == 0 && usage.output_tokens == 0 {
                        continue;
                    }
                    
                    // Parse timestamp
                    if let Ok(timestamp) = self.parser.parse_timestamp(&entry.timestamp) {
                        if timestamp < cutoff_time {
                            continue;
                        }
                        
                        // Check if this is recent activity
                        if timestamp > activity_cutoff {
                            has_recent_activity = true;
                        }
                        
                        // Track earliest and latest activity
                        if earliest_activity.is_none() || timestamp < earliest_activity.unwrap() {
                            earliest_activity = Some(timestamp);
                        }
                        if latest_activity.is_none() || timestamp > latest_activity.unwrap() {
                            latest_activity = Some(timestamp);
                        }
                        
                        // Aggregate usage data
                        if let Some(usage) = &entry.message.usage {
                            total_input_tokens += usage.input_tokens;
                            total_output_tokens += usage.output_tokens;
                            total_cache_creation_tokens += usage.cache_creation_input_tokens;
                            total_cache_read_tokens += usage.cache_read_input_tokens;
                        }
                        
                        // Calculate cost
                        if let Some(cost) = entry.cost_usd {
                            total_cost += cost;
                        } else {
                            // Fallback to cost calculation from tokens
                            let entry_cost = if let Some(usage) = &entry.message.usage {
                                crate::pricing::PricingManager::calculate_cost_from_tokens(
                                    usage, 
                                    &entry.message.model
                                ).await
                            } else {
                                0.0
                            };
                            total_cost += entry_cost;
                        }
                    }
                }
            }
        }
        
        // Only create synthetic session block if we have recent activity
        if !has_recent_activity || earliest_activity.is_none() || latest_activity.is_none() {
            return Ok(None);
        }
        
        let start_time = earliest_activity.unwrap();
        let end_time = latest_activity.unwrap() + chrono::Duration::minutes(10); // Extend end time by 10 minutes
        
        // Create synthetic session block
        let synthetic_block = SessionBlock {
            start_time: start_time.to_rfc3339(),
            end_time: end_time.to_rfc3339(),
            token_counts: TokenCounts {
                input_tokens: total_input_tokens,
                output_tokens: total_output_tokens,
                cache_creation_input_tokens: total_cache_creation_tokens,
                cache_read_input_tokens: total_cache_read_tokens,
            },
            cost_usd: total_cost,
        };
        
        Ok(Some(synthetic_block))
    }
}