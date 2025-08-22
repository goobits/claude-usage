//! Claude Usage Library
//!
//! A comprehensive Rust library for analyzing Claude Code usage data across multiple instances.
//! This library provides high-performance parsing, analysis, and reporting capabilities for
//! Claude Code usage logs stored in JSONL format.
//!
//! ## Core Features
//!
//! - **Multi-instance support**: Automatically discovers and processes Claude instances across
//!   local projects and virtual machines
//! - **High-performance parsing**: Parallel processing with configurable batch sizes and
//!   memory management
//! - **Intelligent deduplication**: Prevents double-counting of usage data with time-windowed
//!   deduplication engine
//! - **Flexible output formats**: JSON and human-readable reports with daily/monthly aggregation
//! - **Cost calculation**: Automatic pricing integration with fallback support
//!
//! ## Architecture Overview
//!
//! The library is organized around several key modules:
//!
//! - [`models`] - Core data structures for usage entries, sessions, and aggregated reports
//! - [`parser`] - File discovery and JSONL parsing with streaming support
//! - [`analyzer`] - Main analysis engine that orchestrates parsing and aggregation
//! - [`dedup`] - Deduplication engine for handling overlapping usage data
//! - [`display`] - Terminal UI and live display components for real-time monitoring
//! - [`reports`] - Output formatting for various report types
//! - [`pricing`] - Cost calculation and pricing data management
//! - [`config`] - Configuration management with environment variable support
//! - [`logging`] - Structured logging with JSON and pretty-print formats
//! - [`memory`] - Memory usage monitoring and management utilities
//!
//! ## Main Entry Point
//!
//! The primary interface is through [`ClaudeUsageAnalyzer`], which provides a unified
//! API for all analysis operations:
//!
//! ```rust
//! use claude_usage::{ClaudeUsageAnalyzer, dedup::ProcessOptions};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let analyzer = ClaudeUsageAnalyzer::new();
//! let options = ProcessOptions {
//!     command: "daily".to_string(),
//!     json_output: false,
//!     limit: Some(30),
//!     since_date: None,
//!     until_date: None,
//!     snapshot: false,
//!     exclude_vms: false,
//! };
//!
//! let sessions = analyzer.aggregate_data("daily", options).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Key Types
//!
//! - [`UsageEntry`] - Individual usage record from Claude logs
//! - [`SessionData`] - Aggregated session information
//! - [`SessionOutput`] - Serializable session data for reports
//! - [`dedup::ProcessOptions`] - Configuration for analysis operations

pub mod analyzer;
pub mod config;
pub mod dedup;
pub mod display;
pub mod file_discovery;
pub mod logging;
pub mod memory;
pub mod models;
pub mod parser;
pub mod parser_wrapper;
pub mod pricing;
pub mod reports;
pub mod session_utils;
pub mod timestamp_parser;

// Live mode modules
pub mod live;
pub mod litellm_pricing;
pub mod parquet;

// Command modules
pub mod commands;

pub use analyzer::ClaudeUsageAnalyzer;
pub use models::*;

// Keeper integration module for schema-resilient parsing
pub mod keeper_integration;

// CCUsage compatibility module for exact parity
pub mod ccusage_compat;
