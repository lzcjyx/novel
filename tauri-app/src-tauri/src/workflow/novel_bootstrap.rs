use crate::ai::client::{EmbeddingInputKind, ModelClient};
use crate::db::connection::Database;
use crate::db::{chapters, knowledge_graph, projects};
use crate::models::*;
use crate::prompts;
use crate::workflow::prompt_rendering;
use std::collections::HashMap;
use tokio::sync::mpsc;

fn log(log_tx: &mpsc::Sender<String>, msg: &str) {
    let _ = log_tx.try_send(format!(
        "[{}] {}",
        chrono::Local::now().format("%H:%M:%S"),
        msg
    ));
}

pub async fn bootstrap_novel(
    db: &Database,
    provider: &dyn ModelClient,
    input: &CreateProjectInput,
    log_tx: &mpsc::Sender<String>,
) -> Result<Project, String> {
    log(
        log_tx,
        &format!("=== Bootstrapping Novel: {} ===", input.name),
    );

    // 1. Create project
    let project = projects::create_project(
        db,
        &input.name,
        input.description.as_deref(),
        input.genre.as_deref(),
        input.sub_genre.as_deref(),
        input.target_audience.as_deref(),
        input.tone.as_deref(),
        input.style_profile_desc.as_deref(),
        input.target_total_words,
        input.daily_target_words,
    )?;
    log(log_tx, &format!("Project created: {}", &project.id[..8]));

    let project_id = project.id.clone();
    match bootstrap_after_project_created(db, provider, input, project, log_tx).await {
        Ok(project) => Ok(project),
        Err(reason) => Err(cleanup_partial_project(db, &project_id, reason)),
    }
}

async fn bootstrap_after_project_created(
    db: &Database,
    provider: &dyn ModelClient,
    input: &CreateProjectInput,
    project: Project,
    log_tx: &mpsc::Sender<String>,
) -> Result<Project, String> {
    // 2. Create paper directory
    let settings = crate::db::settings::get_settings(db)?;
    let slug = projects::slugify(&project.id);
    let dir = format!("{}/{}", settings.data_dir, slug);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Mkdir: {}", e))?;
    projects::set_project_paper_dir(db, &project.id, &dir)?;
    let project = projects::get_project(db, &project.id)?;
    log(log_tx, &format!("Paper directory: {}", dir));

    // 3. Build bible prompt
    let bible_prompt = prompts::load_prompt("bible_generation")?;
    let user_input = serde_json::json!({
        "project_name": input.name,
        "description": input.description,
        "genre": input.genre,
        "sub_genre": input.sub_genre,
        "target_audience": input.target_audience,
        "tone": input.tone,
        "style_description": input.style_profile_desc,
        "target_total_words": input.target_total_words,
        "daily_target_words": input.daily_target_words,
    });
    let vars = HashMap::from([(
        "PROJECT_INPUT_JSON",
        serde_json::to_string_pretty(&user_input).unwrap_or_default(),
    )]);
    let bible_prompt =
        prompt_rendering::render_prompt_strict("bible_generation", &bible_prompt, &vars)?;

    let bible_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "world_overview": {"type": "string"},
            "power_system": {"type": "object"},
            "main_plot_threads": {"type": "array"},
            "characters": {"type": "array"},
            "locations": {"type": "array"},
            "organizations": {"type": "array"},
            "items": {"type": "array"},
            "style_guide": {"type": "object"},
            "canon_rules": {"type": "array"},
            "chapter_plans": {"type": "array"}
        }
    });

    log(log_tx, "Generating bible via AI...");
    let bible = match provider
        .generate_json(
            &bible_prompt,
            "请根据 system prompt 中的项目信息生成小说圣经，只输出 JSON。",
            &bible_schema,
            32768,
        )
        .await
    {
        Ok(b) => {
            log(log_tx, "Bible generated");
            b
        }
        Err(e) => {
            log(log_tx, &format!("Bible generation failed: {}", e));
            return Err(format!("Bible generation failed: {}", e));
        }
    };

    // 4. Insert bible records
    insert_bible_records(db, &project.id, &bible, log_tx)?;

    // 5. Validate required bootstrap artifacts before returning success.
    validate_bootstrap_artifacts(db, &project.id)?;

    // 6. Build vector index for retrieval (use embedding provider if available)
    embed_and_index_bible(db, provider, &project.id, log_tx).await;

    log(log_tx, "=== Bootstrap complete ===");
    Ok(project)
}

