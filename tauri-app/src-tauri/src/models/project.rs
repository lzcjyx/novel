use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub genre: Option<String>,
    pub target_audience: Option<String>,
    #[serde(default)]
    pub style_profile: String,
    pub total_target_words: Option<i32>,
    pub daily_target_words: Option<i32>,
    pub auto_publish: bool,
    pub quality_threshold: i32,
    pub blog_provider: Option<String>,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub genre: Option<String>,
    pub status: String,
    pub target_words: Option<i32>,
    pub chapter_count: i32,
    pub total_words: i64,
    pub plans_left: i32,
    pub chapters_today: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub sub_genre: Option<String>,
    pub target_audience: Option<String>,
    pub tone: Option<String>,
    pub style_profile_desc: Option<String>,
    pub target_total_words: Option<u32>,
    pub daily_target_words: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub id: String,
    pub project_id: String,
    pub sequence: i32,
    pub title: String,
    pub summary: Option<String>,
    pub target_word_count: Option<i32>,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}
