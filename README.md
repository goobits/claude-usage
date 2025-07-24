# 📊 Goobits Claude Usage

A high-performance Rust implementation for comprehensive Claude usage analysis across multiple VMs and instances. Track token consumption, costs, and activity with real-time monitoring capabilities. 

This tool was created for two primary reasons:
1. **Super-fast Rust-based analysis** - High-performance token usage tracking with 3-10x better performance than TypeScript alternatives
2. **Multi-VM support** - Native integration with [Goobits VM](https://github.com/goobits/vm) infrastructure to track Claude usage across multiple virtual machines and instances

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
claude-usage daily                 # Daily usage with project breakdown (includes VMs)
claude-usage monthly               # Monthly aggregation
claude-usage live                  # Real-time monitoring
claude-usage live --snapshot       # One-time live snapshot

# Exclude VMs directory (only analyze main Claude instance)
claude-usage daily --exclude-vms   # Similar to original ccusage behavior
```

## 📸 Example Output

### 🔴 Live Monitoring

```
$ claude-usage live --snapshot

[ CLAUDE USAGE MONITOR ]

⚡ Tokens:  🟢 ▓▓▓░░░░░░░░░░░░░░░░░ 15% (132,451 / 880,000)
💲 Budget:  🟢 ▓▓▓░░░░░░░░░░░░░░░░░ 15% ($198.68 / $1,320.00)
♻️  Reset:   🕐 ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░ 93% (25m remaining)

🔥 453.2 tok/min | 💰 $12.84/hour

🕐 14:32 | 🏁 projects/utils | ♻️  25m

📝 Active session in projects/utils (5m, 1 sessions)
   Model: claude-3-5-sonnet-20241022
   ├─ Input: 28,234 tokens ($0.08)
   ├─ Output: 12,156 tokens ($0.73)
   ├─ Cache: 92,061 tokens created ($0.35), 1,245,892 read ($1.87)
   └─ Total Cost: $3.03

[Snapshot mode - aggregated from active sessions across 22 Claude instances]
```

### 📊 Daily Usage Report

```
$ claude-usage daily --limit 3

🔍 Discovered 22 Claude instances
📁 Found 2008 JSONL files across all instances
📊 Processed 237183 entries, skipped 104857 duplicates

================================================================================
Claude Code Usage Report - Daily with Project Breakdown (All Instances)
================================================================================

📊 3 days • 27 sessions • $1450.17 total

📅 2025-07-23 — $524.73 (13 sessions)
   projects/fractalcode: $1.60 (0%, 1 sessions)
   projects/palette: $0.76 (0%, 2 sessions)
   projects/utils: $18.90 (4%, 1 sessions)
   vms/dev: $143.94 (27%, 3 sessions)
   vms/goobits: $85.97 (16%, 2 sessions)
   vms/promptkeeper: $139.28 (27%, 0 sessions)
   ... and more

📅 2025-07-22 — $455.43 (7 sessions)
   projects/goobits: $28.57 (6%, 1 sessions)
   projects/utils: $31.05 (7%, 1 sessions)
   vms/goobits: $116.41 (26%, 1 sessions)
   vms/vm: $77.59 (17%, 0 sessions)
   ... and more
```

## 📊 Analysis Commands

```bash
# Daily breakdown with project details
claude-usage daily                 # Last 30 days by default
claude-usage daily --limit 7      # Last 7 days only

# Monthly aggregation
claude-usage monthly               # Historical monthly totals
claude-usage monthly --limit 3    # Last 3 months
```

## 🔴 Live Monitoring

Real-time monitoring dashboard that tracks active Claude sessions:

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
claude-usage monthly --since 2024-12-01
```

## 💰 Cost Calculation

```bash
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
claude-usage monthly --json | jq -r '.monthly[].totalCost'

# Integration examples
claude-usage daily --json | jq '[.daily[].totalCost] | add'
```

## 🗂️ Data Sources

The tool automatically discovers and analyzes Claude Code usage data from:

- **Main Instance**: `~/.claude/projects/*/conversation_*.jsonl`
- **VM Instances**: `~/.claude/vms/*/projects/*/conversation_*.jsonl` (when using [Goobits VM](https://github.com/goobits/vm))
- **Session Blocks**: `~/.claude/usage_tracking/session_blocks_*.json` (used by live monitor)

**Key Features**:
- **Multi-VM Support**: Seamlessly aggregates usage across all VMs managed by Goobits VM infrastructure
- **Global Deduplication**: Prevents double-counting when the same conversation appears across multiple VMs using messageId:requestId hashing
- **Flexible Scope**: Use `--exclude-vms` flag to analyze only the main Claude instance

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
- **Session Timing**: Time remaining until session reset (when session blocks available)
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