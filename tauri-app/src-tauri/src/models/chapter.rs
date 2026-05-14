use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub project_id: String,
    pub chapter_plan_id: Option<String>,
    pub sequence: i32,
    pub title: Option<String>,
    pub final_version_id: Option<String>,
    pub status: String,
    pub word_count: Option<i32>,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    pub id: String,
    pub chapter_id: String,
    pub project_id: String,
    pub version_number: i32,
    pub version_type: String,
    pub title: Option<String>,
    pub body_markdown: Option<String>,
    pub summary: Option<String>,
    pub word_count: Option<i32>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub prompt_hash: Option<String>,
    pub context_hash: Option<String>,
    pub created_by_agent: Option<String>,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterPlan {
    pub id: String,
    pub project_id: String,
    pub volume_id: Option<String>,
    pub sequence: i32,
    pub title: Option<String>,
    pub outline: Option<String>,
    pub pov_character_id: Option<String>,
    pub target_word_count: Option<i32>,
    #[serde(default)]
    pub required_characters: String,
    #[serde(default)]
    pub required_locations: String,
    #[serde(default)]
    pub plot_goals: String,
    #[serde(default)]
    pub required_foreshadowing: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub ok: bool,
    pub message: String,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub sequence: Option<i32>,
    pub word_count: Option<i32>,
    pub final_score: Option<f64>,
    pub decision: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub ok: bool,
    pub novel: Option<NovelBrief>,
    pub slug: Option<String>,
    pub chapter_count: Option<i32>,
    pub chapters_today: Option<i32>,
    pub plans_left: Option<i32>,
    pub total_words: Option<i64>,
    pub is_running: bool,
    pub daily_schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelBrief {
    pub name: String,
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterFile {
    pub filename: String,
    pub title: String,
    pub sequence: u32,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleData {
    pub characters: Vec<Character>,
    pub locations: Vec<Location>,
    pub organizations: Vec<Organization>,
    pub items: Vec<Item>,
    pub world_lore: Vec<WorldLore>,
    pub magic_systems: Vec<MagicSystem>,
    pub canon_rules: Vec<CanonRule>,
    pub plot_threads: Vec<PlotThread>,
    pub foreshadowing: Vec<Foreshadowing>,
    pub style_guides: Vec<StyleGuide>,
    pub timeline_events: Vec<TimelineEvent>,
}

impl BibleData {
    pub fn empty() -> Self {
        Self {
            characters: vec![],
            locations: vec![],
            organizations: vec![],
            items: vec![],
            world_lore: vec![],
            magic_systems: vec![],
            canon_rules: vec![],
            plot_threads: vec![],
            foreshadowing: vec![],
            style_guides: vec![],
            timeline_events: vec![],
        }
    }
}

// Re-export bible types
pub use super::bible::*;
