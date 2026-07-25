use crate::engine::llm::LlmMessage;
use crate::engine::router::ModelRouter;
use crate::models::role::RoleArchetype;
use anyhow::Result;

pub struct DeepResearchEngine {
    pub router: ModelRouter,
}

impl DeepResearchEngine {
    pub fn new(router: ModelRouter) -> Self {
        Self { router }
    }

    pub async fn research_role(
        &self,
        role: &RoleArchetype,
        market_data: &str,
    ) -> Result<RoleArchetype> {
        let prompt = format!(
            r#"Given the role "{}" in the {} industry at {} seniority, and the following market data, update the market_demand and requirements.

Market data scraped from job boards:
{}

Return a JSON object with updated fields: title, industry, seniority, market_demand (level as high/medium/low/very_high), typical_requirements (array of strings), top_companies (array of strings)."#,
            role.title,
            role.industry,
            serde_json::to_string(&role.seniority).unwrap_or_default(),
            market_data
        );

        let messages = vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are a concise JSON-only career analyst.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let tier = self.router.tier("balanced");
        let resp = self.router.chat(messages, tier).await?;

        let raw = Self::extract_json_object(&resp.content);
        let data: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

        let demand_level = match data["market_demand"]["level"].as_str().unwrap_or("medium") {
            "very_high" => crate::models::role::DemandLevel::VeryHigh,
            "high" => crate::models::role::DemandLevel::High,
            "medium" => crate::models::role::DemandLevel::Medium,
            "low" => crate::models::role::DemandLevel::Low,
            _ => crate::models::role::DemandLevel::Medium,
        };

        let requirements = data["typical_requirements"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|r| r.as_str().map(|s| s.to_string()))
            .collect();
        let companies = data["top_companies"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| c.as_str().map(|s| s.to_string()))
            .collect();

        let mut updated = role.clone();
        updated.market_demand.level = demand_level;
        updated.typical_requirements = requirements;
        updated.top_companies = companies;
        updated.market_demand.last_updated = chrono::Utc::now();

        Ok(updated)
    }

    pub async fn skill_gaps(
        &self,
        role: &RoleArchetype,
        profile: &crate::models::profile::UserProfile,
    ) -> Result<Vec<crate::models::role::SkillGap>> {
        let prompt = format!(
            r#"For the role "{}" with requirements: {}, and the candidate profile skills: {}, list skill gaps.

Return JSON array of objects with keys: skill (string), required (bool), user_has (bool), severity (critical/major/minor/nice_to_have)."#,
            role.title,
            role.typical_requirements.join(", "),
            profile
                .skills
                .iter()
                .map(|s| format!(
                    "{} ({})",
                    s.name,
                    serde_json::to_string(&s.level).unwrap_or_default()
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let messages = vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are a concise JSON-only career analyst.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let tier = self.router.tier("light");
        let resp = self.router.chat(messages, tier).await?;
        let raw = Self::extract_json_array(&resp.content);
        let gaps = serde_json::from_str(&raw).unwrap_or_default();
        Ok(gaps)
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

    fn extract_json_object(content: &str) -> String {
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                if start < end {
                    return content[start..=end].to_string();
                }
            }
        }
        "{}".to_string()
    }
}
