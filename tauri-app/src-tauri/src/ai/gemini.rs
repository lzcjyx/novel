use async_trait::async_trait;
use serde_json::{json, Value};
use crate::ai::client::ModelClient;

pub struct GeminiProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_secs: u64,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            model: "gemini-2.5-pro".into(),
            embedding_model: "text-embedding-004".into(),
            timeout_secs: 600,
        }
    }

    async fn call_gemini(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build().map_err(|e| format!("Client: {}", e))?;

        let contents = json!({
            "contents": [{
                "parts": [{"text": user}]
            }],
            "systemInstruction": {
                "parts": [{"text": system}]
            },
            "generationConfig": {
                "maxOutputTokens": max_tokens,
                "temperature": 0.7,
            }
        });

        for attempt in 0..3 {
            let resp = client.post(format!("{}/models/{}:generateContent?key={}", self.base_url, self.model, self.api_key))
                .header("Content-Type", "application/json")
                .json(&contents)
                .send().await.map_err(|e| format!("HTTP: {}", e))?;

            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("Read: {}", e))?;

            if status.is_success() {
                let parsed: Value = serde_json::from_str(&text)
                    .map_err(|e| format!("Parse: {} — body: {}", e, &text[..text.len().min(300)]))?;
                return Ok(parsed["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str().unwrap_or("").to_string());
            }

            if status.as_u16() == 429 || status.is_server_error() {
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_secs((attempt + 1) as u64 * 3)).await;
                    continue;
                }
            }
            return Err(format!("Gemini error {}: {}", status, &text[..text.len().min(300)]));
        }
        Err("Max retries".into())
    }
}

#[async_trait]
impl ModelClient for GeminiProvider {
    async fn generate_json(&self, system: &str, user: &str, _schema: &Value, max_tokens: u32) -> Result<Value, String> {
        let prompt = format!("{}\n\nRespond with ONLY valid JSON, no markdown formatting, no extra text.", user);
        let content = self.call_gemini(system, &prompt, max_tokens).await?;
        let content = content.trim()
            .trim_start_matches("```json").trim_start_matches("```")
            .trim_end_matches("```").trim();
        serde_json::from_str(content)
            .map_err(|e| format!("JSON parse: {} — first 200 chars: {}", e, &content[..content.len().min(200)]))
    }

    async fn generate_text(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
        self.call_gemini(system, user, max_tokens).await
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build().map_err(|e| format!("Client: {}", e))?;

        let mut embeddings = Vec::new();
        for text in texts {
            let resp = client.post(format!("{}/models/{}:embedContent?key={}", self.base_url, self.embedding_model, self.api_key))
                .header("Content-Type", "application/json")
                .json(&json!({"content": {"parts": [{"text": text}]}}))
                .send().await.map_err(|e| format!("HTTP: {}", e))?
                .json::<Value>().await.map_err(|e| format!("Parse: {}", e))?;

            let vec: Vec<f32> = resp["embedding"]["values"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect())
                .unwrap_or_default();
            embeddings.push(vec);
        }
        Ok(embeddings)
    }
}
