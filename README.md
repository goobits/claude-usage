# 📊 Goobits Claude Usage

A high-performance Rust implementation for comprehensive Claude usage analysis across multiple VMs and instances. Track token consumption, costs, and session activity with real-time monitoring capabilities. Complete rewrite of the original Node.js-based [ccusage](https://github.com/ryoppippi/ccusage) with 3-10x performance improvements.

## 📋 Table of Contents

- [Installation](#-installation)
- [Basic Usage](#-basic-usage)
- [Analysis Commands](#-analysis-commands)
- [Live Monitoring](#-live-monitoring)
- [Date Filtering](#-date-filtering)
- [Cost Calculation](#-cost-calculation)
- [Output Formats](#-output-formats)
- [Data Sources](#-data-sources)
- [Performance Features](#-performance-features)
- [Tech Stack](#️-tech-stack)

## 📦 Installation

```bash
# Install from source
cargo install --path .

# Or use the setup script
./setup.sh install

# Verify installation
claude-usage --help               # Show available commands
claude-usage daily                # Test basic functionality
```

## 🎯 Basic Usage

```bash
claude-usage daily                 # Daily usage with project breakdown
claude-usage monthly               # Monthly aggregation
claude-usage session              # Recent session activity
claude-usage live                  # Real-time monitoring
claude-usage live --snapshot       # One-time live snapshot
```

## 📊 Analysis Commands

```bash
# Daily breakdown with project details
claude-usage daily                 # Last 30 days by default
claude-usage daily --last 7       # Last 7 days only

# Monthly aggregation
claude-usage monthly               # Historical monthly totals
claude-usage monthly --last 3     # Last 3 months

# Session analysis
claude-usage session              # Recent sessions
claude-usage session --last 10    # Last 10 sessions

# Session blocks (timing data)
claude-usage blocks               # Session timing blocks
```

## 🔴 Live Monitoring

```bash
# Real-time monitoring dashboard
claude-usage live                  # Continuous monitoring loop

# One-time snapshot
claude-usage live --snapshot       # Single view without loop

# JSON output for integration
claude-usage live --snapshot --json | jq .
```

## 📅 Date Filtering

```bash
# Date range filtering
claude-usage daily --since 2024-01-01 --until 2024-01-31
claude-usage session --since 2024-12-01

# Quick filters
claude-usage daily --week         # This week's data
claude-usage daily --month        # This month's data
claude-usage daily --year         # This year's data

# Month-specific filters
claude-usage daily --january      # January data
claude-usage daily --december     # December data
```

## 💰 Cost Calculation

```bash
# Cost calculation modes
claude-usage daily --mode auto        # Use stored costs, fallback to calculation (default)
claude-usage daily --mode calculate   # Always calculate from tokens
claude-usage daily --mode display     # Always use stored costUSD values

# Real-time pricing updates
# Automatically fetches latest pricing from LiteLLM API
# Falls back to hardcoded rates if API unavailable
```

## 📄 Output Formats

```bash
# Human-readable format (default)
claude-usage daily                 # Formatted tables with emojis

# JSON output for scripts
claude-usage daily --json | jq .
claude-usage session --json | jq -r '.session[].sessionId'

# Integration examples
claude-usage daily --json | jq '[.daily[].totalCost] | add'
```

## 🗂️ Data Sources

The tool automatically discovers and analyzes Claude Code usage data from:

- **Main Instance**: `~/.claude/projects/*/conversation_*.jsonl`
- **VM Instances**: `~/.claude/vms/*/projects/*/conversation_*.jsonl`
- **Session Blocks**: `~/.claude/usage_tracking/session_blocks_*.json`

**Global Deduplication**: Prevents double-counting when the same conversation appears across multiple VMs using messageId:requestId hashing.

## 🚀 Performance Features

- **Fast Discovery**: Efficient file system scanning with date pre-filtering
- **Streaming Processing**: Memory-efficient JSONL parsing for large datasets
- **Smart Caching**: 30-second cache for live monitoring data
- **Time-Windowed Deduplication**: Optimized duplicate detection within conversation timeframes
- **Early Exit Optimization**: Stops processing when `--last N` limit is reached

## 📈 Live Monitoring Dashboard

The live monitor provides real-time tracking with:

- **Token Usage**: Progress bars showing consumption vs. limits
- **Cost Tracking**: Budget monitoring with burn rate calculations
- **Session Timing**: Time remaining until session reset
- **Multi-VM Support**: Aggregated view across all active Claude instances
- **Burn Rate Analysis**: Real-time tokens/minute and cost/hour calculations

## 🛠️ Tech Stack

- **Rust 1.80+** with high-performance libraries
- **Serde** for zero-copy JSON serialization
- **Tokio** for async runtime and live monitoring
- **Rayon** for parallel processing
- **DashMap** for concurrent deduplication
- **Chrono** for efficient date/time handling
- **Native binary** with no runtime dependencies

## 🔗 Related Projects

- **[Claude Code](https://claude.ai/code)** - Official Claude coding assistant
- **[ccusage](https://github.com/ryoppippi/ccusage)** - Original Node.js implementation (predecessor)