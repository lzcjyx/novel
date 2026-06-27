use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::director::{self, DirectionCandidate, DirectionCandidateInput};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionGenerationRequest {
    pub project_id: Option<String>,
    pub inspiration: String,
    pub candidate_count: usize,
    pub model_profile_snapshot: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorBootstrapHandoff {
    pub candidate_id: String,
    pub project_id: Option<String>,
    pub inspiration: String,
    pub suggested_title: String,
    pub positioning: String,
    pub target_reader: String,
    pub core_hook: String,
    pub series_promise: String,
    pub first_30_chapter_promise: String,
    pub world_seed: Value,
    pub character_seed: Value,
    pub volume_strategy: Value,
    pub golden_three_chapters: Value,
    pub requires_human_review: bool,
}

pub async fn generate_direction_candidates(
    db: &Database,
    provider: &dyn ModelClient,
    request: DirectionGenerationRequest,
) -> Result<Vec<DirectionCandidate>, String> {
    let inspiration = request.inspiration.trim();
    if inspiration.is_empty() {
        return Err("Direction inspiration is required".to_string());
    }
    if !(2..=3).contains(&request.candidate_count) {
        return Err("Director mode requires 2 to 3 candidates".to_string());
    }

    let system_prompt = "You are a long-form fiction director. Return strict JSON only.";
    let user_prompt = format!(
        "Create exactly {} distinct book direction candidates from this inspiration.\n\nInspiration: {}",
        request.candidate_count, inspiration
    );
    let prompt_hash = stable_hash(&format!("{}\n{}", system_prompt, user_prompt));
    let schema = json!({
        "type": "object",
        "properties": {
            "candidates": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title_options": {"type": "array", "items": {"type": "string"}},
                        "positioning": {"type": "string"},
                        "target_reader": {"type": "string"},
                        "core_hook": {"type": "string"},
                        "series_promise": {"type": "string"},
                        "first_30_chapter_promise": {"type": "string"},
                        "world_seed": {"type": "object"},
                        "character_seed": {"type": "object"},
                        "volume_strategy": {"type": "array"},
                        "golden_three_chapters": {"type": "array"},
                        "revision_note": {"type": "string"}
                    },
                    "required": [
                        "title_options",
                        "positioning",
                        "target_reader",
                        "core_hook",
                        "series_promise",
                        "first_30_chapter_promise",
                        "world_seed",
                        "character_seed",
                        "volume_strategy",
                        "golden_three_chapters"
                    ]
                }
            }
        },
        "required": ["candidates"]
    });
    let output = provider
        .generate_json(system_prompt, &user_prompt, &schema, 8192)
        .await?;
    let raw_candidates = output
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "Director model output missing candidates array".to_string())?;
    if raw_candidates.len() != request.candidate_count {
        return Err(format!(
            "Director model returned {} candidates, expected {}",
            raw_candidates.len(),
            request.candidate_count
        ));
    }

    let mut persisted = Vec::new();
    for raw in raw_candidates {
        let title_options = string_array_field(raw, "title_options")?;
        let input = DirectionCandidateInput {
            id: None,
            project_id: request.project_id.clone(),
            inspiration: inspiration.to_string(),
            title_options,
            positioning: string_field(raw, "positioning")?,
            target_reader: string_field(raw, "target_reader")?,
            core_hook: string_field(raw, "core_hook")?,
            series_promise: string_field(raw, "series_promise")?,
            first_30_chapter_promise: string_field(raw, "first_30_chapter_promise")?,
            world_seed: raw.get("world_seed").cloned().unwrap_or_else(|| json!({})),
            character_seed: raw
                .get("character_seed")
                .cloned()
                .unwrap_or_else(|| json!({})),
            volume_strategy: raw
                .get("volume_strategy")
                .cloned()
                .unwrap_or_else(|| json!([])),
            golden_three_chapters: raw
                .get("golden_three_chapters")
                .cloned()
                .unwrap_or_else(|| json!([])),
            checkpoint_status: "draft".to_string(),
            revision_note: raw
                .get("revision_note")
                .and_then(Value::as_str)
                .map(str::to_string),
            selected: false,
            metadata: json!({
                "prompt_hash": prompt_hash.clone(),
                "model_profile_snapshot": request.model_profile_snapshot.clone(),
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "input_inspiration": inspiration,
            }),
        };
        let id = director::upsert_direction_candidate(db, &input)?;
        persisted.push(director::get_direction_candidate(db, &id)?);
    }

    Ok(persisted)
}

pub fn select_direction_candidate(
    db: &Database,
    candidate_id: &str,
    revision_note: Option<&str>,
) -> Result<DirectionCandidate, String> {
    director::select_direction_candidate(db, candidate_id, revision_note)?;
    director::get_direction_candidate(db, candidate_id)
}

pub fn build_bootstrap_handoff(
    db: &Database,
    candidate_id: &str,
) -> Result<DirectorBootstrapHandoff, String> {
    let candidate = director::get_direction_candidate(db, candidate_id)?;
    Ok(DirectorBootstrapHandoff {
        candidate_id: candidate.id,
        project_id: candidate.project_id,
        inspiration: candidate.inspiration,
        suggested_title: candidate
            .title_options
            .first()
            .cloned()
            .unwrap_or_else(|| "Untitled Novel".to_string()),
        positioning: candidate.positioning,
        target_reader: candidate.target_reader,
        core_hook: candidate.core_hook,
        series_promise: candidate.series_promise,
        first_30_chapter_promise: candidate.first_30_chapter_promise,
        world_seed: candidate.world_seed,
        character_seed: candidate.character_seed,
        volume_strategy: candidate.volume_strategy,
        golden_three_chapters: candidate.golden_three_chapters,
        requires_human_review: true,
    })
}

fn string_field(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("Director candidate missing {}", field))
}

fn string_array_field(value: &Value, field: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Director candidate missing {}", field))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if items.is_empty() {
        return Err(format!("Director candidate {} must not be empty", field));
    }
    Ok(items)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
