use crate::models::*;
use anyhow::Result;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static PRICING_CACHE: Lazy<Mutex<Option<HashMap<String, PricingData>>>> = Lazy::new(|| Mutex::new(None));

pub struct PricingManager;

impl PricingManager {
    pub async fn get_pricing_data() -> Result<HashMap<String, PricingData>> {
        // Check cache first
        {
            let cache = PRICING_CACHE.lock().unwrap();
            if let Some(ref pricing) = *cache {
                return Ok(pricing.clone());
            }
        }
        
        // Fetch from API
        let pricing = Self::fetch_pricing_data().await.unwrap_or_else(|_| Self::get_fallback_pricing());
        
        // Cache the result
        {
            let mut cache = PRICING_CACHE.lock().unwrap();
            *cache = Some(pricing.clone());
        }
        
        Ok(pricing)
    }

    async fn fetch_pricing_data() -> Result<HashMap<String, PricingData>> {
        let url = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
        
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
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