use crate::engine::prompts::Prompts;
use crate::engine::router::ModelRouter;
use crate::models::profile::UserProfile;
use crate::models::role::{
    CompensationBand, DemandLevel, MarketDemand, RoleArchetype, Seniority, TrendDirection,
};
use anyhow::Result;

pub struct RoleInferenceEngine {
    pub router: ModelRouter,
}

impl RoleInferenceEngine {
    pub fn new(router: ModelRouter) -> Self {
        Self { router }
    }

    pub async fn infer_roles(&self, profile: &UserProfile) -> Result<Vec<RoleArchetype>> {
        let messages = Prompts.role_inference_prompt(profile)?;

        let tier = self.router.tier("light");
        let resp = self.router.chat(messages, tier).await?;
        let raw = Self::extract_json_array(&resp.content);
        let roles_data: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();

        let roles = roles_data.into_iter().filter_map(|v| {
            let title = v["title"].as_str()?.to_string();
            let industry = v["industry"].as_str().unwrap_or("General").to_string();
            let seniority_str = v["seniority"].as_str().unwrap_or("mid").to_lowercase();
            let seniority = match seniority_str.as_str() {
                "intern" => Seniority::Intern,
                "junior" => Seniority::Junior,
                "mid" => Seniority::Mid,
                "senior" => Seniority::Senior,
                "lead" => Seniority::Lead,
                "manager" => Seniority::Manager,
                "director" => Seniority::Director,
                "vp" => Seniority::VP,
                "cxo" => Seniority::CXO,
                _ => Seniority::Mid,
            };
            let comp = v["compensation_band"].clone();
            // Sanity bound: found via UAT that the model occasionally
            // returns wildly implausible figures (e.g. "$10,000k median"
            // for an entry-level role, or a raw JPY figure mislabeled as
            // USD for a Japan-based candidate - "$2,000k" for a part-time
            // consulting role). A flat ceiling doesn't catch the second
            // case since $2M merely looks plausible for *someone* even
            // though it's absurd for this specific low-seniority role - so
            // the ceiling scales with seniority instead. Never silently
            // pass an implausible figure through looking legitimate; say
            // so in the source field so it's visible, not hidden.
            let max_plausible_annual_comp: u64 = match seniority {
                Seniority::Intern | Seniority::Junior => 120_000,
                Seniority::Mid => 220_000,
                Seniority::Senior => 350_000,
                Seniority::Lead | Seniority::Manager => 500_000,
                Seniority::Director => 800_000,
                Seniority::VP => 1_200_000,
                Seniority::CXO => 2_000_000,
            };
            let raw_min = comp["min"].as_u64().unwrap_or(0);
            let raw_max = comp["max"].as_u64().unwrap_or(0);
            let raw_median = comp["median"].as_u64().unwrap_or(0);
            let was_clamped = raw_min > max_plausible_annual_comp || raw_max > max_plausible_annual_comp || raw_median > max_plausible_annual_comp;
            let source = comp["source"].as_str().unwrap_or("inferred").to_string();
            let compensation = CompensationBand {
                currency: comp["currency"].as_str().unwrap_or("USD").to_string(),
                min: raw_min.min(max_plausible_annual_comp),
                max: raw_max.min(max_plausible_annual_comp),
                median: raw_median.min(max_plausible_annual_comp),
                source: if was_clamped { format!("{source} (clamped - model returned an implausible figure for this seniority)") } else { source },
            };
            let demand_str = v["market_demand"].as_str().unwrap_or("medium").to_lowercase();
            let market_demand = MarketDemand {
                level: match demand_str.as_str() {
                    "high" => DemandLevel::High,
                    "medium" => DemandLevel::Medium,
                    "low" => DemandLevel::Low,
                    _ => DemandLevel::Medium,
                },
                posting_volume_30d: 0,
                trend: TrendDirection::Stable,
                last_updated: chrono::Utc::now(),
            };
            let requirements = v["typical_requirements"].as_array().unwrap_or(&vec![]).iter().filter_map(|r| r.as_str().map(|s| s.to_string())).collect();
            let companies = v["top_companies"].as_array().unwrap_or(&vec![]).iter().filter_map(|c| c.as_str().map(|s| s.to_string())).collect();

            Some(RoleArchetype {
                id: uuid::Uuid::new_v4().to_string(),
                title,
                industry,
                seniority,
                fit_score: 0.0,
                market_demand,
                compensation_band: compensation,
                typical_requirements: requirements,
                top_companies: companies,
                inferred_from_profile: true,
                created_at: chrono::Utc::now(),
            })
        }).collect();

        Ok(roles)
    }

    fn extract_json_array(content: &str) -> String {
        if let Some(start) = content.find('[') {
            if let Some(end) = content.rfind(']') {
                if start < end {
                    return content[start..=end].to_string();
                }
            }
        }
        "[]".to_string()
    }
}