fn cleanup_partial_project(db: &Database, project_id: &str, reason: String) -> String {
    let mut cleanup_errors = Vec::new();
    let cleanup_dirs = crate::db::settings::get_settings(db)
        .and_then(|settings| {
            projects::project_paper_dirs_for_cleanup(db, project_id, &settings.data_dir)
        })
        .unwrap_or_default();

    if let Err(e) = projects::delete_project(db, project_id) {
        cleanup_errors.push(format!("delete project: {}", e));
    }

    for dir in cleanup_dirs {
        if std::path::Path::new(&dir).exists() {
            if let Err(e) = std::fs::remove_dir_all(&dir) {
                cleanup_errors.push(format!("delete paper dir {}: {}", dir, e));
            }
        }
    }

    if cleanup_errors.is_empty() {
        reason
    } else {
        format!("{}; cleanup failed: {}", reason, cleanup_errors.join("; "))
    }
}

fn validate_bootstrap_artifacts(db: &Database, project_id: &str) -> Result<(), String> {
    let bible_data = crate::db::bible::get_bible(db, project_id)?;
    let plans = chapters::get_chapter_plans(db, project_id)?;
    let graph = knowledge_graph::get_snapshot(db, project_id)?;
    let mut missing = Vec::new();

    if bible_data.characters.len() < 6 {
        missing.push(format!("characters {} < 6", bible_data.characters.len()));
    }
    if bible_data.locations.len() < 4 {
        missing.push(format!("locations {} < 4", bible_data.locations.len()));
    }
    if bible_data.organizations.len() < 2 {
        missing.push(format!(
            "organizations {} < 2",
            bible_data.organizations.len()
        ));
    }
    if bible_data.world_lore.is_empty() {
        missing.push("world_lore 0 < 1".into());
    }
    if bible_data.magic_systems.is_empty() {
        missing.push("magic_systems 0 < 1".into());
    }
    if bible_data.canon_rules.len() < 5 {
        missing.push(format!("canon_rules {} < 5", bible_data.canon_rules.len()));
    }
    if bible_data.plot_threads.len() < 3 {
        missing.push(format!(
            "plot_threads {} < 3",
            bible_data.plot_threads.len()
        ));
    }
    if plans.len() != 10 {
        missing.push(format!("chapter_plans {} != 10", plans.len()));
    }
    if graph.nodes.is_empty() {
        missing.push("graph_nodes 0 < 1".into());
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Bootstrap validation failed: {}",
            missing.join(", ")
        ))
    }
}

