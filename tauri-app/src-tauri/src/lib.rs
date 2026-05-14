use std::sync::Mutex;
use tokio::sync::mpsc;
use tauri::{Manager, Emitter};

pub mod db;
pub mod models;
pub mod ai;
pub mod workflow;
pub mod vector;
pub mod prompts;
pub mod security;
pub mod export;

use db::connection::Database;
use ai::client::ModelClient;
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
        if len > 500 { logs.drain(0..len - 500); }
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
                Err(format!("No API key configured for {}. Go to Settings > Model Provider.", provider))
            } else {
                Ok(key)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(format!("No API key configured for {}. Go to Settings > Model Provider to set it.", provider))
        }
        Err(e) => Err(format!("Cannot read API key: {}", e)),
    }
}

fn get_provider(state: &AppState) -> Result<Box<dyn ModelClient>, String> {
    let settings = db::settings::get_settings(&state.db)?;
    let api_key = get_api_key_fallback(state, &settings.provider)?;
    ai::factory::ProviderConfig {
        provider_type: settings.provider, api_key, base_url: settings.base_url,
        model: settings.model, embedding_model: settings.embedding_model, timeout_secs: 600,
    }.build()
}

/// Get a dedicated embedding provider. Falls back to the main provider if no separate embedding config.
fn get_embedding_provider(state: &AppState) -> Result<Box<dyn ModelClient>, String> {
    let settings = db::settings::get_settings(&state.db)?;
    let emb_provider = &settings.embedding_provider;

    if emb_provider == "none" {
        return Err("Embedding provider is 'none'. Configure an embedding provider in Settings for RAG support.".into());
    }

    // Get the embedding provider's own API key (separate from main LLM key)
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

    ai::factory::ProviderConfig {
        provider_type: emb_provider.clone(), api_key, base_url,
        model: settings.embedding_model.clone(), embedding_model: settings.embedding_model.clone(), timeout_secs: 600,
    }.build()
}

// ============================================================================
// Tauri Commands — Project Management
// ============================================================================

