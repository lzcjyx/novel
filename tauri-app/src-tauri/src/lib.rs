use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

pub mod ai;
pub mod commands;
pub mod db;
pub mod export;
pub mod extensions;
pub mod models;
pub mod prompts;
pub mod security;
pub mod vector;
pub mod workflow;

use ai::client::ModelClient;
use db::connection::Database;
use models::*;
use security::keychain;

pub struct AppState {
    pub db: Database,
    pub logs: Mutex<Vec<String>>,
    pub running: Mutex<bool>,
}

fn add_log(state: &AppState, msg: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{}] {}", now, security::redact::redact_secrets(msg));
    if let Ok(mut logs) = state.logs.lock() {
        logs.push(line);
        let len = logs.len();
        if len > 500 {
            logs.drain(0..len - 500);
        }
    }
}

pub fn save_human_edited_chapter(
    db: &Database,
    chapter_id: &str,
    title: &str,
    body_markdown: &str,
) -> Result<i32, String> {
    let chapter = crate::db::chapters::get_chapter(db, chapter_id)?;
    let last_version = crate::db::chapters::get_latest_version(db, chapter_id)?;
    let next_version = last_version
        .as_ref()
        .map(|v| v.version_number + 1)
        .unwrap_or(2);
    let version_id = Database::new_uuid();
    let word_count = body_markdown.chars().count() as i32;

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count, created_by_agent)
         VALUES (?1, ?2, ?3, ?4, 'revised', ?5, ?6, ?7, 'human_editor')",
        rusqlite::params![version_id, chapter_id, chapter.project_id, next_version, title, body_markdown, word_count],
    ).map_err(|e| format!("Insert edited version: {}", e))?;
    conn.execute(
        "UPDATE chapters SET final_version_id = ?1, title = ?2, word_count = ?3, updated_at = datetime('now') WHERE id = ?4",
        rusqlite::params![version_id, title, word_count, chapter_id],
    ).map_err(|e| format!("Update chapter: {}", e))?;

    Ok(next_version)
}

fn simple_slug(title: &str) -> String {
    let slug = title
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|ch| *ch != '\0')
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "chapter-draft".to_string()
    } else {
        slug
    }
}

fn latest_publication_metadata(
    db: &Database,
    chapter_id: &str,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let result = conn.query_row(
        "SELECT metadata, raw_output FROM agent_reviews
         WHERE chapter_id = ?1 AND agent_name = 'publication_reviewer'
         ORDER BY created_at DESC, rowid DESC LIMIT 1",
        rusqlite::params![chapter_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    let (metadata_raw, raw_output) = match result {
        Ok(value) => value,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(serde_json::json!({})),
        Err(e) => return Err(format!("Get publication review metadata: {}", e)),
    };
    drop(conn);

    let metadata = serde_json::from_str::<serde_json::Value>(&metadata_raw).unwrap_or_default();
    if metadata.get("blog_metadata").is_some() {
        return Ok(metadata);
    }

    let raw = serde_json::from_str::<serde_json::Value>(&raw_output).unwrap_or_default();
    let blog_metadata = raw
        .get("blog_metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(serde_json::json!({
        "blog_metadata": blog_metadata,
        "publication_interface": {
            "provider_kind": "local_draft",
            "target": "blog",
            "external_publish_ready": false
        }
    }))
}

pub fn create_local_blog_draft(db: &Database, chapter_id: &str) -> Result<String, String> {
    let chapter = db::chapters::get_chapter(db, chapter_id)?;
    let version = db::chapters::get_latest_version(db, chapter_id)?.ok_or("No version found")?;
    let settings = db::settings::get_settings(db)?;
    let publication = latest_publication_metadata(db, chapter_id)?;
    let blog_metadata = publication
        .get("blog_metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let fallback_title = version
        .title
        .clone()
        .or_else(|| chapter.title.clone())
        .unwrap_or_else(|| format!("Chapter {}", chapter.sequence));
    let title = blog_metadata
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback_title)
        .to_string();
    let slug = blog_metadata
        .get("slug")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| simple_slug(&title));
    let provider = settings.blog_provider.as_str();
    let metadata = serde_json::json!({
        "publication_metadata": blog_metadata,
        "publication_interface": {
            "target": "blog",
            "provider": provider,
            "provider_kind": "local_draft",
            "external_publish_ready": false
        }
    })
    .to_string();

    db::blog_posts::create_blog_post_with_metadata(
        db,
        &chapter.project_id,
        chapter_id,
        provider,
        &title,
        &slug,
        None,
        &metadata,
    )
}

