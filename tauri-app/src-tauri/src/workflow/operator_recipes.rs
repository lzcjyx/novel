use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{bible, chapters, generation_jobs, projects, settings};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
pub struct OperatorRecipeRunEvent {
    pub step: String,
    pub status: String,
    pub detail: Option<String>,
    pub progress_pct: f64,
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

        let action_detail =
            execute_recipe_action_with_provider(db, &request, &job_id, &recipe, action, provider)
                .await?;
        record_recipe_event(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "done",
            Some(&action_detail),
            5.0 + (((action_index + 1) as f64) / action_count * 90.0),
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

        let action_detail = execute_recipe_action(db, &request, &job_id, &recipe, action)?;
        record_recipe_event(
            db,
            &job_id,
            &mut events,
            &action.kind,
            "done",
            Some(&action_detail),
            5.0 + (((action_index + 1) as f64) / action_count * 90.0),
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
) -> Result<String, String> {
    match action.kind.as_str() {
        "build_context_preview" => {
            let preview = build_context_preview(db, request)?;
            record_recipe_metadata(db, job_id, recipe, Some(preview))?;
            Ok("Context preview assembled without generating prose".to_string())
        }
        "generate_draft_candidate" => Ok(format!(
            "Draft candidate {} persisted",
            create_draft_candidate_from_context(db, request, job_id, recipe, action)?
        )),
        "rerun_review_agent" => Ok(format!(
            "Review action queued with parameters {}",
            action.parameters
        )),
        "repair_canon_consistency" => Ok("Canon consistency repair action queued".to_string()),
        "summarize_source_to_canon_candidate" => {
            Ok("Canon candidate summary action queued".to_string())
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
) -> Result<String, String> {
    match action.kind.as_str() {
        "generate_draft_candidate" => Ok(format!(
            "Draft candidate {} persisted",
            create_draft_candidate_with_provider(db, request, job_id, recipe, action, provider)
                .await?
        )),
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
) -> Result<i32, String> {
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
    crate::db::draft_alternatives::create_draft_candidate(
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
    Ok(candidate_number)
}

fn create_draft_candidate_from_context(
    db: &Database,
    request: &OperatorRecipeRunRequest,
    job_id: &str,
    recipe: &OperatorRecipe,
    action: &OperatorRecipeAction,
) -> Result<i32, String> {
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
    crate::db::draft_alternatives::create_draft_candidate(
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
    Ok(candidate_number)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
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
    });
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
    metadata["operator_recipe"] = json!({
        "recipe_id": recipe.id,
        "name": recipe.name,
        "actions": recipe.actions,
        "context_preview": context_preview.or(existing_preview),
    });

    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![metadata.to_string(), job_id],
    )
    .map_err(|e| format!("Update recipe job metadata: {}", e))?;
    Ok(())
}
