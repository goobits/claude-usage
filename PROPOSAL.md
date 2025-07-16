# Claude Usage: Integration with Claude Keeper Proposal

## Implementation Order
1. **claude-keeper** builds Rust-based foundation + provides stable APIs
2. **claude-insights** migrates to use claude-keeper (removes storage code)  
3. **claudeusage** migrates to use claude-keeper (removes data handling)

## Vision

Migrate claudeusage from standalone data handling to leverage claude-keeper's shared parquet infrastructure while maintaining high-performance usage tracking and real-time monitoring capabilities.

## Core Migration

**From**: Standalone JSONL parsing and data discovery  
**To**: High-performance consumer of claude-keeper's shared data platform

## Architecture Integration

### Current Capabilities (Remove)
- JSONL file discovery and parsing
- Multi-VM conversation data collection
- Deduplication logic
- Raw conversation file processing

### Enhanced Capabilities (Keep & Improve)
- Real-time usage monitoring with live progress bars
- 5-hour window detection for accurate usage tracking
- Token and cost analysis with burn rate calculations
- High-performance Rust implementation
- Sub-second analysis performance

## New Architecture

```
claudeusage/
├── rust/src/
│   ├── main.rs               # CLI entry point
│   ├── monitor.rs            # Enhanced: Real-time monitoring
│   ├── analyzer.rs           # Enhanced: Usage analysis via claude-keeper
│   ├── windows.rs            # New: 5-hour window detection
│   ├── config.rs             # New: Plan configuration (Pro/Max5/Max20)
│   ├── display.rs            # Existing: Output formatting
│   └── models.rs             # Simplified: Usage-specific models
├── claude_usage.py           # Simplified: Python wrapper for compatibility
└── pyproject.toml            # Updated: Add claude-keeper dependency
```

## Integration Strategy

### Phase 1: Add claude-keeper Dependency
1. **Add claude-keeper to requirements** and cargo dependencies
2. **Replace data discovery** with claude-keeper Rust APIs:
   ```rust
   use claude_keeper::storage::ConversationLoader;
   use claude_keeper::discovery::find_conversation_sources;
   ```
3. **Remove local JSONL parsing** and file discovery code

### Phase 2: Migrate Core Functions
1. **Update analyzer.rs** to use claude-keeper's API:
   ```rust
   use claude_keeper::{ConversationLoader, DataMode, OptimizeFor};
   
   let loader = ConversationLoader::new();
   let conversations = loader.load_conversations(
       Some(date_range),
       None, // all projects
       None, // all VMs
       DataMode::Hybrid,
       OptimizeFor::Usage,
   )?;
   ```
2. **Remove deduplication logic** (handled by claude-keeper)
3. **Keep specialized usage calculations** (5-hour windows, burn rates)

### Phase 3: Enhance Usage-Specific Features
1. **Implement 5-hour window detection** using claude-keeper's clean data
2. **Add interactive plan configuration** (Pro/Max5/Max20)
3. **Optimize live monitoring** with claude-keeper's efficient data access

## Key Benefits

### Code Simplification
- **Remove 40% of data handling code** (discovery, parsing, deduplication)
- **Focus on usage-specific analysis** rather than data infrastructure
- **Maintain Rust performance** while leveraging shared parquet storage

### Enhanced Reliability
- **Consistent data** across all Claude analysis tools
- **Format resilience** via claude-keeper's schema monitoring
- **Reduced maintenance** of data pipeline code

### Improved Features
- **Faster startup** using pre-processed parquet data
- **More accurate analysis** with claude-keeper's deduplication
- **Better multi-VM support** via shared infrastructure

## Usage-Specific Optimizations

### Near Real-Time Monitoring (3-second updates)
```rust
// Live monitoring approach
let loader = ConversationLoader::new();

// Get cached historical data
let historical = loader.load_conversations(
    Some(older_date_range),
    None, None,
    DataMode::CacheOnly,  // Fast cached data
    OptimizeFor::Usage,
)?;

// Get live updates every 3 seconds
let live_updates = loader.get_live_updates(
    last_check_timestamp,
    OptimizeFor::Usage,
)?;
```

**Benefits:**
- **No cache corruption risk**: Live monitoring bypasses cache entirely
- **Speed**: Historical data from fast cache, only recent files parsed live
- **Safety**: Atomic cache updates with file locking
- **Efficiency**: Only check modification times every 3 seconds

## Cross-Tool Dependencies

### Prerequisite: claude-keeper Phase 2 completion (ConversationLoader API ready)
### Coordination: claudeusage migrates itself after claude-keeper APIs are stable  
### Self-Migration: claudeusage team handles its own migration to claude-keeper
### No Performance Issues: Direct Rust-to-Rust API calls, no language boundary overhead

---

*This proposal transforms claudeusage into a specialized, high-performance usage analysis tool that leverages shared infrastructure while maintaining its unique real-time monitoring capabilities.*