#[tauri::command]
async fn create_project(state: tauri::State<'_, AppState>, name: String, description: Option<String>,
    genre: Option<String>, sub_genre: Option<String>, target_audience: Option<String>,
    tone: Option<String>, style_profile_desc: Option<String>,
    target_total_words: Option<u32>, daily_target_words: Option<u32>,
) -> Result<Project, String> {
    let provider = get_provider(&state)?;
    let input = CreateProjectInput { name, description, genre, sub_genre, target_audience, tone, style_profile_desc, target_total_words, daily_target_words };
    add_log(&state, &format!("Creating project: {}", input.name));

    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

    // Run bootstrap directly (not in a separate thread) — keeps things simple and reliable
    let result = workflow::novel_bootstrap::bootstrap_novel(&state.db, provider.as_ref(), &input, &log_tx).await?;

    // Drain all logs
    while let Ok(msg) = log_rx.try_recv() { add_log(&state, &msg); }
    add_log(&state, &format!("Project {} created with {} characters, {} chapters, {} lore",
        result.name,
        crate::db::bible::get_bible(&state.db, &result.id).map(|b| b.characters.len().to_string()).unwrap_or("?".into()),
        crate::db::chapters::get_chapter_plans(&state.db, &result.id).map(|p| p.len().to_string()).unwrap_or("?".into()),
        crate::db::bible::get_bible(&state.db, &result.id).map(|b| b.world_lore.len().to_string()).unwrap_or("?".into()),
    ));
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
    let slug = db::projects::slugify(&id);
    let paper_dir = format!("{}/{}", settings.data_dir, slug);
    // DB deletion (FK cascade cleans all related rows)
    db::projects::delete_project(&state.db, &id)?;
    // Clean up files on disk
    if std::path::Path::new(&paper_dir).exists() {
        if let Err(e) = std::fs::remove_dir_all(&paper_dir) {
            add_log(&state, &format!("Warning: could not delete paper dir {}: {}", paper_dir, e));
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
async fn generate_next_chapter(app: tauri::AppHandle, state: tauri::State<'_, AppState>, project_id: String, force: bool) -> Result<GenerationResult, String> {
    {
        let running = state.running.lock().unwrap();
        if *running {
            return Err("A generation job is already running. If this is stuck from a previous crash, restart the app (File > Quit, then reopen).".into());
        }
    }
    *state.running.lock().unwrap() = true;

    // Guard: ensure running is reset even if the pipeline panics
    struct RunningGuard<'a> { running: &'a Mutex<bool> }
    impl Drop for RunningGuard<'_> { fn drop(&mut self) { *self.running.lock().unwrap() = false; } }
    let _guard = RunningGuard { running: &state.running };

    let provider = get_provider(&state)?;
    let emb_provider = get_embedding_provider(&state).ok(); // Ok if configured, None if "none"
    add_log(&state, &format!("Starting chapter generation for project {}", &project_id[..8]));

    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);
    let (event_tx, mut event_rx) = mpsc::channel::<PipelineEvent>(50);
    let result = workflow::chapter_production::generate_next_chapter(&state.db, provider.as_ref(), emb_provider.as_ref().map(|p| p.as_ref()), &project_id, force, &log_tx, &event_tx).await;

    // Drain events — emit to frontend via Tauri
    while let Ok(ev) = event_rx.try_recv() { let _ = app.emit_to("main", "pipeline-step", &ev); }
    while let Ok(msg) = log_rx.try_recv() { add_log(&state, &msg); }

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
async fn learn_from_text(state: tauri::State<'_, AppState>, project_id: String, text: String, source_title: String) -> Result<Vec<LearningEntry>, String> {
    let provider = get_provider(&state)?;
    let entries = workflow::learning::extract_knowledge(provider.as_ref(), &text, &source_title, "manual", None).await?;
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for entry in &entries {
        let id = Database::new_uuid();
        conn.execute(
            "INSERT INTO learning_entries (id, project_id, source_type, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence)
             VALUES (?1, ?2, 'manual', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, project_id, source_title, entry.category, entry.pattern_name, entry.pattern_description, entry.example_text, entry.application_notes, entry.confidence],
        ).map_err(|e| format!("Insert learning: {}", e))?;
    }
    add_log(&state, &format!("Learned {} patterns from '{}'", entries.len(), source_title));
    Ok(entries)
}

#[tauri::command]
async fn learn_from_url(state: tauri::State<'_, AppState>, project_id: String, url: String) -> Result<Vec<LearningEntry>, String> {
    let provider = get_provider(&state)?;
    // Fetch with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().map_err(|e| format!("Client: {}", e))?;
    let resp = client.get(&url).send().await.map_err(|e| format!("Fetch failed: {}", e))?;
    let html = resp.text().await.map_err(|e| format!("Read: {}", e))?;

    // Strip HTML tags → raw text
    let raw = html.replace("<br>", "\n").replace("<p>", "\n").replace("</p>", "\n");
    let raw = regex::Regex::new(r"<[^>]*>").unwrap().replace_all(&raw, "");
    // Decode common HTML entities
    let raw = raw.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'");
    // Keep only lines that look like content (longer than 40 chars, exclude nav/scripts/ads)
    let lines: Vec<&str> = raw.lines()
        .map(|l| l.trim())
        .filter(|l| {
            l.len() > 40 &&
            !l.starts_with("function") && !l.starts_with("var ") && !l.starts_with("if(") &&
            !l.starts_with("<!--") && !l.starts_with("//") && !l.starts_with("}}") &&
            !l.to_lowercase().contains("cookie") && !l.to_lowercase().contains("subscribe")
        })
        .collect();
    let content = lines.join("\n");
    let text = content.chars().take(15000).collect::<String>();

    if text.len() < 200 {
        return Err("Could not extract meaningful content from this URL. Try a different page.".into());
    }

    let title = url.split('/').last().unwrap_or("web source");
    add_log(&state, &format!("Fetched {} chars from {}", text.len(), title));
    let entries = workflow::learning::extract_knowledge(provider.as_ref(), &text, title, "web", Some(&url)).await?;
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for entry in &entries {
        let id = Database::new_uuid();
        conn.execute(
            "INSERT INTO learning_entries (id, project_id, source_type, source_url, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence)
             VALUES (?1, ?2, 'web', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![id, project_id, url, title, entry.category, entry.pattern_name, entry.pattern_description, entry.example_text, entry.application_notes, entry.confidence],
        ).map_err(|e| format!("Insert learning: {}", e))?;
    }
    add_log(&state, &format!("Learned {} patterns from URL", entries.len()));
    Ok(entries)
}

#[tauri::command]
async fn get_learning_entries(state: tauri::State<'_, AppState>, project_id: String) -> Result<Vec<LearningEntry>, String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_type, source_url, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence, usage_count, last_used_at, metadata, created_at, updated_at FROM learning_entries WHERE project_id = ?1 ORDER BY created_at DESC"
    ).map_err(|e| format!("Prepare: {}", e))?;
    let entries = stmt.query_map(rusqlite::params![project_id], |row| Ok(LearningEntry {
        id: row.get(0)?, project_id: row.get(1)?, source_type: row.get(2)?, source_url: row.get(3)?,
        source_title: row.get(4)?, category: row.get(5)?, pattern_name: row.get(6)?,
        pattern_description: row.get(7)?, example_text: row.get(8)?, application_notes: row.get(9)?,
        confidence: row.get(10)?, usage_count: row.get(11)?, last_used_at: row.get(12)?,
        metadata: row.get(13)?, created_at: row.get(14)?, updated_at: row.get(15)?,
    })).map_err(|e| format!("Query: {}", e))?.collect::<Result<Vec<_>, _>>().map_err(|e| format!("Collect: {}", e))?;
    Ok(entries)
}

#[tauri::command]
async fn delete_learning_entry(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute("DELETE FROM learning_entries WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| format!("Delete: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn reset_running(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.running.lock().unwrap() = false;
    add_log(&state, "Running flag manually reset.");
    Ok(())
}

#[tauri::command]
async fn save_edited_chapter(state: tauri::State<'_, AppState>, chapter_id: String, title: String, body_markdown: String) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let chapter = crate::db::chapters::get_chapter(&state.db, &chapter_id)?;
    let last_version = crate::db::chapters::get_latest_version(&state.db, &chapter_id)?;
    let next_version = last_version.as_ref().map(|v| v.version_number + 1).unwrap_or(2);
    let version_id = Database::new_uuid();
    let word_count = body_markdown.len() as i32;
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count, created_by_agent)
         VALUES (?1, ?2, ?3, ?4, 'revised', ?5, ?6, ?7, 'human_editor')",
        rusqlite::params![version_id, chapter_id, chapter.project_id, next_version, title, body_markdown, word_count],
    ).map_err(|e| format!("Insert edited version: {}", e))?;
    conn.execute(
        "UPDATE chapters SET final_version_id = ?1, title = ?2, word_count = ?3, updated_at = datetime('now') WHERE id = ?4",
        rusqlite::params![version_id, title, word_count, chapter_id],
    ).map_err(|e| format!("Update chapter: {}", e))?;
    add_log(&state, &format!("Chapter {} edited by user (v{})", &chapter_id[..8], next_version));
    Ok(())
}

#[tauri::command]
async fn update_chapter_plan(state: tauri::State<'_, AppState>, id: String, title: String, outline: String) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE chapter_plans SET title = ?1, outline = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![title, outline, id],
    ).map_err(|e| format!("Update plan: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn update_bible_entry(state: tauri::State<'_, AppState>, table: String, id: String, data: String) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let parsed: serde_json::Value = serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))?;

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
async fn retry_chapter(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<RevisionResult, String> {
    let provider = get_provider(&state)?;
    add_log(&state, &format!("Retrying chapter: {}", &chapter_id[..8]));
    let result = workflow::review_repair::retry_chapter(&state.db, provider.as_ref(), &chapter_id).await?;
    add_log(&state, &result.message);
    Ok(result)
}

#[tauri::command]
async fn get_chapter_plans(state: tauri::State<'_, AppState>, project_id: String) -> Result<Vec<ChapterPlan>, String> {
    db::chapters::get_chapter_plans(&state.db, &project_id)
}

#[tauri::command]
async fn get_chapters(state: tauri::State<'_, AppState>, project_id: String) -> Result<Vec<Chapter>, String> {
    db::chapters::get_chapters(&state.db, &project_id)
}

#[tauri::command]
async fn get_chapter_versions(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<Vec<ChapterVersion>, String> {
    db::chapters::get_chapter_versions(&state.db, &chapter_id)
}

#[tauri::command]
async fn read_chapter_file(state: tauri::State<'_, AppState>, project_id: String, filename: String) -> Result<String, String> {
    let settings = db::settings::get_settings(&state.db)?;
    db::chapters::read_chapter_file_content(&settings.data_dir, &project_id, &filename)
}

// ============================================================================
// Review Operations
// ============================================================================

#[tauri::command]
async fn get_agent_reviews(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<Vec<AgentReview>, String> {
    db::reviews::get_agent_reviews(&state.db, &chapter_id)
}

#[tauri::command]
async fn get_review_scores(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<Option<ReviewScores>, String> {
    db::reviews::get_review_scores(&state.db, &chapter_id)
}

// ============================================================================
// Job Tracking
// ============================================================================

#[tauri::command]
async fn get_generation_jobs(state: tauri::State<'_, AppState>, project_id: String) -> Result<Vec<GenerationJob>, String> {
    db::generation_jobs::get_generation_jobs(&state.db, &project_id)
}

// ============================================================================
// Weekly Planner
// ============================================================================

#[tauri::command]
async fn run_weekly_arc_planner(state: tauri::State<'_, AppState>, project_id: String) -> Result<WeeklyPlanResult, String> {
    let provider = get_provider(&state)?;
    add_log(&state, "Starting weekly arc planner...");
    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);
    let result = workflow::weekly_planner::run_weekly_arc_planner(&state.db, provider.as_ref(), &project_id, &log_tx).await?;
    while let Ok(msg) = log_rx.try_recv() { add_log(&state, &msg); }
    add_log(&state, &result.message);
    Ok(result)
}

// ============================================================================
// Bible / Canon
// ============================================================================

#[tauri::command]
async fn get_bible(state: tauri::State<'_, AppState>, project_id: String) -> Result<BibleData, String> {
    db::bible::get_bible(&state.db, &project_id)
}

#[tauri::command]
async fn ingest_bible_note(state: tauri::State<'_, AppState>, project_id: String, note: String) -> Result<(), String> {
    let provider = get_provider(&state)?;
    workflow::bible_ingestion::ingest_bible_note(&state.db, provider.as_ref(), &project_id, &note).await
}

#[tauri::command]
async fn update_canon_rule(state: tauri::State<'_, AppState>, rule_id: String, locked: bool) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE canon_rules SET locked = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![locked as i32, rule_id],
    ).map_err(|e| format!("Update: {}", e))?;
    Ok(())
}

// ============================================================================
// Settings
// ============================================================================

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = db::settings::get_settings(&state.db)?;
    // Show masked key if available (try keychain first, then SQLite fallback)
    if let Ok(key) = get_api_key_fallback(&state, &settings.provider) {
        settings.model = format!("{} (key: {})", settings.model, keychain::mask_key(&key));
    }
    Ok(settings)
}

#[tauri::command]
async fn update_settings(state: tauri::State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let mut clean = settings.clone();
    // Don't save the masked key from display
    if clean.model.contains("(key:") {
        if let Some(pos) = clean.model.find(" (key:") { clean.model.truncate(pos); }
    }
    db::settings::save_settings(&state.db, &clean)
}

#[tauri::command]
async fn set_api_key(state: tauri::State<'_, AppState>, provider: String, key: String) -> Result<(), String> {
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
        api_key: api_key.clone(), base_url: base_url.clone(),
        model: model.clone(), embedding_model: model, timeout_secs: 600,
    };
    let start = std::time::Instant::now();
    match client.embed(&["test embedding".to_string()]).await {
        Ok(vecs) if !vecs.is_empty() => Ok(TestResult {
            ok: true,
            message: format!("OK — {} dimensions, {}ms", vecs[0].len(), start.elapsed().as_millis()),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        }),
        Ok(_) => Ok(TestResult { ok: false, message: "Empty response".into(), latency_ms: None }),
        Err(e) => Ok(TestResult { ok: false, message: format!("Failed: {}", e), latency_ms: None }),
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
    if let Some(url) = base_url.clone() { settings.base_url = url; }
    if let Some(m) = model.clone() { settings.model = m; }
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
        model: model.unwrap_or_else(|| "gpt-4o".into()),
        embedding_model: "text-embedding-3-small".into(),
        timeout_secs: 600,
    }.build()?;

    let start = std::time::Instant::now();
    match client.generate_text("You are a helpful assistant.", "Say 'OK' in one word.", 10).await {
        Ok(text) => {
            // Successfully saved and tested — keychain save was best-effort above
            add_log(&state, &format!("Provider {} connected successfully", provider));
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
async fn export_markdown(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<String, String> {
    let settings = db::settings::get_settings(&state.db)?;
    export::markdown::export_chapter_markdown(&state.db, &chapter_id, &settings.data_dir)
}

#[tauri::command]
async fn publish_blog_draft(state: tauri::State<'_, AppState>, chapter_id: String) -> Result<(), String> {
    let chapter = db::chapters::get_chapter(&state.db, &chapter_id)?;
    let version = db::chapters::get_latest_version(&state.db, &chapter_id)?
        .ok_or("No version found")?;
    let settings = db::settings::get_settings(&state.db)?;

    let title = version.title.unwrap_or_else(|| format!("Chapter {}", chapter.sequence));
    let slug = title.to_lowercase().replace(' ', "-");
    db::blog_posts::create_blog_post(&state.db, &chapter.project_id, &chapter_id, &settings.blog_provider, &title, &slug, None)?;
    add_log(&state, &format!("Blog draft created for chapter {}", &chapter_id[..8]));
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
async fn get_status(state: tauri::State<'_, AppState>, project_id: Option<String>) -> Result<StatusResponse, String> {
    let id = match project_id {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => {
            match db::projects::get_active_project(&state.db)? {
                Some(p) => p.id,
                None => return Ok(StatusResponse {
                    ok: false, novel: None, slug: None,
                    chapter_count: None, chapters_today: None, plans_left: None,
                    total_words: None, is_running: *state.running.lock().unwrap(),
                    daily_schedule: None,
                }),
            }
        }
    };

    if let Ok(stats) = db::projects::get_project_stats(&state.db, &id) {
        let project = db::projects::get_project(&state.db, &id).ok();
        Ok(StatusResponse {
            ok: true,
            novel: project.as_ref().map(|p| NovelBrief { name: p.name.clone(), genre: p.genre.clone() }),
            slug: Some(stats.slug),
            chapter_count: Some(stats.chapter_count),
            chapters_today: Some(stats.chapters_today),
            plans_left: Some(stats.plans_left),
            total_words: Some(stats.total_words),
            is_running: *state.running.lock().unwrap(),
            daily_schedule: None,
        })
    } else {
        Ok(StatusResponse {
            ok: false, novel: None, slug: None,
            chapter_count: None, chapters_today: None, plans_left: None,
            total_words: None, is_running: *state.running.lock().unwrap(),
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

            app.manage(AppState {
                db,
                logs: Mutex::new(Vec::new()),
                running: Mutex::new(false),
            });

            // System tray
            use tauri::{
                menu::{MenuBuilder, MenuItemBuilder},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
            };

            let open = MenuItemBuilder::with_id("open", "Open Panel").build(app)?;
            let write = MenuItemBuilder::with_id("write", "Write Chapter Now").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&open).item(&write).separator().item(&quit)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("AI Novel Factory")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show(); let _ = window.set_focus();
                        }
                    }
                    "write" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => { app.exit(0); }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show(); let _ = window.set_focus();
                        }
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
            get_chapters,
            get_chapter_versions,
            read_chapter_file,
            get_agent_reviews,
            get_review_scores,
            get_generation_jobs,
            run_weekly_arc_planner,
            get_bible,
            ingest_bible_note,
            update_canon_rule,
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
            learn_from_url,
            get_learning_entries,
            delete_learning_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
