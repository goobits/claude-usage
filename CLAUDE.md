# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Usage is a fast Python CLI tool for analyzing Claude usage data across multiple VMs and instances. It aggregates token usage, costs, and session data from Claude Code's local storage to provide comprehensive usage reports and real-time monitoring.

## Architecture

The application is built around a single main class `ClaudeUsageAnalyzer` in `claude_usage.py` that handles:

- **Data Discovery**: Automatically finds Claude instances in `~/.claude/` and `~/.claude/vms/*/`
- **Global Deduplication**: Uses messageId:requestId hashing to prevent duplicate counting across VMs
- **Cost Calculation**: Fetches live pricing from LiteLLM API with fallback to hardcoded rates
- **Multiple Display Modes**: Daily, monthly, session, blocks, and live monitoring views
- **Performance Optimization**: File-level date filtering, streaming JSON parsing, and time-windowed deduplication

### Key Data Flow

1. **Discovery**: Scans `~/.claude/projects/*/` directories for `*.jsonl` files
2. **Filtering**: Pre-filters files by modification time and date ranges
3. **Processing**: Streams JSONL entries, deduplicates globally, aggregates by session
4. **Display**: Formats output as text tables or JSON based on command

### Live Monitoring

The live monitor (`claude_usage.py live`) provides real-time usage tracking with:
- Token burn rate calculation from actual conversation timelines
- Session block detection with 30-second caching
- Progress bars for token limits and budget tracking
- Multi-VM session aggregation

## Common Commands

```bash
# Install in development mode
pip install -e .

# Run the tool
python3 claude_usage.py [command] [options]

# Basic usage reports
python3 claude_usage.py daily           # Daily breakdown with projects
python3 claude_usage.py monthly         # Monthly aggregation
python3 claude_usage.py session         # Recent sessions
python3 claude_usage.py blocks          # Session blocks

# Live monitoring
python3 claude_usage.py live            # Real-time monitoring
python3 claude_usage.py live --snapshot # One-time snapshot

# Date filtering
python3 claude_usage.py daily --since 2024-01-01 --until 2024-01-31
python3 claude_usage.py session --last 5

# JSON output
python3 claude_usage.py daily --json

# Cost calculation modes
python3 claude_usage.py daily --mode calculate  # Always calculate from tokens
python3 claude_usage.py daily --mode display    # Use stored costUSD values
python3 claude_usage.py daily --mode auto       # Prefer costUSD, fallback to calculation
```

## Data Sources

The tool reads Claude Code's usage data from:
- `~/.claude/projects/*/conversation_*.jsonl` - Main conversation logs
- `~/.claude/usage_tracking/session_blocks_*.json` - Session timing blocks
- `~/.claude/vms/*/projects/*/` - VM-specific instances

Each JSONL entry contains message usage data with token counts, model info, and timestamps that get aggregated into sessions and cost calculations.

## Testing and Validation

Test the tool by running it against your actual Claude Code usage data:

```bash
# Verify data discovery
python3 claude_usage.py session --last 1

# Test live monitoring (Ctrl+C to exit)
python3 claude_usage.py live --snapshot

# Validate JSON output
python3 claude_usage.py daily --json | jq .
```

## Package Distribution

Built as a standard Python package with entry point:
- `pyproject.toml` defines the `claude-usage` command
- Single module design (`claude_usage.py`) for easy distribution
- Minimal dependencies (only `requests` for pricing data)