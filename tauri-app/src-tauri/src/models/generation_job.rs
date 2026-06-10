use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationJob {
    pub id: String,
    pub project_id: String,
    pub chapter_plan_id: String,
    pub job_date: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPost {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub provider: Option<String>,
    pub external_post_id: Option<String>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub url: Option<String>,
    pub status: String,
    pub published_at: Option<String>,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationQueueItem {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_version_id: Option<String>,
    pub provider: Option<String>,
    pub status: String,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub error_message: Option<String>,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub project_id: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub similarity: Option<f64>,
}

/// Progress event emitted via Tauri events during pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub step: String,
    pub status: String,
    pub elapsed_ms: Option<u64>,
    pub detail: Option<String>,
    pub progress_pct: f64,
    pub timestamp: String,
    #[serde(default)]
    pub preview_title: Option<String>,
    #[serde(default)]
    pub preview_text: Option<String>,
    #[serde(default)]
    pub preview_kind: Option<String>,
}

/// Self-learning knowledge entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub id: String,
    pub project_id: String,
    pub source_type: String,
    pub source_url: Option<String>,
    pub source_title: Option<String>,
    pub category: String,
    pub pattern_name: String,
    pub pattern_description: String,
    pub example_text: Option<String>,
    pub application_notes: Option<String>,
    pub confidence: f64,
    pub usage_count: i32,
    pub last_used_at: Option<String>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}
