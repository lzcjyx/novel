use async_trait::async_trait;
use serde_json::Value;
use crate::ai::client::ModelClient;
use crate::ai::deepseek::DeepSeekProvider;

pub struct OpenAICompatibleProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_secs: u64,
}

impl OpenAICompatibleProvider {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
            embedding_model: "text-embedding-3-small".into(),
            timeout_secs: 600,
        }
    }
}

#[async_trait]
impl ModelClient for OpenAICompatibleProvider {
    async fn generate_json(&self, system: &str, user: &str, schema: &Value, max_tokens: u32) -> Result<Value, String> {
        let inner = DeepSeekProvider {
            api_key: self.api_key.clone(), base_url: self.base_url.clone(),
            model: self.model.clone(), embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        inner.generate_json(system, user, schema, max_tokens).await
    }

    async fn generate_text(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
        let inner = DeepSeekProvider {
            api_key: self.api_key.clone(), base_url: self.base_url.clone(),
            model: self.model.clone(), embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        inner.generate_text(system, user, max_tokens).await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let inner = DeepSeekProvider {
            api_key: self.api_key.clone(), base_url: self.base_url.clone(),
            model: self.model.clone(), embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        inner.embed(texts).await
    }
}
