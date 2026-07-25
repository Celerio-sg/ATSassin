use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub linkedin_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub summary: Option<String>,
    pub skills: Vec<Skill>,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub certifications: Vec<Certification>,
    pub languages: Vec<Language>,
    pub raw_text: String,
    pub inferred_roles: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub category: SkillCategory,
    pub level: SkillLevel,
    pub years: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillCategory {
    Technical,
    Domain,
    Soft,
    Language,
    Tool,
    Certification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub current: bool,
    pub description: String,
    pub achievements: Vec<String>,
    pub skills_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Education {
    pub id: String,
    pub institution: String,
    pub degree: String,
    pub field: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub gpa: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certification {
    pub id: String,
    pub name: String,
    pub issuer: String,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub proficiency: String,
}
