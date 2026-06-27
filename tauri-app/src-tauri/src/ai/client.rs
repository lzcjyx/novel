use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ModelUsageReport {
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingInputKind {
    Document,
    Query,
}

#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn generate_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_schema: &Value,
        max_tokens: u32,
    ) -> Result<Value, String>;

    async fn generate_json_with_usage(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_schema: &Value,
        max_tokens: u32,
    ) -> Result<(Value, Option<ModelUsageReport>), String> {
        self.generate_json(system_prompt, user_prompt, json_schema, max_tokens)
            .await
            .map(|value| (value, None))
    }

    async fn generate_text(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<String, String>;

    async fn generate_text_with_usage(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<(String, Option<ModelUsageReport>), String> {
        self.generate_text(system_prompt, user_prompt, max_tokens)
            .await
            .map(|value| (value, None))
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;

    async fn embed_with_kind(
        &self,
        texts: &[String],
        _kind: EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        self.embed(texts).await
    }
}
