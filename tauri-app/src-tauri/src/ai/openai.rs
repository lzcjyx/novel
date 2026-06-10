use super::deepseek::DeepSeekProvider;
use crate::ai::client::{ModelClient, ModelUsageReport};
use async_trait::async_trait;
use serde_json::Value; // Same API shape, reuse implementation approach

pub struct OpenAIProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_secs: u64,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            embedding_model: "text-embedding-3-small".into(),
            timeout_secs: 600,
        }
    }
}

// OpenAI uses the same REST API shape as DeepSeek, so delegate to the same internal pattern.
// We inline the implementation to avoid tight coupling.

#[async_trait]
impl ModelClient for OpenAIProvider {
    async fn generate_json(
        &self,
        system: &str,
        user: &str,
        _json_schema: &Value,
        max_tokens: u32,
    ) -> Result<Value, String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek
            .generate_json(system, user, _json_schema, max_tokens)
            .await
    }

    async fn generate_json_with_usage(
        &self,
        system: &str,
        user: &str,
        json_schema: &Value,
        max_tokens: u32,
    ) -> Result<(Value, Option<ModelUsageReport>), String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek
            .generate_json_with_usage(system, user, json_schema, max_tokens)
            .await
    }

    async fn generate_text(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek.generate_text(system, user, max_tokens).await
    }

    async fn generate_text_with_usage(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<(String, Option<ModelUsageReport>), String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek
            .generate_text_with_usage(system, user, max_tokens)
            .await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek.embed(texts).await
    }
}
