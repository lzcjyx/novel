use serde::{Deserialize, Serialize};

use crate::db::model_profiles::ModelProfile;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelWorkflow {
    Draft,
    Review,
    Repair,
    StructuredExtraction,
    Embedding,
    GraphRagDraft,
    Summarization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCapabilityWarning {
    pub code: String,
    pub severity: String,
    pub message: String,
}

fn warning(code: &str, severity: &str, message: &str) -> ModelCapabilityWarning {
    ModelCapabilityWarning {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
    }
}

pub fn validate_model_profile_for_workflow(
    profile: &ModelProfile,
    workflow: ModelWorkflow,
) -> Vec<ModelCapabilityWarning> {
    let mut warnings = Vec::new();

    match workflow {
        ModelWorkflow::GraphRagDraft => {
            if profile.context_window < 16_000 {
                warnings.push(warning(
                    "context_window_too_small",
                    "warning",
                    "Graph-RAG draft generation works best with at least a 16k context window.",
                ));
            }
            if !profile.supports_json {
                warnings.push(warning(
                    "json_not_guaranteed",
                    "warning",
                    "Draft, review, and canon extraction workflows expect reliable JSON output.",
                ));
            }
            if !profile.supports_embeddings {
                warnings.push(warning(
                    "embeddings_unsupported",
                    "warning",
                    "This profile cannot be reused as an embedding profile for RAG retrieval.",
                ));
            }
        }
        ModelWorkflow::StructuredExtraction | ModelWorkflow::Summarization => {
            if !profile.supports_json {
                warnings.push(warning(
                    "json_not_guaranteed",
                    "error",
                    "Structured extraction and summarization require reliable JSON output.",
                ));
            }
        }
        ModelWorkflow::Embedding => {
            if !profile.supports_embeddings {
                warnings.push(warning(
                    "embeddings_unsupported",
                    "error",
                    "Embedding workflows require an embedding-capable profile.",
                ));
            }
        }
        ModelWorkflow::Draft | ModelWorkflow::Review | ModelWorkflow::Repair => {
            if profile.context_window < 8_000 {
                warnings.push(warning(
                    "context_window_too_small",
                    "warning",
                    "Long-form writing workflows need enough context for canon and recent chapters.",
                ));
            }
            if !profile.supports_json {
                warnings.push(warning(
                    "json_not_guaranteed",
                    "warning",
                    "This workflow expects JSON-shaped model output.",
                ));
            }
        }
    }

    warnings
}