pub async fn embed_and_index_bible(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    log_tx: &mpsc::Sender<String>,
) {
    let bible_data = match crate::db::bible::get_bible(db, project_id) {
        Ok(b) => b,
        Err(e) => {
            log(log_tx, &format!("Vector indexing skipped: {}", e));
            return;
        }
    };

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

    if candidates.is_empty() {
        return;
    }

    let candidate_count = candidates.len();
    let pending =
        match crate::db::vector_store::filter_vector_index_candidates(db, project_id, candidates) {
            Ok(pending) => pending,
            Err(e) => {
                log(log_tx, &format!("Vector indexing skipped: {}", e));
                return;
            }
        };
    let skipped = candidate_count.saturating_sub(pending.len());
    if pending.is_empty() {
        log(
            log_tx,
            &format!("Vector index: all {} documents up to date", skipped),
        );
        return;
    }

    let text_contents: Vec<String> = pending
        .iter()
        .map(|candidate| candidate.content.clone())
        .collect();
    match provider
        .embed_with_kind(&text_contents, EmbeddingInputKind::Document)
        .await
    {
        Ok(embeddings) => {
            let mut inserted = 0;
            for (i, candidate) in pending.iter().enumerate() {
                if i < embeddings.len() {
                    let _ = crate::db::vector_store::insert_vector_document(
                        db,
                        project_id,
                        &candidate.source_type,
                        Some(&candidate.source_id),
                        &candidate.title,
                        &candidate.content,
                        &candidate.metadata,
                        &embeddings[i],
                    );
                    inserted += 1;
                }
            }
            log(
                log_tx,
                &format!(
                    "Vector index: {} documents embedded, {} unchanged skipped",
                    inserted, skipped
                ),
            );
        }
        Err(e) => {
            log(log_tx, &format!("Vector indexing unavailable: {}", e));
        }
    }
}

