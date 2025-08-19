//! Parquet file processing module
//!
//! This module provides utilities for reading parquet files created by claude-keeper
//! backups. It focuses on extracting summary information efficiently without loading
//! all detailed data into memory.

pub mod reader;