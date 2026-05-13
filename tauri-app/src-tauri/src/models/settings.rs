use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub provider: String,           // "deepseek" | "openai" | "openai_compat" | "anthropic" | "gemini"
    pub model: String,
    pub base_url: String,
    pub embedding_model: String,
    pub embedding_provider: String,   // "openai" | "zhipu" | "openai_compat" | "none"
    pub embedding_base_url: String,   // separate base URL for embedding API
    pub embedding_dim: i32,
    pub quality_threshold: i32,
    pub auto_publish: bool,
    pub max_revise_count: i32,
    pub daily_target_words: i32,
    pub data_dir: String,
    pub debug_mode: bool,
    pub blog_provider: String,
    pub blog_url: Option<String>,
    pub blog_username: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "deepseek".into(),
            model: "deepseek-v4-pro".into(),
            base_url: "https://api.deepseek.com".into(),
            embedding_model: "text-embedding-3-small".into(),
            embedding_provider: "none".into(),
            embedding_base_url: String::new(),
            embedding_dim: 1536,
            quality_threshold: 85,
            auto_publish: false,
            max_revise_count: 2,
            daily_target_words: 3000,
            data_dir: dirs::document_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("AI-Novels")
                .to_string_lossy()
                .to_string(),
            debug_mode: false,
            blog_provider: "none".into(),
            blog_url: None,
            blog_username: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}
