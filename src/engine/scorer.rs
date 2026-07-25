use crate::engine::llm::LlmMessage;
use crate::engine::prompts::Prompts;
use crate::engine::router::ModelRouter;
use crate::models::job::{DimensionScore, Evaluation, Job, Recommendation};
use anyhow::Result;

pub struct Scorer {
    pub router: ModelRouter,
    pub prompts: Prompts,
}

impl Scorer {
    pub fn new(router: ModelRouter, prompts: Prompts) -> Self {
        Self { router, prompts }
    }

    pub async fn evaluate(
        &self,
        job: &Job,
        profile: &crate::models::profile::UserProfile,
    ) -> Result<Evaluation> {
        let fact_patches = self.patch_job_facts(job).await?;
        let messages = self.prompts.scoring_prompt(job, profile);
        let tier = self.router.tier("balanced");
        let resp = self.router.chat(messages, tier).await?;

        let eval_json = self.extract_eval_json(&resp.content);
        let mut eval_data: serde_json::Value = match serde_json::from_str(&eval_json) {
            Ok(v) => v,
            Err(_) => self.fallback_eval(job),
        };

        if !fact_patches.is_empty() {
            eval_data["job_fact_patches"] = serde_json::to_value(&fact_patches)?;
        }

        let overall = eval_data["overall_score"].as_f64().unwrap_or(0.5);
        let grade = match overall {
            s if s >= 0.9 => "A",
            s if s >= 0.8 => "B+",
            s if s >= 0.7 => "B",
            s if s >= 0.6 => "C+",
            s if s >= 0.5 => "C",
            _ => "D",
        }
        .to_string();

        let dimensions = eval_data["dimensions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|d| {
                Some(DimensionScore {
                    name: d["name"].as_str()?.to_string(),
                    score: d["score"].as_f64().unwrap_or(0.0),
                    max: d["max"].as_f64().unwrap_or(5.0),
                    rationale: d["rationale"].as_str().unwrap_or("").to_string(),
                })
            })
            .collect();

        Ok(Evaluation {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job.id.clone(),
            overall_score: overall,
            overall_grade: grade,
            dimensions,
            match_summary: eval_data["match_summary"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            strengths: eval_data["strengths"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            gaps: eval_data["gaps"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            red_flags: eval_data["red_flags"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            recommendation: if overall >= 0.7 {
                Recommendation::Apply
            } else if overall >= 0.5 {
                Recommendation::Maybe
            } else {
                Recommendation::Skip
            },
            model_used: "balanced".to_string(),
            evaluated_at: chrono::Utc::now(),
        })
    }

    fn fallback_eval(&self, _job: &Job) -> serde_json::Value {
        serde_json::json!({
            "overall_score": 0.5,
            "dimensions": [],
            "match_summary": "Auto-generated fallback evaluation.",
            "strengths": [],
            "gaps": ["Review manually"],
            "red_flags": [],
        })
    }

    fn extract_eval_json(&self, content: &str) -> String {
        if let Some(start) = content.find("```json") {
            if let Some(end) = content[start + 7..].find("```") {
                return content[start + 7..start + 7 + end].trim().to_string();
            }
        }
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                if start < end {
                    return content[start..=end].to_string();
                }
            }
        }
        content.to_string()
    }

    async fn patch_job_facts(&self, job: &Job) -> Result<Vec<serde_json::Value>> {
        let prompt = format!(
            "Review this job posting for factual errors or missing fields. Propose corrections ONLY for missing/inc whitelisted fields with exact evidence quotes.\n\nTitle: {}\nCompany: {}\nLocation: {}\nDescription: {}\n\nReturn JSON array of patches: [{{\"field\": \"...\", \"value\": \"...\", \"confidence\": \"high/medium/low\", \"evidence\": \"...\"}}]",
            job.title, job.company, job.location, job.description
        );
        let messages = vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are a fact-checker. Output JSON only.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        let tier = self.router.tier("light");
        match self.router.chat(messages, tier).await {
            Ok(resp) => {
                let raw = self.extract_eval_json(&resp.content);
                match serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
                    Ok(patches) => Ok(patches),
                    Err(_) => Ok(vec![]),
                }
            }
            Err(_) => Ok(vec![]),
        }
    }

    pub fn detect_ghost_jobs(&self, job: &Job) -> Vec<String> {
        let mut flags = Vec::new();
        let desc = job.description.to_lowercase();
        let title = job.title.to_lowercase();

        if (desc.contains("ai transformation") || desc.contains("ai-driven transformation"))
            && (desc.contains("legacy") || desc.contains("mainframe") || desc.contains("on-prem"))
        {
            flags.push("AI-buzzword vs infrastructure mismatch: buzzword-heavy description for legacy-heavy role".to_string());
        }

        let buzzword_count = [
            "synergy",
            "paradigm",
            "disrupt",
            "innovation",
            "blockchain",
            "metaverse",
        ]
        .iter()
        .filter(|b| desc.contains(*b))
        .count();
        if buzzword_count >= 2 {
            flags.push(format!(
                "High buzzword density: {} buzzwords detected",
                buzzword_count
            ));
        }

        if desc.contains("100% remote") && (title.contains("on-site") || title.contains("hybrid")) {
            flags.push("Location terminology mismatch: description says remote but title suggests on-site/hybrid".to_string());
        }

        if job.salary_range.is_none() && desc.len() > 2000 {
            flags.push("Missing salary information for detailed posting".to_string());
        }

        flags
    }
}
