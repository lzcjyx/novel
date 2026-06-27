use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{bible, chapters, generation_jobs, projects, settings};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRecipe {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<OperatorRecipeAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRecipeAction {
    pub kind: String,
    pub label: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRecipeRunRequest {
    pub project_id: String,
    pub chapter_plan_id: String,
    pub recipe_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOperatorRecipeInput {
    pub id: Option<String>,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub parameter_schema: serde_json::Value,
    pub actions: Vec<OperatorRecipeAction>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOperatorRecipe {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub parameter_schema: serde_json::Value,
    pub actions: Vec<OperatorRecipeAction>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRecipeRunEvent {
    pub step: String,
    pub status: String,
    pub detail: Option<String>,
    pub progress_pct: f64,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub output: Value,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRecipeRunResult {
    pub ok: bool,
    pub job_id: String,
    pub recipe_id: String,
    pub status: String,
    pub events: Vec<OperatorRecipeRunEvent>,
    pub error_message: Option<String>,
}

const ALLOWED_ACTION_KINDS: &[&str] = &[
    "build_context_preview",
    "generate_draft_candidate",
    "rerun_review_agent",
    "repair_canon_consistency",
    "summarize_source_to_canon_candidate",
];

pub fn is_allowed_action_kind(kind: &str) -> bool {
    ALLOWED_ACTION_KINDS.contains(&kind)
}

#[derive(Debug, Clone)]
struct ActionOutcome {
    detail: String,
    output: Value,
    artifact_refs: Vec<String>,
}

pub fn upsert_user_recipe(
    db: &Database,
    input: &UserOperatorRecipeInput,
) -> Result<String, String> {
    if input.project_id.trim().is_empty() {
        return Err("user recipe project_id is required".to_string());
    }
    if input.name.trim().is_empty() {
        return Err("user recipe name is required".to_string());
    }
    if input.actions.is_empty() {
        return Err("user recipe requires at least one action".to_string());
    }
    for action in &input.actions {
        if !is_allowed_action_kind(&action.kind) {
            return Err(format!("Unsupported recipe action '{}'", action.kind));
        }
    }

    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let parameter_schema = serde_json::to_string(&input.parameter_schema)
        .map_err(|e| format!("Serialize user recipe parameter schema: {}", e))?;
    let actions = serde_json::to_string(&input.actions)
        .map_err(|e| format!("Serialize user recipe actions: {}", e))?;
    let metadata = serde_json::to_string(&input.metadata)
        .map_err(|e| format!("Serialize user recipe metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO user_operator_recipes
            (id, project_id, name, description, parameter_schema, actions, enabled, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            name = excluded.name,
            description = excluded.description,
            parameter_schema = excluded.parameter_schema,
            actions = excluded.actions,
            enabled = excluded.enabled,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        rusqlite::params![
            id,
            input.project_id.trim(),
            input.name.trim(),
            input.description.trim(),
            parameter_schema,
            actions,
            input.enabled as i32,
            metadata,
        ],
    )
    .map_err(|e| format!("Upsert user recipe: {}", e))?;
    Ok(id)
}

pub fn list_user_recipes(
    db: &Database,
    project_id: &str,
    enabled_only: bool,
) -> Result<Vec<UserOperatorRecipe>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = if enabled_only {
        "SELECT id, project_id, name, description, parameter_schema, actions, enabled, metadata
         FROM user_operator_recipes
         WHERE project_id = ?1 AND enabled = 1
         ORDER BY name ASC, id ASC"
    } else {
        "SELECT id, project_id, name, description, parameter_schema, actions, enabled, metadata
         FROM user_operator_recipes
         WHERE project_id = ?1
         ORDER BY name ASC, id ASC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare user recipes: {}", e))?;
    let recipes = stmt
        .query_map(rusqlite::params![project_id], |row| {
            let parameter_schema_raw: String = row.get(4)?;
            let actions_raw: String = row.get(5)?;
            let metadata_raw: String = row.get(7)?;
            Ok(UserOperatorRecipe {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                parameter_schema: serde_json::from_str(&parameter_schema_raw)
                    .unwrap_or_else(|_| json!({})),
                actions: serde_json::from_str(&actions_raw).unwrap_or_default(),
                enabled: row.get::<_, i32>(6)? != 0,
                metadata: serde_json::from_str(&metadata_raw).unwrap_or_else(|_| json!({})),
            })
        })
        .map_err(|e| format!("Query user recipes: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect user recipes: {}", e))?;
    Ok(recipes)
}

fn action(kind: &str, label: &str, parameters: serde_json::Value) -> OperatorRecipeAction {
    OperatorRecipeAction {
        kind: kind.to_string(),
        label: label.to_string(),
        parameters,
    }
}

pub fn built_in_recipes() -> Vec<OperatorRecipe> {
    vec![
        OperatorRecipe {
            id: "generate_three_draft_candidates".to_string(),
            name: "Generate three draft candidates".to_string(),
            description: "Build one context package and generate three candidate chapter drafts."
                .to_string(),
            actions: vec![
                action(
                    "build_context_preview",
                    "Build context preview",
                    serde_json::json!({}),
                ),
                action(
                    "generate_draft_candidate",
                    "Generate candidate 1",
                    serde_json::json!({"candidate_number": 1}),
                ),
                action(
                    "generate_draft_candidate",
                    "Generate candidate 2",
                    serde_json::json!({"candidate_number": 2}),
                ),
                action(
                    "generate_draft_candidate",
                    "Generate candidate 3",
                    serde_json::json!({"candidate_number": 3}),
                ),
            ],
        },
        OperatorRecipe {
            id: "rerun_style_reviewer".to_string(),
            name: "Rerun style reviewer".to_string(),
            description: "Run only the style reviewer against the selected chapter version."
                .to_string(),
            actions: vec![action(
                "rerun_review_agent",
                "Run style reviewer",
                serde_json::json!({"agent_name": "style_reviewer"}),
            )],
        },
        OperatorRecipe {
            id: "build_context_preview".to_string(),
            name: "Build context preview".to_string(),
            description: "Build prompt and context preview without generating prose.".to_string(),
            actions: vec![action(
                "build_context_preview",
                "Build context preview",
                serde_json::json!({}),
            )],
        },
        OperatorRecipe {
            id: "repair_canon_consistency".to_string(),
            name: "Repair canon consistency".to_string(),
            description: "Run a canon-only repair pass for deterministic consistency issues."
                .to_string(),
            actions: vec![action(
                "repair_canon_consistency",
                "Repair canon consistency",
                serde_json::json!({}),
            )],
        },
        OperatorRecipe {
            id: "summarize_source_to_canon_candidate".to_string(),
            name: "Summarize source to canon candidate".to_string(),
            description: "Convert selected source material into a candidate canon note."
                .to_string(),
            actions: vec![action(
                "summarize_source_to_canon_candidate",
                "Summarize selected source",
                serde_json::json!({}),
            )],
        },
    ]
}

pub fn extension_recipes_from_metadata(metadata: &Value) -> Result<Vec<OperatorRecipe>, String> {
    crate::extensions::host::extension_contribution_payloads(metadata, "recipe_pack")
        .into_iter()
        .enumerate()
        .map(|(index, payload)| {
            let recipe_id = payload
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("recipe_pack contribution {} missing id", index))?
                .to_string();
            let name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&recipe_id)
                .to_string();
            let description = payload
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let actions = payload
                .get("actions")
                .cloned()
                .ok_or_else(|| format!("recipe_pack contribution {} missing actions", index))
                .and_then(|value| {
                    serde_json::from_value::<Vec<OperatorRecipeAction>>(value)
                        .map_err(|e| format!("Parse recipe_pack actions: {}", e))
                })?;
            if actions.is_empty() {
                return Err(format!(
                    "recipe_pack contribution {} requires at least one action",
                    index
                ));
            }
            for action in &actions {
                if !is_allowed_action_kind(&action.kind) {
                    return Err(format!(
                        "recipe_pack contribution {} uses unsupported action '{}'",
                        index, action.kind
                    ));
                }
            }
            Ok(OperatorRecipe {
                id: recipe_id,
                name,
                description,
                actions,
            })
        })
        .collect()
}

pub fn execute_builtin_recipe(
    db: &Database,
    request: OperatorRecipeRunRequest,
) -> Result<OperatorRecipeRunResult, String> {
    execute_builtin_recipe_with_cancel_check(db, request, |_action_index, _action| false)
}

pub fn execute_builtin_recipe_with_cancel_check<F>(
    db: &Database,
    request: OperatorRecipeRunRequest,
    is_cancelled: F,
) -> Result<OperatorRecipeRunResult, String>
where
    F: FnMut(usize, &OperatorRecipeAction) -> bool,
{
    let recipe = built_in_recipes()
        .into_iter()
        .find(|recipe| recipe.id == request.recipe_id)
        .ok_or_else(|| format!("Unknown operator recipe '{}'", request.recipe_id))?;
    execute_recipe(db, request, recipe, is_cancelled)
}

pub async fn execute_builtin_recipe_with_provider(
    db: &Database,
    request: OperatorRecipeRunRequest,
    provider: &dyn ModelClient,
) -> Result<OperatorRecipeRunResult, String> {
    let recipe = built_in_recipes()
        .into_iter()
        .find(|recipe| recipe.id == request.recipe_id)
        .ok_or_else(|| format!("Unknown operator recipe '{}'", request.recipe_id))?;
    execute_recipe_with_provider(db, request, recipe, provider).await
}

pub async fn execute_recipe_with_provider(
    db: &Database,
    request: OperatorRecipeRunRequest,
    recipe: OperatorRecipe,
    provider: &dyn ModelClient,
) -> Result<OperatorRecipeRunResult, String> {
    let job_id =
        generation_jobs::create_generation_job(db, &request.project_id, &request.chapter_plan_id)?;
    record_recipe_metadata(db, &job_id, &recipe, None)?;

    let mut events = Vec::new();
    record_recipe_event(
        db,
        &job_id,
        &mut events,
        "operator_recipe",
        "running",
        Some(&format!("Running recipe {}", recipe.name)),
        1.0,
    )?;

    let action_count = recipe.actions.len().max(1) as f64;
    for (action_index, action) in recipe.actions.iter().enumerate() {
        let progress_pct = 5.0 + ((action_index as f64) / action_count * 90.0);
        record_recipe_event(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "running",
            Some(&action.label),
            progress_pct,
        )?;

        if !is_allowed_action_kind(&action.kind) {
            let reason = format!("Unsupported recipe action '{}'", action.kind);
            record_recipe_event(
                db,
                &job_id,
                &mut events,
                &action.kind,
                "failed",
                Some(&reason),
                progress_pct,
            )?;
            generation_jobs::update_job_status(db, &job_id, "failed", Some(&reason))?;
            return Ok(OperatorRecipeRunResult {
                ok: false,
                job_id,
                recipe_id: recipe.id,
                status: "failed".to_string(),
                events,
                error_message: Some(reason),
            });
        }

        let action_started = Instant::now();
        let action_outcome =
            execute_recipe_action_with_provider(db, &request, &job_id, &recipe, action, provider)
                .await?;
        record_recipe_event_with_io(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "done",
            Some(&action_outcome.detail),
            5.0 + (((action_index + 1) as f64) / action_count * 90.0),
            action_input(action),
            action_outcome.output,
            None,
            elapsed_ms(action_started),
            action_outcome.artifact_refs,
        )?;
    }

    record_recipe_event(
        db,
        &job_id,
        &mut events,
        "operator_recipe",
        "done",
        Some("Recipe completed"),
        100.0,
    )?;
    generation_jobs::update_job_status(db, &job_id, "completed", None)?;

    Ok(OperatorRecipeRunResult {
        ok: true,
        job_id,
        recipe_id: recipe.id,
        status: "completed".to_string(),
        events,
        error_message: None,
    })
}

pub fn execute_recipe<F>(
    db: &Database,
    request: OperatorRecipeRunRequest,
    recipe: OperatorRecipe,
    mut is_cancelled: F,
) -> Result<OperatorRecipeRunResult, String>
where
    F: FnMut(usize, &OperatorRecipeAction) -> bool,
{
    let job_id =
        generation_jobs::create_generation_job(db, &request.project_id, &request.chapter_plan_id)?;
    record_recipe_metadata(db, &job_id, &recipe, None)?;

    let mut events = Vec::new();
    record_recipe_event(
        db,
        &job_id,
        &mut events,
        "operator_recipe",
        "running",
        Some(&format!("Running recipe {}", recipe.name)),
        1.0,
    )?;

    let action_count = recipe.actions.len().max(1) as f64;
    for (action_index, action) in recipe.actions.iter().enumerate() {
        let progress_pct = 5.0 + ((action_index as f64) / action_count * 90.0);
        if is_cancelled(action_index, action) {
            let reason = "operator cancellation requested";
            record_recipe_event(
                db,
                &job_id,
                &mut events,
                &action.kind,
                "cancelled",
                Some(reason),
                progress_pct,
            )?;
            generation_jobs::update_job_status(db, &job_id, "cancelled", Some(reason))?;
            return Ok(OperatorRecipeRunResult {
                ok: false,
                job_id,
                recipe_id: recipe.id,
                status: "cancelled".to_string(),
                events,
                error_message: Some(reason.to_string()),
            });
        }

        record_recipe_event(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "running",
            Some(&action.label),
            progress_pct,
        )?;

        if !is_allowed_action_kind(&action.kind) {
            let reason = format!("Unsupported recipe action '{}'", action.kind);
            record_recipe_event(
                db,
                &job_id,
                &mut events,
                &action.kind,
                "failed",
                Some(&reason),
                progress_pct,
            )?;
            generation_jobs::update_job_status(db, &job_id, "failed", Some(&reason))?;
            return Ok(OperatorRecipeRunResult {
                ok: false,
                job_id,
                recipe_id: recipe.id,
                status: "failed".to_string(),
                events,
                error_message: Some(reason),
            });
        }

        let action_started = Instant::now();
        let action_outcome = execute_recipe_action(db, &request, &job_id, &recipe, action)?;
        record_recipe_event_with_io(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "done",
            Some(&action_outcome.detail),
            5.0 + (((action_index + 1) as f64) / action_count * 90.0),
            action_input(action),
            action_outcome.output,
            None,
            elapsed_ms(action_started),
            action_outcome.artifact_refs,
        )?;
    }

    record_recipe_event(
        db,
        &job_id,
        &mut events,
        "operator_recipe",
        "done",
        Some("Recipe completed"),
        100.0,
    )?;
    generation_jobs::update_job_status(db, &job_id, "completed", None)?;

    Ok(OperatorRecipeRunResult {
        ok: true,
        job_id,
        recipe_id: recipe.id,
        status: "completed".to_string(),
        events,
        error_message: None,
    })
}

fn execute_recipe_action(
    db: &Database,
    request: &OperatorRecipeRunRequest,
    job_id: &str,
    recipe: &OperatorRecipe,
    action: &OperatorRecipeAction,
) -> Result<ActionOutcome, String> {
    match action.kind.as_str() {
        "build_context_preview" => {
            let preview = build_context_preview(db, request)?;
            let prompt_name = preview["prompt_runtime"]["prompt_name"]
                .as_str()
                .unwrap_or("draft_writer")
                .to_string();
            record_recipe_metadata(db, job_id, recipe, Some(preview))?;
            Ok(ActionOutcome {
                detail: "Context preview ready without generating prose".to_string(),
                output: json!({
                    "kind": "context_preview",
                    "prompt_name": prompt_name,
                    "chapter_plan_id": request.chapter_plan_id,
                }),
                artifact_refs: vec![format!("recipe_action:{}:context_preview", job_id)],
            })
        }
        "generate_draft_candidate" => {
            let (candidate_number, candidate_id) =
                create_draft_candidate_from_context(db, request, job_id, recipe, action)?;
            Ok(ActionOutcome {
                detail: format!("Draft candidate {} persisted", candidate_number),
                output: json!({
                    "kind": "draft_candidate",
                    "candidate_number": candidate_number,
                    "candidate_id": candidate_id,
                    "chapter_plan_id": request.chapter_plan_id,
                }),
                artifact_refs: vec![format!("draft_candidate:{}", candidate_id)],
            })
        }
        "rerun_review_agent" => {
            let agent_name = action
                .parameters
                .get("agent_name")
                .and_then(Value::as_str)
                .unwrap_or("style_reviewer");
            Ok(ActionOutcome {
                detail: format!("Review agent preview ready for {}", agent_name),
                output: json!({
                    "kind": "review_agent_preview",
                    "agent_name": agent_name,
                    "chapter_plan_id": request.chapter_plan_id,
                    "status": "ready",
                }),
                artifact_refs: vec![format!("recipe_action:{}:rerun_review_agent", job_id)],
            })
        }
        "repair_canon_consistency" => Ok(ActionOutcome {
            detail: "Canon consistency repair preview ready".to_string(),
            output: json!({
                "kind": "canon_repair_preview",
                "chapter_plan_id": request.chapter_plan_id,
                "status": "ready",
            }),
            artifact_refs: vec![format!("recipe_action:{}:repair_canon_consistency", job_id)],
        }),
        "summarize_source_to_canon_candidate" => {
            let source_key = action
                .parameters
                .get("source_key")
                .and_then(Value::as_str)
                .unwrap_or("manual_source");
            Ok(ActionOutcome {
                detail: "Canon candidate summary preview ready".to_string(),
                output: json!({
                    "kind": "canon_candidate_summary_preview",
                    "source_key": source_key,
                    "chapter_plan_id": request.chapter_plan_id,
                    "status": "ready",
                }),
                artifact_refs: vec![format!(
                    "recipe_action:{}:summarize_source_to_canon_candidate",
                    job_id
                )],
            })
        }
        other => Err(format!("Unsupported recipe action '{}'", other)),
    }
}

async fn execute_recipe_action_with_provider(
    db: &Database,
    request: &OperatorRecipeRunRequest,
    job_id: &str,
    recipe: &OperatorRecipe,
    action: &OperatorRecipeAction,
    provider: &dyn ModelClient,
) -> Result<ActionOutcome, String> {
    match action.kind.as_str() {
        "generate_draft_candidate" => {
            let (candidate_number, candidate_id) =
                create_draft_candidate_with_provider(db, request, job_id, recipe, action, provider)
                    .await?;
            Ok(ActionOutcome {
                detail: format!("Draft candidate {} persisted", candidate_number),
                output: json!({
                    "kind": "draft_candidate",
                    "candidate_number": candidate_number,
                    "candidate_id": candidate_id,
                    "chapter_plan_id": request.chapter_plan_id,
                    "provider_generated": true,
                }),
                artifact_refs: vec![format!("draft_candidate:{}", candidate_id)],
            })
        }
        _ => execute_recipe_action(db, request, job_id, recipe, action),
    }
}

async fn create_draft_candidate_with_provider(
    db: &Database,
    request: &OperatorRecipeRunRequest,
    job_id: &str,
    recipe: &OperatorRecipe,
    action: &OperatorRecipeAction,
    provider: &dyn ModelClient,
) -> Result<(i32, String), String> {
    let candidate_number = action
        .parameters
        .get("candidate_number")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, i32::MAX as i64) as i32;
    let preview = build_context_preview(db, request)?;
    let prompt_runtime = preview
        .get("prompt_runtime")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let system_prompt = prompt_runtime
        .get("system_prompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let user_prompt = format!(
        "{}\n\nCandidate number: {}. Generate a distinct alternative for operator comparison.",
        prompt_runtime
            .get("user_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Generate a chapter draft as JSON."),
        candidate_number
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "body_markdown": {"type": "string"},
            "summary": {"type": "string"},
            "word_count": {"type": "integer"}
        }
    });
    let generated = provider
        .generate_json(&system_prompt, &user_prompt, &schema, 32768)
        .await?;
    let body_markdown = generated
        .get("body_markdown")
        .and_then(Value::as_str)
        .ok_or_else(|| "Draft candidate model output missing body_markdown".to_string())?
        .to_string();
    let title = generated
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Candidate {}", candidate_number));
    let summary = generated
        .get("summary")
        .and_then(Value::as_str)
        .map(str::to_string);
    let word_count = generated
        .get("word_count")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| body_markdown.chars().count() as i64) as i32;
    let mut context_only = preview.clone();
    let prompt_runtime_for_hash = context_only
        .as_object_mut()
        .and_then(|object| object.remove("prompt_runtime"))
        .unwrap_or_else(|| json!({}));
    let context_json = serde_json::to_string(&context_only)
        .map_err(|e| format!("Serialize provider candidate context: {}", e))?;
    let prompt_json = serde_json::to_string(&json!({
        "system_prompt": system_prompt,
        "user_prompt": user_prompt,
        "prompt_runtime": prompt_runtime_for_hash,
    }))
    .map_err(|e| format!("Serialize provider candidate prompt: {}", e))?;
    let candidate_id = crate::db::draft_alternatives::create_draft_candidate(
        db,
        &crate::db::draft_alternatives::DraftCandidateInput {
            project_id: request.project_id.clone(),
            chapter_plan_id: request.chapter_plan_id.clone(),
            candidate_number,
            title,
            body_markdown,
            summary,
            word_count,
            prompt_hash: stable_hash(&prompt_json),
            context_hash: stable_hash(&context_json),
            model_profile_id: None,
            review_notes: json!({}),
            estimated_cost_usd: None,
            metadata: json!({
                "operator_recipe": {
                    "recipe_id": recipe.id,
                    "job_id": job_id,
                    "action": action.kind,
                    "provider_generated": true,
                },
                "prompt_runtime": prompt_runtime,
                "context_activation": preview.get("context_activation").cloned().unwrap_or_else(|| json!({})),
            }),
        },
    )?;
    record_recipe_metadata(db, job_id, recipe, Some(preview))?;
    Ok((candidate_number, candidate_id))
}

fn create_draft_candidate_from_context(
    db: &Database,
    request: &OperatorRecipeRunRequest,
    job_id: &str,
    recipe: &OperatorRecipe,
    action: &OperatorRecipeAction,
) -> Result<(i32, String), String> {
    let candidate_number = action
        .parameters
        .get("candidate_number")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, i32::MAX as i64) as i32;
    let preview = build_context_preview(db, request)?;
    let mut context_only = preview.clone();
    let prompt_runtime = context_only
        .as_object_mut()
        .and_then(|object| object.remove("prompt_runtime"))
        .unwrap_or_else(|| json!({}));
    let context_json = serde_json::to_string(&context_only)
        .map_err(|e| format!("Serialize candidate context: {}", e))?;
    let prompt_json = serde_json::to_string(&prompt_runtime)
        .map_err(|e| format!("Serialize candidate prompt: {}", e))?;
    let context_hash = stable_hash(&context_json);
    let prompt_hash = stable_hash(&prompt_json);
    let chapter_title = preview
        .get("chapter_plan")
        .and_then(|plan| plan.get("title"))
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("Untitled chapter");
    let title = format!("Candidate {} - {}", candidate_number, chapter_title);
    let body_markdown = format!(
        "Candidate {} draft for {}.\n\nThis deterministic draft candidate was produced from the same prompt and context package for operator comparison.",
        candidate_number, chapter_title
    );
    let word_count = body_markdown.chars().count() as i32;
    let candidate_id = crate::db::draft_alternatives::create_draft_candidate(
        db,
        &crate::db::draft_alternatives::DraftCandidateInput {
            project_id: request.project_id.clone(),
            chapter_plan_id: request.chapter_plan_id.clone(),
            candidate_number,
            title,
            body_markdown,
            summary: Some(format!(
                "Deterministic candidate {} generated by operator recipe.",
                candidate_number
            )),
            word_count,
            prompt_hash,
            context_hash,
            model_profile_id: None,
            review_notes: json!({}),
            estimated_cost_usd: None,
            metadata: json!({
                "operator_recipe": {
                    "recipe_id": recipe.id,
                    "job_id": job_id,
                    "action": action.kind,
                },
                "prompt_runtime": prompt_runtime,
                "context_activation": preview.get("context_activation").cloned().unwrap_or_else(|| json!({})),
            }),
        },
    )?;
    record_recipe_metadata(db, job_id, recipe, Some(preview))?;
    Ok((candidate_number, candidate_id))
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn action_input(action: &OperatorRecipeAction) -> Value {
    json!({
        "kind": action.kind,
        "label": action.label,
        "parameters": action.parameters,
    })
}

fn build_context_preview(
    db: &Database,
    request: &OperatorRecipeRunRequest,
) -> Result<Value, String> {
    let project = projects::get_project(db, &request.project_id)?;
    let plan = chapters::get_chapter_plans(db, &request.project_id)?
        .into_iter()
        .find(|plan| plan.id == request.chapter_plan_id)
        .ok_or_else(|| {
            format!(
                "Chapter plan '{}' was not found for project '{}'",
                request.chapter_plan_id, request.project_id
            )
        })?;
    let canon = bible::get_bible(db, &request.project_id)?;
    let settings = settings::get_settings(db)?;
    let package = crate::workflow::writing_context::build_writing_context(
        db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    )?;
    let writing_context_json =
        serde_json::to_string_pretty(&package).map_err(|e| format!("Serialize context: {}", e))?;
    let assembled_prompt =
        crate::workflow::prompt_runtime::assemble_builtin_draft_prompt(&writing_context_json)?;
    let mut preview =
        serde_json::to_value(package).map_err(|e| format!("Serialize context preview: {}", e))?;
    preview["prompt_runtime"] =
        crate::workflow::prompt_runtime::assembled_prompt_preview_payload(&assembled_prompt);
    Ok(preview)
}

fn record_recipe_event(
    db: &Database,
    job_id: &str,
    events: &mut Vec<OperatorRecipeRunEvent>,
    step: &str,
    status: &str,
    detail: Option<&str>,
    progress_pct: f64,
) -> Result<(), String> {
    generation_jobs::record_job_phase_event(db, job_id, step, status, detail, progress_pct)?;
    events.push(OperatorRecipeRunEvent {
        step: step.to_string(),
        status: status.to_string(),
        detail: detail.map(str::to_string),
        progress_pct,
        input: json!({}),
        output: json!({}),
        error: None,
        duration_ms: 0,
        artifact_refs: Vec::new(),
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_recipe_event_with_io(
    db: &Database,
    job_id: &str,
    events: &mut Vec<OperatorRecipeRunEvent>,
    step: &str,
    status: &str,
    detail: Option<&str>,
    progress_pct: f64,
    input: Value,
    output: Value,
    error: Option<String>,
    duration_ms: u64,
    artifact_refs: Vec<String>,
) -> Result<(), String> {
    generation_jobs::record_job_phase_event(db, job_id, step, status, detail, progress_pct)?;
    let event = OperatorRecipeRunEvent {
        step: step.to_string(),
        status: status.to_string(),
        detail: detail.map(str::to_string),
        progress_pct,
        input,
        output,
        error,
        duration_ms,
        artifact_refs,
    };
    if event.status == "done" && event.step != "operator_recipe" {
        append_recipe_step_output(db, job_id, &event)?;
    }
    events.push(event);
    Ok(())
}

fn append_recipe_step_output(
    db: &Database,
    job_id: &str,
    event: &OperatorRecipeRunEvent,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load recipe step metadata: {}", e))?;
    let mut metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    if !metadata
        .get("operator_recipe")
        .is_some_and(|value| value.is_object())
    {
        metadata["operator_recipe"] = json!({});
    }
    if !metadata["operator_recipe"]
        .get("step_outputs")
        .is_some_and(|value| value.is_array())
    {
        metadata["operator_recipe"]["step_outputs"] = json!([]);
    }
    metadata["operator_recipe"]["step_outputs"]
        .as_array_mut()
        .ok_or_else(|| "operator_recipe.step_outputs must be an array".to_string())?
        .push(json!({
            "step": event.step,
            "status": event.status,
            "detail": event.detail,
            "input": event.input,
            "output": event.output,
            "error": event.error,
            "duration_ms": event.duration_ms,
            "artifact_refs": event.artifact_refs,
        }));
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![metadata.to_string(), job_id],
    )
    .map_err(|e| format!("Persist recipe step output: {}", e))?;
    Ok(())
}

fn record_recipe_metadata(
    db: &Database,
    job_id: &str,
    recipe: &OperatorRecipe,
    context_preview: Option<Value>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load recipe job metadata: {}", e))?;
    let mut metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }

    let existing_preview = metadata
        .get("operator_recipe")
        .and_then(|value| value.get("context_preview"))
        .cloned();
    let existing_step_outputs = metadata
        .get("operator_recipe")
        .and_then(|value| value.get("step_outputs"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    metadata["operator_recipe"] = json!({
        "recipe_id": recipe.id,
        "name": recipe.name,
        "actions": recipe.actions,
        "context_preview": context_preview.or(existing_preview),
        "step_outputs": existing_step_outputs,
    });

    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![metadata.to_string(), job_id],
    )
    .map_err(|e| format!("Update recipe job metadata: {}", e))?;
    Ok(())
}
