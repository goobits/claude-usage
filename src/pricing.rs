//! Pricing Management and Cost Calculation
//!
//! This module provides comprehensive pricing data management and cost calculation
//! capabilities for Claude usage analysis. It automatically fetches current pricing
//! information from external sources and provides fallback pricing for reliability.
//!
//! ## Core Functionality
//!
//! ### Pricing Data Management
//! - **External API Integration**: Fetches live pricing data from LiteLLM's pricing database
//! - **Intelligent Caching**: Caches pricing data globally to minimize API calls
//! - **Fallback Pricing**: Provides hardcoded fallback prices for critical models
//! - **Model-Specific Pricing**: Supports different pricing for different Claude models
//!
//! ### Cost Calculation
//! - **Token-Based Pricing**: Calculates costs based on different token types:
//!   - Input tokens (prompt processing)
//!   - Output tokens (response generation)
//!   - Cache creation tokens (building prompt cache)
//!   - Cache read tokens (using existing prompt cache)
//! - **Per-Model Pricing**: Applies correct pricing based on the specific Claude model used
//! - **Graceful Degradation**: Returns zero cost when pricing data is unavailable
//!
//! ## Key Types
//!
//! - [`PricingManager`] - Main interface for pricing operations
//! - [`PricingData`] - Structure containing per-token costs for a model
//!
//! ## Data Sources
//!
//! ### Primary Source: LiteLLM API
//! The module fetches pricing data from the LiteLLM model database:
//! ```
//! https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json
//! ```
//!
//! This provides up-to-date pricing for all supported Claude models with detailed
//! breakdowns for different token types.
//!
//! ### Fallback Pricing
//! When external API is unavailable, the module uses hardcoded pricing for:
//! - `claude-sonnet-4-20250514`: $3/1M input, $15/1M output tokens
//! - `claude-opus-4-20250514`: $15/1M input, $75/1M output tokens
//!
//! ## Caching Strategy
//!
//! - **Global Cache**: Uses `OnceLock<Mutex<Option<HashMap>>>` for thread-safe caching
//! - **Single Fetch**: Pricing data is fetched once per application run
//! - **Memory Efficient**: Caches only Claude-specific pricing data
//! - **Error Handling**: Falls back to hardcoded pricing on fetch failures
//!
//! ## Usage Example
//!
//! ```rust
//! use claude_usage::pricing::PricingManager;
//! use claude_usage::models::{UsageData};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let usage = UsageData {
//!     input_tokens: 1000,
//!     output_tokens: 500,
//!     cache_creation_input_tokens: 0,
//!     cache_read_input_tokens: 0,
//! };
//!
//! let cost = PricingManager::calculate_cost_from_tokens(
//!     &usage,
//!     "claude-sonnet-4-20250514"
//! ).await;
//!
//! println!("Total cost: ${:.4}", cost);
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Handling
//!
//! The pricing system is designed for resilience:
//! - Network failures fall back to hardcoded pricing
//! - Unknown models return zero cost (no analysis failure)
//! - Missing pricing fields are treated as free (conservative approach)
//! - All operations are non-blocking and performance-focused
//!
//! ## Integration Points
//!
//! The pricing manager integrates with:
//! - [`crate::dedup::DeduplicationEngine`] for cost calculation during processing
//! - [`crate::models::UsageData`] for token consumption data
//! - External LiteLLM pricing API for current rates

use crate::models::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::Mutex;

static PRICING_CACHE: OnceLock<Mutex<Option<HashMap<String, PricingData>>>> = OnceLock::new();

pub struct PricingManager;

impl PricingManager {
    pub async fn get_pricing_data() -> Result<HashMap<String, PricingData>> {
        // Check cache first
        {
            let cache = PRICING_CACHE.get_or_init(|| Mutex::new(None)).lock()
                .expect("Failed to acquire pricing cache mutex lock for reading - this indicates a critical synchronization error");
            if let Some(ref pricing) = *cache {
                return Ok(pricing.clone());
            }
        }
        
        // Fetch from API
        let pricing = Self::fetch_pricing_data().await.unwrap_or_else(|_| Self::get_fallback_pricing());
        
        // Cache the result
        {
            let mut cache = PRICING_CACHE.get_or_init(|| Mutex::new(None)).lock()
                .expect("Failed to acquire pricing cache mutex lock for writing - this indicates a critical synchronization error");
            *cache = Some(pricing.clone());
        }
        
        Ok(pricing)
    }

