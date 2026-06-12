use crate::ai::client::ModelClient;
use crate::ai::preview_chars;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct AnthropicProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com".into(),
            model: "claude-sonnet-4-6".into(),
            timeout_secs: 600,
        }
    }

    async fn call_claude(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("Client: {}", e))?;

        let body = json!({
            "model": self.model,
            "system": system,
            "messages": [{"role": "user", "content": user}],
            "max_tokens": max_tokens,
            "temperature": 0.7,
        });

        for attempt in 0..3 {
            let resp = client
                .post(format!("{}/v1/messages", self.base_url))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("HTTP: {}", e))?;

            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read: {}", e))?;

            if status.is_success() {
                let parsed: Value = serde_json::from_str(&text)
                    .map_err(|e| format!("Parse: {} — body: {}", e, preview_chars(&text, 300)))?;
                return Ok(parsed["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string());
            }

            if status.as_u16() == 429 || status.is_server_error() {
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_secs((attempt + 1) as u64 * 2))
                        .await;
                    continue;
                }
            }
            return Err(format!(
                "Anthropic error {}: {}",
                status,
                preview_chars(&text, 300)
            ));
        }
        Err("Max retries".into())
    }

    fn extract_json(content: &str) -> Result<Value, String> {
        let content = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        serde_json::from_str(content).map_err(|e| {
            format!(
                "JSON parse: {} — first 200 chars: {}",
                e,
                preview_chars(content, 200)
            )
        })
    }
}

#[async_trait]
impl ModelClient for AnthropicProvider {
    async fn generate_json(
        &self,
        system: &str,
        user: &str,
        _schema: &Value,
        max_tokens: u32,
    ) -> Result<Value, String> {
        let prompt = format!(
            "{}\n\nYou MUST respond with ONLY valid JSON. No markdown, no explanation.",
            user
        );
        let content = self.call_claude(system, &prompt, max_tokens).await?;
        Self::extract_json(&content)
    }

    async fn generate_text(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        self.call_claude(system, user, max_tokens).await
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Err("Anthropic does not support embeddings. Use OpenAI or DeepSeek for embeddings.".into())
    }
}
