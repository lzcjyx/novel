use async_trait::async_trait;
use serde_json::{json, Value};
use crate::ai::client::ModelClient;
use crate::ai::types::*;

pub struct DeepSeekProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_secs: u64,
}

impl DeepSeekProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.deepseek.com".into(),
            model: "deepseek-v4-pro".into(),
            embedding_model: "text-embedding-3-small".into(),
            timeout_secs: 600,
        }
    }

    async fn chat(&self, system: &str, user: &str, max_tokens: u32, json_mode: bool) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("Client build: {}", e))?;

        let mut body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "max_tokens": max_tokens,
            "temperature": 1.0,
            "stream": false
        });

        if json_mode {
            body["response_format"] = json!({"type": "json_object"});
        }

        for attempt in 0..3 {
            let api_url = if self.base_url.ends_with("/v1") || self.base_url.ends_with("/v1/") {
                format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
            } else {
                format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'))
            };
            let resp = client.post(&api_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("HTTP error: {}", e))?;

            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read: {}", e))?;

            if status.is_success() {
                let parsed: ChatCompletionResponse = serde_json::from_str(&text)
                    .map_err(|e| format!("Parse response: {} — body: {}", e, &text[..text.len().min(500)]))?;
                return Ok(parsed.choices.first()
                    .and_then(|c| c.message.as_ref())
                    .map(|m| m.content.clone())
                    .unwrap_or_default());
            }

            if status.as_u16() == 429 || status.is_server_error() {
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_secs((attempt + 1) as u64 * 2)).await;
                    continue;
                }
            }
            return Err(format!("API error {}: {}", status, &text[..text.len().min(300)]));
        }
        Err("Max retries exceeded".into())
    }

    fn extract_json(content: &str) -> Result<Value, String> {
        let content = content.trim();
        // Try direct parse first
        if let Ok(v) = serde_json::from_str::<Value>(content) {
            return Ok(v);
        }
        // Strip markdown code blocks robustly
        let cleaned = Self::strip_code_block(content);
        serde_json::from_str(&cleaned)
            .map_err(|e| format!("JSON parse error: {} — first 400 chars: {}", e, &cleaned[..cleaned.len().min(400)]))
    }

    fn strip_code_block(s: &str) -> String {
        let s = s.trim();
        // Remove opening ```json or ``` (can be followed by newline)
        let s = s.strip_prefix("```json")
            .or_else(|| s.strip_prefix("```"))
            .map(|rest| rest.trim_start())
            .unwrap_or(s);
        // Remove closing ```
        let s = s.strip_suffix("```")
            .map(|rest| rest.trim_end())
            .unwrap_or(s);
        s.to_string()
    }
}

#[async_trait]
impl ModelClient for DeepSeekProvider {
    async fn generate_json(&self, system: &str, user: &str, _json_schema: &Value, max_tokens: u32) -> Result<Value, String> {
        let content = self.chat(system, user, max_tokens, true).await?;
        Self::extract_json(&content)
    }

    async fn generate_text(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
        self.chat(system, user, max_tokens, false).await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("Client: {}", e))?;

        let emb_url = if self.base_url.ends_with("/v1") || self.base_url.ends_with("/v1/") {
            format!("{}/embeddings", self.base_url.trim_end_matches('/'))
        } else {
            format!("{}/v1/embeddings", self.base_url.trim_end_matches('/'))
        };
        let resp = client.post(&emb_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({"model": self.embedding_model, "input": texts}))
            .send().await.map_err(|e| format!("HTTP: {}", e))?
            .json::<EmbeddingResponse>().await
            .map_err(|e| format!("Parse: {}", e))?;

        Ok(resp.data.into_iter().map(|d| d.embedding).collect())
    }
}
