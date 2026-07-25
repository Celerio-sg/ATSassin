use crate::config::LlmProvider;
use crate::models::job::Job;
use crate::models::profile::UserProfile;
use anyhow::Result;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub overall: f64,
    pub keywords: f64,
    pub sections: f64,
    pub formatting: f64,
    pub semantic: f64,
    pub details: Vec<String>,
}

pub struct Matcher {
    _embed_provider: LlmProvider,
    _embed_endpoint: String,
}

impl Matcher {
    pub fn new(embed_provider: LlmProvider, embed_endpoint: String) -> Self {
        Self {
            _embed_provider: embed_provider,
            _embed_endpoint: embed_endpoint,
        }
    }

    pub async fn score_job_vs_profile(
        &self,
        job: &Job,
        profile: &UserProfile,
    ) -> Result<ScoreResult> {
        let jd_text = format!(
            "Title: {}\nCompany: {}\nLocation: {}\nType: {:?}\nSalary: {:?}\nDescription: {}\nRequirements: {}",
            job.title,
            job.company,
            job.location,
            job.job_type,
            job.salary_range,
            job.description,
            job.requirements.join(", ")
        );
        self.score_texts(&jd_text, &profile.raw_text, &profile.skills)
            .await
    }

    pub async fn score_texts(
        &self,
        jd: &str,
        resume: &str,
        skills: &[crate::models::profile::Skill],
    ) -> Result<ScoreResult> {
        let keywords = self.keyword_score(jd, resume, skills);
        let sections = self.section_score(resume);
        let formatting = self.formatting_score(resume);
        let semantic = self.semantic_score(jd, resume).await?;
        let overall =
            (keywords * 0.35 + sections * 0.15 + formatting * 0.1 + semantic * 0.4).clamp(0.0, 1.0);

        let details = vec![
            format!("Keywords: {:.0}%", keywords * 100.0),
            format!("Sections: {:.0}%", sections * 100.0),
            format!("Formatting: {:.0}%", formatting * 100.0),
            format!("Semantic: {:.0}%", semantic * 100.0),
        ];

        debug!("Score result: overall={:.2}", overall);
        Ok(ScoreResult {
            overall,
            keywords,
            sections,
            formatting,
            semantic,
            details,
        })
    }

    fn keyword_score(
        &self,
        jd: &str,
        resume: &str,
        skills: &[crate::models::profile::Skill],
    ) -> f64 {
        let jd_lower = jd.to_lowercase();
        let resume_lower = resume.to_lowercase();

        let mut matched = 0usize;
        let mut total = 0usize;

        for skill in skills {
            let name_lower = skill.name.to_lowercase();
            if jd_lower.contains(&name_lower) {
                total += 1;
                if resume_lower.contains(&name_lower) {
                    matched += 1;
                }
            }
        }

        if total == 0 {
            return 0.5;
        }

        matched as f64 / total as f64
    }

    fn section_score(&self, resume: &str) -> f64 {
        let sections = [
            "summary",
            "objective",
            "experience",
            "education",
            "skills",
            "certifications",
            "projects",
            "contact",
            "phone",
            "email",
        ];
        let lower = resume.to_lowercase();
        let found = sections.iter().filter(|s| lower.contains(*s)).count();
        found as f64 / sections.len() as f64
    }

    fn formatting_score(&self, resume: &str) -> f64 {
        let has_contact = resume.contains('@') || resume.contains("+");
        let has_dates = regex::Regex::new(r"\d{4}").unwrap().is_match(resume);
        let has_bullets = resume.contains('-') || resume.contains("•");
        let not_too_long = resume.len() < 20000;

        let mut score = 0.0;
        if has_contact {
            score += 0.25;
        }
        if has_dates {
            score += 0.25;
        }
        if has_bullets {
            score += 0.25;
        }
        if not_too_long {
            score += 0.25;
        }
        score
    }

    async fn semantic_score(&self, jd: &str, resume: &str) -> Result<f64> {
        let client = match reqwest::Client::builder()
            .user_agent("ATSassin/0.1")
            .build()
        {
            Ok(c) => c,
            Err(_) => return Ok(0.0),
        };

        let payload = serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": format!("{}\n\n{}", jd, resume)
        });

        let resp = client
            .post("http://localhost:11434/api/embeddings")
            .json(&payload)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(_) => return Ok(0.0),
        };

        if !resp.status().is_success() {
            return Ok(0.0);
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(_) => return Ok(0.0),
        };

        let embedding = data["embedding"].as_array();
        match embedding {
            Some(vec) => {
                let magnitude: f64 = vec.iter().filter_map(|v| v.as_f64()).map(|v| v * v).sum();
                Ok(magnitude.sqrt())
            }
            None => Ok(0.0),
        }
    }
}
