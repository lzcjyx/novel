use rusqlite::params;
use crate::db::connection::Database;
use crate::models::{Chapter, ChapterVersion, ChapterPlan, ChapterFile};

pub fn get_chapters(db: &Database, project_id: &str) -> Result<Vec<Chapter>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, chapter_plan_id, sequence, title, final_version_id, status,
                word_count, summary, published_at, metadata, created_at, updated_at
         FROM chapters WHERE project_id = ?1 ORDER BY sequence"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let chapters = stmt.query_map(params![project_id], |row| {
        Ok(Chapter {
            id: row.get(0)?, project_id: row.get(1)?, chapter_plan_id: row.get(2)?,
            sequence: row.get(3)?, title: row.get(4)?, final_version_id: row.get(5)?,
            status: row.get(6)?, word_count: row.get(7)?, summary: row.get(8)?,
            published_at: row.get(9)?, metadata: row.get(10)?,
            created_at: row.get(11)?, updated_at: row.get(12)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect: {}", e))?;

    Ok(chapters)
}

pub fn get_chapter(db: &Database, id: &str) -> Result<Chapter, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT id, project_id, chapter_plan_id, sequence, title, final_version_id, status,
                word_count, summary, published_at, metadata, created_at, updated_at
         FROM chapters WHERE id = ?1",
        params![id],
        |row| Ok(Chapter {
            id: row.get(0)?, project_id: row.get(1)?, chapter_plan_id: row.get(2)?,
            sequence: row.get(3)?, title: row.get(4)?, final_version_id: row.get(5)?,
            status: row.get(6)?, word_count: row.get(7)?, summary: row.get(8)?,
            published_at: row.get(9)?, metadata: row.get(10)?,
            created_at: row.get(11)?, updated_at: row.get(12)?,
        }),
    ).map_err(|e| format!("Get chapter: {}", e))
}

pub fn get_chapter_versions(db: &Database, chapter_id: &str) -> Result<Vec<ChapterVersion>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, chapter_id, project_id, version_number, version_type, title, body_markdown,
                summary, word_count, model_provider, model_name, prompt_hash, context_hash,
                created_by_agent, metadata, created_at, updated_at
         FROM chapter_versions WHERE chapter_id = ?1 ORDER BY version_number DESC"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let versions = stmt.query_map(params![chapter_id], |row| {
        Ok(ChapterVersion {
            id: row.get(0)?, chapter_id: row.get(1)?, project_id: row.get(2)?,
            version_number: row.get(3)?, version_type: row.get(4)?, title: row.get(5)?,
            body_markdown: row.get(6)?, summary: row.get(7)?, word_count: row.get(8)?,
            model_provider: row.get(9)?, model_name: row.get(10)?, prompt_hash: row.get(11)?,
            context_hash: row.get(12)?, created_by_agent: row.get(13)?,
            metadata: row.get(14)?, created_at: row.get(15)?, updated_at: row.get(16)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect: {}", e))?;

    Ok(versions)
}

pub fn get_latest_version(db: &Database, chapter_id: &str) -> Result<Option<ChapterVersion>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let result = conn.query_row(
        "SELECT id, chapter_id, project_id, version_number, version_type, title, body_markdown,
                summary, word_count, model_provider, model_name, prompt_hash, context_hash,
                created_by_agent, metadata, created_at, updated_at
         FROM chapter_versions WHERE chapter_id = ?1 ORDER BY version_number DESC LIMIT 1",
        params![chapter_id],
        |row| Ok(ChapterVersion {
            id: row.get(0)?, chapter_id: row.get(1)?, project_id: row.get(2)?,
            version_number: row.get(3)?, version_type: row.get(4)?, title: row.get(5)?,
            body_markdown: row.get(6)?, summary: row.get(7)?, word_count: row.get(8)?,
            model_provider: row.get(9)?, model_name: row.get(10)?, prompt_hash: row.get(11)?,
            context_hash: row.get(12)?, created_by_agent: row.get(13)?,
            metadata: row.get(14)?, created_at: row.get(15)?, updated_at: row.get(16)?,
        }),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Get latest version: {}", e)),
    }
}

