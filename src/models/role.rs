use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleArchetype {
    pub id: String,
    pub title: String,
    pub industry: String,
    pub seniority: Seniority,
    pub fit_score: f64,
    pub market_demand: MarketDemand,
    pub compensation_band: CompensationBand,
    pub typical_requirements: Vec<String>,
    pub top_companies: Vec<String>,
    pub inferred_from_profile: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Seniority {
    Intern,
    Junior,
    Mid,
    Senior,
    Lead,
    Manager,
    Director,
    VP,
    CXO,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDemand {
    pub level: DemandLevel,
    pub posting_volume_30d: u32,
    pub trend: TrendDirection,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DemandLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Growing,
    Stable,
    Declining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationBand {
    pub currency: String,
    pub min: u64,
    pub max: u64,
    pub median: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillGap {
    pub skill: String,
    pub required: bool,
    pub user_has: bool,
    pub severity: GapSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GapSeverity {
    Critical,
    Major,
    Minor,
    NiceToHave,
}
