//! LiteLLM pricing integration for accurate cost calculations
//!
//! This module fetches model pricing data from LiteLLM's GitHub API
//! and provides cost calculation functions that match ccusage behavior.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

const LITELLM_PRICING_URL: &str = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub max_tokens: Option<u32>,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_creation_input_token_cost: Option<f64>,
    pub cache_read_input_token_cost: Option<f64>,
    pub litellm_provider: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PricingCache {
    models: HashMap<String, ModelPricing>,
}

impl PricingCache {
    /// Fetch pricing data from LiteLLM API
    pub async fn fetch() -> Result<Self> {
        info!("Fetching model pricing from LiteLLM API");
        
        let client = reqwest::Client::new();
        let response = client
            .get(LITELLM_PRICING_URL)
            .send()
            .await
            .context("Failed to fetch pricing data from LiteLLM")?;
        
        let pricing_data: HashMap<String, ModelPricing> = response
            .json()
            .await
            .context("Failed to parse pricing data JSON")?;
        
        info!("Successfully fetched pricing for {} models", pricing_data.len());
        debug!("Available models: {:?}", pricing_data.keys().collect::<Vec<_>>());
        
        Ok(PricingCache {
            models: pricing_data,
        })
    }
    
    /// Get pricing for a model by name
    pub fn get_pricing(&self, model_name: &str) -> Option<&ModelPricing> {
        // Try exact match first
        if let Some(pricing) = self.models.get(model_name) {
            return Some(pricing);
        }
        
        // Try common variations and mappings
        let normalized_name = self.normalize_model_name(model_name);
        self.models.get(&normalized_name)
    }
    
    /// Normalize model names to match LiteLLM's naming convention
    fn normalize_model_name(&self, model_name: &str) -> String {
        match model_name {
            // Claude 4 models - map to LiteLLM names
            "claude-opus-4-1-20250805" => "claude-opus-4-1-20250805".to_string(),
            "claude-sonnet-4-20250514" => "claude-sonnet-4-20250514".to_string(),
            "opus-4" => "claude-opus-4-1-20250805".to_string(),
            "sonnet-4" => "claude-sonnet-4-20250514".to_string(),
            
            // Claude 3.5 models
            "claude-3-5-sonnet-20241022" => "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-20240620" => "claude-3-5-sonnet-20240620".to_string(),
            
            // Claude 3 models
            "claude-3-opus-20240229" => "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229" => "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307" => "claude-3-haiku-20240307".to_string(),
            
            // Default fallback
            _ => model_name.to_string(),
        }
    }
    
    /// Calculate cost for a message using LiteLLM pricing
    pub fn calculate_cost(
        &self,
        model_name: &str,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        let pricing = match self.get_pricing(model_name) {
            Some(p) => p,
            None => {
                warn!("No pricing found for model: {}, using fallback pricing", model_name);
                return self.calculate_fallback_cost(input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens);
            }
        };
        
        let mut total_cost = 0.0;
        
        // Input tokens
        total_cost += (input_tokens as f64) * pricing.input_cost_per_token;
        
        // Output tokens
        total_cost += (output_tokens as f64) * pricing.output_cost_per_token;
        
        // Cache creation tokens
        if cache_creation_tokens > 0 {
            if let Some(cache_creation_cost) = pricing.cache_creation_input_token_cost {
                total_cost += (cache_creation_tokens as f64) * cache_creation_cost;
            } else {
                // Fallback: use input cost for cache creation
                total_cost += (cache_creation_tokens as f64) * pricing.input_cost_per_token;
            }
        }
        
        // Cache read tokens
        if cache_read_tokens > 0 {
            if let Some(cache_read_cost) = pricing.cache_read_input_token_cost {
                total_cost += (cache_read_tokens as f64) * cache_read_cost;
            } else {
                // Fallback: use input cost for cache reads
                total_cost += (cache_read_tokens as f64) * pricing.input_cost_per_token;
            }
        }
        
        total_cost
    }
    
    /// Fallback cost calculation when model pricing is not available
    fn calculate_fallback_cost(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        // Use Claude 3.5 Sonnet pricing as fallback (most common model)
        let input_cost_per_token = 0.000003; // $3 per million input tokens
        let output_cost_per_token = 0.000015; // $15 per million output tokens
        let cache_creation_cost_per_token = 0.00000375; // $3.75 per million cache creation tokens
        let cache_read_cost_per_token = 0.0000003; // $0.30 per million cache read tokens
        
        let mut total_cost = 0.0;
        total_cost += (input_tokens as f64) * input_cost_per_token;
        total_cost += (output_tokens as f64) * output_cost_per_token;
        total_cost += (cache_creation_tokens as f64) * cache_creation_cost_per_token;
        total_cost += (cache_read_tokens as f64) * cache_read_cost_per_token;
        
        total_cost
    }
    
    /// List all available models
    pub fn list_models(&self) -> Vec<&String> {
        self.models.keys().collect()
    }
    
    /// Get model count
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}