pub fn save_draft_version(
    db: &Database, project_id: &str, chapter_plan_id: &str, sequence: i32,
    title: &str, body_markdown: &str, word_count: i32, summary: &str,
    model_provider: &str, model_name: &str, prompt_hash: &str, context_hash: &str,
) -> Result<(String, String), String> {
    let chapter_id = Database::new_uuid();
    let version_id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    // 1. INSERT chapter first (FK: chapter_versions.chapter_id → chapters.id)
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, final_version_id, status, word_count, summary)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'draft', ?6, ?7)",
        params![chapter_id, project_id, chapter_plan_id, sequence, title, word_count, summary],
    ).map_err(|e| format!("Insert chapter: {}", e))?;

    // 2. INSERT chapter_version (now chapter exists, FK satisfied)
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count, model_provider, model_name, prompt_hash, context_hash, created_by_agent)
         VALUES (?1, ?2, ?3, 1, 'draft', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'draft_writer')",
        params![version_id, chapter_id, project_id, title, body_markdown, summary, word_count,
            model_provider, model_name, prompt_hash, context_hash],
    ).map_err(|e| format!("Insert version: {}", e))?;

    // 3. UPDATE chapter with final_version_id
    conn.execute(
        "UPDATE chapters SET final_version_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![version_id, chapter_id],
    ).map_err(|e| format!("Update chapter final_version: {}", e))?;

    // 4. UPDATE chapter_plan status
    conn.execute(
        "UPDATE chapter_plans SET status = 'in_progress', updated_at = datetime('now') WHERE id = ?1",
        params![chapter_plan_id],
    ).map_err(|e| format!("Update plan: {}", e))?;

    Ok((chapter_id, version_id))
}

pub fn update_chapter_after_revision(
    db: &Database, chapter_id: &str, _project_id: &str,
    version_id: &str, title: &str, _body_markdown: &str,
    word_count: i32, summary: &str, status: &str, _score: f64, _decision: &str,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE chapters SET title = ?1, final_version_id = ?2, status = ?3, word_count = ?4,
         summary = ?5, updated_at = datetime('now') WHERE id = ?6",
        params![title, version_id, status, word_count, summary, chapter_id],
    ).map_err(|e| format!("Update chapter: {}", e))?;
    Ok(())
}

pub fn get_chapter_plans(db: &Database, project_id: &str) -> Result<Vec<ChapterPlan>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, volume_id, sequence, title, outline, pov_character_id,
                target_word_count, required_characters, required_locations, plot_goals,
                required_foreshadowing, status, metadata, created_at, updated_at
         FROM chapter_plans WHERE project_id = ?1 ORDER BY sequence"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let plans = stmt.query_map(params![project_id], |row| {
        Ok(ChapterPlan {
            id: row.get(0)?, project_id: row.get(1)?, volume_id: row.get(2)?,
            sequence: row.get(3)?, title: row.get(4)?, outline: row.get(5)?,
            pov_character_id: row.get(6)?, target_word_count: row.get(7)?,
            required_characters: row.get(8)?, required_locations: row.get(9)?,
            plot_goals: row.get(10)?, required_foreshadowing: row.get(11)?,
            status: row.get(12)?, metadata: row.get(13)?,
            created_at: row.get(14)?, updated_at: row.get(15)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect: {}", e))?;

    Ok(plans)
}

pub fn get_next_chapter_plan(db: &Database, project_id: &str) -> Result<Option<ChapterPlan>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let result = conn.query_row(
        "SELECT id, project_id, volume_id, sequence, title, outline, pov_character_id,
                target_word_count, required_characters, required_locations, plot_goals,
                required_foreshadowing, status, metadata, created_at, updated_at
         FROM chapter_plans WHERE project_id = ?1 AND status = 'planned'
         ORDER BY sequence LIMIT 1",
        params![project_id],
        |row| Ok(ChapterPlan {
            id: row.get(0)?, project_id: row.get(1)?, volume_id: row.get(2)?,
            sequence: row.get(3)?, title: row.get(4)?, outline: row.get(5)?,
            pov_character_id: row.get(6)?, target_word_count: row.get(7)?,
            required_characters: row.get(8)?, required_locations: row.get(9)?,
            plot_goals: row.get(10)?, required_foreshadowing: row.get(11)?,
            status: row.get(12)?, metadata: row.get(13)?,
            created_at: row.get(14)?, updated_at: row.get(15)?,
        }),
    );
    match result {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Get next plan: {}", e)),
    }
}

pub fn list_chapter_files(_db: &Database, project_id: &str, data_dir: &str) -> Result<Vec<ChapterFile>, String> {
    let slug = crate::db::projects::slugify(project_id);
    let dir_path = format!("{}/{}", data_dir, slug);
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "md") { continue; }
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let metadata = match std::fs::metadata(&path) { Ok(m) => m, Err(_) => continue };

            let seq: u32 = filename
                .split("ch").last()
                .and_then(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
                .unwrap_or(0);

            let title = std::fs::read_to_string(&path).ok()
                .and_then(|c| c.lines().find(|l| l.starts_with("# ")).map(|l| l.trim_start_matches("# ").to_string()))
                .unwrap_or_else(|| filename.clone());

            let modified = metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);

            files.push(ChapterFile { filename, title, sequence: seq, size: metadata.len(), modified });
        }
    }
    files.sort_by_key(|c| c.sequence);
    Ok(files)
}

pub fn read_chapter_file_content(data_dir: &str, project_id: &str, filename: &str) -> Result<String, String> {
    let slug = crate::db::projects::slugify(project_id);
    let path = format!("{}/{}/{}", data_dir, slug, filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))
}