pub fn project_is_running(
    db: &Database,
    memory_running: bool,
    project_id: &str,
) -> Result<bool, String> {
    if memory_running {
        return Ok(true);
    }
    db::generation_jobs::is_job_running(db, project_id)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = app.emit_to(
            "main",
            "app-resume",
            serde_json::json!({
                "reason": "tray_restore",
                "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            }),
        );
    }
}

fn get_api_key_fallback(state: &AppState, provider: &str) -> Result<String, String> {
    // Try keychain first, then SQLite fallback
    if let Ok(key) = keychain::get_api_key(provider) {
        return Ok(key);
    }
    // Fallback: read from system_settings table
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let key_name = format!("api_key_{}", provider);
    let result = conn.query_row(
        "SELECT value FROM system_settings WHERE key = ?1 AND status = 'active'",
        rusqlite::params![key_name],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(v) => {
            let key = v.trim_matches('"').to_string();
            if key.is_empty() {
                Err(format!(
                    "No API key configured for {}. Go to Settings > Model Provider.",
                    provider
                ))
            } else {
                Ok(key)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(format!(
            "No API key configured for {}. Go to Settings > Model Provider to set it.",
            provider
        )),
        Err(e) => Err(format!("Cannot read API key: {}", e)),
    }
}

fn get_provider(state: &AppState) -> Result<Box<dyn ModelClient>, String> {
    let settings = db::settings::get_settings(&state.db)?;
    provider_config_for_workflow(state, &settings, "base")?.build()
}

fn selected_model_profile_id<'a>(settings: &'a AppSettings, workflow: &str) -> Option<&'a str> {
    match workflow {
        "draft" => settings.draft_model_profile_id.as_deref(),
        "review" => settings.review_model_profile_id.as_deref(),
        "repair" | "revise" => settings.repair_model_profile_id.as_deref(),
        "summarization" | "postprocess" => settings.summarization_model_profile_id.as_deref(),
        _ => None,
    }
}

pub(crate) fn provider_config_for_workflow(
    state: &AppState,
    settings: &AppSettings,
    workflow: &str,
) -> Result<ai::factory::ProviderConfig, String> {
    let profile = selected_model_profile_id(settings, workflow)
        .map(|profile_id| db::model_profiles::get_model_profile(&state.db, profile_id))
        .transpose()?;
    let provider_for_key = profile
        .as_ref()
        .map(|profile| profile.provider.as_str())
        .unwrap_or(&settings.provider);
    let api_key = get_api_key_fallback(state, provider_for_key)?;
    Ok(ai::factory::provider_config_for_model_profile(
        settings,
        profile.as_ref(),
        api_key,
    ))
}

fn embedding_provider_config(
    state: &AppState,
    settings: &AppSettings,
) -> Result<ai::factory::ProviderConfig, String> {
    if let Some(profile_id) = settings.embedding_model_profile_id.as_deref() {
        let profile = db::model_profiles::get_model_profile(&state.db, profile_id)?;
        let api_key = get_api_key_fallback(state, &format!("emb_{}", profile.provider))?;
        return Ok(ai::factory::ProviderConfig {
            provider_type: profile.provider,
            api_key,
            base_url: profile.base_url,
            model: profile.model.clone(),
            embedding_model: profile.model,
            timeout_secs: 600,
        });
    }

    let emb_provider = &settings.embedding_provider;

    if emb_provider == "none" {
        return Err("Embedding provider is 'none'. Configure an embedding provider in Settings for RAG support.".into());
    }

    let api_key = get_api_key_fallback(state, &format!("emb_{}", emb_provider))?;
    let base_url = if !settings.embedding_base_url.is_empty() {
        settings.embedding_base_url.clone()
    } else {
        match emb_provider.as_str() {
            "openai" => "https://api.openai.com/v1".into(),
            "zhipu" => "https://open.bigmodel.cn/api/paas/v4".into(),
            _ => "https://api.openai.com/v1".into(),
        }
    };

    Ok(ai::factory::ProviderConfig {
        provider_type: emb_provider.clone(),
        api_key,
        base_url,
        model: settings.embedding_model.clone(),
        embedding_model: settings.embedding_model.clone(),
        timeout_secs: 600,
    })
}

/// Get a dedicated embedding provider. Falls back to the main provider if no separate embedding config.
pub(crate) fn get_embedding_provider(state: &AppState) -> Result<Box<dyn ModelClient>, String> {
    let settings = db::settings::get_settings(&state.db)?;
    embedding_provider_config(state, &settings)?.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("provider-routing.db");
        let db = Database::open(&db_path).unwrap();
        db::run_migrations(&db).unwrap();
        (
            dir,
            AppState {
                db,
                logs: Mutex::new(Vec::new()),
                running: Mutex::new(false),
            },
        )
    }

    fn save_api_key(db: &Database, key_name: &str, value: &str) {
        db::settings::save_setting(db, key_name, &format!("\"{}\"", value)).unwrap();
    }

    fn profile_input(
        id: &str,
        provider: &str,
        model: &str,
        intended_use: &str,
    ) -> db::model_profiles::ModelProfileInput {
        db::model_profiles::ModelProfileInput {
            id: Some(id.to_string()),
            name: format!("{intended_use} profile"),
            provider: provider.to_string(),
            base_url: format!("https://{provider}.example.test/v1"),
            model: model.to_string(),
            context_window: 32000,
            supports_json: true,
            supports_streaming: true,
            supports_embeddings: intended_use == "embedding",
            input_cost_per_million: None,
            output_cost_per_million: None,
            intended_use: intended_use.to_string(),
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn workflow_provider_config_uses_profile_key_and_settings_fallback() {
        let (_dir, state) = setup_state();
        let mut settings = db::settings::get_settings(&state.db).unwrap();
        settings.provider = "unit_base_provider".to_string();
        settings.base_url = "https://base.example.test/v1".to_string();
        settings.model = "base-model".to_string();
        settings.draft_model_profile_id = Some("profile-draft".to_string());
        db::settings::save_settings(&state.db, &settings).unwrap();
        save_api_key(&state.db, "api_key_unit_base_provider", "base-key");
        save_api_key(&state.db, "api_key_unit_profile_provider", "profile-key");
        db::model_profiles::upsert_model_profile(
            &state.db,
            &profile_input(
                "profile-draft",
                "unit_profile_provider",
                "profile-draft-model",
                "draft",
            ),
        )
        .unwrap();

        let draft_config = provider_config_for_workflow(&state, &settings, "draft").unwrap();
        assert_eq!(draft_config.provider_type, "unit_profile_provider");
        assert_eq!(
            draft_config.base_url,
            "https://unit_profile_provider.example.test/v1"
        );
        assert_eq!(draft_config.model, "profile-draft-model");
        assert_eq!(draft_config.api_key, "profile-key");

        let repair_config = provider_config_for_workflow(&state, &settings, "repair").unwrap();
        assert_eq!(repair_config.provider_type, "unit_base_provider");
        assert_eq!(repair_config.base_url, "https://base.example.test/v1");
        assert_eq!(repair_config.model, "base-model");
        assert_eq!(repair_config.api_key, "base-key");
    }

    #[test]
    fn embedding_provider_config_uses_embedding_profile_and_embedding_key() {
        let (_dir, state) = setup_state();
        let mut settings = db::settings::get_settings(&state.db).unwrap();
        settings.embedding_provider = "unit_legacy_embedding_provider".to_string();
        settings.embedding_base_url = "https://legacy-emb.example.test/v1".to_string();
        settings.embedding_model = "legacy-embedding-model".to_string();
        settings.embedding_model_profile_id = Some("profile-embedding".to_string());
        db::settings::save_settings(&state.db, &settings).unwrap();
        save_api_key(
            &state.db,
            "api_key_emb_unit_embedding_provider",
            "embedding-key",
        );
        db::model_profiles::upsert_model_profile(
            &state.db,
            &profile_input(
                "profile-embedding",
                "unit_embedding_provider",
                "profile-embedding-model",
                "embedding",
            ),
        )
        .unwrap();

        let config = embedding_provider_config(&state, &settings).unwrap();
        assert_eq!(config.provider_type, "unit_embedding_provider");
        assert_eq!(
            config.base_url,
            "https://unit_embedding_provider.example.test/v1"
        );
        assert_eq!(config.model, "profile-embedding-model");
        assert_eq!(config.embedding_model, "profile-embedding-model");
        assert_eq!(config.api_key, "embedding-key");
    }
}

// ============================================================================
// Tauri Commands — Project Management
// ============================================================================

#[tauri::command]
async fn create_project(
    state: tauri::State<'_, AppState>,
    name: String,
    description: Option<String>,
    genre: Option<String>,
    sub_genre: Option<String>,
    target_audience: Option<String>,
    tone: Option<String>,
    style_profile_desc: Option<String>,
    target_total_words: Option<u32>,
    daily_target_words: Option<u32>,
) -> Result<Project, String> {
    let provider = get_provider(&state)?;
    let input = CreateProjectInput {
        name,
        description,
        genre,
        sub_genre,
        target_audience,
        tone,
        style_profile_desc,
        target_total_words,
        daily_target_words,
    };
    add_log(&state, &format!("Creating project: {}", input.name));

    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

    // Run bootstrap directly (not in a separate thread) — keeps things simple and reliable
    let result =
        workflow::novel_bootstrap::bootstrap_novel(&state.db, provider.as_ref(), &input, &log_tx)
            .await?;

    // Drain all logs
    while let Ok(msg) = log_rx.try_recv() {
        add_log(&state, &msg);
    }
    add_log(
        &state,
        &format!(
            "Project {} created with {} characters, {} chapters, {} lore",
            result.name,
            crate::db::bible::get_bible(&state.db, &result.id)
                .map(|b| b.characters.len().to_string())
                .unwrap_or("?".into()),
            crate::db::chapters::get_chapter_plans(&state.db, &result.id)
                .map(|p| p.len().to_string())
                .unwrap_or("?".into()),
            crate::db::bible::get_bible(&state.db, &result.id)
                .map(|b| b.world_lore.len().to_string())
                .unwrap_or("?".into()),
        ),
    );
    Ok(result)
}

#[tauri::command]
async fn get_projects(state: tauri::State<'_, AppState>) -> Result<Vec<ProjectStats>, String> {
    let projects = db::projects::list_projects(&state.db)?;
    let mut stats = Vec::new();
    for p in projects {
        if let Ok(s) = db::projects::get_project_stats(&state.db, &p.id) {
            stats.push(s);
        }
    }
    Ok(stats)
}

#[tauri::command]
async fn get_project(state: tauri::State<'_, AppState>, id: String) -> Result<Project, String> {
    db::projects::get_project(&state.db, &id)
}

#[tauri::command]
async fn delete_project(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    add_log(&state, &format!("Deleting project: {}", &id[..8]));
    let settings = db::settings::get_settings(&state.db)?;
    let cleanup_dirs =
        db::projects::project_paper_dirs_for_cleanup(&state.db, &id, &settings.data_dir)?;
    // DB deletion (FK cascade cleans all related rows)
    db::projects::delete_project(&state.db, &id)?;
    // Clean up files on disk
    for paper_dir in cleanup_dirs {
        if !std::path::Path::new(&paper_dir).exists() {
            continue;
        }
        if let Err(e) = std::fs::remove_dir_all(&paper_dir) {
            add_log(
                &state,
                &format!("Warning: could not delete paper dir {}: {}", paper_dir, e),
            );
        } else {
            add_log(&state, &format!("Deleted paper dir: {}", paper_dir));
        }
    }
    Ok(())
}

// ============================================================================
// Chapter Operations
// ============================================================================

#[tauri::command]
async fn generate_next_chapter(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    project_id: String,
    force: bool,
    operator_controls: Option<workflow::writing_context::OperatorControls>,
) -> Result<GenerationResult, String> {
    {
        let running = state.running.lock().unwrap();
        if *running {
            return Err("A generation job is already running. If this is stuck from a previous crash, restart the app (File > Quit, then reopen).".into());
        }
    }
    *state.running.lock().unwrap() = true;

    // Guard: ensure running is reset even if the pipeline panics
    struct RunningGuard<'a> {
        running: &'a Mutex<bool>,
    }
    impl Drop for RunningGuard<'_> {
        fn drop(&mut self) {
            *self.running.lock().unwrap() = false;
        }
    }
    let _guard = RunningGuard {
        running: &state.running,
    };

    let settings = db::settings::get_settings(&state.db)?;
    let draft_provider = provider_config_for_workflow(&state, &settings, "draft")?.build()?;
    let review_provider = provider_config_for_workflow(&state, &settings, "review")?.build()?;
    let repair_provider = provider_config_for_workflow(&state, &settings, "repair")?.build()?;
    let postprocess_provider =
        provider_config_for_workflow(&state, &settings, "summarization")?.build()?;
    let emb_provider = get_embedding_provider(&state).ok(); // Ok if configured, None if "none"
    add_log(
        &state,
        &format!(
            "Starting chapter generation for project {}",
            &project_id[..8]
        ),
    );

    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);
    let (event_tx, mut event_rx) = mpsc::channel::<PipelineEvent>(50);
    let app_for_events = app.clone();
    let event_relay = tokio::spawn(async move {
        while let Some(ev) = event_rx.recv().await {
            let _ = app_for_events.emit_to("main", "pipeline-step", &ev);
        }
    });
    let result = workflow::chapter_production::generate_next_chapter_with_stage_providers(
        &state.db,
        workflow::chapter_production::ChapterPipelineProviders {
            draft: draft_provider.as_ref(),
            review: review_provider.as_ref(),
            repair: repair_provider.as_ref(),
            postprocess: postprocess_provider.as_ref(),
        },
        emb_provider.as_ref().map(|p| p.as_ref()),
        &project_id,
        force,
        &log_tx,
        &event_tx,
        operator_controls,
    )
    .await;

    drop(event_tx);
    let _ = event_relay.await;

    while let Ok(msg) = log_rx.try_recv() {
        add_log(&state, &msg);
    }

    match result {
        Ok(r) => {
            add_log(&state, &r.message);
            Ok(r)
        }
        Err(e) => {
            let _ = db::generation_jobs::mark_latest_job_failed(&state.db, &project_id, &e);
            add_log(&state, &format!("Pipeline failed: {}", e));
            Err(e)
        }
    }
}

