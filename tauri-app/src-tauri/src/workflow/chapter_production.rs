use crate::ai::client::{EmbeddingInputKind, ModelClient, ModelUsageReport};
use crate::db::connection::Database;
use crate::db::model_profiles::ModelProfile;
use crate::db::{
    bible, blog_posts, chapters, generation_jobs, projects, publication_queue, reviews,
};
use crate::export::markdown;
use crate::models::*;
use crate::prompts;
use crate::workflow::{
    canon_updater, context_activation, hard_fact_ledger, learning, lock, prompt_rendering,
    prompt_runtime, review_agents, review_arbiter, task_transaction, writing_context,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub fn enqueue_publication_if_enabled(
    db: &Database,
    settings: &AppSettings,
    project: &Project,
    chapter_id: &str,
    chapter_version_id: Option<&str>,
    title: &str,
) -> Result<Option<String>, String> {
    if !settings.publish_schedule_enabled {
        return Ok(None);
    }
    let provider = if settings.publication_target_provider.trim().is_empty() {
        "firefly_git"
    } else {
        settings.publication_target_provider.as_str()
    };
    let slug = crate::workflow::static_site_publish::sanitize_post_slug(title);
    let scheduled_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let input = PublicationQueueInput {
        project_id: project.id.clone(),
        chapter_id: chapter_id.to_string(),
        chapter_version_id: chapter_version_id.map(str::to_string),
        provider: provider.to_string(),
        scheduled_at: Some(scheduled_at),
        metadata: serde_json::json!({
            "title": title,
            "slug": slug,
            "target": {
                "provider": provider,
                "posts_dir": settings.publication_posts_dir,
                "remote": settings.publication_remote_name,
                "branch": settings.publication_branch,
                "push_enabled": settings.publication_push_enabled,
                "validate_build": settings.publication_validate_build,
                "dry_run": settings.publication_dry_run
            }
        }),
    };
    publication_queue::upsert_pending_publication(db, &input).map(Some)
}

fn log(log_tx: &mpsc::Sender<String>, msg: &str) {
    let _ = log_tx.try_send(format!(
        "[{}] {}",
        chrono::Local::now().format("%H:%M:%S"),
        msg
    ));
}

fn emit(
    event_tx: &mpsc::Sender<PipelineEvent>,
    step: &str,
    status: &str,
    detail: Option<&str>,
    progress_pct: f64,
) {
    let _ = event_tx.try_send(PipelineEvent {
        step: step.into(),
        status: status.into(),
        elapsed_ms: None,
        detail: detail.map(|s| s.into()),
        progress_pct,
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        preview_title: None,
        preview_text: None,
        preview_kind: None,
    });
}

fn emit_job_event(
    db: &Database,
    job_id: &str,
    event_tx: &mpsc::Sender<PipelineEvent>,
    step: &str,
    status: &str,
    detail: Option<&str>,
    progress_pct: f64,
) {
    emit(event_tx, step, status, detail, progress_pct);
    let _ = generation_jobs::record_job_phase_event(db, job_id, step, status, detail, progress_pct);
}

fn preview_excerpt(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn emit_preview(
    event_tx: &mpsc::Sender<PipelineEvent>,
    step: &str,
    title: &str,
    body: &str,
    preview_kind: &str,
    progress_pct: f64,
) {
    let _ = event_tx.try_send(PipelineEvent {
        step: step.into(),
        status: "preview".into(),
        elapsed_ms: None,
        detail: Some(format!("{} preview ready", preview_kind)),
        progress_pct,
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        preview_title: Some(title.to_string()),
        preview_text: Some(preview_excerpt(body, 8000)),
        preview_kind: Some(preview_kind.to_string()),
    });
}

fn record_model_usage(
    db: &Database,
    job_id: &str,
    phase: &str,
    settings: &AppSettings,
    model_profile: Option<&ModelProfile>,
    system_prompt: &str,
    user_prompt: &str,
    output_text: &str,
    provider_usage: Option<&ModelUsageReport>,
) {
    let estimated_prompt_tokens = generation_jobs::estimate_tokens(system_prompt)
        + generation_jobs::estimate_tokens(user_prompt);
    let estimated_completion_tokens = generation_jobs::estimate_tokens(output_text);
    let has_provider_usage = provider_usage.is_some_and(|usage| {
        usage.prompt_tokens.is_some()
            || usage.completion_tokens.is_some()
            || usage.total_tokens.is_some()
    });
    let prompt_tokens = provider_usage
        .and_then(|usage| usage.prompt_tokens)
        .unwrap_or(estimated_prompt_tokens);
    let completion_tokens = provider_usage
        .and_then(|usage| usage.completion_tokens)
        .unwrap_or_else(|| {
            provider_usage
                .and_then(|usage| usage.total_tokens)
                .map(|total| total.saturating_sub(prompt_tokens))
                .unwrap_or(estimated_completion_tokens)
        });
    let usage_source = if has_provider_usage {
        "provider"
    } else {
        "estimated"
    };
    let (provider, model) = model_identity(settings, model_profile);
    let input_cost_per_million = model_profile
        .and_then(|profile| profile.input_cost_per_million)
        .or(settings.input_cost_per_million);
    let output_cost_per_million = model_profile
        .and_then(|profile| profile.output_cost_per_million)
        .or(settings.output_cost_per_million);
    let _ = generation_jobs::record_job_model_usage_with_source_and_profile(
        db,
        job_id,
        phase,
        &provider,
        &model,
        prompt_tokens,
        completion_tokens,
        input_cost_per_million,
        output_cost_per_million,
        usage_source,
        model_profile.map(model_profile_snapshot),
    );
}

fn model_identity(settings: &AppSettings, profile: Option<&ModelProfile>) -> (String, String) {
    profile
        .map(|profile| (profile.provider.clone(), profile.model.clone()))
        .unwrap_or_else(|| (settings.provider.clone(), settings.model.clone()))
}

fn model_profile_snapshot(profile: &ModelProfile) -> serde_json::Value {
    serde_json::json!({
        "id": profile.id,
        "name": profile.name,
        "provider": profile.provider,
        "model": profile.model,
        "context_window": profile.context_window,
        "supports_json": profile.supports_json,
        "supports_streaming": profile.supports_streaming,
        "supports_embeddings": profile.supports_embeddings,
        "input_cost_per_million": profile.input_cost_per_million,
        "output_cost_per_million": profile.output_cost_per_million,
        "intended_use": profile.intended_use,
    })
}

fn review_usage_context(canon: &review_agents::CanonContext) -> String {
    serde_json::json!({
        "writing_brief_json": &canon.writing_brief_json,
        "characters_json": &canon.characters_json,
        "character_states_json": &canon.character_states_json,
        "previous_chapters_json": &canon.previous_chapters_json,
        "active_plot_threads_json": &canon.active_plot_threads_json,
        "unresolved_foreshadowing_json": &canon.unresolved_foreshadowing_json,
        "world_lore_json": &canon.world_lore_json,
        "locations_json": &canon.locations_json,
        "organizations_json": &canon.organizations_json,
        "items_json": &canon.items_json,
        "magic_systems_json": &canon.magic_systems_json,
        "canon_rules_json": &canon.canon_rules_json,
        "timeline_json": &canon.timeline_json,
        "style_guide_json": &canon.style_guide_json,
        "extension_review_rubrics_json": &canon.extension_review_rubrics_json,
        "blog_config_json": &canon.blog_config_json,
        "project_policy_json": &canon.project_policy_json,
    })
    .to_string()
}

fn resolve_model_profile(
    db: &Database,
    settings: &AppSettings,
    workflow: &str,
) -> Result<Option<ModelProfile>, String> {
    let profile_id = match workflow {
        "draft" => settings.draft_model_profile_id.as_deref(),
        "review" => settings.review_model_profile_id.as_deref(),
        "repair" | "revise" => settings.repair_model_profile_id.as_deref(),
        "embedding" => settings.embedding_model_profile_id.as_deref(),
        "summarization" => settings.summarization_model_profile_id.as_deref(),
        _ => None,
    };
    profile_id
        .map(|id| crate::db::model_profiles::get_model_profile(db, id).map(Some))
        .unwrap_or(Ok(None))
}

fn build_context_metadata(
    context: &writing_context::WritingContextPackage,
    prompt_runtime: Option<&prompt_runtime::AssembledPrompt>,
) -> serde_json::Value {
    let selected_retrieval_source_keys = context
        .retrieval_trace
        .sources
        .iter()
        .filter_map(|source| {
            source
                .source_id
                .as_ref()
                .map(|source_id| format!("{}:{}", source.source_type, source_id))
        })
        .collect::<Vec<_>>();
    let selected_retrieval_document_ids = context
        .retrieval_trace
        .sources
        .iter()
        .map(|source| source.document_id.clone())
        .collect::<Vec<_>>();
    let selected_learning_entries = context
        .learned_patterns
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "category": entry.category,
                "pattern_name": entry.pattern_name,
                "source_type": entry.source_type,
                "confidence": entry.confidence,
            })
        })
        .collect::<Vec<_>>();
    let selected_learning_entry_ids = context
        .learned_patterns
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    let learning_context_hash = {
        let payload = serde_json::to_string(&selected_learning_entries).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hex::encode(hasher.finalize())
    };
    let style_assets = context
        .style
        .get("style_assets")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let style_asset_ids = style_assets
        .get("asset_ids")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    let mut metadata = serde_json::json!({
        "selected_retrieval_source_keys": selected_retrieval_source_keys,
        "selected_retrieval_document_ids": selected_retrieval_document_ids,
        "retrieval_trace": context.retrieval_trace,
        "graph_context": context.graph_context,
        "context_activation": context.context_activation,
        "selected_learning_entry_ids": selected_learning_entry_ids,
        "selected_learning_entries": selected_learning_entries,
        "learning_context_hash": learning_context_hash,
        "style_asset_ids": style_asset_ids,
        "style_assets": style_assets,
    });

    if let Some(prompt_runtime) = prompt_runtime {
        metadata["prompt_runtime"] = serde_json::json!({
            "prompt_name": prompt_runtime.prompt_name,
            "generation_phase": prompt_runtime.generation_phase,
            "token_estimate": prompt_runtime.token_estimate,
            "unit_traces": prompt_runtime.unit_traces,
        });
    }

    metadata
}

