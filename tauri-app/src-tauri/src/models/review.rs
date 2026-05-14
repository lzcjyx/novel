use serde::{Deserialize, Serialize};
use crate::models::chapter::ChapterPlan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReview {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_version_id: Option<String>,
    pub agent_name: String,
    pub score: Option<i32>,
    pub pass: Option<bool>,
    #[serde(default)]
    pub blocking_issues: String,
    #[serde(default)]
    pub minor_issues: String,
    #[serde(default)]
    pub recommendations: String,
    #[serde(default)]
    pub raw_output: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewScores {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_version_id: Option<String>,
    pub average_score: Option<f64>,
    pub final_score: Option<f64>,
    pub decision: Option<String>,
    pub publish_allowed: bool,
    pub blocking_issue_count: i32,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAggregation {
    pub average_score: f64,
    pub final_score: f64,
    pub decision: String,
    pub publish_allowed: bool,
    pub blocking_issue_count: i32,
    pub all_passed: bool,
    pub safety_passed: bool,
    pub reviews: Vec<AgentReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyPlanResult {
    pub ok: bool,
    pub message: String,
    pub plans_created: i32,
    pub plans: Vec<ChapterPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionResult {
    pub ok: bool,
    pub message: String,
    pub chapter_id: Option<String>,
    pub version_number: Option<i32>,
    pub new_score: Option<f64>,
    pub decision: Option<String>,
}