#[tauri::command]
async fn rebuild_vector_index(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    let bible_data = crate::db::bible::get_bible(&state.db, &project_id)?;

    let mut candidates: Vec<crate::db::vector_store::VectorIndexCandidate> = Vec::new();
    for c in &bible_data.characters {
        let content = format!(
            "角色: {}\n性格: {}\n动机: {}\n说话风格: {}\n外貌: {}\n背景: {}",
            c.name,
            c.personality.as_deref().unwrap_or_default(),
            c.motivation.as_deref().unwrap_or_default(),
            c.speech_style.as_deref().unwrap_or_default(),
            c.appearance.as_deref().unwrap_or_default(),
            c.backstory.as_deref().unwrap_or_default()
        );
        let title = content.chars().take(40).collect::<String>();
        candidates.push(crate::db::vector_store::VectorIndexCandidate::new(
            &c.id,
            "character",
            title,
            content,
            "{}",
        ));
    }
    for l in &bible_data.locations {
        let content = l.description.as_deref().unwrap_or_default().to_string();
        let title = content.chars().take(40).collect::<String>();
        candidates.push(crate::db::vector_store::VectorIndexCandidate::new(
            &l.id, "location", title, content, "{}",
        ));
    }
    for l in &bible_data.world_lore {
        let content = l.content.as_deref().unwrap_or_default().to_string();
        let title = content.chars().take(40).collect::<String>();
        candidates.push(crate::db::vector_store::VectorIndexCandidate::new(
            &l.id,
            "world_lore",
            title,
            content,
            "{}",
        ));
    }
    for r in &bible_data.canon_rules {
        let content = r.rule_text.as_deref().unwrap_or_default().to_string();
        let title = content.chars().take(40).collect::<String>();
        candidates.push(crate::db::vector_store::VectorIndexCandidate::new(
            &r.id,
            "canon_rule",
            title,
            content,
            "{}",
        ));
    }
    for pt in &bible_data.plot_threads {
        let content = pt.description.as_deref().unwrap_or_default().to_string();
        let title = content.chars().take(40).collect::<String>();
        candidates.push(crate::db::vector_store::VectorIndexCandidate::new(
            &pt.id,
            "plot_thread",
            title,
            content,
            "{}",
        ));
    }

    let settings = db::settings::get_settings(&state.db)?;
    let embedding_config = embedding_provider_config(&state, &settings)
        .map_err(|e| format!("Embedding provider is not configured: {}", e))?;
    let expected_embedding_metadata = crate::db::vector_store::VectorEmbeddingMetadata {
        provider: embedding_config.provider_type.clone(),
        model: embedding_config.embedding_model.clone(),
        kind: ai::client::EmbeddingInputKind::Document,
        dim: settings.embedding_dim,
    };

    let candidate_count = candidates.len();
    let pending = crate::db::vector_store::filter_vector_index_candidates_with_embedding_metadata(
        &state.db,
        &project_id,
        candidates,
        &expected_embedding_metadata,
    )?;
    let skipped = candidate_count.saturating_sub(pending.len());
    if pending.is_empty() {
        add_log(
            &state,
            &format!("Vector index already up to date: {} documents", skipped),
        );
        return Ok(format!(
            "Vector index already up to date with {} documents",
            skipped
        ));
    }

    let emb_provider = embedding_config
        .build()
        .map_err(|e| format!("Embedding provider is not configured: {}", e))?;
    let embed = emb_provider.as_ref();
    let contents: Vec<String> = pending
        .iter()
        .map(|candidate| candidate.content.clone())
        .collect();
    let embeddings = embed
        .embed_with_kind(&contents, ai::client::EmbeddingInputKind::Document)
        .await
        .map_err(|e| format!("Embed: {}", e))?;
    let mut inserted = 0;
    for (i, candidate) in pending.iter().enumerate() {
        if i < embeddings.len() {
            let embedding_metadata = crate::db::vector_store::VectorEmbeddingMetadata::new(
                &embedding_config.provider_type,
                &embedding_config.embedding_model,
                ai::client::EmbeddingInputKind::Document,
                &embeddings[i],
            );
            crate::db::vector_store::insert_vector_document_with_embedding_metadata(
                &state.db,
                &project_id,
                &candidate.source_type,
                Some(&candidate.source_id),
                &candidate.title,
                &candidate.content,
                &candidate.metadata,
                &embeddings[i],
                &embedding_metadata,
            )
            .ok();
            inserted += 1;
        }
    }
    add_log(
        &state,
        &format!(
            "Vector index rebuilt: {} documents embedded, {} unchanged skipped",
            inserted, skipped
        ),
    );
    Ok(format!(
        "Rebuilt vector index with {} documents ({} unchanged skipped)",
        inserted, skipped
    ))
}

