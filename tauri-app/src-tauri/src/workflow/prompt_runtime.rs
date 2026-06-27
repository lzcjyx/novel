use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::workflow::prompt_rendering::render_prompt_strict;

pub const DRAFT_WRITER_USER_INSTRUCTION: &str =
    "请基于 system prompt 中的 writing_context 生成本章正文，只输出合法 JSON。";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUnit {
    pub identifier: String,
    pub role: String,
    pub order: i32,
    pub enabled: bool,
    pub injection_position: String,
    pub generation_phase: String,
    pub content: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRuntimeRequest {
    pub prompt_name: String,
    pub generation_phase: String,
    pub vars: HashMap<String, String>,
    pub units: Vec<PromptUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUnitTrace {
    pub identifier: String,
    pub role: String,
    pub order: i32,
    pub injection_position: String,
    pub generation_phase: String,
    pub token_estimate: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledPrompt {
    pub prompt_name: String,
    pub generation_phase: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub token_estimate: i32,
    pub unit_traces: Vec<PromptUnitTrace>,
}

fn unit_matches_phase(unit: &PromptUnit, generation_phase: &str) -> bool {
    let phase = unit.generation_phase.trim();
    phase.is_empty() || phase == "all" || phase == generation_phase
}

pub fn assemble_prompt_runtime(request: PromptRuntimeRequest) -> Result<AssembledPrompt, String> {
    let mut units = request
        .units
        .into_iter()
        .filter(|unit| unit.enabled && unit_matches_phase(unit, &request.generation_phase))
        .collect::<Vec<_>>();
    units.sort_by(|a, b| a.order.cmp(&b.order).then(a.identifier.cmp(&b.identifier)));

    let vars = request
        .vars
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect::<HashMap<_, _>>();
    let mut system_units = Vec::new();
    let mut user_units = Vec::new();
    let mut traces = Vec::new();

    for unit in units {
        let rendered = render_prompt_strict(
            &format!("{}:{}", request.prompt_name, unit.identifier),
            &unit.content,
            &vars,
        )
        .map_err(|err| {
            format!(
                "Prompt '{}' unit '{}' failed: {}",
                request.prompt_name, unit.identifier, err
            )
        })?;
        let token_estimate = crate::db::generation_jobs::estimate_tokens(&rendered);
        match unit.role.as_str() {
            "system" => system_units.push(rendered),
            "user" => user_units.push(rendered),
            other => {
                return Err(format!(
                    "Prompt '{}' unit '{}' has unsupported role '{}'",
                    request.prompt_name, unit.identifier, other
                ));
            }
        }
        traces.push(PromptUnitTrace {
            identifier: unit.identifier,
            role: unit.role,
            order: unit.order,
            injection_position: unit.injection_position,
            generation_phase: unit.generation_phase,
            token_estimate,
        });
    }

    let system_prompt = system_units.join("\n\n");
    let user_prompt = user_units.join("\n\n");
    let token_estimate = crate::db::generation_jobs::estimate_tokens(&system_prompt)
        + crate::db::generation_jobs::estimate_tokens(&user_prompt);

    Ok(AssembledPrompt {
        prompt_name: request.prompt_name,
        generation_phase: request.generation_phase,
        system_prompt,
        user_prompt,
        token_estimate,
        unit_traces: traces,
    })
}

pub fn assemble_builtin_draft_prompt(
    writing_context_json: &str,
) -> Result<AssembledPrompt, String> {
    assemble_builtin_draft_prompt_with_extra_units(writing_context_json, vec![])
}

pub fn assemble_builtin_draft_prompt_with_extra_units(
    writing_context_json: &str,
    mut extra_units: Vec<PromptUnit>,
) -> Result<AssembledPrompt, String> {
    let draft_template = crate::prompts::load_prompt("draft_writer")?;
    let mut units = vec![
        PromptUnit {
            identifier: "draft_writer.system".to_string(),
            role: "system".to_string(),
            order: 10,
            enabled: true,
            injection_position: "system".to_string(),
            generation_phase: "draft".to_string(),
            content: draft_template,
            metadata: serde_json::json!({"source": "built_in"}),
        },
        PromptUnit {
            identifier: "draft_writer.user".to_string(),
            role: "user".to_string(),
            order: 20,
            enabled: true,
            injection_position: "user".to_string(),
            generation_phase: "draft".to_string(),
            content: DRAFT_WRITER_USER_INSTRUCTION.to_string(),
            metadata: serde_json::json!({"source": "built_in"}),
        },
    ];
    units.append(&mut extra_units);
    assemble_prompt_runtime(PromptRuntimeRequest {
        prompt_name: "draft_writer".to_string(),
        generation_phase: "draft".to_string(),
        vars: HashMap::from([(
            "WRITING_CONTEXT_JSON".to_string(),
            writing_context_json.to_string(),
        )]),
        units,
    })
}

pub fn extension_prompt_units_from_metadata(
    workflow_metadata: &serde_json::Value,
) -> Result<Vec<PromptUnit>, String> {
    let Some(contributions) = workflow_metadata
        .get("extension_contributions")
        .and_then(|value| value.get("prompt_pack"))
        .and_then(|value| value.as_array())
    else {
        return Ok(Vec::new());
    };

    contributions
        .iter()
        .enumerate()
        .map(|(index, contribution)| {
            let payload = contribution.get("payload").ok_or_else(|| {
                format!("prompt_pack contribution {} missing payload", index)
            })?;
            let identifier = payload
                .get("unit_identifier")
                .or_else(|| payload.get("identifier"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    format!("prompt_pack contribution {} missing unit_identifier", index)
                })?;
            let content = payload
                .get("content")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("prompt_pack contribution {} missing content", index))?;
            Ok(PromptUnit {
                identifier: identifier.to_string(),
                role: payload
                    .get("role")
                    .and_then(|value| value.as_str())
                    .unwrap_or("system")
                    .to_string(),
                order: payload
                    .get("order")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(15) as i32,
                enabled: payload
                    .get("enabled")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true),
                injection_position: payload
                    .get("injection_position")
                    .and_then(|value| value.as_str())
                    .unwrap_or("system")
                    .to_string(),
                generation_phase: payload
                    .get("generation_phase")
                    .and_then(|value| value.as_str())
                    .unwrap_or("draft")
                    .to_string(),
                content: content.to_string(),
                metadata: serde_json::json!({
                    "source": "extension_prompt_pack",
                    "extension_id": contribution.get("extension_id").cloned().unwrap_or(serde_json::Value::Null),
                    "contribution_id": contribution.get("contribution_id").cloned().unwrap_or(serde_json::Value::Null),
                }),
            })
        })
        .collect()
}

pub fn assembled_prompt_preview_payload(assembled: &AssembledPrompt) -> serde_json::Value {
    serde_json::json!({
        "prompt_name": assembled.prompt_name,
        "generation_phase": assembled.generation_phase,
        "system_prompt": assembled.system_prompt,
        "user_prompt": assembled.user_prompt,
        "token_estimate": assembled.token_estimate,
        "unit_traces": assembled.unit_traces,
    })
}
