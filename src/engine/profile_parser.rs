use anyhow::{Context, Result};
use chrono::Utc;
use csv::ReaderBuilder;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::profile::{
    Education, Experience, Skill, SkillCategory, SkillLevel, UserProfile,
};

/// Read a text file tolerating UTF-8, UTF-16LE, and UTF-16BE (BOM-detected).
pub fn read_text_lossy(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("Failed to read file: {}", path.display()))?;
    Ok(decode_text_lossy(&bytes))
}

pub fn decode_text_lossy(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[derive(Debug)]
pub enum ProfileInput {
    Markdown { path: PathBuf },
    Pdf { path: PathBuf },
    Docx { path: PathBuf },
    LinkedInExport { path: PathBuf },
    PortfolioUrl { url: String },
}

pub struct ProfileParser;

impl ProfileParser {
    pub fn parse(input: ProfileInput) -> Result<UserProfile> {
        match input {
            ProfileInput::Markdown { path } => Self::parse_markdown(&path),
            ProfileInput::Pdf { path } => Self::parse_pdf(&path),
            ProfileInput::Docx { path } => Self::parse_docx(&path),
            ProfileInput::LinkedInExport { path } => Self::parse_linkedin_export(&path),
            ProfileInput::PortfolioUrl { url } => {
                let body = reqwest::blocking::get(&url)
                    .context("Failed to fetch portfolio URL")?
                    .text()
                    .context("Failed to read portfolio body")?;
                let document = scraper::Html::parse_document(&body);
                let selector = scraper::Selector::parse(
                    "main, article, .content, #content, .portfolio, #portfolio",
                )
                .unwrap();
                let text = document
                    .select(&selector)
                    .map(|el| el.text().collect::<Vec<_>>().join(" "))
                    .collect::<Vec<_>>()
                    .join(" ");
                if text.is_empty() {
                    anyhow::bail!("Portfolio site returned no readable content");
                }
                Self::profile_from_text(&text)
            }
        }
    }

    fn parse_markdown(path: &Path) -> Result<UserProfile> {
        let text = read_text_lossy(path).context("Failed to read markdown profile")?;
        Self::profile_from_text(&text)
    }

    fn parse_pdf(_path: &Path) -> Result<UserProfile> {
        anyhow::bail!("PDF extraction requires a text extraction library. Convert your PDF to markdown and retry.")
    }

    fn parse_docx(path: &Path) -> Result<UserProfile> {
        let text = Self::extract_docx_text(path)?;
        Self::profile_from_text(&text)
    }

    fn parse_linkedin_export(dir: &Path) -> Result<UserProfile> {
        let mut profile = UserProfile {
            name: String::new(),
            email: None,
            phone: None,
            location: None,
            linkedin_url: None,
            portfolio_url: None,
            summary: None,
            skills: Vec::new(),
            experience: Vec::new(),
            education: Vec::new(),
            certifications: Vec::new(),
            languages: Vec::new(),
            raw_text: String::new(),
            inferred_roles: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Parse Profile.csv
        let profile_csv = dir.join("Profile.csv");
        if profile_csv.exists() {
            let mut rdr = ReaderBuilder::new()
                .has_headers(true)
                .from_path(&profile_csv)?;
            for result in rdr.records() {
                let record = result?;
                if record.len() > 6 {
                    profile.name = format!(
                        "{} {}",
                        record.get(0).unwrap_or(""),
                        record.get(1).unwrap_or("")
                    )
                    .trim()
                    .to_string();
                    profile.location = record.get(9).map(|s| s.to_string());
                    profile.summary = record.get(6).map(|s| s.to_string());
                    profile.raw_text.push_str(&format!(
                        "Name: {}\nHeadline: {}\nSummary: {}\n\n",
                        profile.name,
                        record.get(5).unwrap_or(""),
                        profile.summary.as_deref().unwrap_or("")
                    ));
                }
            }
        }

        // Parse Positions.csv
        let positions_csv = dir.join("Positions.csv");
        if positions_csv.exists() {
            let mut rdr = ReaderBuilder::new()
                .has_headers(true)
                .from_path(&positions_csv)?;
            for result in rdr.records() {
                let record = result?;
                if record.len() >= 6 {
                    let title = record.get(1).unwrap_or("").to_string();
                    let company = record.get(0).unwrap_or("").to_string();
                    let description = record.get(2).unwrap_or("").to_string();
                    let location = record.get(3).map(|s| s.to_string());
                    let start = record.get(4).map(|s| s.to_string());
                    let end = record.get(5).map(|s| s.to_string());
                    let current = end.as_ref().map(|s| s.is_empty()).unwrap_or(false);

                    profile.raw_text.push_str(&format!(
                        "Experience: {} at {} ({})\n{}\n\n",
                        title,
                        company,
                        start.as_deref().unwrap_or("?"),
                        description
                    ));

                    profile.experience.push(Experience {
                        id: uuid::Uuid::new_v4().to_string(),
                        title,
                        company,
                        location,
                        start_date: start,
                        end_date: end,
                        current,
                        description,
                        achievements: Vec::new(),
                        skills_used: Vec::new(),
                    });
                }
            }
        }

        // Parse Skills.csv
        let skills_csv = dir.join("Skills.csv");
        if skills_csv.exists() {
            let mut rdr = ReaderBuilder::new()
                .has_headers(false)
                .from_path(&skills_csv)?;
            for result in rdr.records() {
                let record = result?;
                if let Some(skill_name) = record.get(0) {
                    let skill_name = skill_name.trim();
                    if !skill_name.is_empty() {
                        profile.skills.push(Skill {
                            name: skill_name.to_string(),
                            category: Self::categorize_skill(skill_name),
                            level: SkillLevel::Advanced,
                            years: None,
                        });
                    }
                }
            }
            profile.raw_text.push_str(&format!(
                "Skills: {}\n\n",
                profile
                    .skills
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Parse Education.csv
        let education_csv = dir.join("Education.csv");
        if education_csv.exists() {
            let mut rdr = ReaderBuilder::new()
                .has_headers(true)
                .from_path(&education_csv)?;
            for result in rdr.records() {
                let record = result?;
                if record.len() >= 5 {
                    profile.education.push(Education {
                        id: uuid::Uuid::new_v4().to_string(),
                        institution: record.get(0).unwrap_or("").to_string(),
                        degree: record.get(4).unwrap_or("").to_string(),
                        field: String::new(),
                        start_date: record.get(1).map(|s| s.to_string()),
                        end_date: record.get(2).map(|s| s.to_string()),
                        gpa: None,
                    });
                }
            }
        }

        if profile.name.is_empty() {
            profile.name = "LinkedIn User".to_string();
        }

        Ok(profile)
    }

    pub fn profile_from_text(text: &str) -> Result<UserProfile> {
        let name = Self::extract_name(text).unwrap_or_else(|| "Unknown".to_string());
        let email = Self::extract_email(text);
        let phone = Self::extract_phone(text);
        let location = Self::extract_location(text);
        let summary = Self::extract_summary(text);
        let skills = Self::extract_skills(text);
        let experience = Self::extract_experience(text);
        let education = Self::extract_education(text);

        Ok(UserProfile {
            name,
            email,
            phone,
            location,
            linkedin_url: None,
            portfolio_url: None,
            summary,
            skills,
            experience,
            education,
            certifications: Vec::new(),
            languages: Vec::new(),
            raw_text: text.to_string(),
            inferred_roles: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    fn extract_name(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?i)^(?:name\s*[:\-]\s*)?(.*)").ok()?;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(cap) = re.captures(trimmed) {
                let candidate = cap
                    .get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                if !candidate.is_empty() && candidate.len() < 120 {
                    return Some(candidate);
                }
            }
        }
        text.lines()
            .find(|l| !l.trim().is_empty())
            .map(|s| s.trim().to_string())
    }

    fn extract_email(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }

    fn extract_phone(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"\+?\d[\d\s\-()]{8,}").ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }

    fn extract_location(text: &str) -> Option<String> {
        for line in text.lines().take(10) {
            if line.contains(",") && line.len() < 100 {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    fn extract_summary(text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?i)(summary|objective|profile)\s*[:\-]\s*(.+)").ok()?;
        re.captures(text)
            .and_then(|c| c.get(2).map(|m| m.as_str().trim().to_string()))
    }

    fn extract_skills(text: &str) -> Vec<Skill> {
        let mut skills = Vec::new();
        let skill_patterns = vec![
            r"(?i)skills?\s*[:\-]\s*(.+)",
            r"(?i)technical skills?\s*[:\-]\s*(.+)",
            r"(?i)competencies?\s*[:\-]\s*(.+)",
        ];
        for pattern in skill_patterns {
            if let Some(cap) = regex::Regex::new(pattern)
                .ok()
                .and_then(|re| re.captures(text))
            {
                let list = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                // Note: '-' is deliberately excluded - it only shreds hyphenated
                // skill names like "Solution-oriented" into two entries.
                for skill in list.split([',', ';', '|', '•', '\n']) {
                    let skill = skill.trim();
                    if !skill.is_empty() && skill.len() < 60 {
                        skills.push(Skill {
                            name: skill.to_string(),
                            category: Self::categorize_skill(skill),
                            level: SkillLevel::Intermediate,
                            years: None,
                        });
                    }
                }
                if !skills.is_empty() {
                    return skills;
                }
            }
        }
        skills
    }

    fn extract_experience(text: &str) -> Vec<Experience> {
        let mut experiences = Vec::new();

        // Primary path: structured "Experience: <Title> at <Company> (<Date>)"
        // headers, as produced by profile init and used throughout the UAT
        // fixtures. No cap on the number of entries.
        if let Ok(header_re) = regex::Regex::new(r"(?m)^Experience:\s*(.+)$") {
            let matches: Vec<_> = header_re.find_iter(text).collect();
            if !matches.is_empty() {
                let entry_re = regex::Regex::new(r"^(.*)\s+at\s+(.+?)\s*\(([^)]*)\)\s*$").ok();
                for (i, m) in matches.iter().enumerate() {
                    let header_text = m
                        .as_str()
                        .strip_prefix("Experience:")
                        .unwrap_or(m.as_str())
                        .trim();
                    let body_start = m.end();
                    let body_end = matches
                        .get(i + 1)
                        .map(|next| next.start())
                        .unwrap_or(text.len());
                    let description = text[body_start..body_end].trim().to_string();

                    let (title, company, start_date) =
                        match entry_re.as_ref().and_then(|re| re.captures(header_text)) {
                            Some(cap) => (
                                cap.get(1)
                                    .map(|g| g.as_str().trim().to_string())
                                    .unwrap_or_default(),
                                cap.get(2)
                                    .map(|g| g.as_str().trim().to_string())
                                    .unwrap_or_default(),
                                cap.get(3)
                                    .map(|g| g.as_str().trim().to_string())
                                    .filter(|s| !s.is_empty()),
                            ),
                            None => (header_text.to_string(), "Unknown".to_string(), None),
                        };

                    experiences.push(Experience {
                        id: uuid::Uuid::new_v4().to_string(),
                        title,
                        company,
                        location: None,
                        start_date,
                        end_date: None,
                        current: false,
                        description,
                        achievements: Vec::new(),
                        skills_used: Vec::new(),
                    });
                }
            }
        }

        // Fallback: old heuristic, for profiles without structured headers.
        if experiences.is_empty() {
            let re =
                regex::Regex::new(r"(?i)(experience|work history|professional experience)").ok();
            if let Some(start) = re.and_then(|re| re.find(text)) {
                let block = &text[start.end()..];
                for chunk in block.split("\n\n") {
                    let lines: Vec<&str> = chunk.lines().filter(|l| !l.trim().is_empty()).collect();
                    if lines.len() >= 2 {
                        let title = lines[0].trim().to_string();
                        let company = lines[1].trim().to_string();
                        let description = lines[2..].join("\n");
                        experiences.push(Experience {
                            id: uuid::Uuid::new_v4().to_string(),
                            title,
                            company,
                            location: None,
                            start_date: None,
                            end_date: None,
                            current: false,
                            description,
                            achievements: Vec::new(),
                            skills_used: Vec::new(),
                        });
                    } else if lines.len() == 1 {
                        let parts: Vec<&str> = lines[0].splitn(3, ['-', '|']).collect();
                        if parts.len() >= 2 {
                            experiences.push(Experience {
                                id: uuid::Uuid::new_v4().to_string(),
                                title: parts[0].trim().to_string(),
                                company: parts[1].trim().to_string(),
                                location: parts.get(2).map(|s| s.trim().to_string()),
                                start_date: None,
                                end_date: None,
                                current: false,
                                description: String::new(),
                                achievements: Vec::new(),
                                skills_used: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        if experiences.is_empty() {
            experiences.push(Experience {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Professional".to_string(),
                company: "Various".to_string(),
                location: None,
                start_date: None,
                end_date: None,
                current: false,
                description: text[..text.len().min(2000)].to_string(),
                achievements: Vec::new(),
                skills_used: Vec::new(),
            });
        }
        experiences
    }

    fn extract_education(_text: &str) -> Vec<crate::models::profile::Education> {
        vec![crate::models::profile::Education {
            id: uuid::Uuid::new_v4().to_string(),
            institution: "Unknown".to_string(),
            degree: "N/A".to_string(),
            field: "N/A".to_string(),
            start_date: None,
            end_date: None,
            gpa: None,
        }]
    }

    fn extract_docx_text(path: &Path) -> Result<String> {
        let data = fs::read(path).context("Failed to read DOCX")?;
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(data)).context("Failed to open DOCX zip")?;
        let mut file = archive
            .by_name("word/document.xml")
            .context("DOCX missing document.xml")?;
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut file, &mut xml).context("Failed to read DOCX XML")?;
        let text = xml
            .split('<')
            .filter_map(|s| s.split('>').nth(1).filter(|t| !t.is_empty()))
            .collect::<Vec<_>>()
            .join(" ");
        Ok(text)
    }

    fn categorize_skill(skill: &str) -> SkillCategory {
        let lower = skill.to_lowercase();
        if lower.contains("sales")
            || lower.contains("business development")
            || lower.contains("revenue")
            || lower.contains("account")
        {
            SkillCategory::Domain
        } else if lower.contains("management")
            || lower.contains("leadership")
            || lower.contains("coaching")
            || lower.contains("strategy")
        {
            SkillCategory::Soft
        } else if lower.contains("cloud")
            || lower.contains("software")
            || lower.contains("data")
            || lower.contains("security")
        {
            SkillCategory::Technical
        } else {
            SkillCategory::Tool
        }
    }
}
