use rusqlite::params;
use crate::db::connection::Database;
use crate::models::{Project, ProjectStats};

pub fn create_project(db: &Database, name: &str, description: Option<&str>, genre: Option<&str>,
    sub_genre: Option<&str>, target_audience: Option<&str>, tone: Option<&str>,
    style_profile_desc: Option<&str>, total_target_words: Option<u32>, daily_target_words: Option<u32>,
) -> Result<Project, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    let genre_val = genre.unwrap_or("");
    let audience_val = target_audience.unwrap_or("");
    let tone_val = tone.unwrap_or("neutral");
    let style_desc = style_profile_desc.unwrap_or("");
    let style = serde_json::json!({
        "narrative_perspective": "第三人称",
        "tense": "过去时",
        "tone": tone_val,
        "description": style_desc,
        "forbidden_phrases": [],
        "preferred_techniques": []
    });

    let total_words = total_target_words.unwrap_or(500000) as i32;
    let daily_words = daily_target_words.unwrap_or(3000) as i32;
    let desc = description.unwrap_or("");
    let sub = sub_genre.unwrap_or("");

    conn.execute(
        "INSERT INTO projects (id, name, genre, target_audience, style_profile, total_target_words, daily_target_words, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
        params![id, name, genre_val, audience_val, style.to_string(), total_words, daily_words,
            serde_json::json!({"description": desc, "sub_genre": sub, "tone": tone_val}).to_string()],
    ).map_err(|e| format!("Insert project: {}", e))?;

    drop(conn);
    get_project(db, &id)
}

pub fn get_project(db: &Database, id: &str) -> Result<Project, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT id, name, genre, target_audience, style_profile, total_target_words, daily_target_words,
                auto_publish, quality_threshold, blog_provider, status, metadata, created_at, updated_at
         FROM projects WHERE id = ?1",
        params![id],
        |row| {
            Ok(Project {
                id: row.get(0)?, name: row.get(1)?, genre: row.get(2)?, target_audience: row.get(3)?,
                style_profile: row.get(4)?, total_target_words: row.get(5)?, daily_target_words: row.get(6)?,
                auto_publish: row.get::<_, i32>(7)? != 0, quality_threshold: row.get(8)?,
                blog_provider: row.get(9)?, status: row.get(10)?, metadata: row.get(11)?,
                created_at: row.get(12)?, updated_at: row.get(13)?,
            })
        },
    ).map_err(|e| format!("Get project: {}", e))
}

pub fn list_projects(db: &Database) -> Result<Vec<Project>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, name, genre, target_audience, style_profile, total_target_words, daily_target_words,
                auto_publish, quality_threshold, blog_provider, status, metadata, created_at, updated_at
         FROM projects ORDER BY created_at DESC"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?, name: row.get(1)?, genre: row.get(2)?, target_audience: row.get(3)?,
            style_profile: row.get(4)?, total_target_words: row.get(5)?, daily_target_words: row.get(6)?,
            auto_publish: row.get::<_, i32>(7)? != 0, quality_threshold: row.get(8)?,
            blog_provider: row.get(9)?, status: row.get(10)?, metadata: row.get(11)?,
            created_at: row.get(12)?, updated_at: row.get(13)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect: {}", e))?;

    Ok(projects)
}

pub fn get_project_stats(db: &Database, id: &str) -> Result<ProjectStats, String> {
    let proj = get_project(db, id)?;
    let slug = slugify(&proj.id);
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    let chapter_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM chapters WHERE project_id = ?1", params![id],
        |r| r.get(0),
    ).unwrap_or(0);

    let total_words: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM chapters WHERE project_id = ?1", params![id],
        |r| r.get(0),
    ).unwrap_or(0);

    let plans_left: i32 = conn.query_row(
        "SELECT COUNT(*) FROM chapter_plans WHERE project_id = ?1 AND status IN ('planned','in_progress')",
        params![id], |r| r.get(0),
    ).unwrap_or(0);

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let chapters_today: i32 = conn.query_row(
        "SELECT COUNT(*) FROM chapters WHERE project_id = ?1 AND date(created_at) = ?2",
        params![id, today], |r| r.get(0),
    ).unwrap_or(0);

    drop(conn);

    Ok(ProjectStats {
        id: proj.id, name: proj.name, slug, genre: proj.genre, status: proj.status,
        target_words: proj.total_target_words, chapter_count, total_words, plans_left,
        chapters_today, created_at: proj.created_at,
    })
}

pub fn delete_project(db: &Database, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    // FK cascade handles most tables: chapters, chapter_versions, generation_jobs, etc.
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| format!("Delete project: {}", e))?;
    Ok(())
}

pub fn get_active_project(db: &Database) -> Result<Option<Project>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let result = conn.query_row(
        "SELECT id, name, genre, target_audience, style_profile, total_target_words, daily_target_words,
                auto_publish, quality_threshold, blog_provider, status, metadata, created_at, updated_at
         FROM projects WHERE status = 'active' ORDER BY created_at DESC LIMIT 1",
        [],
        |row| {
            Ok(Project {
                id: row.get(0)?, name: row.get(1)?, genre: row.get(2)?, target_audience: row.get(3)?,
                style_profile: row.get(4)?, total_target_words: row.get(5)?, daily_target_words: row.get(6)?,
                auto_publish: row.get::<_, i32>(7)? != 0, quality_threshold: row.get(8)?,
                blog_provider: row.get(9)?, status: row.get(10)?, metadata: row.get(11)?,
                created_at: row.get(12)?, updated_at: row.get(13)?,
            })
        },
    );
    match result {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Get active project: {}", e)),
    }
}

pub fn slugify(project_id: &str) -> String {
    format!("novel-{}", &project_id[..8.min(project_id.len())])
}

pub fn paper_dir(data_dir: &str, project_id: &str) -> String {
    format!("{}/{}", data_dir, slugify(project_id))
}