fn insert_bible_records(
    db: &Database,
    project_id: &str,
    bible: &serde_json::Value,
    log_tx: &mpsc::Sender<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    // Style guide — serialize the full object into style_text
    if let Some(sg) = bible.get("style_guide") {
        let id = Database::new_uuid();
        let style_text = sg.to_string(); // entire JSON object as style_text
        let name = sg
            .get("tone")
            .and_then(|v| v.as_str())
            .map(|t| format!("{} Style", t))
            .unwrap_or_else(|| "Default Style Guide".into());
        conn.execute(
            "INSERT OR IGNORE INTO style_guides (id, project_id, name, style_text) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, project_id, name, style_text],
        )
        .map_err(|e| format!("Insert style guide: {}", e))?;
    }

    // Characters
    if let Some(arr) = bible["characters"].as_array() {
        for c in arr {
            let id = Database::new_uuid();
            let name = c["name"].as_str().unwrap_or("Unknown");
            let role = c.get("role").and_then(|v| v.as_str());
            let personality = c.get("personality").and_then(|v| v.as_str());
            let backstory = c.get("backstory").and_then(|v| v.as_str());
            conn.execute(
                "INSERT OR IGNORE INTO characters (id, project_id, name, role, personality, backstory) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![id, project_id, name, role, personality, backstory],
            )
            .map_err(|e| format!("Insert character: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} characters", arr.len()));
    }

    // Locations
    if let Some(arr) = bible["locations"].as_array() {
        for loc in arr {
            let id = Database::new_uuid();
            let name = loc["name"].as_str().unwrap_or("Unknown");
            let desc = loc.get("description").and_then(|v| v.as_str());
            let loc_type = loc.get("type").and_then(|v| v.as_str());
            conn.execute(
                "INSERT OR IGNORE INTO locations (id, project_id, name, description, type) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, project_id, name, desc, loc_type],
            )
            .map_err(|e| format!("Insert location: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} locations", arr.len()));
    }

    // Organizations
    if let Some(arr) = bible["organizations"].as_array() {
        for org in arr {
            let id = Database::new_uuid();
            let name = org["name"].as_str().unwrap_or("Unknown");
            let desc = org.get("description").and_then(|v| v.as_str());
            conn.execute(
                "INSERT OR IGNORE INTO organizations (id, project_id, name, description) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, project_id, name, desc],
            )
            .map_err(|e| format!("Insert organization: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} organizations", arr.len()));
    }

    // Items
    if let Some(arr) = bible["items"].as_array() {
        for item in arr {
            let id = Database::new_uuid();
            let name = item["name"].as_str().unwrap_or("Unknown");
            let desc = item.get("description").and_then(|v| v.as_str());
            conn.execute(
                "INSERT OR IGNORE INTO items (id, project_id, name, description) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, project_id, name, desc],
            )
            .map_err(|e| format!("Insert item: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} items", arr.len()));
    }

    // World Overview → world_lore (lore_type = "world_history")
    if let Some(world) = bible.get("world_overview").and_then(|v| v.as_str()) {
        let id = Database::new_uuid();
        conn.execute(
            "INSERT OR IGNORE INTO world_lore (id, project_id, lore_type, title, content) VALUES (?1, ?2, 'world_history', 'World Overview', ?3)",
            rusqlite::params![id, project_id, world],
        )
        .map_err(|e| format!("Insert world lore: {}", e))?;
    }

    // Power system → both magic_or_power_systems AND world_lore
    if let Some(ps) = bible.get("power_system") {
        if let Some(name) = ps.get("name").and_then(|v| v.as_str()) {
            let id = Database::new_uuid();
            let desc = ps.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let rules = ps.get("rules").and_then(|v| v.as_str()).unwrap_or("");
            let limits = ps.get("limitations").and_then(|v| v.as_str()).unwrap_or("");
            let progression = ps.get("progression").and_then(|v| v.as_str()).unwrap_or("");
            let full_desc = format!(
                "{}\n\nRules: {}\n\nLimitations: {}\n\nProgression: {}",
                desc, rules, limits, progression
            );
            conn.execute(
                "INSERT OR IGNORE INTO magic_or_power_systems (id, project_id, name, description, rules, limitations, progression) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![id, project_id, name, desc, rules, limits, progression],
            )
            .map_err(|e| format!("Insert magic system: {}", e))?;
            // Also insert as world_lore for vector search
            let lore_id = Database::new_uuid();
            conn.execute(
                "INSERT OR IGNORE INTO world_lore (id, project_id, lore_type, title, content) VALUES (?1, ?2, 'power_system', ?3, ?4)",
                rusqlite::params![lore_id, project_id, name, full_desc],
            )
            .map_err(|e| format!("Insert power system lore: {}", e))?;
        }
    }

    // Canon rules
    if let Some(arr) = bible["canon_rules"].as_array() {
        for rule in arr {
            let id = Database::new_uuid();
            let rule_type = rule.get("rule_type").and_then(|v| v.as_str()).unwrap_or("");
            let rule_text = rule.get("rule_text").and_then(|v| v.as_str()).unwrap_or("");
            let severity = rule
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("hard");
            conn.execute(
                "INSERT OR IGNORE INTO canon_rules (id, project_id, rule_type, rule_text, severity, locked) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                rusqlite::params![id, project_id, rule_type, rule_text, severity],
            )
            .map_err(|e| format!("Insert canon rule: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} canon rules", arr.len()));
    }

    // Plot threads
    if let Some(arr) = bible["main_plot_threads"].as_array() {
        for (_i, thread) in arr.iter().enumerate() {
            let id = Database::new_uuid();
            let name = thread
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            let desc = thread
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let priority = thread.get("priority").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            conn.execute(
                "INSERT OR IGNORE INTO plot_threads (id, project_id, name, description, priority, arc_status) VALUES (?1, ?2, ?3, ?4, ?5, 'open')",
                rusqlite::params![id, project_id, name, desc, priority],
            )
            .map_err(|e| format!("Insert plot thread: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} plot threads", arr.len()));
    }

    // Chapter plans (initial from bible)
    if let Some(arr) = bible["chapter_plans"].as_array() {
        for (i, plan) in arr.iter().enumerate() {
            let id = Database::new_uuid();
            let title = plan.get("title").and_then(|v| v.as_str());
            let outline = plan.get("outline").and_then(|v| v.as_str());
            let target_word_count = plan
                .get("target_word_count")
                .and_then(|v| v.as_i64())
                .map(|value| value as i32);
            conn.execute(
                "INSERT OR IGNORE INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'planned')",
                rusqlite::params![id, project_id, (i + 1) as i32, title, outline, target_word_count],
            )
            .map_err(|e| format!("Insert chapter plan: {}", e))?;
        }
        log(log_tx, &format!("Inserted {} chapter plans", arr.len()));
    }

    drop(conn);
    Ok(())
}