#[tauri::command]
async fn learn_from_text(
    state: tauri::State<'_, AppState>,
    project_id: String,
    text: String,
    source_title: String,
    source_type: Option<String>,
) -> Result<Vec<LearningEntry>, String> {
    if project_id.is_empty() {
        return Err("No project selected. Select a project first.".into());
    }
    let stype = source_type.unwrap_or_else(|| "manual".into());
    let source_title = if source_title.trim().is_empty() {
        "User Input".into()
    } else {
        source_title
    };
    let provider = get_provider(&state)?;
    let entries = workflow::learning::extract_knowledge(
        provider.as_ref(),
        &text,
        &source_title,
        &stype,
        None,
    )
    .await?;
    persist_learning_entries(&state.db, &project_id, &entries)?;
    add_log(
        &state,
        &format!("Learned {} patterns from '{}'", entries.len(), source_title),
    );
    Ok(entries)
}

#[tauri::command]
async fn learn_from_file_text(
    state: tauri::State<'_, AppState>,
    project_id: String,
    file_name: String,
    byte_len: usize,
    text: String,
    source_title: Option<String>,
) -> Result<Vec<LearningEntry>, String> {
    if project_id.is_empty() {
        return Err("No project selected. Select a project first.".into());
    }

    let validated =
        workflow::learning_intake::validate_user_file_text(&file_name, byte_len, &text)?;
    let title = source_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&validated.source_title)
        .to_string();
    let provider = get_provider(&state)?;
    let entries = workflow::learning::extract_knowledge(
        provider.as_ref(),
        &validated.text,
        &title,
        "manual_file",
        None,
    )
    .await?;
    persist_learning_entries(&state.db, &project_id, &entries)?;
    add_log(
        &state,
        &format!("Learned {} patterns from file '{}'", entries.len(), title),
    );
    Ok(entries)
}

