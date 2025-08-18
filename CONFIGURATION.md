# Claude Usage Configuration Guide

## Configuration Sources (Priority Order)

1. **Environment Variables** (highest priority)
2. **Configuration File** 
3. **Built-in Defaults** (lowest priority)

## Configuration File Locations

The application searches for configuration files in this order:
1. `./claude-usage.toml` (current directory)
2. `./.claude-usage.toml` (hidden file in current directory)
3. `~/.config/claude-usage/config.toml` (user config directory)

## Environment Variables

### Logging
- `LOG_LEVEL` - Logging level (DEBUG, INFO, WARN, ERROR)
- `LOG_FORMAT` - Output format (pretty, json)
- `LOG_OUTPUT` - Output destination (console, file, both)

### Processing
- `CLAUDE_USAGE_BATCH_SIZE` - Files to process in parallel (default: 10)
- `CLAUDE_USAGE_PARALLEL_CHUNKS` - Parallel processing threads (default: 4)

### Memory
- `CLAUDE_USAGE_MAX_MEMORY_MB` - Maximum memory usage in MB (default: 512)
- `CLAUDE_USAGE_BUFFER_SIZE_KB` - Stream buffer size in KB (default: 8)

### Deduplication
- `CLAUDE_USAGE_DEDUP_WINDOW_HOURS` - Dedup time window (default: 24)
- `CLAUDE_USAGE_DEDUP_ENABLED` - Enable/disable dedup (default: true)

### Paths
- `CLAUDE_HOME` - Claude Desktop directory (default: ~/.claude)
- `CLAUDE_VMS_DIR` - VMs directory (default: ~/.claude/vms)
- `CLAUDE_LOG_DIR` - Log file directory (default: ./logs)

## Example Usage

### Development
```bash
export LOG_LEVEL=DEBUG
export LOG_FORMAT=pretty
export CLAUDE_USAGE_BATCH_SIZE=5
./claude-usage daily
```

### Production
```bash
export LOG_LEVEL=INFO
export LOG_FORMAT=json
export LOG_OUTPUT=both
export CLAUDE_USAGE_MAX_MEMORY_MB=1024
./claude-usage daily --json
```

### Docker
```dockerfile
ENV LOG_LEVEL=INFO
ENV LOG_FORMAT=json
ENV CLAUDE_USAGE_MAX_MEMORY_MB=512
ENV CLAUDE_USAGE_DEDUP_ENABLED=true
```

## Performance Tuning

### Low Memory Systems (<512MB)
```toml
[memory]
max_memory_mb = 256
buffer_size_kb = 4

[processing]
batch_size = 5
parallel_chunks = 2
```

### High Performance Systems (>2GB)
```toml
[memory]
max_memory_mb = 2048
buffer_size_kb = 32

[processing]
batch_size = 50
parallel_chunks = 8
```

## Troubleshooting

### Out of Memory
- Reduce `CLAUDE_USAGE_MAX_MEMORY_MB`
- Reduce `CLAUDE_USAGE_BATCH_SIZE`
- Reduce `CLAUDE_USAGE_BUFFER_SIZE_KB`

### Slow Processing
- Increase `CLAUDE_USAGE_BATCH_SIZE`
- Increase `CLAUDE_USAGE_PARALLEL_CHUNKS`
- Increase `CLAUDE_USAGE_BUFFER_SIZE_KB`

### High CPU Usage
- Reduce `CLAUDE_USAGE_PARALLEL_CHUNKS`
- Reduce `CLAUDE_USAGE_BATCH_SIZE`