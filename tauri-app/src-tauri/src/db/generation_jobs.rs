use rusqlite::params;
use crate::db::connection::Database;
use crate::models::GenerationJob;

pub fn create_generation_job(db: &Database, project_id: &str, chapter_plan_id: &str) -> Result<String, String> {
    let id = Database::new_uuid();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR IGNORE INTO generation_jobs (id, project_id, chapter_plan_id, job_date, status)
         VALUES (?1, ?2, ?3, ?4, 'started')",
        params![id, project_id, chapter_plan_id, today],
    ).map_err(|e| format!("Create job: {}", e))?;
    Ok(id)
}

pub fn update_job_status(db: &Database, job_id: &str, status: &str, error_message: Option<&str>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let completed = if status == "completed" || status == "failed" || status == "needs_human_review" {
        Some(chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    } else { None };

    conn.execute(
        "UPDATE generation_jobs SET status = ?1, completed_at = ?2, error_message = ?3, updated_at = datetime('now') WHERE id = ?4",
        params![status, completed, error_message, job_id],
    ).map_err(|e| format!("Update job: {}", e))?;
    Ok(())
}

pub fn get_generation_jobs(db: &Database, project_id: &str) -> Result<Vec<GenerationJob>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, chapter_plan_id, job_date, status, started_at, completed_at,
                error_message, retry_count, metadata, created_at, updated_at
         FROM generation_jobs WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 100"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let jobs = stmt.query_map(params![project_id], |row| {
        Ok(GenerationJob {
            id: row.get(0)?, project_id: row.get(1)?, chapter_plan_id: row.get(2)?,
            job_date: row.get(3)?, status: row.get(4)?, started_at: row.get(5)?,
            completed_at: row.get(6)?, error_message: row.get(7)?, retry_count: row.get(8)?,
            metadata: row.get(9)?, created_at: row.get(10)?, updated_at: row.get(11)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| format!("Collect: {}", e))?;

    Ok(jobs)
}

pub fn get_today_chapter_count(db: &Database, project_id: &str) -> Result<i32, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM generation_jobs WHERE project_id = ?1 AND job_date = ?2 AND status = 'completed'",
        params![project_id, today],
        |r| r.get(0),
    ).unwrap_or(0);
    Ok(count)
}

pub fn is_job_running(db: &Database, project_id: &str) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM generation_jobs WHERE project_id = ?1 AND status IN ('started','draft_created','reviewing','revising','publishing')",
        params![project_id],
        |r| r.get(0),
    ).unwrap_or(0);
    Ok(count > 0)
}

/// Mark the most recent non-completed/non-failed job for a project as failed.
/// Used for error recovery — ensures stuck jobs don't show "reviewing" forever.
pub fn mark_latest_job_failed(db: &Database, project_id: &str, error: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE generation_jobs SET status = 'failed', error_message = ?1, completed_at = ?2, updated_at = datetime('now')
         WHERE id = (SELECT id FROM generation_jobs WHERE project_id = ?3 AND status NOT IN ('completed','failed','needs_human_review','skipped') ORDER BY started_at DESC LIMIT 1)",
        params![error, now, project_id],
    ).map_err(|e| format!("Mark job failed: {}", e))?;
    Ok(())
}
