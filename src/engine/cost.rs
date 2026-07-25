use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

#[derive(Debug, Clone)]
pub struct CostResult {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_usd: f64,
}

impl CostResult {
    pub fn zero() -> Self {
        Self {
            input_cost: 0.0,
            output_cost: 0.0,
            total_usd: 0.0,
        }
    }
}

static FREE_TIER_PROVIDERS: &[&str] = &[
    "groq",
    "openrouter",
    "ollama",
    "cerebras",
    "google_ai_studio",
    "github_models",
    "huggingface",
    "cloudflare",
];
static DEFAULT_PRICING: &[(&str, ModelPricing)] = &[
    (
        "llama-3.3-70b-versatile",
        ModelPricing {
            input_per_mtok: 0.59,
            output_per_mtok: 0.79,
        },
    ),
    (
        "llama-3.1-8b-instant",
        ModelPricing {
            input_per_mtok: 0.05,
            output_per_mtok: 0.08,
        },
    ),
    (
        "qwen/qwen3-32b",
        ModelPricing {
            input_per_mtok: 0.29,
            output_per_mtok: 0.59,
        },
    ),
    (
        "kimi-k2.6",
        ModelPricing {
            input_per_mtok: 1.00,
            output_per_mtok: 3.00,
        },
    ),
    (
        "glm-5.2",
        ModelPricing {
            input_per_mtok: 0.50,
            output_per_mtok: 1.50,
        },
    ),
    (
        "meta-llama/Llama-4-Maverick-17B",
        ModelPricing {
            input_per_mtok: 0.85,
            output_per_mtok: 2.00,
        },
    ),
];

#[derive(Debug, Clone)]
pub struct CostCalculator {
    provider_pricing: HashMap<&'static str, ModelPricing>,
    current_provider: String,
}

impl CostCalculator {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        for (model, pricing) in DEFAULT_PRICING {
            map.insert(*model, *pricing);
        }
        Self {
            provider_pricing: map,
            current_provider: String::new(),
        }
    }

    pub fn with_provider(mut self, provider: &str) -> Self {
        self.current_provider = provider.to_lowercase();
        self
    }

    pub fn set_provider(&mut self, provider: &str) {
        self.current_provider = provider.to_lowercase();
    }

    pub fn calculate(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> CostResult {
        let provider_lower = provider.to_lowercase();
        if FREE_TIER_PROVIDERS.contains(&provider_lower.as_str())
            || FREE_TIER_PROVIDERS.contains(&self.current_provider.as_str())
        {
            return CostResult::zero();
        }

        let pricing = self
            .provider_pricing
            .get(model)
            .copied()
            .unwrap_or(ModelPricing {
                input_per_mtok: 1.0,
                output_per_mtok: 3.0,
            });

        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * pricing.input_per_mtok;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * pricing.output_per_mtok;
        let total = input_cost + output_cost;

        CostResult {
            input_cost,
            output_cost,
            total_usd: total,
        }
    }

    pub fn budget_gate(&self, spent_usd: f64, budget_usd: Option<f64>) -> (bool, String) {
        match budget_usd {
            Some(budget) if spent_usd > budget => (
                false,
                format!("budget exceeded: ${:.4} > ${:.4}", spent_usd, budget),
            ),
            _ => (true, "within budget".to_string()),
        }
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_tier_providers_return_zero() {
        let calc = CostCalculator::new().with_provider("groq");
        let result = calc.calculate("groq", "llama-3.1-8b-instant", 100, 200);
        assert_eq!(result.total_usd, 0.0);
    }

    #[test]
    fn test_metered_provider_calculates_cost() {
        let calc = CostCalculator::new();
        let result = calc.calculate("openai", "gpt-4", 1_000_000, 2_000_000);
        assert!(result.total_usd > 0.0);
    }

    #[test]
    fn test_budget_gate_blocks_over_budget() {
        let calc = CostCalculator::new();
        let (pass, reason) = calc.budget_gate(10.0, Some(5.0));
        assert!(!pass);
        assert!(reason.contains("exceeded"));
    }

    #[test]
    fn test_budget_gate_passes_under_budget() {
        let calc = CostCalculator::new();
        let (pass, _) = calc.budget_gate(1.0, Some(5.0));
        assert!(pass);
    }
}
