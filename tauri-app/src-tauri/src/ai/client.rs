use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn generate_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_schema: &Value,
        max_tokens: u32,
    ) -> Result<Value, String>;

    async fn generate_text(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<String, String>;

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
}