#[tauri::command]
async fn learn_from_url(
    state: tauri::State<'_, AppState>,
    project_id: String,
    url: String,
) -> Result<Vec<LearningEntry>, String> {
    if project_id.is_empty() {
        return Err("No project selected. Select a project first.".into());
    }

    let normalized_url = workflow::learning_intake::normalize_learning_url(&url)?;
    let html = fetch_url_text(normalized_url.clone()).await?;
    let text = workflow::learning_intake::extract_meaningful_text_from_html(&html)?;
    let source_title = workflow::learning_intake::extract_source_title(&normalized_url, &html);
    let provider = get_provider(&state)?;
    let entries = workflow::learning::extract_knowledge(
        provider.as_ref(),
        &text,
        &source_title,
        "web",
        Some(&normalized_url),
    )
    .await?;
    persist_learning_entries(&state.db, &project_id, &entries)?;
    add_log(
        &state,
        &format!(
            "Learned {} patterns from web source '{}'",
            entries.len(),
            source_title
        ),
    );
    Ok(entries)
}

fn persist_learning_entries(
    db: &Database,
    project_id: &str,
    entries: &[LearningEntry],
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for entry in entries {
        let id = Database::new_uuid();
        conn.execute(
            "INSERT OR IGNORE INTO learning_entries (id, project_id, source_type, source_url, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id,
                project_id,
                entry.source_type,
                entry.source_url.as_deref(),
                entry.source_title.as_deref(),
                entry.category,
                entry.pattern_name,
                entry.pattern_description,
                entry.example_text.as_deref(),
                entry.application_notes.as_deref(),
                entry.confidence
            ],
        ).map_err(|e| format!("Insert learning: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn fetch_url_text(url: String) -> Result<String, String> {
    let normalized_url = workflow::learning_intake::normalize_learning_url(&url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Client: {}", e))?;
    let resp = client
        .get(&normalized_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; AI-Novel-Factory/0.1)",
        )
        .header(
            "Accept",
            "text/html,application/xhtml+xml,text/plain;q=0.9,*/*;q=0.5",
        )
        .header(
            "Range",
            format!(
                "bytes=0-{}",
                workflow::learning_intake::MAX_SOURCE_BYTES.saturating_sub(1)
            ),
        )
        .send()
        .await
        .map_err(|e| format!("Fetch: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len > workflow::learning_intake::MAX_SOURCE_BYTES as u64 {
            return Err("Page too large (>1 MiB).".into());
        }
    }
    let html = resp.text().await.map_err(|e| format!("Read: {}", e))?;
    if html.len() > workflow::learning_intake::MAX_SOURCE_BYTES {
        return Err("Page too large (>1 MiB).".into());
    }
    Ok(html)
}

#[tauri::command]
async fn get_learning_entries(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<LearningEntry>, String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_type, source_url, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence, usage_count, last_used_at, metadata, created_at, updated_at FROM learning_entries WHERE project_id = ?1 ORDER BY created_at DESC"
    ).map_err(|e| format!("Prepare: {}", e))?;
    let entries = stmt
        .query_map(rusqlite::params![project_id], |row| {
            Ok(LearningEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_type: row.get(2)?,
                source_url: row.get(3)?,
                source_title: row.get(4)?,
                category: row.get(5)?,
                pattern_name: row.get(6)?,
                pattern_description: row.get(7)?,
                example_text: row.get(8)?,
                application_notes: row.get(9)?,
                confidence: row.get(10)?,
                usage_count: row.get(11)?,
                last_used_at: row.get(12)?,
                metadata: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect: {}", e))?;
    Ok(entries)
}

#[tauri::command]
async fn delete_learning_entry(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "DELETE FROM learning_entries WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("Delete: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn reset_running(
    state: tauri::State<'_, AppState>,
    project_id: Option<String>,
) -> Result<(), String> {
    let recovered = match project_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(project_id) => db::generation_jobs::recover_project_interrupted_jobs(
            &state.db,
            project_id,
            "operator reset stuck job",
        )?,
        None => db::generation_jobs::recover_interrupted_generation_jobs(
            &state.db,
            0,
            "operator reset stuck job",
        )?,
    };
    *state.running.lock().unwrap() = false;
    add_log(
        &state,
        &format!(
            "Running flag reset; recovered {} interrupted job(s).",
            recovered
        ),
    );
    Ok(())
}

#[tauri::command]
async fn save_edited_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    title: String,
    body_markdown: String,
) -> Result<(), String> {
    let next_version = save_human_edited_chapter(&state.db, &chapter_id, &title, &body_markdown)?;
    add_log(
        &state,
        &format!(
            "Chapter {} edited by user (v{})",
            &chapter_id[..8],
            next_version
        ),
    );
    Ok(())
}

#[tauri::command]
async fn update_chapter_plan(
    state: tauri::State<'_, AppState>,
    id: String,
    title: String,
    outline: String,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE chapter_plans SET title = ?1, outline = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![title, outline, id],
    ).map_err(|e| format!("Update plan: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn update_bible_entry(
    state: tauri::State<'_, AppState>,
    table: String,
    id: String,
    data: String,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))?;

    match table.as_str() {
        "characters" => {
            conn.execute(
                "UPDATE characters SET name=?1, role=?2, personality=?3, motivation=?4, speech_style=?5, appearance=?6, backstory=?7, updated_at=datetime('now') WHERE id=?8",
                rusqlite::params![
                    parsed["name"].as_str().unwrap_or(""), parsed["role"].as_str(), parsed["personality"].as_str(),
                    parsed["motivation"].as_str(), parsed["speech_style"].as_str(), parsed["appearance"].as_str(),
                    parsed["backstory"].as_str(), id,
                ],
            ).map_err(|e| format!("Update character: {}", e))?;
        }
        "locations" => {
            conn.execute(
                "UPDATE locations SET name=?1, type=?2, description=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["type"].as_str(), parsed["description"].as_str(), id],
            ).map_err(|e| format!("Update location: {}", e))?;
        }
        "organizations" => {
            conn.execute(
                "UPDATE organizations SET name=?1, description=?2, goals=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["description"].as_str(), parsed["goals"].as_str(), id],
            ).map_err(|e| format!("Update org: {}", e))?;
        }
        "items" => {
            conn.execute(
                "UPDATE items SET name=?1, description=?2, abilities=?3, limitations=?4, updated_at=datetime('now') WHERE id=?5",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["description"].as_str(), parsed["abilities"].as_str(), parsed["limitations"].as_str(), id],
            ).map_err(|e| format!("Update item: {}", e))?;
        }
        "world_lore" => {
            conn.execute(
                "UPDATE world_lore SET title=?1, lore_type=?2, content=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![parsed["title"].as_str().unwrap_or(""), parsed["lore_type"].as_str(), parsed["content"].as_str(), id],
            ).map_err(|e| format!("Update lore: {}", e))?;
        }
        "magic_systems" => {
            conn.execute(
                "UPDATE magic_or_power_systems SET name=?1, description=?2, rules=?3, limitations=?4, updated_at=datetime('now') WHERE id=?5",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["description"].as_str(), parsed["rules"].as_str(), parsed["limitations"].as_str(), id],
            ).map_err(|e| format!("Update magic: {}", e))?;
        }
        "canon_rules" => {
            conn.execute(
                "UPDATE canon_rules SET rule_type=?1, rule_text=?2, severity=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![parsed["rule_type"].as_str(), parsed["rule_text"].as_str(), parsed["severity"].as_str(), id],
            ).map_err(|e| format!("Update rule: {}", e))?;
        }
        "plot_threads" => {
            conn.execute(
                "UPDATE plot_threads SET name=?1, description=?2, priority=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["description"].as_str(), parsed["priority"].as_i64().unwrap_or(3), id],
            ).map_err(|e| format!("Update thread: {}", e))?;
        }
        "foreshadowing" => {
            conn.execute(
                "UPDATE foreshadowing SET clue_text=?1, intended_payoff=?2, updated_at=datetime('now') WHERE id=?3",
                rusqlite::params![parsed["clue_text"].as_str(), parsed["intended_payoff"].as_str(), id],
            ).map_err(|e| format!("Update foreshadowing: {}", e))?;
        }
        "style_guides" => {
            conn.execute(
                "UPDATE style_guides SET name=?1, style_text=?2, updated_at=datetime('now') WHERE id=?3",
                rusqlite::params![parsed["name"].as_str().unwrap_or(""), parsed["style_text"].as_str(), id],
            ).map_err(|e| format!("Update style: {}", e))?;
        }
        _ => return Err(format!("Unknown bible table: {}", table)),
    }
    Ok(())
}