    async fn fetch_pricing_data() -> Result<HashMap<String, PricingData>> {
        let url = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
        
        // Create client with timeout and security settings
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))  // 10 second timeout
            .connect_timeout(std::time::Duration::from_secs(5))  // 5 second connection timeout
            .build()?;
        
        let response = client.get(url)
            .header("User-Agent", "claude-usage/1.0.1")  // Identify ourselves
            .send()
            .await?;
        
        // Validate response status
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch pricing data: HTTP {}", response.status());
        }
        
        // Check content length to prevent huge responses (max 5MB)
        if let Some(content_length) = response.content_length() {
            if content_length > 5_000_000 {
                anyhow::bail!("Response too large: {} bytes", content_length);
            }
        }
        
        let all_pricing: serde_json::Value = response.json().await?;
        
        let mut claude_pricing = HashMap::new();
        
        if let Some(pricing_obj) = all_pricing.as_object() {
            for (model_name, pricing_data) in pricing_obj {
                if model_name.starts_with("claude-") {
                    let pricing = PricingData {
                        input_cost_per_token: pricing_data.get("input_cost_per_token").and_then(|v| v.as_f64()),
                        output_cost_per_token: pricing_data.get("output_cost_per_token").and_then(|v| v.as_f64()),
                        cache_creation_input_token_cost: pricing_data.get("cache_creation_input_token_cost").and_then(|v| v.as_f64()),
                        cache_read_input_token_cost: pricing_data.get("cache_read_input_token_cost").and_then(|v| v.as_f64()),
                    };
                    claude_pricing.insert(model_name.clone(), pricing);
                }
            }
        }
        
        Ok(claude_pricing)
    }

    fn get_fallback_pricing() -> HashMap<String, PricingData> {
        let mut pricing = HashMap::new();
        
        pricing.insert("claude-sonnet-4-20250514".to_string(), PricingData {
            input_cost_per_token: Some(3e-06),  // $3 per 1M tokens
            output_cost_per_token: Some(1.5e-05),  // $15 per 1M tokens
            cache_creation_input_token_cost: None,
            cache_read_input_token_cost: None,
        });
        
        pricing.insert("claude-opus-4-20250514".to_string(), PricingData {
            input_cost_per_token: Some(1.5e-05),  // $15 per 1M tokens
            output_cost_per_token: Some(7.5e-05),  // $75 per 1M tokens
            cache_creation_input_token_cost: None,
            cache_read_input_token_cost: None,
        });
        
        pricing
    }

    pub async fn calculate_cost_from_tokens(usage: &UsageData, model_name: &str) -> f64 {
        let pricing_data = match Self::get_pricing_data().await {
            Ok(data) => data,
            Err(_) => return 0.0,
        };
        
        let pricing = match pricing_data.get(model_name) {
            Some(pricing) => pricing,
            None => return 0.0,
        };
        
        let mut cost = 0.0;
        
        // Input tokens cost
        if let Some(input_cost) = pricing.input_cost_per_token {
            cost += usage.input_tokens as f64 * input_cost;
        }
        
        // Output tokens cost
        if let Some(output_cost) = pricing.output_cost_per_token {
            cost += usage.output_tokens as f64 * output_cost;
        }
        
        // Cache creation tokens cost
        if let Some(cache_creation_cost) = pricing.cache_creation_input_token_cost {
            cost += usage.cache_creation_input_tokens as f64 * cache_creation_cost;
        }
        
        // Cache read tokens cost
        if let Some(cache_read_cost) = pricing.cache_read_input_token_cost {
            cost += usage.cache_read_input_tokens as f64 * cache_read_cost;
        }
        
        cost
    }
}