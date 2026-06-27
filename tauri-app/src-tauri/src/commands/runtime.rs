use crate::models::AppSettings;
use crate::AppState;
use crate::{ai, db, extensions, get_embedding_provider, provider_config_for_workflow, workflow};

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
            if let Ok(embeddings) = embed_client.embed(&[retrieval_query]).await {
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