#[tauri::command]
async fn retry_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<RevisionResult, String> {
    let settings = db::settings::get_settings(&state.db)?;
    let provider = provider_config_for_workflow(&state, &settings, "repair")?.build()?;
    add_log(&state, &format!("Retrying chapter: {}", &chapter_id[..8]));
    let result =
        workflow::review_repair::retry_chapter(&state.db, provider.as_ref(), &chapter_id).await?;
    add_log(&state, &result.message);
    Ok(result)
}

#[tauri::command]
async fn get_chapter_plans(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ChapterPlan>, String> {
    db::chapters::get_chapter_plans(&state.db, &project_id)
}

#[tauri::command]
async fn get_chapters(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Chapter>, String> {
    db::chapters::get_chapters(&state.db, &project_id)
}

#[tauri::command]
async fn get_chapter_versions(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<Vec<ChapterVersion>, String> {
    db::chapters::get_chapter_versions(&state.db, &chapter_id)
}

#[tauri::command]
async fn read_chapter_file(
    state: tauri::State<'_, AppState>,
    project_id: String,
    filename: String,
) -> Result<String, String> {
    let settings = db::settings::get_settings(&state.db)?;
    db::chapters::read_chapter_file_content(&state.db, &settings.data_dir, &project_id, &filename)
}

// ============================================================================
// Review Operations
// ============================================================================

#[tauri::command]
async fn get_agent_reviews(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<Vec<AgentReview>, String> {
    db::reviews::get_agent_reviews(&state.db, &chapter_id)
}

#[tauri::command]
async fn get_review_scores(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<Option<ReviewScores>, String> {
    db::reviews::get_review_scores(&state.db, &chapter_id)
}

#[tauri::command]
async fn get_project_quality_summary(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<ProjectQualitySummary, String> {
    db::reviews::get_project_quality_summary(&state.db, &project_id)
}

// ============================================================================
// Job Tracking
// ============================================================================

#[tauri::command]
async fn get_generation_jobs(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<GenerationJob>, String> {
    db::generation_jobs::get_generation_jobs(&state.db, &project_id)
}

// ============================================================================
// Weekly Planner
// ============================================================================

#[tauri::command]
async fn run_weekly_arc_planner(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<WeeklyPlanResult, String> {
    let provider = get_provider(&state)?;
    add_log(&state, "Starting weekly arc planner...");
    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);
    let result = workflow::weekly_planner::run_weekly_arc_planner(
        &state.db,
        provider.as_ref(),
        &project_id,
        &log_tx,
    )
    .await?;
    while let Ok(msg) = log_rx.try_recv() {
        add_log(&state, &msg);
    }
    add_log(&state, &result.message);
    Ok(result)
}

// ============================================================================
// Bible / Canon
// ============================================================================

#[tauri::command]
async fn get_bible(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<BibleData, String> {
    db::bible::get_bible(&state.db, &project_id)
}

#[tauri::command]
async fn ingest_bible_note(
    state: tauri::State<'_, AppState>,
    project_id: String,
    note: String,
) -> Result<(), String> {
    let provider = get_provider(&state)?;
    workflow::bible_ingestion::ingest_bible_note(&state.db, provider.as_ref(), &project_id, &note)
        .await
}

#[tauri::command]
async fn update_canon_rule(
    state: tauri::State<'_, AppState>,
    rule_id: String,
    locked: bool,
) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE canon_rules SET locked = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![locked as i32, rule_id],
    )
    .map_err(|e| format!("Update: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_knowledge_graph(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<db::knowledge_graph::KnowledgeGraphSnapshot, String> {
    db::knowledge_graph::get_snapshot(&state.db, &project_id)
}

#[tauri::command]
async fn get_knowledge_graph_neighborhood(
    state: tauri::State<'_, AppState>,
    project_id: String,
    node_id: String,
    node_type: String,
) -> Result<db::knowledge_graph::KnowledgeGraphNeighborhood, String> {
    db::knowledge_graph::get_node_neighborhood(&state.db, &project_id, &node_id, &node_type)
}

#[tauri::command]
async fn create_knowledge_graph_edge(
    state: tauri::State<'_, AppState>,
    project_id: String,
    source_id: String,
    source_type: String,
    target_id: String,
    target_type: String,
    edge_type: String,
    description: Option<String>,
) -> Result<db::knowledge_graph::KnowledgeGraphEdge, String> {
    db::knowledge_graph::create_edge(
        &state.db,
        &project_id,
        &source_id,
        &source_type,
        &target_id,
        &target_type,
        &edge_type,
        description.as_deref(),
    )
}

#[tauri::command]
async fn delete_knowledge_graph_edge(
    state: tauri::State<'_, AppState>,
    edge_id: String,
) -> Result<(), String> {
    db::knowledge_graph::delete_edge(&state.db, &edge_id)
}

// ============================================================================
// Settings
// ============================================================================

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    db::settings::get_settings(&state.db)
}

#[tauri::command]
async fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    db::settings::save_settings(&state.db, &settings)
}

#[tauri::command]
async fn set_api_key(
    state: tauri::State<'_, AppState>,
    provider: String,
    key: String,
) -> Result<(), String> {
    // Try keychain first (best effort)
    let _ = keychain::store_api_key(&provider, &key);
    // Always save to SQLite as fallback
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let key_name = format!("api_key_{}", provider);
    let value = format!("\"{}\"", key);
    conn.execute(
        "INSERT OR REPLACE INTO system_settings (id, key, value, status, updated_at)
         VALUES (COALESCE((SELECT id FROM system_settings WHERE key = ?1 AND project_id IS NULL), ?2), ?1, ?3, 'active', datetime('now'))",
        rusqlite::params![key_name, Database::new_uuid(), value],
    ).map_err(|e| format!("Save key to DB: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn test_embedding_provider(
    state: tauri::State<'_, AppState>,
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<TestResult, String> {
    // Save embedding settings
    let mut settings = db::settings::get_settings(&state.db)?;
    settings.embedding_provider = provider.clone();
    settings.embedding_base_url = base_url.clone();
    settings.embedding_model = model.clone();
    db::settings::save_settings(&state.db, &settings)?;
    // Store API key (uses "emb_{provider}" key to separate from main LLM key)
    let _ = keychain::store_api_key(&format!("emb_{}", provider), &api_key);
    // Save fallback to SQLite
    {
        let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let _ = conn.execute(
            "INSERT OR REPLACE INTO system_settings (id, key, value, status, updated_at) VALUES (COALESCE((SELECT id FROM system_settings WHERE key = ?1 AND project_id IS NULL), ?2), ?1, ?3, 'active', datetime('now'))",
            rusqlite::params![format!("api_key_emb_{}", provider), Database::new_uuid(), format!("\"{}\"", api_key)],
        );
    }
    // Test by calling embedding endpoint
    let client = ai::deepseek::DeepSeekProvider {
        api_key: api_key.clone(),
        base_url: base_url.clone(),
        model: model.clone(),
        embedding_model: model,
        timeout_secs: 600,
    };
    let start = std::time::Instant::now();
    let document_result = client
        .embed_with_kind(
            &["test document embedding".to_string()],
            ai::client::EmbeddingInputKind::Document,
        )
        .await;
    let query_result = client
        .embed_with_kind(
            &["test query embedding".to_string()],
            ai::client::EmbeddingInputKind::Query,
        )
        .await;
    match (document_result, query_result) {
        (Ok(document_vecs), Ok(query_vecs))
            if !document_vecs.is_empty() && !query_vecs.is_empty() =>
        {
            Ok(TestResult {
                ok: true,
                message: format!(
                    "OK — document {} dimensions, query {} dimensions, {}ms",
                    document_vecs[0].len(),
                    query_vecs[0].len(),
                    start.elapsed().as_millis()
                ),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            })
        }
        (Ok(_), Ok(_)) => Ok(TestResult {
            ok: false,
            message: "Empty response".into(),
            latency_ms: None,
        }),
        (Err(e), _) | (_, Err(e)) => Ok(TestResult {
            ok: false,
            message: format!("Failed: {}", e),
            latency_ms: None,
        }),
    }
}

#[tauri::command]
async fn test_model_provider(
    state: tauri::State<'_, AppState>,
    provider: String,
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<TestResult, String> {
    // Save provider settings to DB
    let mut settings = db::settings::get_settings(&state.db)?;
    settings.provider = provider.clone();
    if let Some(url) = base_url.clone() {
        settings.base_url = url;
    }
    if let Some(m) = model.clone() {
        settings.model = m;
    }
    db::settings::save_settings(&state.db, &settings)?;

    // Try to save to keychain (best-effort, don't fail if it doesn't work)
    let _ = keychain::store_api_key(&provider, &api_key);
    // Always save to SQLite as fallback (scoped so MutexGuard drops before await)
    {
        let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let key_name = format!("api_key_{}", provider);
        let _ = conn.execute(
            "INSERT OR REPLACE INTO system_settings (id, key, value, status, updated_at)
             VALUES (COALESCE((SELECT id FROM system_settings WHERE key = ?1 AND project_id IS NULL), ?2), ?1, ?3, 'active', datetime('now'))",
            rusqlite::params![key_name, Database::new_uuid(), format!("\"{}\"", api_key)],
        );
    }

    // Build provider via ProviderFactory (single source of truth)
    let client: Box<dyn ModelClient> = ai::factory::ProviderConfig {
        provider_type: provider.clone(),
        api_key: api_key.clone(),
        base_url: base_url.unwrap_or_else(|| match provider.as_str() {
            "deepseek" => "https://api.deepseek.com".into(),
            "kimi" => "https://api.moonshot.cn/v1".into(),
            "zhipu" => "https://open.bigmodel.cn/api/paas/v4".into(),
            "openai" => "https://api.openai.com/v1".into(),
            "anthropic" => "https://api.anthropic.com".into(),
            "gemini" => "https://generativelanguage.googleapis.com/v1beta".into(),
            _ => "https://api.openai.com/v1".into(),
        }),
        model: model.unwrap_or_else(|| match provider.as_str() {
            "deepseek" => "deepseek-v4-pro".into(),
            "kimi" => "moonshot-v1-8k".into(),
            "zhipu" => "glm-4-flash".into(),
            "openai" => "gpt-4o".into(),
            "anthropic" => "claude-sonnet-4-6".into(),
            "gemini" => "gemini-2.5-pro".into(),
            _ => "gpt-4o".into(),
        }),
        embedding_model: "text-embedding-3-small".into(),
        timeout_secs: 600,
    }
    .build()?;

    let start = std::time::Instant::now();
    match client
        .generate_text("You are a helpful assistant.", "Say 'OK' in one word.", 10)
        .await
    {
        Ok(text) => {
            // Successfully saved and tested — keychain save was best-effort above
            add_log(
                &state,
                &format!("Provider {} connected successfully", provider),
            );
            Ok(TestResult {
                ok: true,
                message: format!("Connection OK. Response: {}", text),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            })
        }
        Err(e) => Ok(TestResult {
            ok: false,
            message: format!("Connection failed: {}", e),
            latency_ms: None,
        }),
    }
}

// ============================================================================
// Export / Publish
// ============================================================================

#[tauri::command]
async fn export_markdown(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<String, String> {
    let settings = db::settings::get_settings(&state.db)?;
    export::markdown::export_chapter_markdown(&state.db, &chapter_id, &settings.data_dir)
}

#[tauri::command]
async fn publish_blog_draft(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<(), String> {
    create_local_blog_draft(&state.db, &chapter_id)?;
    add_log(
        &state,
        &format!("Blog draft created for chapter {}", &chapter_id[..8]),
    );
    Ok(())
}

// ============================================================================
// Logs + Status
// ============================================================================

#[tauri::command]
async fn get_logs(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    if let Ok(logs) = state.logs.lock() {
        Ok(logs.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn get_status(
    state: tauri::State<'_, AppState>,
    project_id: Option<String>,
) -> Result<StatusResponse, String> {
    let memory_running = *state.running.lock().unwrap();
    let id = match project_id {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => match db::projects::get_active_project(&state.db)? {
            Some(p) => p.id,
            None => {
                return Ok(StatusResponse {
                    ok: false,
                    novel: None,
                    slug: None,
                    chapter_count: None,
                    chapters_today: None,
                    plans_left: None,
                    total_words: None,
                    is_running: memory_running,
                    daily_schedule: None,
                })
            }
        },
    };

    let is_running = project_is_running(&state.db, memory_running, &id).unwrap_or(memory_running);

    if let Ok(stats) = db::projects::get_project_stats(&state.db, &id) {
        let project = db::projects::get_project(&state.db, &id).ok();
        Ok(StatusResponse {
            ok: true,
            novel: project.as_ref().map(|p| NovelBrief {
                name: p.name.clone(),
                genre: p.genre.clone(),
            }),
            slug: Some(stats.slug),
            chapter_count: Some(stats.chapter_count),
            chapters_today: Some(stats.chapters_today),
            plans_left: Some(stats.plans_left),
            total_words: Some(stats.total_words),
            is_running,
            daily_schedule: None,
        })
    } else {
        Ok(StatusResponse {
            ok: false,
            novel: None,
            slug: None,
            chapter_count: None,
            chapters_today: None,
            plans_left: None,
            total_words: None,
            is_running,
            daily_schedule: None,
        })
    }
}

// ============================================================================
// App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Determine data directory
            let data_dir = dirs::document_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("AI-Novels");
            std::fs::create_dir_all(&data_dir).ok();

            let db_path = data_dir.join("ai-novel-factory.db");
            let db = Database::open(&db_path)?;
            db::run_migrations(&db)?;
            workflow::lock::cleanup_stale_locks(&db, 600);
            let _ = db::generation_jobs::recover_interrupted_generation_jobs(
                &db,
                600,
                "Application restarted while this generation job was still running.",
            );

            app.manage(AppState {
                db,
                logs: Mutex::new(Vec::new()),
                running: Mutex::new(false),
            });

            // Close button minimizes to system tray (quit via tray menu)
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = handle.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

            // System tray
            use tauri::{
                menu::{MenuBuilder, MenuItemBuilder},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
            };

            let open = MenuItemBuilder::with_id("open", "Open AI Novel Factory").build(app)?;
            let write = MenuItemBuilder::with_id("write", "Open Writing Console").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit Completely").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&open)
                .item(&write)
                .separator()
                .item(&quit)
                .build()?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("AI Novel Factory - close hides to tray; Quit Completely exits");

            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            let _tray = tray_builder
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        show_main_window(app);
                    }
                    "write" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            let _ = db::generation_jobs::recover_interrupted_generation_jobs(
                                &state.db,
                                0,
                                "Application quit before this generation job completed.",
                            );
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            get_projects,
            get_project,
            delete_project,
            generate_next_chapter,
            retry_chapter,
            get_chapter_plans,
            commands::runtime::get_next_chapter_context_preview,
            commands::runtime::get_rag_health,
            get_chapters,
            get_chapter_versions,
            read_chapter_file,
            get_agent_reviews,
            get_review_scores,
            get_project_quality_summary,
            get_generation_jobs,
            run_weekly_arc_planner,
            get_bible,
            ingest_bible_note,
            update_canon_rule,
            commands::runtime::get_context_rules,
            commands::runtime::upsert_context_rule,
            commands::runtime::import_sillytavern_lorebook,
            commands::runtime::export_novel_bible_package,
            commands::runtime::import_novel_bible_package,
            commands::runtime::export_project_package,
            commands::runtime::import_project_package,
            commands::runtime::upsert_prompt_preset,
            commands::runtime::upsert_prompt_unit,
            commands::runtime::list_prompt_presets,
            commands::runtime::get_prompt_preset_package,
            commands::runtime::import_prompt_preset_package,
            commands::runtime::upsert_model_profile,
            commands::runtime::get_model_profile,
            commands::runtime::list_model_profiles,
            commands::runtime::set_workflow_model_profile,
            commands::runtime::validate_model_profile,
            commands::runtime::get_builtin_operator_recipes,
            commands::runtime::run_operator_recipe,
            commands::runtime::get_draft_candidates,
            commands::runtime::select_draft_candidate,
            commands::runtime::validate_extension_manifest,
            commands::runtime::import_extension_package,
            commands::runtime::list_extension_packages,
            commands::runtime::set_extension_enabled,
            get_knowledge_graph,
            get_knowledge_graph_neighborhood,
            create_knowledge_graph_edge,
            delete_knowledge_graph_edge,
            get_settings,
            update_settings,
            set_api_key,
            test_model_provider,
            test_embedding_provider,
            export_markdown,
            publish_blog_draft,
            get_logs,
            get_status,
            reset_running,
            save_edited_chapter,
            update_chapter_plan,
            update_bible_entry,
            learn_from_text,
            learn_from_file_text,
            learn_from_url,
            get_learning_entries,
            fetch_url_text,
            delete_learning_entry,
            rebuild_vector_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
