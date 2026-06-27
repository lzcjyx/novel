use crate::models::AppSettings;
use crate::AppState;
use crate::{ai, db, extensions, get_embedding_provider, provider_config_for_workflow, workflow};
use std::collections::HashMap;

#[tauri::command]
pub async fn get_next_chapter_context_preview(
    state: tauri::State<'_, AppState>,
    project_id: String,
    operator_controls: Option<workflow::writing_context::OperatorControls>,
) -> Result<serde_json::Value, String> {
    let project = db::projects::get_project(&state.db, &project_id)?;
    let plan = db::chapters::get_next_chapter_plan(&state.db, &project_id)?
        .ok_or_else(|| "No planned chapter found. Generate a weekly plan first.".to_string())?;
    let canon = db::bible::get_bible(&state.db, &project_id)?;
    let settings = db::settings::get_settings(&state.db)?;
    let retrieval_query =
        workflow::writing_context::build_retrieval_query(&plan, operator_controls.as_ref());
    let mut retrieval_documents = Vec::new();

    if !retrieval_query.trim().is_empty() {
        if let Ok(embed_client) = get_embedding_provider(&state) {
            if let Ok(embeddings) = embed_client
                .embed_with_kind(&[retrieval_query], ai::client::EmbeddingInputKind::Query)
                .await
            {
                if let Some(query_embedding) = embeddings.first() {
                    retrieval_documents = db::vector_store::search_similar_documents(
                        &state.db,
                        &project_id,
                        query_embedding,
                        8,
                    )
                    .unwrap_or_default();
                }
            }
        }
    }

    let package = workflow::writing_context::build_writing_context(
        &state.db,
        &project,
        &plan,
        &canon,
        &settings,
        retrieval_documents,
        operator_controls,
    )?;

    let writing_context_json =
        serde_json::to_string_pretty(&package).map_err(|e| format!("Serialize context: {}", e))?;
    let assembled_prompt =
        workflow::prompt_runtime::assemble_builtin_draft_prompt(&writing_context_json)?;
    let mut preview =
        serde_json::to_value(package).map_err(|e| format!("Serialize context preview: {}", e))?;
    preview["prompt_runtime"] =
        workflow::prompt_runtime::assembled_prompt_preview_payload(&assembled_prompt);
    Ok(preview)
}