fn run_extension_hook_for_job(
    db: &Database,
    job_id: &str,
    hook: &str,
    workflow_metadata: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let extensions = crate::extensions::host::list_extension_packages(db)?;
    if extensions.is_empty() {
        return Ok(workflow_metadata);
    }
    let output = crate::extensions::host::execute_extension_hook_for_job(
        db,
        job_id,
        crate::extensions::host::ExtensionHookRequest {
            hook: hook.to_string(),
            workflow_metadata,
            extensions,
        },
    )?;
    Ok(output.workflow_metadata)
}

pub struct ChapterPipelineProviders<'a> {
    pub draft: &'a dyn ModelClient,
    pub review: &'a dyn ModelClient,
    pub repair: &'a dyn ModelClient,
    pub postprocess: &'a dyn ModelClient,
}

impl<'a> ChapterPipelineProviders<'a> {
    pub fn single(provider: &'a dyn ModelClient) -> Self {
        Self {
            draft: provider,
            review: provider,
            repair: provider,
            postprocess: provider,
        }
    }
}

pub async fn generate_next_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    emb_provider: Option<&dyn ModelClient>,
    project_id: &str,
    force: bool,
    log_tx: &mpsc::Sender<String>,
    event_tx: &mpsc::Sender<PipelineEvent>,
    operator_controls: Option<writing_context::OperatorControls>,
) -> Result<GenerationResult, String> {
    generate_next_chapter_with_stage_providers(
        db,
        ChapterPipelineProviders::single(provider),
        emb_provider,
        project_id,
        force,
        log_tx,
        event_tx,
        operator_controls,
    )
    .await
}

