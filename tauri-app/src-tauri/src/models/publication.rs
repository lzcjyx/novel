use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationQueueInput {
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_version_id: Option<String>,
    pub provider: String,
    pub scheduled_at: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticSitePost {
    pub title: String,
    pub slug: String,
    pub published: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub lang: Option<String>,
    pub body_markdown: String,
}
