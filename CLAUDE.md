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

**Use cargo for installation** as this is a Rust CLI tool:

```bash
# Production install
cargo install --path .

# Development build
cargo build --release

# Run the tool
claude-usage [command] [options]

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