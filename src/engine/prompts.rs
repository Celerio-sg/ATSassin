use super::egress::{PromptEgressBuilder, PromptEgressPayload};
use crate::models::job::Job;
use crate::models::profile::UserProfile;
use anyhow::Result;

const AI_PHRASE_BLACKLIST: [&str; 74] = [
    "spearheaded",
    "synergized",
    "paradigm shift",
    "leveraged",
    "utilized",
    "orchestrated",
    "spearheading",
    "synergizing",
    "parlaying",
    "reimagined",
    "reimagining",
    "disruptive",
    "ecosystem",
    "bandwidth",
    "move the needle",
    "boil the ocean",
    "low-hanging fruit",
    "circle back",
    "touch base",
    "deep dive",
    "drill down",
    "best practices",
    "thought leadership",
    "value add",
    "value-add",
    "result-driven",
    "results-driven",
    "data-driven",
    "strategic",
    "synergy",
    "synergies",
    "optimized",
    "optimization",
    "streamlined",
    "streamlining",
    "implemented",
    "implementation",
    "deployed",
    "deployment",
    "integrated",
    "integration",
    "developed",
    "development",
    "designed",
    "design",
    "architected",
    "architecture",
    "led",
    "leading",
    "managed",
    "managing",
    "coordinated",
    "coordinating",
    "responsible for",
    "accountable for",
    "played a key role",
    "key role",
    "team player",
    "self-starter",
    "detail-oriented",
    "hardworking",
    "dedicated",
    "proven track record",
    "track record of",
    "extensive experience",
    "proficient in",
    "expertise in",
    "knowledge of",
    "familiar with",
    "understanding of",
    "passionate about",
    "motivated by",
    "driven by",
    "committed to",
];

pub struct Prompts;

