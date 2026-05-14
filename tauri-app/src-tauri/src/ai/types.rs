use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    #[serde(rename = "max_tokens")]
    pub max_tokens: Option<u32>,
    #[serde(rename = "response_format")]
    pub response_format: Option<serde_json::Value>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: Option<i32>,
    pub message: Option<ChatMessage>,
    pub delta: Option<ChatMessage>,
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: Option<i32>,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: Option<i32>,
    #[serde(rename = "total_tokens")]
    pub total_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: Option<i32>,
}
