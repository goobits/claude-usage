use crate::models::*;
use crate::parser::FileParser;
use crate::dedup::{DeduplicationEngine, ProcessOptions};
use crate::display::DisplayManager;
use crate::monitor::LiveMonitor;
use anyhow::Result;

pub struct ClaudeUsageAnalyzer {
    parser: FileParser,
    dedup_engine: DeduplicationEngine,
    display_manager: DisplayManager,
    live_monitor: LiveMonitor,
}

impl ClaudeUsageAnalyzer {
    pub fn new(cost_mode: CostMode) -> Self {
        Self {
            parser: FileParser::new(cost_mode.clone()),
            dedup_engine: DeduplicationEngine::new(cost_mode.clone()),
            display_manager: DisplayManager::new(),
            live_monitor: LiveMonitor::new(cost_mode),
        }
    }

    pub async fn aggregate_data(&self, _command: &str, options: ProcessOptions) -> Result<Vec<SessionOutput>> {
        // Discover Claude paths
        let paths = self.parser.discover_claude_paths()?;
        
        if !options.json_output {
            println!("🔍 Discovered {} Claude instances", paths.len());
        }
        
        // Find all JSONL files
        let mut all_jsonl_files = Vec::new();
        let mut files_filtered = 0;
        
        for claude_path in &paths {
            let file_tuples = self.parser.find_jsonl_files(&[claude_path.clone()])?;
            
            for (jsonl_file, session_dir) in file_tuples {
                // Pre-filter files by date range
                if self.parser.should_include_file(&jsonl_file, options.since_date.as_ref(), options.until_date.as_ref()) {
                    all_jsonl_files.push((jsonl_file, session_dir));
                } else {
                    files_filtered += 1;
                }
            }
        }
        
        if !options.json_output {
            if files_filtered > 0 {
                println!("📁 Found {} JSONL files (filtered out {} by date)", all_jsonl_files.len(), files_filtered);
            } else {
                println!("📁 Found {} JSONL files across all instances", all_jsonl_files.len());
            }
        }
        
        // Sort files by timestamp
        let sorted_files = self.parser.sort_files_by_timestamp(all_jsonl_files);
        
        // Process with global deduplication
        self.dedup_engine.process_files_with_global_dedup(sorted_files, &options).await
    }

    pub async fn run_command(&mut self, command: &str, options: ProcessOptions) -> Result<()> {
        match command {
            "live" => {
                self.live_monitor.run_live_monitor(options.json_output, options.snapshot).await
            }
            _ => {
                let data = self.aggregate_data(command, options.clone()).await?;
                
                if data.is_empty() {
                    if options.json_output {
                        println!("[]");
                    } else {
                        println!("No Claude usage data found across all instances.");
                    }
                    return Ok(());
                }
                
                match command {
                    "daily" => self.display_manager.display_daily(&data, options.last, options.json_output),
                    "monthly" => self.display_manager.display_monthly(&data, options.last, options.json_output),
                    "session" => self.display_manager.display_session(&data, options.last, options.json_output),
                    "blocks" => {
                        // Load session blocks for blocks command (without time filtering)
                        let blocks = self.parser.load_session_blocks_with_filter(false)?;
                        self.display_manager.display_blocks(&blocks, options.last, options.json_output);
                    }
                    _ => {
                        anyhow::bail!("Unknown command: {}", command);
                    }
                }
                
                Ok(())
            }
        }
    }
}