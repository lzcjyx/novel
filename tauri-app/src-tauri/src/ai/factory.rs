use super::client::ModelClient;
use super::{anthropic, deepseek, gemini, openai, openai_compat};
use crate::db::model_profiles::ModelProfile;
use crate::models::AppSettings;

pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_secs: u64,
}

impl ProviderConfig {
    pub fn build(&self) -> Result<Box<dyn ModelClient>, String> {
        match self.provider_type.as_str() {
            "deepseek" => Ok(Box::new(deepseek::DeepSeekProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "openai" => Ok(Box::new(openai::OpenAIProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "openai_compat" => Ok(Box::new(openai_compat::OpenAICompatibleProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "anthropic" => Ok(Box::new(anthropic::AnthropicProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "gemini" => Ok(Box::new(gemini::GeminiProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "kimi" => Ok(Box::new(deepseek::DeepSeekProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "zhipu" => Ok(Box::new(deepseek::DeepSeekProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            "custom" => Ok(Box::new(openai_compat::OpenAICompatibleProvider {
                api_key: self.api_key.clone(),
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                embedding_model: self.embedding_model.clone(),
                timeout_secs: self.timeout_secs,
            })),
            _ => Err(format!("Unknown provider: {}", self.provider_type)),
        }
    }
}

pub fn provider_config_for_model_profile(
    settings: &AppSettings,
    profile: Option<&ModelProfile>,
    api_key: String,
) -> ProviderConfig {
    let (provider_type, base_url, model) = profile
        .map(|profile| {
            (
                profile.provider.clone(),
                profile.base_url.clone(),
                profile.model.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                settings.provider.clone(),
                settings.base_url.clone(),
                settings.model.clone(),
            )
        });

    ProviderConfig {
        provider_type,
        api_key,
        base_url,
        model,
        embedding_model: settings.embedding_model.clone(),
        timeout_secs: 600,
    }
}
