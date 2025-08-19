//! Processing Options
//!
//! This module contains the ProcessOptions struct used to configure
//! analysis operations.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub json_output: bool,
    pub limit: Option<usize>,
    pub since_date: Option<DateTime<Utc>>,
    pub until_date: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub snapshot: bool,
    #[allow(dead_code)]
    pub command: String,
    #[allow(dead_code)]
    pub exclude_vms: bool,
}