impl Prompts {
    pub fn scoring_prompt(&self, job: &Job, profile: &UserProfile) -> Result<PromptEgressPayload> {
        let requirements = job.requirements.join(", ");
        let skills = profile
            .skills
            .iter()
            .map(|skill| skill.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let experience = profile
            .experience
            .iter()
            .map(|entry| {
                format!(
                    "- {} at {} ({})",
                    entry.title,
                    entry.company,
                    entry.start_date.as_deref().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut builder = PromptEgressBuilder::new(
            "You are an expert ATS and career analyst. Evaluate the candidate's fit for the supplied job and profile using these 6 dimensions (1-5 each): Role match, North-star alignment, Comp, Cultural signals, Red flags, Global fit. Output a JSON object with overall_score (0.0-1.0), dimensions (array of objects with name/score/max/rationale), match_summary, strengths (array), gaps (array), red_flags (array). Do not invent achievements.",
            "Evaluate fit using only the labelled data blocks below.",
        );
        builder.add_untrusted("job_title", &job.title)?;
        builder.add_untrusted("job_company", &job.company)?;
        builder.add_untrusted("job_location", &job.location)?;
        builder.add_untrusted("job_description", &job.description)?;
        builder.add_untrusted("job_requirements", &requirements)?;
        builder.add_untrusted("candidate_name", &profile.name)?;
        builder.add_untrusted(
            "candidate_summary",
            profile.summary.as_deref().unwrap_or(""),
        )?;
        builder.add_untrusted("candidate_skills", &skills)?;
        builder.add_untrusted("candidate_experience", &experience)?;
        builder.build()
    }

    pub fn tailor_resume_prompt(
        &self,
        job: &Job,
        profile: &UserProfile,
    ) -> Result<PromptEgressPayload> {
        let contact_line = [
            profile.email.clone(),
            profile.phone.clone(),
            profile.location.clone(),
            profile.linkedin_url.clone(),
            profile.portfolio_url.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" | ");

        let education = if profile.education.is_empty() {
            "(none provided in candidate profile - omit the Education section entirely rather than inventing one)".to_string()
        } else {
            profile
                .education
                .iter()
                .map(|e| {
                    format!(
                        "- {} in {}, {} ({} - {})",
                        e.degree,
                        e.field,
                        e.institution,
                        e.start_date.as_deref().unwrap_or("?"),
                        e.end_date.as_deref().unwrap_or("?")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let requirements = job.requirements.join(", ");
        let skills = profile
            .skills
            .iter()
            .map(|skill| skill.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let experience = format!(
            "{} entries total - the resume must include all {} of them:\n{}",
            profile.experience.len(),
            profile.experience.len(),
            profile
                .experience
                .iter()
                .map(|entry| format!(
                    "- {} at {} ({} - {})\n  {}",
                    entry.title,
                    entry.company,
                    entry.start_date.as_deref().unwrap_or("?"),
                    if entry.current {
                        "Present".to_string()
                    } else {
                        entry.end_date.clone().unwrap_or_else(|| "?".to_string())
                    },
                    entry.description
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let contact = if contact_line.is_empty() {
            "(not provided - omit contact details rather than inventing them)"
        } else {
            &contact_line
        };

        let mut builder = PromptEgressBuilder::new(
            "You are an expert resume writer optimizing for both ATS (applicant tracking system) parsing and human review. Tailor the candidate's resume for the supplied job description while staying 100% truthful to their experience - never invent roles, dates, employers, or achievements. \
\
ATS rules: plain text only (no tables, columns, icons, or text boxes); standard section headers exactly as: Contact, Summary, Experience, Skills, Education (omit Education only if the candidate profile has none); consistent date formatting per entry; mirror the job description's exact keywords and phrasing wherever the candidate's real experience genuinely supports it. \
\
Completeness rule: list EVERY experience entry given to you, in reverse-chronological order, exactly as many roles as appear in the candidate profile below - do not omit, merge, or cut it down to only the 'most relevant' few. This is a complete work-history document, not a highlights-only pitch. You may vary how much space/detail each entry gets (older or less relevant roles can be one line), but every entry must appear. \
\
Output only the tailored resume in Markdown.",
            "Tailor the resume for this role using only the labelled data blocks below. Never invent experience.",
        );
        builder.add_untrusted("job_title", &job.title)?;
        builder.add_untrusted("job_company", &job.company)?;
        builder.add_untrusted("job_description", &job.description)?;
        builder.add_untrusted("job_requirements", &requirements)?;
        builder.add_untrusted("candidate_name", &profile.name)?;
        builder.add_untrusted("candidate_contact", contact)?;
        builder.add_untrusted(
            "candidate_summary",
            profile.summary.as_deref().unwrap_or(""),
        )?;
        builder.add_untrusted("candidate_skills", &skills)?;
        builder.add_untrusted("candidate_education", &education)?;
        builder.add_untrusted("candidate_experience", &experience)?;
        builder.build()
    }

    pub fn cover_letter_prompt(
        &self,
        job: &Job,
        profile: &UserProfile,
    ) -> Result<PromptEgressPayload> {
        let top_skills = profile
            .skills
            .iter()
            .take(5)
            .map(|skill| skill.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let top_achievement = profile
            .experience
            .first()
            .map(|entry| format!("{} at {}", entry.title, entry.company))
            .unwrap_or_default();

        let mut builder = PromptEgressBuilder::new(
            "You are an expert cover letter writer. Write a concise, targeted cover letter (150-250 words) for the candidate. Mirror 2-3 job-description keywords naturally. Explain why the candidate is a strong fit. Never invent experience.",
            "Write the cover letter using only the labelled data blocks below.",
        );
        builder.add_untrusted("job_title", &job.title)?;
        builder.add_untrusted("job_company", &job.company)?;
        builder.add_untrusted("job_description", &job.description)?;
        builder.add_untrusted("candidate_name", &profile.name)?;
        builder.add_untrusted(
            "candidate_summary",
            profile.summary.as_deref().unwrap_or(""),
        )?;
        builder.add_untrusted("candidate_top_skills", &top_skills)?;
        builder.add_untrusted("candidate_top_achievement", &top_achievement)?;
        builder.build()
    }

    pub fn role_inference_prompt(&self, profile: &UserProfile) -> Result<PromptEgressPayload> {
        let skills = profile
            .skills
            .iter()
            .map(|skill| {
                format!(
                    "{} ({})",
                    skill.name,
                    serde_json::to_string(&skill.level).unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let experience = profile
            .experience
            .iter()
            .map(|entry| {
                format!(
                    "- {} at {} ({})",
                    entry.title,
                    entry.company,
                    entry.start_date.as_deref().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut builder = PromptEgressBuilder::new(
            "You are a concise JSON-only career analyst. Infer 5-10 realistic job role archetypes that match the supplied candidate background. For each role provide: title; industry; seniority (intern, junior, mid, senior, lead, manager, director, vp, or cxo); market_demand (high, medium, or low); compensation_band with currency, min/max/median integer USD-equivalent annual figures and a source string; 3-6 typical_requirements; and 3-5 top_companies. Sanity-check compensation against seniority: entry-level, part-time, and mentor roles should remain well under $100k USD, while only VP/CXO roles should approach $1-2M USD. Return only a compact parsable JSON array with no Markdown.",
            "Infer role archetypes using only the labelled candidate data blocks below.",
        );
        builder.add_untrusted("candidate_name", &profile.name)?;
        builder.add_untrusted(
            "candidate_summary",
            profile.summary.as_deref().unwrap_or(""),
        )?;
        builder.add_untrusted("candidate_skills", &skills)?;
        builder.add_untrusted("candidate_experience", &experience)?;
        builder.build()
    }

    pub fn deep_research_prompt(
        &self,
        role_title: &str,
        scraped_jobs: &[String],
    ) -> Result<PromptEgressPayload> {
        let summaries = scraped_jobs.join("\n---\n");
        let mut builder = PromptEgressBuilder::new(
            "You are a labor-market analyst. Given scraped job data for a role, synthesize market demand, typical requirements, and top companies. Output JSON only.",
            "Synthesize market insights using only the labelled data blocks below.",
        );
        builder.add_untrusted("role_title", role_title)?;
        builder.add_untrusted("scraped_job_summaries", &summaries)?;
        builder.build()
    }

    pub fn sanitize_output(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        for phrase in AI_PHRASE_BLACKLIST.iter() {
            let lower = cleaned.to_lowercase();
            if let Some(pos) = lower.find(phrase) {
                let replacement = match *phrase {
                    "spearheaded" | "spearheading" => "led",
                    "synergized" | "synergizing" | "synergy" | "synergies" => "collaborated",
                    "leveraged" | "utilized" => "used",
                    "orchestrated" | "orchestrating" => "coordinated",
                    "streamlined" | "streamlining" => "simplified",
                    "optimized" | "optimization" => "improved",
                    _ => "",
                };
                if !replacement.is_empty() {
                    cleaned = format!(
                        "{}{}{}",
                        &cleaned[..pos],
                        replacement,
                        &cleaned[pos + phrase.len()..]
                    );
                } else {
                    cleaned = format!("{}{}", &cleaned[..pos], &cleaned[pos + phrase.len()..]);
                }
            }
        }
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::job::Job;
    use crate::models::profile::{Experience, Skill, SkillCategory, SkillLevel, UserProfile};
    use chrono::Utc;

    fn job_dummy() -> Job {
        Job {
            id: "1".to_string(),
            title: "Engineer".to_string(),
            company: "Acme".to_string(),
            location: "Remote".to_string(),
            remote: true,
            job_type: None,
            salary_range: None,
            description: "do stuff".to_string(),
            requirements: vec![],
            posted_at: None,
            source: "test".to_string(),
            url: String::new(),
            applied: false,
            scraped_at: Utc::now(),
        }
    }

    fn profile_with_n_experiences(n: usize) -> UserProfile {
        UserProfile {
            name: "Test Candidate".to_string(),
            email: Some("test@example.com".to_string()),
            phone: None,
            location: Some("Remote".to_string()),
            linkedin_url: None,
            portfolio_url: None,
            summary: Some("Test summary".to_string()),
            skills: vec![Skill {
                name: "Rust".to_string(),
                category: SkillCategory::Technical,
                level: SkillLevel::Expert,
                years: None,
            }],
            experience: (0..n)
                .map(|i| Experience {
                    id: format!("exp-{i}"),
                    title: format!("Engineer {i}"),
                    company: format!("Co {i}"),
                    location: None,
                    start_date: Some("2020".to_string()),
                    end_date: None,
                    current: i == 0,
                    description: format!("Role {i} description"),
                    achievements: vec![],
                    skills_used: vec![],
                })
                .collect(),
            education: vec![],
            certifications: vec![],
            languages: vec![],
            raw_text: String::new(),
            inferred_roles: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Regression test for issue #11: a real candidate's resume was
    /// silently dropping 13 of 16 experience entries before the prompt
    /// explicitly required every entry. This test pins the prompt
    /// text so any future `prompts.rs` edit that loosens the rule
    /// fails here, *before* a real candidate gets a cut-down resume
    /// again. Independent of the LLM - tests the prompt surface, not
    /// the model's faithfulness.
    #[test]
    fn tailor_resume_prompt_requires_every_experience_entry() {
        let prompts = Prompts;
        let job = job_dummy();
        let profile = profile_with_n_experiences(5);

        let prompt = prompts.tailor_resume_prompt(&job, &profile).unwrap();
        let msgs = prompt.messages_for_test();
        let system = msgs.first().map(|m| m.content.as_str()).unwrap_or("");
        let user = msgs.get(1).map(|m| m.content.as_str()).unwrap_or("");

        assert!(
            system.contains("Completeness rule"),
            "system prompt must explicitly carry a completeness instruction"
        );
        assert!(
            system.to_lowercase().contains("every entry")
                || system.contains("EVERY experience entry"),
            "system prompt must state every entry is required"
        );

        for i in 0..5 {
            let marker = format!("Engineer {i}");
            assert!(
                user.contains(&marker),
                "user prompt must inline every experience entry; missing {marker}"
            );
        }
        assert!(
            user.contains("5 entries total"),
            "user prompt must declare the total entry count for an honest framing"
        );
    }

    /// Mirror of the same test for the cover-letter prompt, which is
    /// intentionally a hi-3-achievement short letter (so no full
    /// completeness rule there) but still must not silently fabricate
    /// experience the profile doesn't have.
    #[test]
    fn cover_letter_prompt_does_not_invent_experience() {
        let prompts = Prompts;
        let job = job_dummy();
        let profile = profile_with_n_experiences(2);

        let prompt = prompts.cover_letter_prompt(&job, &profile).unwrap();
        let msgs = prompt.messages_for_test();
        let system = msgs.first().map(|m| m.content.as_str()).unwrap_or("");
        assert!(system.contains("Never invent experience"));
    }

    #[test]
    fn synthetic_long_profile_fits_configured_route_budgets() {
        let prompts = Prompts;
        let profile = crate::engine::profile_parser::ProfileParser::profile_from_text(
            include_str!("../../tests/uat/scenario_1_synthetic_apac_gtm/profile.md"),
        )
        .unwrap();
        let job = job_dummy();

        prompts
            .role_inference_prompt(&profile)
            .unwrap()
            .into_request("light".into(), 0.2, 2048, 4096)
            .expect("the long UAT profile must fit the configured light inference context");
        prompts
            .scoring_prompt(&job, &profile)
            .unwrap()
            .into_request("balanced".into(), 0.2, 2048, 8192)
            .expect("the long UAT profile must fit the configured balanced scoring context");
        prompts
            .tailor_resume_prompt(&job, &profile)
            .unwrap()
            .into_request("balanced".into(), 0.2, 2048, 8192)
            .expect("the complete 16-entry UAT profile must fit the tailoring context");
        prompts
            .cover_letter_prompt(&job, &profile)
            .unwrap()
            .into_request("balanced".into(), 0.2, 2048, 8192)
            .expect("the UAT profile must fit the cover-letter context");
    }
}
