# PROPOSAL: Real-Time Claude Usage Intelligence Platform

**Status:** Draft  
**Date:** 2025-01-24  
**Author:** Research Analysis  
**Target:** Claude Usage Enhanced Architecture  

## Executive Summary

Transform the current polling-based Claude usage tool into a real-time intelligence platform using file system watchers, streaming JSONL parsing, and WebSocket event distribution. This enables live developer analytics, sentiment analysis, and proactive usage insights.

## Current State Analysis

### What We Have
- **Polling-based monitoring** (3-second intervals)
- **30-second caching** for session blocks
- **Disk I/O overhead** on every poll cycle
- **Batch processing** of JSONL files
- **Terminal-only display** with basic progress bars

### Performance Limitations
- Fixed 3-second latency for all updates
- Unnecessary disk reads when no changes occur
- Single-threaded file processing
- No real-time event propagation
- Limited to CLI consumption

## Proposed Architecture

### Core Components

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   File System   │    │    Streaming     │    │   Event Bus &   │
│    Watchers     │───▶│  JSONL Parser    │───▶│   WebSockets    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                        │                        │
         ▼                        ▼                        ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ • ~/.claude/    │    │ • Position       │    │ • Live Dashboard│
│ • projects/*/   │    │   Tracking       │    │ • CLI Monitor   │
│ • *.jsonl       │    │ • Incremental    │    │ • Browser Ext   │
│ • session_*.json│    │   Parsing        │    │ • API Clients   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Technology Stack

**File Watching**
```toml
notify = "6.0"              # Cross-platform file system events
tokio-stream = "0.1"        # Async stream processing
```

**Streaming Processing**
```toml
tokio-util = "0.7"          # Codec for line-by-line parsing
serde_json = "1.0"          # JSON parsing (existing)
```

**WebSocket Server**
```toml
tokio-tungstenite = "0.20"  # WebSocket server implementation
tower = "0.4"               # Service framework
axum = "0.7"                # Web framework for REST API
```

**Analytics Engine**
```toml
regex = "1.0"               # Pattern matching for sentiment
lru = "0.12"                # LRU cache for deduplication
# tokenizers = "0.15"         # Text analysis (would need message content)
```

## Implementation Phases

### Phase 1: File System Watching (Weeks 1-2)

**Goals:**
- Replace polling with event-driven file monitoring
- Maintain backward compatibility with existing CLI

**Deliverables:**
```rust
struct FileWatcher {
    watcher: RecommendedWatcher,
    event_tx: mpsc::Sender<FileEvent>,
}

enum FileEvent {
    JsonlAppend { path: PathBuf, new_lines: Vec<String> },
    SessionBlockUpdate { path: PathBuf, block: SessionBlock },
    FileCreated { path: PathBuf },
}
```

**Testing:**
- Performance benchmarks vs current polling
- Multi-VM instance monitoring
- File rotation handling

### Phase 2: Streaming JSONL Parser (Weeks 3-4)

**Goals:**
- Track file positions to read only new content
- Parse incremental updates in real-time
- Maintain global deduplication

**Deliverables:**
```rust
struct StreamingParser {
    file_positions: HashMap<PathBuf, u64>,
    dedup_cache: LruCache<String, ()>,  // Requires: lru = "0.12" in Cargo.toml
}

impl StreamingParser {
    async fn process_new_lines(&mut self, path: &Path, lines: Vec<String>) -> Vec<UsageEvent>;
}
```

**Features:**
- Position persistence across restarts
- Error recovery for corrupted lines
- Memory-efficient LRU caching

### Phase 3: WebSocket Event Distribution (Weeks 5-6)

**Goals:**
- Real-time event broadcasting to multiple clients
- Filtered subscriptions (project-specific, token thresholds)
- RESTful API for historical data

**Deliverables:**
```rust
#[derive(Serialize)]
enum UsageEvent {
    NewMessage { entry: UsageEntry, sentiment_score: f32 },
    SessionStart { session_id: String, project: String },
    TokenThreshold { current: u32, limit: u32, percentage: f32 },
    BudgetAlert { cost: f64, burn_rate: f64 },
}

struct WebSocketServer {
    subscribers: HashMap<ClientId, SubscriptionFilter>,
    event_bus: broadcast::Receiver<UsageEvent>,
}
```

**API Endpoints:**
- `GET /api/usage/live` - WebSocket connection
- `GET /api/usage/history` - Historical data
- `POST /api/subscriptions` - Filter management

### Phase 4: Intelligence & Analytics (Weeks 7-8)

**Goals:**
- Real-time sentiment analysis
- Usage pattern detection
- Predictive analytics

**Analytics Features:**

**Sentiment Analysis:**
```rust
// LIMITATION: Current JSONL format only contains token counts, not message content
// Sentiment analysis would require:
// 1. Claude Code to log message content (privacy concerns)
// 2. Integration with Claude Code's internal message store
// 3. Real-time interception during conversation

struct SentimentAnalyzer {
    frustration_patterns: Vec<Regex>,
    satisfaction_indicators: Vec<String>,
}

#[derive(Serialize)]
struct SentimentScore {
    user_sentiment: f32,      // -1.0 (frustrated) to 1.0 (satisfied)
    claude_confidence: f32,   // 0.0 (uncertain) to 1.0 (confident)
    conversation_health: f32, // Overall interaction quality
}
```

**Usage Intelligence:**
```rust
#[derive(Serialize)]
struct UsageInsight {
    productivity_score: f32,     // Cost per meaningful output
    iteration_efficiency: f32,   // Successful first responses
    model_effectiveness: HashMap<String, f32>,
    predicted_budget_depletion: DateTime<Utc>,
}
```

## Real-Time Dashboard Features

### Live Developer Experience
- **Token Usage Preview**: Show current token consumption as responses are generated (Note: actual message content not available in current JSONL format)
- **Sentiment Heatmap**: Visual representation of user frustration/satisfaction over time
- **Token Burn Visualization**: Real-time graph of token consumption rate
- **Multi-Model Comparison**: Live performance metrics across different Claude models

### Proactive Alerts
- **Frustration Detection**: Alert when user sentiment drops significantly
- **Budget Burn Warnings**: Predictive alerts before hitting limits
- **Conversation Health**: Identify when interactions become unproductive
- **Peak Usage Predictions**: Forecast high usage periods

### Analytics Dashboard
```javascript
// WebSocket client connection
const ws = new WebSocket('ws://localhost:8080/api/usage/live');

ws.onmessage = (event) => {
    const usageEvent = JSON.parse(event.data);
    
    switch(usageEvent.type) {
        case 'NewMessage':
            updateSentimentGraph(usageEvent.sentiment_score);
            updateTokenCounter(usageEvent.entry.message.usage);
            break;
            
        case 'TokenThreshold':
            showBurnRateAlert(usageEvent.percentage);
            break;
    }
};
```

## Performance Expectations

### Current vs Proposed
| Metric | Current (Polling) | Proposed (Event-Driven) |
|--------|-------------------|-------------------------|
| **Latency** | 3 seconds fixed | ~50ms (file event → client) |
| **CPU Usage** | Continuous polling | Event-driven (idle when quiet) |
| **Disk I/O** | Every 3 seconds | Only on actual changes |
| **Memory** | Full file reads | Incremental line processing |
| **Scalability** | Single terminal | Multiple concurrent clients |

### Benchmarking Targets
- **Event propagation**: < 100ms from file write to WebSocket delivery
- **Memory usage**: < 50MB for 10,000 active sessions  
- **Concurrent clients**: Support 100+ WebSocket connections
- **File processing**: Handle 1000+ JSONL entries/second

**Note**: These targets assume optimal conditions. Real-world performance may vary based on:
- File system type (SSD vs HDD)
- Network latency for WebSocket clients
- JSONL file size and complexity

## Migration Strategy

### Backward Compatibility
- Existing CLI commands remain unchanged
- Add new `--live-server` flag for WebSocket mode
- Gradual feature rollout with feature flags

### Deployment Options
```bash
# Traditional CLI (unchanged)
claude-usage daily --limit 10

# Start real-time server
claude-usage live-server --port 8080

# Connect live CLI to server
claude-usage live --connect ws://localhost:8080
```

## Risk Assessment

### Technical Risks
- **File system event reliability** across different OS platforms
- **WebSocket connection management** for unstable networks  
- **Memory growth** from position tracking and caching
- **Concurrent access** to JSONL files during writes
- **Limited sentiment analysis**: Current JSONL format doesn't include message content
- **File append detection**: Distinguishing new lines from file modifications
- **Cross-platform file locking**: Handling concurrent writes by Claude Code

### Mitigation Strategies
- Comprehensive cross-platform testing (Linux, macOS, Windows)
- Connection retry logic and graceful degradation
- Configurable cache limits and cleanup routines
- File locking and atomic write detection

## Success Metrics

### Technical KPIs
- Event latency < 100ms (95th percentile)
- Zero data loss during file rotation
- Memory usage growth < 1MB/hour under normal load
- 99.9% uptime for WebSocket server

### User Experience KPIs
- Event propagation latency < 100ms (95th percentile)
- Budget prediction accuracy within 10%
- Developer productivity insights adoption
- Reduced time-to-insight for usage analysis
- Note: Full sentiment analysis requires message content access

## Future Enhancements

### Advanced Analytics
- **Machine Learning Models**: Train on usage patterns for personalized insights
- **Cross-Session Analysis**: Track developer behavior across multiple projects
- **Team Collaboration Metrics**: Multi-user usage pattern analysis
- **Integration APIs**: Connect with IDEs, project management tools

### Ecosystem Integration
- **VS Code Extension**: Live usage display in editor status bar
- **Slack/Discord Bots**: Team usage notifications and alerts
- **Grafana Dashboard**: Enterprise monitoring integration
- **Claude Code Plugin**: Direct integration with Claude Code CLI

## Conclusion

This proposal transforms the Claude usage tool from a periodic reporting utility into a comprehensive real-time intelligence platform. The event-driven architecture enables immediate insights, proactive alerts, and rich developer experience analytics that were impossible with the current polling approach.

The phased implementation approach ensures we can deliver incremental value while maintaining stability and backward compatibility. The WebSocket-based event distribution opens up endless possibilities for client applications and integrations.

**Recommendation**: Proceed with Phase 1 (File System Watching) as a proof of concept, with full implementation targeting an 8-week timeline.

---

*This proposal represents a significant architectural evolution that positions the Claude usage tool as a foundational platform for AI-assisted development analytics.*