use super::llm::LlmMessage;
use crate::models::job::Job;
use crate::models::profile::UserProfile;

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
    pub fn scoring_prompt(&self, job: &Job, profile: &UserProfile) -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are an expert ATS and career analyst. Evaluate the candidate's fit for the job description using these 6 dimensions (1-5 each): Role match, North-star alignment, Comp, Cultural signals, Red flags, Global fit. Output a JSON object with overall_score (0.0-1.0), dimensions (array of objects with name/score/max/rationale), match_summary, strengths (array), gaps (array), red_flags (array). Do not invent achievements.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: format!(
                    "Job:\nTitle: {}\nCompany: {}\nLocation: {}\nDescription: {}\nRequirements: {}\n\nCandidate Profile:\nName: {}\nSummary: {}\nSkills: {}\nExperience:\n{}\n\nEvaluate fit now.",
                    job.title, job.company, job.location, job.description,
                    job.requirements.join(", "), profile.name,
                    profile.summary.as_deref().unwrap_or(""),
                    profile.skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(", "),
                    profile.experience.iter().map(|e| format!("- {} at {} ({})", e.title, e.company, e.start_date.as_deref().unwrap_or("?"))).collect::<Vec<_>>().join("\n")
                ),
            },
        ]
    }

    pub fn tailor_resume_prompt(&self, job: &Job, profile: &UserProfile) -> Vec<LlmMessage> {
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

        vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are an expert resume writer optimizing for both ATS (applicant tracking system) parsing and human review. Tailor the candidate's resume for the specific job description while staying 100% truthful to their experience - never invent roles, dates, employers, or achievements. \
\
ATS rules: plain text only (no tables, columns, icons, or text boxes); standard section headers exactly as: Contact, Summary, Experience, Skills, Education (omit Education only if the candidate profile has none); consistent date formatting per entry; mirror the job description's exact keywords and phrasing wherever the candidate's real experience genuinely supports it. \
\
Completeness rule: list EVERY experience entry given to you, in reverse-chronological order, exactly as many roles as appear in the candidate profile below - do not omit, merge, or cut it down to only the 'most relevant' few. This is a complete work-history document, not a highlights-only pitch. You may vary how much space/detail each entry gets (older or less relevant roles can be one line), but every entry must appear. \
\
Output only the tailored resume in Markdown.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: format!(
                    "Job Description:\nTitle: {}\nCompany: {}\nDescription: {}\nRequirements: {}\n\nCandidate Profile:\nName: {}\nContact: {}\nSummary: {}\nSkills: {}\n\nEducation:\n{}\n\nExperience ({} entries total - the resume must include all {} of them):\n{}\n\nTailor the resume for this role. Never invent experience.",
                    job.title, job.company, job.description,
                    job.requirements.join(", "), profile.name,
                    if contact_line.is_empty() { "(not provided - omit contact details rather than inventing them)".to_string() } else { contact_line },
                    profile.summary.as_deref().unwrap_or(""),
                    profile.skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(", "),
                    education,
                    profile.experience.len(),
                    profile.experience.len(),
                    profile.experience.iter().map(|e| format!("- {} at {} ({} - {})\n  {}", e.title, e.company, e.start_date.as_deref().unwrap_or("?"), if e.current { "Present".to_string() } else { e.end_date.clone().unwrap_or_else(|| "?".to_string()) }, e.description)).collect::<Vec<_>>().join("\n")
                ),
            },
        ]
    }

    pub fn cover_letter_prompt(&self, job: &Job, profile: &UserProfile) -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are an expert cover letter writer. Write a concise, targeted cover letter (150-250 words) for the candidate. Mirror 2-3 JD keywords naturally. Explain why the candidate is a strong fit. Never invent experience.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: format!(
                    "Job Description:\nTitle: {}\nCompany: {}\nDescription: {}\n\nCandidate Profile:\nName: {}\nSummary: {}\nTop Skills: {}\nTop Achievement: {}\n\nWrite the cover letter.",
                    job.title, job.company, job.description, profile.name,
                    profile.summary.as_deref().unwrap_or(""),
                    profile.skills.iter().take(5).map(|s| s.name.clone()).collect::<Vec<_>>().join(", "),
                    profile.experience.first().map(|e| format!("{} at {}", e.title, e.company)).unwrap_or_default()
                ),
            },
        ]
    }

    pub fn role_inference_prompt(&self, profile: &UserProfile) -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are a career analyst. Given a candidate profile, infer 5-10 realistic job archetypes. Output JSON only.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: format!(
                    "Profile:\nName: {}\nSummary: {}\nSkills: {}\nExperience: {}\n\nInfer suitable roles.",
                    profile.name,
                    profile.summary.as_deref().unwrap_or(""),
                    profile.skills.iter().map(|s| format!("{} ({})", s.name, serde_json::to_string(&s.level).unwrap_or_default())).collect::<Vec<_>>().join(", "),
                    profile.experience.iter().map(|e| format!("- {} at {}", e.title, e.company)).collect::<Vec<_>>().join("\n")
                ),
            },
        ]
    }

    pub fn deep_research_prompt(
        &self,
        role_title: &str,
        scraped_jobs: &[String],
    ) -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: "system".to_string(),
                content: "You are a labor-market analyst. Given scraped job data for a role, synthesize market demand, typical requirements, and top companies. Output JSON only.".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: format!("Role: {}\nScraped job summaries:\n{}\n\nSynthesize market insights.", role_title, scraped_jobs.join("\n---\n")),
            },
        ]
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
