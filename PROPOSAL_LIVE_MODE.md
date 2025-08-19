# Proposal: Live Mode Architecture for claude-usage

## Executive Summary
Implement a real-time monitoring mode for claude-usage that provides live updates with minimal memory footprint by leveraging claude-keeper's streaming capabilities.

## Motivation
Users need to monitor Claude Desktop usage in real-time without:
- Manual refresh/re-running commands
- Waiting for full dataset processing
- High memory consumption
- Complex state management

## Proposed Architecture

### Core Concept: Baseline + Stream
```
Parquet Baseline (startup) → Real-time Updates (claude-keeper watch) → Live Display
```

### Memory Profile
- Baseline Summary: ~1MB (aggregates only, not full data)
- Ring Buffer: ~100KB (last 100 entries for display)
- Display State: ~10KB (current totals)
- **Total: <2MB constant** (vs current approach of unbounded growth)

## Implementation Plan

### Phase 1: Foundation Components

#### 1.1 CLI Structure
```rust
// New command in main.rs
enum Commands {
    Live {
        #[arg(long)]
        no_baseline: bool,  // Skip initial backup
    }
}
```

#### 1.2 Directory Structure
```
claude-usage/
├── src/
│   ├── commands/
│   │   ├── mod.rs
│   │   └── live.rs         # Core orchestration
│   ├── live/
│   │   ├── mod.rs
│   │   ├── baseline.rs     # Parquet summary reader
│   │   └── watcher.rs       # Claude-keeper integration
│   ├── display/
│   │   ├── mod.rs
│   │   ├── tui.rs          # Terminal UI
│   │   └── state.rs        # Display state management
│   └── parquet/
│       └── reader.rs        # Summary-only parquet reader
```

### Phase 2: Core Flow

#### 2.1 Startup Sequence
```rust
async fn run_live_mode(no_baseline: bool) -> Result<()> {
    // 1. Create/load baseline
    let baseline = if !no_baseline {
        ensure_fresh_baseline().await?  // Runs backup if >5min old
    } else {
        BaselineSummary::default()
    };
    
    // 2. Initialize display
    let display = LiveDisplay::new(baseline);
    
    // 3. Start claude-keeper watch
    let watcher = start_keeper_watch(display.clone());
    
    // 4. Run until user exits
    display.run_until_exit().await;
    
    Ok(())
}
```

#### 2.2 Claude-Keeper Integration
```rust
async fn start_keeper_watch(display: Arc<Mutex<LiveDisplay>>) -> Result<()> {
    let mut child = Command::new("claude-keeper")
        .args(["watch", "~/.claude", "--format", "json", "--auto-process"])
        .stdout(Stdio::piped())
        .spawn()?;
    
    // Stream JSON updates to display
    let stdout = BufReader::new(child.stdout.take().unwrap());
    for line in stdout.lines() {
        if let Ok(update) = serde_json::from_str::<Update>(&line?) {
            display.lock().unwrap().update(update);
        }
    }
    
    Ok(())
}
```

#### 2.3 Baseline Summary Structure
```rust
pub struct BaselineSummary {
    pub total_cost: f64,
    pub total_tokens: u64,
    pub sessions_today: u32,
    pub last_backup: SystemTime,
    // Just aggregates, no individual entries!
}

impl BaselineSummary {
    pub async fn from_parquet(path: &Path) -> Result<Self> {
        // Use arrow compute kernels for aggregation
        // Don't load individual records
    }
}
```

### Phase 3: Display Implementation

#### 3.1 TUI Layout
```
┌─ Claude Usage Live ─────────────────────────┐
│ Total: $45.23 | Tokens: 1.2M | Sessions: 15 │
├──────────────────────────────────────────────┤
│ Current Session                              │
│ ├─ Cost: $2.10                              │
│ ├─ Duration: 5m 23s                         │
│ └─ Tokens: In 10K / Out 15K                 │
├──────────────────────────────────────────────┤
│ Recent Activity (↑/↓ to scroll)             │
│ [12:05:23] Project A: +500 tokens ($0.05)   │
│ [12:04:15] Project B: +1200 tokens ($0.12)  │
│ [12:03:45] Project A: +300 tokens ($0.03)   │
└─────────────────────── Ctrl+C to exit ──────┘
```

#### 3.2 State Management
```rust
pub struct LiveDisplay {
    baseline: BaselineSummary,
    recent_entries: VecDeque<Entry>,  // Ring buffer (max 100)
    current_session: Option<SessionStats>,
    running_totals: RunningTotals,
}

impl LiveDisplay {
    pub fn update(&mut self, entry: Entry) {
        // Update totals
        self.running_totals.add(&entry);
        
        // Maintain ring buffer
        if self.recent_entries.len() >= 100 {
            self.recent_entries.pop_front();
        }
        self.recent_entries.push_back(entry);
        
        // Trigger render
        self.render();
    }
}
```

## Configuration

### New Configuration Section
```toml
# claude-usage.toml
[live]
baseline_ttl = "5m"              # Reuse baseline if fresh
ring_buffer_size = 100           # Recent entries to display
update_interval = "1s"           # UI refresh rate
claude_keeper_path = "claude-keeper"  # Path to binary
auto_backup = true               # Run backup on startup
```

## Dependencies

### New Cargo.toml Additions
```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Terminal UI
crossterm = "0.27"
ratatui = "0.26"

# Already have
claude-keeper = "1.0.0-rc1"
parquet = "50.0"
arrow = "50.0"
```

## Benefits

### Performance
- **Constant Memory**: O(1) instead of O(n)
- **Instant Updates**: Real-time via file watching
- **Fast Startup**: Cached baseline (<2s)
- **Low CPU**: <5% usage during monitoring

### User Experience
- **Zero Configuration**: Smart defaults
- **Just Works**: Single command to start
- **Clean Display**: Intuitive TUI
- **Responsive**: Immediate feedback

### Architecture
- **Separation of Concerns**: Clean boundaries
- **Leverages claude-keeper**: Full capability usage
- **Streaming Model**: No accumulation
- **Graceful Degradation**: Works even if baseline fails

## Migration Path

1. **Parallel Development**: Build alongside existing code
2. **Feature Flag**: `--experimental-live` initially
3. **Testing Period**: Beta users test for 1 week
4. **Default Switch**: Make live mode the default
5. **Legacy Removal**: Remove old accumulation code

## Success Criteria

### Quantitative
- Memory usage: <5MB after 24h runtime
- Startup time: <2s with cached baseline
- Update latency: <100ms from file change to display
- Zero memory leaks

### Qualitative
- Users prefer it over current approach
- No configuration required for basic usage
- Intuitive display that "just makes sense"
- Handles edge cases gracefully

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| claude-keeper API changes | Use JSON output (stable contract) |
| Subprocess crashes | Restart with exponential backoff |
| TUI performance | Batch updates, differential rendering |
| Baseline calculation slow | Progress bar, --no-baseline option |

## Timeline

- **Week 1**: Core implementation
- **Week 2**: TUI polish & error handling
- **Week 3**: Testing & documentation
- **Week 4**: Release as experimental feature

## Alternative Approaches Considered

1. **In-process watching**: Too complex, duplicates claude-keeper
2. **Database backend**: Overengineered for this use case
3. **Web UI**: Adds complexity, most users want CLI
4. **Polling approach**: Inefficient compared to file watching

## Conclusion

This architecture provides a clean, efficient solution for real-time Claude usage monitoring by:
- Leveraging claude-keeper's existing capabilities
- Maintaining minimal memory footprint
- Providing excellent user experience
- Following Unix philosophy (do one thing well)

The implementation is straightforward, testable, and maintainable.