#[tauri::command]
pub async fn get_rag_health(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<db::vector_store::RagHealth, String> {
    let settings = db::settings::get_settings(&state.db)?;
    let (embedding_provider, embedding_model) =
        if let Some(profile_id) = settings.embedding_model_profile_id.as_deref() {
            let profile = db::model_profiles::get_model_profile(&state.db, profile_id)?;
            (profile.provider, profile.model)
        } else {
            (
                settings.embedding_provider.clone(),
                settings.embedding_model.clone(),
            )
        };

    if embedding_provider != "none" {
        if let Err(e) = get_embedding_provider(&state) {
            return Ok(db::vector_store::RagHealth {
                state: "missing_key".into(),
                message: format!("RAG embedding 密钥不可用：{e}"),
                document_count: 0,
                stale_count: 0,
                embedding_provider,
                embedding_model,
                embedding_dim: settings.embedding_dim,
                last_indexed_at: None,
            });
        }
    }

    db::vector_store::get_rag_health(
        &state.db,
        &project_id,
        &embedding_provider,
        &embedding_model,
        settings.embedding_dim,
    )
}

#[tauri::command]
pub async fn get_context_rules(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<db::context_rules::ContextRule>, String> {
    db::context_rules::list_context_rules(&state.db, &project_id)
}

#[tauri::command]
pub async fn generate_direction_candidates(
    state: tauri::State<'_, AppState>,
    project_id: Option<String>,
    inspiration: String,
    candidate_count: usize,
) -> Result<Vec<db::director::DirectionCandidate>, String> {
    let settings = db::settings::get_settings(&state.db)?;
    let provider = provider_config_for_workflow(&state, &settings, "draft")?.build()?;
    let model_profile_snapshot =
        serde_json::to_value(&settings).map_err(|e| format!("Serialize settings: {}", e))?;
    workflow::director_mode::generate_direction_candidates(
        &state.db,
        provider.as_ref(),
        workflow::director_mode::DirectionGenerationRequest {
            project_id,
            inspiration,
            candidate_count,
            model_profile_snapshot,
        },
    )
    .await
}

#[tauri::command]
pub async fn list_direction_candidates(
    state: tauri::State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<db::director::DirectionCandidate>, String> {
    db::director::list_direction_candidates(&state.db, project_id.as_deref())
}

#[tauri::command]
pub async fn select_direction_candidate(
    state: tauri::State<'_, AppState>,
    candidate_id: String,
    revision_note: Option<String>,
) -> Result<db::director::DirectionCandidate, String> {
    workflow::director_mode::select_direction_candidate(
        &state.db,
        &candidate_id,
        revision_note.as_deref(),
    )
}

#[tauri::command]
pub async fn get_director_bootstrap_handoff(
    state: tauri::State<'_, AppState>,
    candidate_id: String,
) -> Result<workflow::director_mode::DirectorBootstrapHandoff, String> {
    workflow::director_mode::build_bootstrap_handoff(&state.db, &candidate_id)
}

#[tauri::command]
pub async fn upsert_hard_fact(
    state: tauri::State<'_, AppState>,
    input: db::hard_facts::HardFactInput,
) -> Result<String, String> {
    db::hard_facts::upsert_hard_fact(&state.db, &input)
}

#[tauri::command]
pub async fn list_hard_facts(
    state: tauri::State<'_, AppState>,
    project_id: String,
    active_only: bool,
) -> Result<Vec<db::hard_facts::HardFact>, String> {
    db::hard_facts::list_hard_facts(&state.db, &project_id, active_only)
}

#[tauri::command]
pub async fn upsert_style_asset(
    state: tauri::State<'_, AppState>,
    input: db::style_assets::StyleAssetInput,
) -> Result<String, String> {
    db::style_assets::upsert_style_asset(&state.db, &input)
}

#[tauri::command]
pub async fn list_style_assets(
    state: tauri::State<'_, AppState>,
    project_id: String,
    enabled_only: bool,
) -> Result<Vec<db::style_assets::StyleAsset>, String> {
    db::style_assets::list_style_assets(&state.db, &project_id, enabled_only)
}

#[tauri::command]
pub async fn get_author_memory_banks(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<workflow::memory_banks::AuthorMemoryBanksSnapshot, String> {
    workflow::memory_banks::build_author_memory_banks(&state.db, &project_id)
}

#[tauri::command]
pub async fn upsert_user_recipe(
    state: tauri::State<'_, AppState>,
    input: workflow::operator_recipes::UserOperatorRecipeInput,
) -> Result<String, String> {
    workflow::operator_recipes::upsert_user_recipe(&state.db, &input)
}

#[tauri::command]
pub async fn list_user_recipes(
    state: tauri::State<'_, AppState>,
    project_id: String,
    enabled_only: bool,
) -> Result<Vec<workflow::operator_recipes::UserOperatorRecipe>, String> {
    workflow::operator_recipes::list_user_recipes(&state.db, &project_id, enabled_only)
}

#[tauri::command]
pub async fn create_feedback_revision_candidate(
    state: tauri::State<'_, AppState>,
    input: workflow::feedback_decisions::FeedbackRevisionCandidateInput,
) -> Result<String, String> {
    workflow::feedback_decisions::create_feedback_revision_candidate(&state.db, &input)
}

#[tauri::command]
pub async fn list_feedback_decisions(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<workflow::feedback_decisions::FeedbackRevisionDecision>, String> {
    workflow::feedback_decisions::list_feedback_decisions(&state.db, &project_id)
}

#[tauri::command]
pub async fn decide_feedback_revision(
    state: tauri::State<'_, AppState>,
    decision_id: String,
    action: String,
    decision_note: Option<String>,
) -> Result<workflow::feedback_decisions::FeedbackRevisionDecision, String> {
    let action = match action.as_str() {
        "approve" | "approved" | "Approve" => {
            workflow::feedback_decisions::FeedbackDecisionAction::Approve
        }
        "reject" | "rejected" | "Reject" => {
            workflow::feedback_decisions::FeedbackDecisionAction::Reject
        }
        "defer" | "deferred" | "Defer" => {
            workflow::feedback_decisions::FeedbackDecisionAction::Defer
        }
        other => return Err(format!("Unsupported feedback decision action '{}'", other)),
    };
    workflow::feedback_decisions::decide_feedback_revision(
        &state.db,
        &decision_id,
        action,
        decision_note.as_deref(),
    )
}

#[tauri::command]
pub async fn write_run_artifacts(
    state: tauri::State<'_, AppState>,
    job_id: String,
    base_dir: String,
    payload: workflow::run_artifacts::RunArtifactPayload,
) -> Result<workflow::run_artifacts::RunArtifactManifest, String> {
    workflow::run_artifacts::write_run_artifacts(
        &state.db,
        &job_id,
        std::path::Path::new(&base_dir),
        &payload,
    )
}

#[tauri::command]
pub async fn export_audit_sidecar(
    state: tauri::State<'_, AppState>,
    project_id: String,
    base_dir: String,
) -> Result<crate::export::audit::AuditSidecarManifest, String> {
    crate::export::audit::export_audit_sidecar(
        &state.db,
        &project_id,
        std::path::Path::new(&base_dir),
    )
}

#[tauri::command]
pub async fn create_context_compression_summary(
    state: tauri::State<'_, AppState>,
    input: db::context_compression::ContextCompressionSummaryInput,
) -> Result<String, String> {
    db::context_compression::create_context_compression_summary(&state.db, &input)
}

#[tauri::command]
pub async fn set_context_compression_status(
    state: tauri::State<'_, AppState>,
    summary_id: String,
    status: String,
) -> Result<(), String> {
    db::context_compression::set_context_compression_status(&state.db, &summary_id, &status)
}

#[tauri::command]
pub async fn list_context_compression_summaries(
    state: tauri::State<'_, AppState>,
    project_id: String,
    approved_only: bool,
) -> Result<Vec<db::context_compression::ContextCompressionSummary>, String> {
    db::context_compression::list_context_compression_summaries(
        &state.db,
        &project_id,
        approved_only,
    )
}

#[tauri::command]
pub async fn upsert_context_rule(
    state: tauri::State<'_, AppState>,
    input: db::context_rules::ContextRuleInput,
) -> Result<String, String> {
    db::context_rules::upsert_context_rule(&state.db, input)
}

#[tauri::command]
pub async fn import_sillytavern_lorebook(
    state: tauri::State<'_, AppState>,
    project_id: String,
    lorebook_json: String,
) -> Result<workflow::lorebook_import::LorebookImportSummary, String> {
    workflow::lorebook_import::import_sillytavern_lorebook(&state.db, &project_id, &lorebook_json)
}

#[tauri::command]
pub async fn export_novel_bible_package(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<workflow::package_io::NovelBiblePackage, String> {
    workflow::package_io::export_novel_bible_package(&state.db, &project_id)
}

#[tauri::command]
pub async fn import_novel_bible_package(
    state: tauri::State<'_, AppState>,
    project_id: String,
    package: workflow::package_io::NovelBiblePackage,
) -> Result<workflow::package_io::NovelBibleImportSummary, String> {
    workflow::package_io::import_novel_bible_package(&state.db, &project_id, &package)
}

#[tauri::command]
pub async fn export_project_package(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<workflow::package_io::ProjectPackage, String> {
    workflow::package_io::export_project_package(&state.db, &project_id)
}

#[tauri::command]
pub async fn import_project_package(
    state: tauri::State<'_, AppState>,
    package: workflow::package_io::ProjectPackage,
) -> Result<String, String> {
    workflow::package_io::import_project_package(&state.db, &package)
}

#[tauri::command]
pub async fn upsert_prompt_preset(
    state: tauri::State<'_, AppState>,
    input: db::prompt_presets::PromptPresetInput,
) -> Result<String, String> {
    db::prompt_presets::upsert_prompt_preset(&state.db, &input)
}

#[tauri::command]
pub async fn upsert_prompt_unit(
    state: tauri::State<'_, AppState>,
    input: db::prompt_presets::PromptUnitInput,
) -> Result<String, String> {
    db::prompt_presets::upsert_prompt_unit(&state.db, &input)
}

#[tauri::command]
pub async fn list_prompt_presets(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<db::prompt_presets::PromptPreset>, String> {
    db::prompt_presets::list_prompt_presets(&state.db)
}

#[tauri::command]
pub async fn get_prompt_preset_package(
    state: tauri::State<'_, AppState>,
    preset_id: String,
) -> Result<db::prompt_presets::PromptPresetPackage, String> {
    db::prompt_presets::export_prompt_preset_package(&state.db, &preset_id)
}

#[tauri::command]
pub async fn import_prompt_preset_package(
    state: tauri::State<'_, AppState>,
    package: db::prompt_presets::PromptPresetPackage,
) -> Result<String, String> {
    db::prompt_presets::import_prompt_preset_package(&state.db, &package)
}

#[tauri::command]
pub async fn create_prompt_preset_snapshot(
    state: tauri::State<'_, AppState>,
    preset_id: String,
    note: Option<String>,
) -> Result<db::prompt_presets::PromptPresetSnapshot, String> {
    db::prompt_presets::create_prompt_preset_snapshot(&state.db, &preset_id, note.as_deref())
}

#[tauri::command]
pub async fn list_prompt_preset_snapshots(
    state: tauri::State<'_, AppState>,
    preset_id: String,
) -> Result<Vec<db::prompt_presets::PromptPresetSnapshot>, String> {
    db::prompt_presets::list_prompt_preset_snapshots(&state.db, &preset_id)
}

#[tauri::command]
pub async fn clone_prompt_preset(
    state: tauri::State<'_, AppState>,
    source_preset_id: String,
    new_id: Option<String>,
    new_name: String,
) -> Result<String, String> {
    db::prompt_presets::clone_prompt_preset(&state.db, &source_preset_id, new_id, &new_name)
}

#[tauri::command]
pub async fn dry_run_prompt_preset(
    state: tauri::State<'_, AppState>,
    preset_id: String,
    generation_phase: String,
    temporary_overrides: HashMap<String, String>,
) -> Result<workflow::prompt_runtime::AssembledPrompt, String> {
    db::prompt_presets::dry_run_prompt_preset(
        &state.db,
        &preset_id,
        &generation_phase,
        temporary_overrides,
    )
}

#[tauri::command]
pub async fn upsert_model_profile(
    state: tauri::State<'_, AppState>,
    input: db::model_profiles::ModelProfileInput,
) -> Result<String, String> {
    db::model_profiles::upsert_model_profile(&state.db, &input)
}

#[tauri::command]
pub async fn get_model_profile(
    state: tauri::State<'_, AppState>,
    profile_id: String,
) -> Result<db::model_profiles::ModelProfile, String> {
    db::model_profiles::get_model_profile(&state.db, &profile_id)
}

#[tauri::command]
pub async fn list_model_profiles(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<db::model_profiles::ModelProfile>, String> {
    db::model_profiles::list_model_profiles(&state.db)
}

#[tauri::command]
pub async fn set_workflow_model_profile(
    state: tauri::State<'_, AppState>,
    workflow: String,
    profile_id: Option<String>,
) -> Result<AppSettings, String> {
    if let Some(profile_id) = profile_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        db::model_profiles::get_model_profile(&state.db, profile_id)?;
    }
    let mut settings = db::settings::get_settings(&state.db)?;
    let cleaned = profile_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    match workflow.as_str() {
        "draft" => settings.draft_model_profile_id = cleaned,
        "review" => settings.review_model_profile_id = cleaned,
        "repair" => settings.repair_model_profile_id = cleaned,
        "embedding" => settings.embedding_model_profile_id = cleaned,
        "summarization" => settings.summarization_model_profile_id = cleaned,
        other => return Err(format!("Unsupported workflow '{}'", other)),
    }
    db::settings::save_settings(&state.db, &settings)?;
    db::settings::get_settings(&state.db)
}

#[tauri::command]
pub async fn validate_model_profile(
    state: tauri::State<'_, AppState>,
    profile_id: String,
    workflow: ai::provider_capabilities::ModelWorkflow,
) -> Result<Vec<ai::provider_capabilities::ModelCapabilityWarning>, String> {
    let profile = db::model_profiles::get_model_profile(&state.db, &profile_id)?;
    Ok(ai::provider_capabilities::validate_model_profile_for_workflow(&profile, workflow))
}

#[tauri::command]
pub async fn get_builtin_operator_recipes(
) -> Result<Vec<workflow::operator_recipes::OperatorRecipe>, String> {
    Ok(workflow::operator_recipes::built_in_recipes())
}

#[tauri::command]
pub async fn run_operator_recipe(
    state: tauri::State<'_, AppState>,
    request: workflow::operator_recipes::OperatorRecipeRunRequest,
) -> Result<workflow::operator_recipes::OperatorRecipeRunResult, String> {
    if request.recipe_id == "generate_three_draft_candidates" {
        let settings = db::settings::get_settings(&state.db)?;
        let provider = provider_config_for_workflow(&state, &settings, "draft")?.build()?;
        workflow::operator_recipes::execute_builtin_recipe_with_provider(
            &state.db,
            request,
            provider.as_ref(),
        )
        .await
    } else {
        workflow::operator_recipes::execute_builtin_recipe(&state.db, request)
    }
}

#[tauri::command]
pub async fn get_draft_candidates(
    state: tauri::State<'_, AppState>,
    chapter_plan_id: String,
) -> Result<Vec<db::draft_alternatives::DraftCandidate>, String> {
    db::draft_alternatives::list_draft_candidates(&state.db, &chapter_plan_id)
}

#[tauri::command]
pub async fn select_draft_candidate(
    state: tauri::State<'_, AppState>,
    candidate_id: String,
    selection_reason: String,
) -> Result<(), String> {
    db::draft_alternatives::select_draft_candidate(&state.db, &candidate_id, &selection_reason)
}

#[tauri::command]
pub async fn validate_extension_manifest(
    manifest: extensions::manifest::ExtensionManifest,
) -> Result<(), String> {
    extensions::manifest::validate_extension_manifest(&manifest)
}

#[tauri::command]
pub async fn import_extension_package(
    state: tauri::State<'_, AppState>,
    package: extensions::host::ExtensionPackage,
) -> Result<String, String> {
    extensions::host::import_extension_package(&state.db, &package)
}

#[tauri::command]
pub async fn list_extension_packages(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<extensions::host::ExtensionPackage>, String> {
    extensions::host::list_extension_packages(&state.db)
}

#[tauri::command]
pub async fn set_extension_enabled(
    state: tauri::State<'_, AppState>,
    extension_id: String,
    enabled: bool,
) -> Result<(), String> {
    extensions::host::set_extension_enabled(&state.db, &extension_id, enabled)
}
