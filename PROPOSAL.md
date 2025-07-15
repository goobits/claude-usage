# CLAUDE USAGE IMPROVEMENT PROPOSAL

## Overview
This proposal outlines a 5-phase implementation plan to improve Claude Usage with proper 5-hour window detection, interactive plan configuration, and complete Rust-Python parity.

## Phase 1: Interactive Plan Configuration
**Goal**: Add user-friendly plan selection on first run

### Files to Modify:
1. **rust/src/models.rs** (line ~50)
   - Add `PlanType` enum:
   ```rust
   pub enum PlanType {
       Pro,     // 200K tokens
       Max5,    // 400K tokens  
       Max20,   // 880K tokens
   }
   ```

2. **rust/src/config.rs** (NEW FILE)
   - Create config management module
   - Functions to add:
     - `load_config() -> Option<Config>`
     - `save_config(config: &Config) -> Result<()>`
     - `prompt_for_plan() -> PlanType`
   - Config location: `~/.claude-usage/config.json`

3. **rust/src/main.rs** (line ~150, in run_command)
   - Before executing commands, check for config:
   ```rust
   let config = config::load_config()
       .unwrap_or_else(|| {
           let plan = config::prompt_for_plan();
           let config = Config::new(plan);
           config::save_config(&config).unwrap();
           config
       });
   ```

4. **rust/src/cli.rs** (line ~30)
   - Add `--plan` flag to Args struct
   - Add new `config` subcommand

### Config Format:
```json
{
  "plan": "max20",
  "token_limits": {
    "pro": 200000,
    "max5": 400000,
    "max20": 880000
  },
  "window_duration_hours": 5
}
```

## Phase 2: 5-Hour Window Detection
**Goal**: Implement gap-based window detection for accurate usage tracking

### Files to Modify:
1. **rust/src/windows.rs** (NEW FILE)
   - Core window detection logic
   - Functions to add:
     - `detect_usage_windows(entries: Vec<Entry>) -> Vec<Window>`
     - `find_window_gaps(entries: &[Entry]) -> Vec<usize>`
     - `split_at_boundaries(entries: Vec<Entry>, boundary: DateTime) -> (Vec<Entry>, Vec<Entry>)`

2. **rust/src/parser.rs** (line ~100)
   - Modify `parse_entries()` to collect ALL timestamps first
   - Remove current session-based grouping
   - Add `sort_by_timestamp()` helper

3. **rust/src/analyzer.rs** (line ~200)
   - Replace session-based analysis with window-based
   - Update `analyze_usage()` to call `windows::detect_usage_windows()`
   - Remove `group_by_session()` calls

4. **rust/src/models.rs** (line ~80)
   - Add `Window` struct:
   ```rust
   pub struct Window {
       pub start_time: DateTime<Utc>,
       pub end_time: DateTime<Utc>,
       pub entries: Vec<UsageEntry>,
       pub total_tokens: TokenCounts,
       pub exceeded_limit: bool,
   }
   ```

### Window Detection Algorithm:
```
1. Parse all JSONL entries across all files
2. Sort globally by timestamp
3. Find gaps >5 hours between consecutive entries
4. Create windows at gap boundaries
5. Split conversations that span boundaries
```

## Phase 3: Fix Remaining Parity Issues
**Goal**: Resolve final 0.2% precision differences between Rust and Python

### Files to Investigate:
1. **rust/src/pricing.rs** vs **claude_usage.py** (lines 70-100)
   - Compare cost calculation precision
   - Check for f32 vs f64 differences
   - Verify pricing data matches exactly

2. **rust/src/dedup.rs** (line ~150)
   - Verify deduplication timing matches Python
   - Check hash calculation consistency
   - Ensure messageId:requestId format identical

3. **rust/src/parser.rs** (line ~50)
   - Compare token extraction logic
   - Verify all token types counted
   - Check for edge cases in parsing

### Specific Checks:
- Float precision: Ensure all calculations use f64
- Rounding: Match Python's rounding behavior
- Token counting: Include all token types consistently
- Cost aggregation: Sum in same order as Python

## Phase 4: Update Live Monitoring
**Goal**: Show accurate window-based progress instead of hardcoded limits

### Files to Modify:
1. **rust/src/monitor.rs** (line ~25)
   - Remove hardcoded TOKEN_LIMIT constant
   - Replace with config-based limits:
   ```rust
   let token_limit = match config.plan {
       PlanType::Pro => 200_000,
       PlanType::Max5 => 400_000,
       PlanType::Max20 => 880_000,
   };
   ```

2. **rust/src/monitor.rs** (line ~274, find_active_session_block)
   - Replace session-based logic with window detection
   - Show current window instead of session
   - Calculate time remaining in window

3. **rust/src/monitor.rs** (line ~112, display_active_session)
   - Update displays to show:
     - Current 5-hour window progress
     - Time until window reset
     - Tokens used vs plan limit
     - Better burn rate calculation

### Display Updates:
```
⚡ Tokens: [████████░░] 440K / 880K (Max20)
⏰ Window: 2h 15m remaining (resets at 14:30)
🔥 Burn rate: 3,200 tok/min
```

## Phase 5: Performance Optimization
**Goal**: Cache processed data to avoid reprocessing

### Files to Create/Modify:
1. **rust/src/cache.rs** (NEW FILE)
   - Window cache management
   - Functions to add:
     - `load_cache() -> Option<WindowCache>`
     - `save_cache(windows: &[Window]) -> Result<()>`
     - `merge_new_entries(cache: WindowCache, new: Vec<Entry>) -> WindowCache`

2. **rust/src/analyzer.rs** (line ~100)
   - Check cache before processing
   - Only process new entries since last cache
   - Update cache after processing

3. **Cache Format Options**:
   - JSON for simplicity: `~/.claude-usage/window_cache.json`
   - Parquet for performance (if combining with other project)
   - Include last_processed timestamp

### Cache Strategy:
```
1. Load existing cache
2. Find last processed timestamp
3. Only parse JSONL entries after that timestamp
4. Merge new windows with cached
5. Save updated cache
```

## Implementation Notes

### Testing Strategy:
- Each phase should maintain backward compatibility
- Test against Python version for parity
- Verify window detection with edge cases
- Ensure config migration for existing users

### Error Handling:
- Graceful fallbacks for missing config
- Clear error messages for limit hits
- Handle corrupted cache files
- Timezone handling for global users

### Future Enhancements:
- Multi-user support (if Claude adds user IDs)
- Webhook notifications for limit approaching
- Historical window analysis
- Export to various formats (CSV, Parquet)

## Success Metrics
1. Window detection accurately captures 5-hour usage patterns
2. Plan configuration reduces user confusion
3. Rust-Python parity within 0.1%
4. Live monitoring shows real-time window progress
5. Performance: <1 second for daily analysis with cache