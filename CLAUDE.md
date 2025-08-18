# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Usage is a high-performance Rust CLI tool for analyzing Claude usage data across multiple VMs and instances. It aggregates token usage, costs, and session data from Claude Code's local storage to provide comprehensive usage reports and real-time monitoring.

## Architecture

The application is built using a modular Rust architecture with these key components:

- **Data Discovery**: Automatically finds Claude instances in `~/.claude/` and `~/.claude/vms/*/`
- **Global Deduplication**: Uses messageId:requestId hashing to prevent duplicate counting across VMs
- **Cost Calculation**: Fetches live pricing from LiteLLM API with fallback to hardcoded rates
- **Multiple Display Modes**: Daily, monthly, and live monitoring views
- **Performance Optimization**: File-level date filtering, streaming JSON parsing, and time-windowed deduplication

### Production Features (New)

- **Claude-Keeper Integration**: Schema-resilient parsing that handles field name variations (camelCase/snake_case)
- **Structured Logging**: JSON logging for production with tracing spans and correlation IDs
- **Streaming Parser**: Memory-safe line-by-line processing (8KB buffer for any file size)
- **Configuration System**: Environment variables and TOML config files for runtime tuning
- **Memory Safety**: Bounded memory usage with configurable limits and monitoring
- **Error Recovery**: Continues processing despite malformed JSON lines

### Key Data Flow

1. **Discovery**: Scans `~/.claude/projects/*/` directories for `*.jsonl` files
2. **Filtering**: Pre-filters files by modification time and date ranges
3. **Processing**: Streams JSONL entries, deduplicates globally, aggregates by session
4. **Display**: Formats output as text tables or JSON based on command

### Live Monitoring

The live monitor (`claude-usage live`) provides real-time usage tracking with:
- Token burn rate calculation from actual conversation timelines
- Session block detection with 30-second caching
- Progress bars for token limits and budget tracking
- Multi-VM session aggregation

## Installation & Common Commands

### Prerequisites

**Install Rust if not already present:**
```bash
# Check if Rust is installed
which rustc && rustc --version

# If not installed, install Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Rust installation locations:**
- Binary: `~/.cargo/bin/rustc` and `~/.cargo/bin/cargo`
- Configuration: `~/.cargo/config.toml`
- Environment: `~/.cargo/env` (sourced in shell profile)

### Building and Running

**Use cargo for installation** as this is a Rust CLI tool:

```bash
# Production build with keeper integration
cargo build --release --features keeper-integration

# The binary will be at:
# target/release/claude-usage

# Install globally (optional)
cargo install --path . --features keeper-integration

# Run the tool
./target/release/claude-usage [command] [options]

# Basic usage reports
claude-usage daily           # Daily breakdown with projects
claude-usage monthly         # Monthly aggregation

# Live monitoring
claude-usage live            # Real-time monitoring
claude-usage live --snapshot # One-time snapshot

# Date filtering
claude-usage daily --since 2024-01-01 --until 2024-01-31
claude-usage daily --limit 5

# JSON output
claude-usage daily --json
```

### Production Configuration

```bash
# Environment variables for production
export LOG_LEVEL=INFO                    # DEBUG, INFO, WARN, ERROR
export LOG_FORMAT=json                   # json for production, pretty for dev
export CLAUDE_USAGE_MAX_MEMORY_MB=1024   # Memory limit
export CLAUDE_USAGE_BATCH_SIZE=20        # Parallel processing batch size

# Or use config file (claude-usage.toml)
cp claude-usage.toml.example claude-usage.toml
```

## Data Sources

The tool reads Claude Code's usage data from:
- `~/.claude/projects/*/conversation_*.jsonl` - Main conversation logs
- `~/.claude/usage_tracking/session_blocks_*.json` - Session timing blocks (used by live monitor)
- `~/.claude/vms/*/projects/*/` - VM-specific instances

Each JSONL entry contains message usage data with token counts, model info, and timestamps that get aggregated into sessions and cost calculations.

## Testing and Validation

Test the tool by running it against your actual Claude Code usage data:

```bash
# Verify data discovery
claude-usage daily --limit 1

# Test live monitoring (Ctrl+C to exit)
claude-usage live --snapshot

# Validate JSON output
claude-usage daily --json | jq .
```

## Package Distribution

Built as a standard Rust binary with entry point:
- `Cargo.toml` defines the `claude-usage` binary
- Modular source code in `src/` directory for maintainability
- Minimal dependencies (only `reqwest` for pricing data)

### Temporary Files
When creating temporary debug or test scripts, use `/tmp` directory to keep the project clean.