pub async fn generate_next_chapter_with_stage_providers(
    db: &Database,
    providers: ChapterPipelineProviders<'_>,
    emb_provider: Option<&dyn ModelClient>,
    project_id: &str,
    force: bool,
    log_tx: &mpsc::Sender<String>,
    event_tx: &mpsc::Sender<PipelineEvent>,
    operator_controls: Option<writing_context::OperatorControls>,
) -> Result<GenerationResult, String> {
    // 1. Check if already has a chapter today (unless force)
    let today_count = generation_jobs::get_today_chapter_count(db, project_id)?;
    if today_count > 0 && !force {
        return Ok(GenerationResult {
            ok: false,
            message: "Already generated a chapter today. Use force=true to override.".into(),
            chapter_id: None,
            chapter_title: None,
            sequence: None,
            word_count: None,
            final_score: None,
            decision: None,
            filename: None,
        });
    }

    // 2. Check if a job is already running
    if generation_jobs::is_job_running(db, project_id)? {
        return Ok(GenerationResult {
            ok: false,
            message: "A generation job is already running for this project.".into(),
            chapter_id: None,
            chapter_title: None,
            sequence: None,
            word_count: None,
            final_score: None,
            decision: None,
            filename: None,
        });
    }

    // 3. Acquire advisory lock (RAII guard auto-releases on drop)
    let _lock_guard =
        match lock::GenerationLock::acquire(db, project_id, lock::LockType::ChapterGeneration) {
            Ok(g) => g,
            Err(_) => {
                return Ok(GenerationResult {
                    ok: false,
                    message: "Could not acquire generation lock. Another job may be running."
                        .into(),
                    chapter_id: None,
                    chapter_title: None,
                    sequence: None,
                    word_count: None,
                    final_score: None,
                    decision: None,
                    filename: None,
                })
            }
        };
    emit(event_tx, "acquire_lock", "done", None, 3.0);

    log(log_tx, "=== Starting Daily Chapter Production ===");

    // 4. Load project config
    let project = projects::get_project(db, project_id)?;
    let settings = crate::db::settings::get_settings(db)?;
    log(
        log_tx,
        &format!(
            "Project: {} | Quality threshold: {}",
            project.name, project.quality_threshold
        ),
    );

    // 5. Select next chapter plan
    let plan = match chapters::get_next_chapter_plan(db, project_id)? {
        Some(p) => p,
        None => {
            log(
                log_tx,
                "No chapter plan found. Run Weekly Arc Planner first.",
            );
            return Ok(GenerationResult {
                ok: false,
                message: "No chapter plans available. Please run 'Generate Weekly Plan' first."
                    .into(),
                chapter_id: None,
                chapter_title: None,
                sequence: None,
                word_count: None,
                final_score: None,
                decision: None,
                filename: None,
            });
        }
    };
    log(
        log_tx,
        &format!(
            "Chapter plan: Ch.{} — {}",
            plan.sequence,
            plan.title.as_deref().unwrap_or("Untitled")
        ),
    );

    // 6. Create generation job (idempotent)
    let job_id = generation_jobs::create_generation_job(db, project_id, &plan.id)?;
    task_transaction::begin_generation_task_snapshot(db, &job_id, project_id, &plan.id)?;
    log(log_tx, &format!("Job created: {}", &job_id[..8]));
    let _ = generation_jobs::record_job_phase_event(db, &job_id, "acquire_lock", "done", None, 3.0);

    // 7. Load structured canon
    let canon_data = bible::get_bible(db, project_id)?;
    log(
        log_tx,
        &format!(
            "Loaded canon: {} chars, {} lore, {} threads, {} foreshadowing",
            canon_data.characters.len(),
            canon_data.world_lore.len(),
            canon_data.plot_threads.len(),
            canon_data.foreshadowing.len()
        ),
    );
    emit_job_event(
        db,
        &job_id,
        event_tx,
        "load_canon",
        "done",
        Some(&format!(
            "{} chars, {} lore",
            canon_data.characters.len(),
            canon_data.world_lore.len()
        )),
        10.0,
    );

    // 8. Build retrieval query + retrieve vector context
    let retrieval_query = writing_context::build_retrieval_query(&plan, operator_controls.as_ref());
    log(log_tx, "Retrieving vector context...");

    let mut retrieval_documents = Vec::new();
    let retrieval_detail: String;
    if retrieval_query.trim().is_empty() {
        retrieval_detail = "empty retrieval query; using structured context".to_string();
    } else if let Some(embed_client) = emb_provider {
        match embed_client
            .embed_with_kind(&[retrieval_query.clone()], EmbeddingInputKind::Query)
            .await
        {
            Ok(embeddings) if !embeddings.is_empty() => {
                match crate::db::vector_store::search_similar_documents(
                    db,
                    project_id,
                    &embeddings[0],
                    12,
                ) {
                    Ok(docs) => {
                        log(log_tx, &format!("Found {} relevant documents", docs.len()));
                        retrieval_detail = format!("{} docs", docs.len());
                        retrieval_documents = docs;
                    }
                    Err(e) => {
                        retrieval_detail = format!("vector search skipped: {}", e);
                        log(log_tx, &format!("Vector search fallback: {}", e));
                    }
                }
            }
            _ => {
                retrieval_detail =
                    "embedding failed or empty; using structured context".to_string();
                log(
                    log_tx,
                    "Embedding failed or empty, continuing without vector context",
                );
            }
        }
    } else {
        retrieval_detail = "RAG disabled; using structured context".to_string();
        log(log_tx, "RAG disabled; using structured context");
    }
    emit_job_event(
        db,
        &job_id,
        event_tx,
        "retrieve_context",
        "done",
        Some(&retrieval_detail),
        18.0,
    );
    let _before_context_metadata = run_extension_hook_for_job(
        db,
        &job_id,
        "before_context_build",
        serde_json::json!({
            "project_id": project_id,
            "chapter_plan_id": &plan.id,
            "retrieval_query": retrieval_query,
            "retrieval_document_count": retrieval_documents.len(),
        }),
    )?;

    // 9. Build writing context package
    let prev_chapters = chapters::get_chapters(db, project_id)?;
    let prev_context = prev_chapters
        .iter()
        .map(|c| format!("Ch.{}: {}", c.sequence, c.summary.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");

    let operator_controls_for_extension = operator_controls.clone();
    let mut writing_context = writing_context::build_writing_context(
        db,
        &project,
        &plan,
        &canon_data,
        &settings,
        retrieval_documents.clone(),
        operator_controls,
    )?;
    let after_context_metadata = run_extension_hook_for_job(
        db,
        &job_id,
        "after_context_build",
        build_context_metadata(&writing_context, None),
    )?;
    context_activation::append_extension_context_rules(
        &mut writing_context.context_activation,
        &after_context_metadata,
        &plan,
        operator_controls_for_extension.as_ref(),
    )?;
    let used_learning_ids = writing_context
        .learned_patterns
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    generation_jobs::record_job_learning_context(db, &job_id, &used_learning_ids)?;
    let writing_context_json = serde_json::to_string_pretty(&writing_context).unwrap_or_default();
    let draft_model_profile = resolve_model_profile(db, &settings, "draft")?;
    let review_model_profile = resolve_model_profile(db, &settings, "review")?;
    let repair_model_profile = resolve_model_profile(db, &settings, "repair")?;

    let context_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&writing_context_json);
        hex::encode(hasher.finalize())
    };

    // 10. Render draft writer prompt
    let extension_prompt_units =
        prompt_runtime::extension_prompt_units_from_metadata(&after_context_metadata)?;
    let assembled_draft_prompt = prompt_runtime::assemble_builtin_draft_prompt_with_extra_units(
        &writing_context_json,
        extension_prompt_units,
    )?;
    let system_prompt = assembled_draft_prompt.system_prompt.clone();
    let user_prompt = assembled_draft_prompt.user_prompt.clone();
    let prompt_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&system_prompt);
        hasher.update("\n---USER---\n");
        hasher.update(&user_prompt);
        hex::encode(hasher.finalize())
    };

    log(log_tx, "Calling draft writer...");

    // 11. Call draft writer
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "body_markdown": {"type": "string"},
            "summary": {"type": "string"},
            "word_count": {"type": "integer"},
            "pov_character": {"type": "string"},
            "major_events": {"type": "array"},
            "character_state_changes": {"type": "array"},
            "timeline_events": {"type": "array"},
            "foreshadowing_used": {"type": "array"},
            "foreshadowing_planted": {"type": "array"},
            "new_canon_candidates": {"type": "array"},
            "continuity_notes": {"type": "string"},
            "used_context_ids": {"type": "array"}
        }
    });

    let draft = match providers
        .draft
        .generate_json_with_usage(&system_prompt, &user_prompt, &json_schema, 32768)
        .await
    {
        Ok((d, usage)) => {
            let output_text = serde_json::to_string(&d).unwrap_or_default();
            record_model_usage(
                db,
                &job_id,
                "generate_draft",
                &settings,
                draft_model_profile.as_ref(),
                &system_prompt,
                &user_prompt,
                &output_text,
                usage.as_ref(),
            );
            let wc = d["word_count"].as_i64().unwrap_or(0);
            emit_job_event(
                db,
                &job_id,
                event_tx,
                "generate_draft",
                "done",
                Some(&format!("{} words", wc)),
                35.0,
            );
            log(log_tx, "Draft generated successfully");
            d
        }
        Err(e) => {
            emit_job_event(
                db,
                &job_id,
                event_tx,
                "generate_draft",
                "failed",
                Some(&e),
                35.0,
            );
            generation_jobs::update_job_status(db, &job_id, "failed", Some(&e))?;
            log(log_tx, &format!("Draft generation failed: {}", e));
            return Err(format!("Draft generation failed: {}", e));
        }
    };

    // 12. Validate & save draft version
    let title = draft["title"].as_str().unwrap_or("Untitled").to_string();
    let body = draft["body_markdown"].as_str().unwrap_or("").to_string();
    let summary = draft["summary"].as_str().unwrap_or("").to_string();

    // Reject empty body — AI returned invalid content
    if body.len() < 50 {
        let err_msg = format!(
            "AI returned insufficient content. body_markdown length: {} chars. Raw keys: {:?}",
            body.len(),
            draft
                .as_object()
                .map(|o| o.keys().collect::<Vec<_>>())
                .unwrap_or_default()
        );
        log(log_tx, &err_msg);
        emit_job_event(
            db,
            &job_id,
            event_tx,
            "generate_draft",
            "failed",
            Some(&err_msg),
            35.0,
        );
        generation_jobs::update_job_status(db, &job_id, "failed", Some(&err_msg))?;
        return Err(err_msg);
    }

    let word_count = draft["word_count"].as_i64().unwrap_or(body.len() as i64) as i32;

    let (chapter_id, version_id) = chapters::save_draft_version(
        db,
        project_id,
        &plan.id,
        plan.sequence,
        &title,
        &body,
        word_count,
        &summary,
        &model_identity(&settings, draft_model_profile.as_ref()).0,
        &model_identity(&settings, draft_model_profile.as_ref()).1,
        &prompt_hash,
        &context_hash,
    )?;
    task_transaction::record_task_owned_row(db, &job_id, "chapters", &chapter_id)?;
    task_transaction::record_task_owned_row(db, &job_id, "chapter_versions", &version_id)?;
    let context_metadata = build_context_metadata(&writing_context, Some(&assembled_draft_prompt));
    chapters::update_chapter_version_metadata(db, &version_id, &context_metadata)?;

    log(
        log_tx,
        &format!("Draft saved: {} ({} words)", &chapter_id[..8], word_count),
    );
    if !used_learning_ids.is_empty() {
        task_transaction::record_learning_entry_usage_snapshot(db, &job_id, &used_learning_ids)?;
        learning::mark_learning_entries_used(db, &used_learning_ids)?;
    }
    emit_preview(event_tx, "draft_preview", &title, &body, "draft", 38.0);

    generation_jobs::update_job_status(db, &job_id, "reviewing", None)?;

    // 13. Run review agents
    let chapter = chapters::get_chapter(db, &chapter_id)?;
    let version = chapters::get_latest_version(db, &chapter_id)?.ok_or("Version not found")?;

    // Build previous chapter body context (last 2 chapters, truncated to 3000 chars each)
    let prev_bodies: Vec<String> = prev_chapters
        .iter()
        .rev()
        .take(2)
        .filter_map(|c| {
            chapters::get_latest_version(db, &c.id)
                .ok()
                .flatten()
                .and_then(|v| v.body_markdown)
                .map(|body| {
                    let truncated: String = body.chars().take(3000).collect();
                    format!(
                        "Ch.{} — {}:\n{}",
                        c.sequence,
                        c.title.as_deref().unwrap_or(""),
                        truncated
                    )
                })
        })
        .collect();
    let prev_body_context = prev_bodies.join("\n\n---\n\n");

    // Load character states for continuity checking
    let character_states =
        crate::db::bible::get_character_states(db, project_id).unwrap_or_default();

    let canon_ctx = review_agents::CanonContext {
        writing_brief_json: writing_context_json.clone(),
        characters_json: serde_json::to_string(&canon_data.characters).unwrap_or_default(),
        character_states_json: serde_json::to_string(&character_states).unwrap_or_default(),
        previous_chapters_json: if !prev_body_context.is_empty() {
            prev_body_context
        } else {
            prev_context
        },
        active_plot_threads_json: serde_json::to_string(&canon_data.plot_threads)
            .unwrap_or_default(),
        unresolved_foreshadowing_json: serde_json::to_string(&canon_data.foreshadowing)
            .unwrap_or_default(),
        world_lore_json: serde_json::to_string(&canon_data.world_lore).unwrap_or_default(),
        locations_json: serde_json::to_string(&canon_data.locations).unwrap_or_default(),
        organizations_json: serde_json::to_string(&canon_data.organizations).unwrap_or_default(),
        items_json: serde_json::to_string(&canon_data.items).unwrap_or_default(),
        magic_systems_json: serde_json::to_string(&canon_data.magic_systems).unwrap_or_default(),
        canon_rules_json: serde_json::to_string(&canon_data.canon_rules).unwrap_or_default(),
        timeline_json: serde_json::to_string(&canon_data.timeline_events).unwrap_or_default(),
        style_guide_json: serde_json::to_string(&canon_data.style_guides).unwrap_or_default(),
        extension_review_rubrics_json: serde_json::to_string(
            &review_agents::extension_review_rubrics_from_metadata(&after_context_metadata),
        )
        .unwrap_or_default(),
        blog_config_json: serde_json::json!({
            "provider": project.blog_provider.as_deref().unwrap_or("local"),
            "status": "draft",
            "auto_publish": project.auto_publish,
        })
        .to_string(),
        project_policy_json: serde_json::json!({
            "auto_publish": project.auto_publish,
            "quality_threshold": project.quality_threshold,
        })
        .to_string(),
    };

    log(log_tx, "Running 7 parallel review agents...");
    let _before_review_metadata = run_extension_hook_for_job(
        db,
        &job_id,
        "before_review",
        serde_json::json!({
            "project_id": project_id,
            "chapter_id": &chapter_id,
            "chapter_version_id": &version_id,
            "chapter_title": chapter.title.as_deref(),
        }),
    )?;
    let agent_reviews = review_agents::run_review_agents(
        providers.review,
        &chapter,
        &version,
        &canon_ctx,
        &project,
    )
    .await?;
    let _after_review_metadata = run_extension_hook_for_job(
        db,
        &job_id,
        "after_review",
        serde_json::json!({
            "project_id": project_id,
            "chapter_id": &chapter_id,
            "chapter_version_id": &version_id,
            "reviews": agent_reviews.iter().map(|review| serde_json::json!({
                "agent_name": &review.agent_name,
                "score": review.score,
                "pass": review.pass,
            })).collect::<Vec<_>>(),
        }),
    )?;
    record_model_usage(
        db,
        &job_id,
        "review_agents",
        &settings,
        review_model_profile.as_ref(),
        &review_usage_context(&canon_ctx),
        version.body_markdown.as_deref().unwrap_or(""),
        &serde_json::to_string(&agent_reviews).unwrap_or_default(),
        None,
    );

    // Log each review score
    for review in &agent_reviews {
        log(
            log_tx,
            &format!(
                "  {} score={} pass={}",
                review.agent_name,
                review.score.unwrap_or(0),
                review.pass.unwrap_or(false)
            ),
        );
    }

    // Save review records with retry
    for review in &agent_reviews {
        let mut saved = false;
        for attempt in 0..2 {
            match reviews::save_agent_review(
                db,
                project_id,
                &chapter_id,
                &version_id,
                &review.agent_name,
                review.score.unwrap_or(0),
                review.pass.unwrap_or(false),
                &review.blocking_issues,
                &review.minor_issues,
                &review.recommendations,
                &review.raw_output,
            ) {
                Ok(review_id) => {
                    task_transaction::record_task_owned_row(
                        db,
                        &job_id,
                        "agent_reviews",
                        &review_id,
                    )?;
                    saved = true;
                    break;
                }
                Err(e) if attempt == 0 => {
                    log(
                        log_tx,
                        &format!("Save review {} failed, retrying: {}", review.agent_name, e),
                    );
                }
                Err(e) => {
                    log(
                        log_tx,
                        &format!(
                            "Save review {} failed after retry: {}",
                            review.agent_name, e
                        ),
                    );
                }
            }
        }
        if !saved {
            log(
                log_tx,
                &format!("CRITICAL: Could not save review for {}", review.agent_name),
            );
        }
    }

    // 14. Aggregate reviews
    let aggregation = review_arbiter::aggregate_reviews(
        &agent_reviews,
        project.quality_threshold,
        settings.max_revise_count,
        0,
    );
    log(
        log_tx,
        &format!(
            "Reviews: avg={:.1} final={:.1} decision={} blocking={}",
            aggregation.average_score,
            aggregation.final_score,
            aggregation.decision,
            aggregation.blocking_issue_count
        ),
    );
    emit_job_event(
        db,
        &job_id,
        event_tx,
        "aggregate_reviews",
        "done",
        Some(&format!(
            "avg={:.1} decision={}",
            aggregation.average_score, aggregation.decision
        )),
        65.0,
    );

    let review_score_id = reviews::save_review_scores(
        db,
        project_id,
        &chapter_id,
        &version_id,
        aggregation.average_score,
        aggregation.final_score,
        &aggregation.decision,
        aggregation.publish_allowed,
        aggregation.blocking_issue_count,
    )?;
    task_transaction::record_task_owned_row(db, &job_id, "review_scores", &review_score_id)?;

    // 15. Revise loop: keep revising until score >= threshold or retries exhausted
    let mut revise_count = 0;
    let mut current_draft = draft.clone();
    let mut current_version_id = version_id.clone();
    let mut current_aggregation = aggregation;
    let mut current_reviews = agent_reviews.clone();

    let (final_decision, final_score, final_version_id) = loop {
        if current_aggregation.decision == "needs_human_review"
            || current_aggregation.decision == "publish_ready"
        {
            break (
                current_aggregation.decision.clone(),
                current_aggregation.final_score,
                current_version_id.clone(),
            );
        }
        if current_aggregation.decision != "revise" {
            break (
                current_aggregation.decision.clone(),
                current_aggregation.final_score,
                current_version_id.clone(),
            );
        }
        if revise_count >= settings.max_revise_count {
            log(
                log_tx,
                &format!(
                    "Max revisions ({}) reached. Marking for human review.",
                    settings.max_revise_count
                ),
            );
            generation_jobs::update_job_status(db, &job_id, "needs_human_review", None)?;
            break (
                "needs_human_review".into(),
                current_aggregation.final_score,
                current_version_id.clone(),
            );
        }

        revise_count += 1;
        log(
            log_tx,
            &format!(
                "Revision {}/{} needed (score {:.0} < threshold {}). Calling revision writer...",
                revise_count,
                settings.max_revise_count,
                current_aggregation.final_score,
                project.quality_threshold
            ),
        );

        let revision_template = prompts::load_prompt("revision_writer")?;
        let rev_input = serde_json::json!({
            "chapter": current_draft,
            "reviews": current_reviews.iter().map(|r| serde_json::json!({
                "agent": r.agent_name, "score": r.score, "pass": r.pass,
                "blocking_issues": r.blocking_issues, "minor_issues": r.minor_issues,
                "recommendations": r.recommendations,
            })).collect::<Vec<_>>(),
            "average_score": current_aggregation.average_score,
            "blocking_issue_count": current_aggregation.blocking_issue_count,
        });
        let mut rev_vars = HashMap::new();
        rev_vars.insert(
            "REVISION_INPUT_JSON".to_string(),
            serde_json::to_string_pretty(&rev_input).unwrap_or_default(),
        );
        let rev_rendered = prompt_rendering::render_prompt_strict(
            "revision_writer",
            &revision_template,
            &rev_vars
                .iter()
                .map(|(k, v)| (k.as_str(), v.clone()))
                .collect(),
        )?;
        let (rev_sys, rev_user) = if let Some(pos) = rev_rendered.find("\n\n") {
            let (s, u) = rev_rendered.split_at(pos);
            (s.to_string(), u.trim_start_matches('\n').to_string())
        } else {
            (rev_rendered.clone(), rev_rendered)
        };

        match providers
            .repair
            .generate_json_with_usage(&rev_sys, &rev_user, &json_schema, 32768)
            .await
        {
            Ok((revised, usage)) => {
                let output_text = serde_json::to_string(&revised).unwrap_or_default();
                record_model_usage(
                    db,
                    &job_id,
                    "revise",
                    &settings,
                    repair_model_profile.as_ref(),
                    &rev_sys,
                    &rev_user,
                    &output_text,
                    usage.as_ref(),
                );
                let rev_body = revised["body_markdown"].as_str().unwrap_or("").to_string();
                let rev_wc = revised["word_count"]
                    .as_i64()
                    .unwrap_or(rev_body.len() as i64) as i32;
                if rev_body.len() < 100 {
                    log(log_tx, "Revision too short, keeping previous version");
                    break (
                        current_aggregation.decision.clone(),
                        current_aggregation.final_score,
                        current_version_id.clone(),
                    );
                }
                let rev_version_id = Database::new_uuid();
                {
                    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
                    conn.execute(
                        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count, model_provider, model_name, created_by_agent)
                         VALUES (?1, ?2, ?3, ?4, 'revised', ?5, ?6, ?7, ?8, ?9, 'revision_writer')",
                        rusqlite::params![rev_version_id, chapter_id, project_id, 1 + revise_count, title, rev_body, rev_wc, model_identity(&settings, repair_model_profile.as_ref()).0, model_identity(&settings, repair_model_profile.as_ref()).1],
                    ).map_err(|e| format!("Insert revision: {}", e))?;
                }
                task_transaction::record_task_owned_row(
                    db,
                    &job_id,
                    "chapter_versions",
                    &rev_version_id,
                )?;
                let _ = chapters::update_chapter_after_revision(
                    db,
                    &chapter_id,
                    project_id,
                    &rev_version_id,
                    &title,
                    &rev_body,
                    rev_wc,
                    &summary,
                    "revised",
                    current_aggregation.final_score,
                    &current_aggregation.decision,
                );
                log(
                    log_tx,
                    &format!("Revision {} saved ({} words)", revise_count, rev_wc),
                );
                emit_preview(
                    event_tx,
                    "revision_preview",
                    &title,
                    &rev_body,
                    "revision",
                    72.0,
                );
                current_draft = revised;
                current_version_id = rev_version_id;

                // Re-run reviews after revision
                log(
                    log_tx,
                    &format!("Re-running reviews after revision {}...", revise_count),
                );
                emit_job_event(
                    db,
                    &job_id,
                    event_tx,
                    "revise",
                    "done",
                    Some(&format!("revision {} complete", revise_count)),
                    70.0 + (revise_count as f64 * 10.0),
                );
                let chapter = chapters::get_chapter(db, &chapter_id)?;
                let version =
                    chapters::get_latest_version(db, &chapter_id)?.ok_or("Version not found")?;
                current_reviews = review_agents::run_review_agents(
                    providers.review,
                    &chapter,
                    &version,
                    &canon_ctx,
                    &project,
                )
                .await?;
                record_model_usage(
                    db,
                    &job_id,
                    "review_agents_after_revise",
                    &settings,
                    review_model_profile.as_ref(),
                    &review_usage_context(&canon_ctx),
                    version.body_markdown.as_deref().unwrap_or(""),
                    &serde_json::to_string(&current_reviews).unwrap_or_default(),
                    None,
                );
                for review in &current_reviews {
                    log(
                        log_tx,
                        &format!(
                            "  {} score={} pass={}",
                            review.agent_name,
                            review.score.unwrap_or(0),
                            review.pass.unwrap_or(false)
                        ),
                    );
                    let review_id = reviews::save_agent_review(
                        db,
                        project_id,
                        &chapter_id,
                        &current_version_id,
                        &review.agent_name,
                        review.score.unwrap_or(0),
                        review.pass.unwrap_or(false),
                        &review.blocking_issues,
                        &review.minor_issues,
                        &review.recommendations,
                        &review.raw_output,
                    )?;
                    task_transaction::record_task_owned_row(
                        db,
                        &job_id,
                        "agent_reviews",
                        &review_id,
                    )?;
                }
                current_aggregation = review_arbiter::aggregate_reviews(
                    &current_reviews,
                    project.quality_threshold,
                    settings.max_revise_count,
                    revise_count,
                );
                log(
                    log_tx,
                    &format!(
                        "Post-revision reviews: avg={:.1} decision={}",
                        current_aggregation.average_score, current_aggregation.decision
                    ),
                );
            }
            Err(e) => {
                log(log_tx, &format!("Revision {} failed: {}", revise_count, e));
                break (
                    current_aggregation.decision.clone(),
                    current_aggregation.final_score,
                    current_version_id.clone(),
                );
            }
        }
    };

    // Update chapter status from the final draft, not the original draft.
    let final_title = current_draft["title"]
        .as_str()
        .unwrap_or(&title)
        .to_string();
    let final_body = current_draft["body_markdown"]
        .as_str()
        .unwrap_or(&body)
        .to_string();
    let final_summary = current_draft["summary"]
        .as_str()
        .unwrap_or(&summary)
        .to_string();
    let final_word_count = current_draft["word_count"]
        .as_i64()
        .unwrap_or_else(|| final_body.chars().count() as i64) as i32;
    let final_status = if final_decision == "needs_human_review" {
        "needs_human_review"
    } else if final_decision == "publish_ready" {
        "final"
    } else {
        "revised"
    };
    chapters::update_chapter_after_revision(
        db,
        &chapter_id,
        project_id,
        &current_version_id,
        &final_title,
        &final_body,
        final_word_count,
        &final_summary,
        final_status,
        final_score,
        &final_decision,
    )?;
    if final_status == "final" {
        chapters::update_chapter_version_type(db, &current_version_id, "final")?;
        let hard_facts = hard_fact_ledger::materialize_hard_facts_from_chapter_version(
            db,
            project_id,
            &chapter_id,
            &current_version_id,
        )?;
        for fact in &hard_facts {
            task_transaction::record_task_owned_row(db, &job_id, "hard_facts", &fact.id)?;
        }
        emit_job_event(
            db,
            &job_id,
            event_tx,
            "hard_facts",
            "done",
            Some(&format!("{} hard facts materialized", hard_facts.len())),
            84.0,
        );
    }

    // 16. Generate blog metadata if auto_publish
    let export_result = markdown::export_chapter_markdown(db, &chapter_id, &settings.data_dir);
    let filename = match export_result {
        Ok(path) => {
            log(log_tx, &format!("Chapter exported: {}", path));
            let _export_target_metadata = run_extension_hook_for_job(
                db,
                &job_id,
                "export_target",
                serde_json::json!({
                    "project_id": project_id,
                    "chapter_id": &chapter_id,
                    "chapter_title": &final_title,
                    "export_path": &path,
                    "decision": &final_decision,
                }),
            )?;
            emit_job_event(db, &job_id, event_tx, "export", "done", Some(&path), 85.0);
            if project.auto_publish && final_decision == "publish_ready" {
                log(log_tx, "Generating blog metadata...");
                let provider_name = project.blog_provider.as_deref().unwrap_or("local");
                let blog_id = blog_posts::create_blog_post(
                    db,
                    project_id,
                    &chapter_id,
                    provider_name,
                    &final_title,
                    "",
                    None,
                )?;
                task_transaction::record_task_owned_row(db, &job_id, "blog_posts", &blog_id)?;
                if let Some(queue_id) = enqueue_publication_if_enabled(
                    db,
                    &settings,
                    &project,
                    &chapter_id,
                    Some(&final_version_id),
                    &final_title,
                )? {
                    task_transaction::record_task_owned_row(
                        db,
                        &job_id,
                        "publication_queue",
                        &queue_id,
                    )?;
                    emit_job_event(
                        db,
                        &job_id,
                        event_tx,
                        "publication_queue",
                        "queued",
                        Some(&queue_id),
                        88.0,
                    );
                }
            }
            Some(path)
        }
        Err(e) => {
            log(log_tx, &format!("Export failed: {}", e));
            emit_job_event(db, &job_id, event_tx, "export", "failed", Some(&e), 85.0);
            let err = format!("Export failed: {}", e);
            generation_jobs::update_job_status(db, &job_id, "failed", Some(&err))?;
            return Err(err);
        }
    };

    // 17. Update canon
    log(log_tx, "Updating canon...");
    match canon_updater::update_canon_after_chapter(
        db,
        providers.postprocess,
        project_id,
        &chapter_id,
        &current_draft,
        Some(&job_id),
    )
    .await
    {
        Ok(()) => {
            emit_job_event(db, &job_id, event_tx, "update_canon", "done", None, 92.0);
        }
        Err(e) => {
            log(
                log_tx,
                &format!("Canon update skipped after chapter save: {}", e),
            );
            emit_job_event(
                db,
                &job_id,
                event_tx,
                "update_canon",
                "failed_noncritical",
                Some(&e),
                92.0,
            );
        }
    }

    let reflection_scores = serde_json::json!({
        "average_score": current_aggregation.average_score,
        "final_score": final_score,
        "decision": final_decision,
        "blocking_issue_count": current_aggregation.blocking_issue_count,
    })
    .to_string();
    match learning::reflect_on_chapter(
        providers.postprocess,
        &final_title,
        &final_body,
        &reflection_scores,
        &writing_context.learned_patterns,
    )
    .await
    {
        Ok(entries) if !entries.is_empty() => {
            match learning::save_reflection_entries(db, project_id, &entries) {
                Ok(()) => log(
                    log_tx,
                    &format!("Saved {} self-reflection learning entries", entries.len()),
                ),
                Err(e) => log(log_tx, &format!("Self-reflection save skipped: {}", e)),
            }
        }
        Ok(_) => {}
        Err(e) => log(log_tx, &format!("Self-reflection skipped: {}", e)),
    }

    if final_decision == "publish_ready" {
        chapters::mark_chapter_plan_completed(db, &plan.id)?;
    }

    // 18. Update job status
    let final_job_status = if final_decision == "needs_human_review" {
        "needs_human_review"
    } else {
        "completed"
    };
    emit_job_event(
        db,
        &job_id,
        event_tx,
        "complete",
        "done",
        Some(&format!(
            "{} words, score {:.0}",
            final_word_count, final_score
        )),
        100.0,
    );
    generation_jobs::update_job_status(db, &job_id, final_job_status, None)?;
    log(log_tx, "=== Chapter production complete ===");

    Ok(GenerationResult {
        ok: true,
        message: format!(
            "Chapter {} generated: {} (score: {:.0})",
            plan.sequence, final_title, final_score
        ),
        chapter_id: Some(chapter_id),
        chapter_title: Some(final_title),
        sequence: Some(plan.sequence),
        word_count: Some(final_word_count),
        final_score: Some(final_score),
        decision: Some(final_decision),
        filename,
    })
}
