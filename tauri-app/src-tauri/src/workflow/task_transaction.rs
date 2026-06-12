use crate::db::connection::Database;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskOwnedRows {
    pub chapters: Vec<String>,
    pub chapter_versions: Vec<String>,
    pub agent_reviews: Vec<String>,
    pub review_scores: Vec<String>,
    pub blog_posts: Vec<String>,
    pub publication_queue: Vec<String>,
    pub vector_document_metadata: Vec<String>,
    pub character_states: Vec<String>,
    pub timeline_events: Vec<String>,
    pub foreshadowing: Vec<String>,
    pub knowledge_graph_edges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntryUsageSnapshot {
    pub id: String,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTaskSnapshot {
    pub job_id: String,
    pub project_id: String,
    pub chapter_plan_id: String,
    pub plan_status_before: String,
    pub plan_metadata_before: String,
    pub started_at: String,
    pub owned_rows: TaskOwnedRows,
    #[serde(default)]
    pub learning_entry_usage_before: Vec<LearningEntryUsageSnapshot>,
}

fn now_timestamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn normalize_metadata(raw: &str) -> Value {
    let mut metadata = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata
}

impl TaskOwnedRows {
    fn ids_mut(&mut self, table: &str) -> Result<&mut Vec<String>, String> {
        match table {
            "chapters" => Ok(&mut self.chapters),
            "chapter_versions" => Ok(&mut self.chapter_versions),
            "agent_reviews" => Ok(&mut self.agent_reviews),
            "review_scores" => Ok(&mut self.review_scores),
            "blog_posts" => Ok(&mut self.blog_posts),
            "publication_queue" => Ok(&mut self.publication_queue),
            "vector_document_metadata" => Ok(&mut self.vector_document_metadata),
            "character_states" => Ok(&mut self.character_states),
            "timeline_events" => Ok(&mut self.timeline_events),
            "foreshadowing" => Ok(&mut self.foreshadowing),
            "knowledge_graph_edges" => Ok(&mut self.knowledge_graph_edges),
            _ => Err(format!("Unsupported task-owned table: {}", table)),
        }
    }

    fn ids(&self, table: &str) -> Result<&[String], String> {
        match table {
            "chapters" => Ok(&self.chapters),
            "chapter_versions" => Ok(&self.chapter_versions),
            "agent_reviews" => Ok(&self.agent_reviews),
            "review_scores" => Ok(&self.review_scores),
            "blog_posts" => Ok(&self.blog_posts),
            "publication_queue" => Ok(&self.publication_queue),
            "vector_document_metadata" => Ok(&self.vector_document_metadata),
            "character_states" => Ok(&self.character_states),
            "timeline_events" => Ok(&self.timeline_events),
            "foreshadowing" => Ok(&self.foreshadowing),
            "knowledge_graph_edges" => Ok(&self.knowledge_graph_edges),
            _ => Err(format!("Unsupported task-owned table: {}", table)),
        }
    }
}

fn metadata_with_failure(mut metadata: Value, reason: &str, completed_at: &str) -> Value {
    metadata["phase_summary"]["last_status"] = json!("failed");
    metadata["phase_summary"]["failure_reason"] = json!(reason);
    metadata["phase_summary"]["completed_at"] = json!(completed_at);
    metadata["phase_summary"]["updated_at"] = json!(completed_at);
    metadata
}

fn load_snapshot(metadata: &Value) -> Result<Option<GenerationTaskSnapshot>, String> {
    let Some(snapshot) = metadata.get("task_snapshot") else {
        return Ok(None);
    };
    serde_json::from_value::<GenerationTaskSnapshot>(snapshot.clone())
        .map(Some)
        .map_err(|e| format!("Parse task snapshot: {}", e))
}

pub fn begin_generation_task_snapshot(
    db: &Database,
    job_id: &str,
    project_id: &str,
    chapter_plan_id: &str,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (plan_status_before, plan_metadata_before): (String, String) = conn
        .query_row(
            "SELECT status, metadata FROM chapter_plans WHERE id = ?1 AND project_id = ?2",
            params![chapter_plan_id, project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Load plan snapshot: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let mut metadata = normalize_metadata(&metadata_raw);
    metadata["task_snapshot"] = serde_json::to_value(GenerationTaskSnapshot {
        job_id: job_id.to_string(),
        project_id: project_id.to_string(),
        chapter_plan_id: chapter_plan_id.to_string(),
        plan_status_before,
        plan_metadata_before,
        started_at: now_timestamp(),
        owned_rows: TaskOwnedRows::default(),
        learning_entry_usage_before: Vec::new(),
    })
    .map_err(|e| format!("Serialize task snapshot: {}", e))?;
    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("Serialize job metadata: {}", e))?;
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata_json, job_id],
    )
    .map_err(|e| format!("Store task snapshot: {}", e))?;
    Ok(())
}

pub fn record_learning_entry_usage_snapshot(
    db: &Database,
    job_id: &str,
    entry_ids: &[String],
) -> Result<(), String> {
    if entry_ids.is_empty() {
        return Ok(());
    }

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let mut metadata = normalize_metadata(&metadata_raw);
    let mut snapshot = load_snapshot(&metadata)?
        .ok_or_else(|| format!("Task snapshot missing for job {}", job_id))?;

    for entry_id in entry_ids {
        let entry_id = entry_id.trim();
        if entry_id.is_empty()
            || snapshot
                .learning_entry_usage_before
                .iter()
                .any(|entry| entry.id == entry_id)
        {
            continue;
        }
        let before = conn
            .query_row(
                "SELECT usage_count, last_used_at
                 FROM learning_entries
                 WHERE id = ?1 AND project_id = ?2",
                params![entry_id, snapshot.project_id],
                |row| {
                    Ok(LearningEntryUsageSnapshot {
                        id: entry_id.to_string(),
                        usage_count: row.get(0)?,
                        last_used_at: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("Load learning usage snapshot: {}", e))?;
        if let Some(before) = before {
            snapshot.learning_entry_usage_before.push(before);
        }
    }

    metadata["task_snapshot"] =
        serde_json::to_value(snapshot).map_err(|e| format!("Serialize snapshot: {}", e))?;
    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("Serialize job metadata: {}", e))?;
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata_json, job_id],
    )
    .map_err(|e| format!("Record learning usage snapshot: {}", e))?;
    Ok(())
}

pub fn record_task_owned_row(
    db: &Database,
    job_id: &str,
    table: &str,
    row_id: &str,
) -> Result<(), String> {
    let row_id = row_id.trim();
    if row_id.is_empty() {
        return Ok(());
    }
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let mut metadata = normalize_metadata(&metadata_raw);
    let mut snapshot = load_snapshot(&metadata)?
        .ok_or_else(|| format!("Task snapshot missing for job {}", job_id))?;
    let ids = snapshot.owned_rows.ids_mut(table)?;
    if !ids.iter().any(|id| id == row_id) {
        ids.push(row_id.to_string());
    }
    metadata["task_snapshot"] =
        serde_json::to_value(snapshot).map_err(|e| format!("Serialize snapshot: {}", e))?;
    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("Serialize job metadata: {}", e))?;
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata_json, job_id],
    )
    .map_err(|e| format!("Record task-owned row: {}", e))?;
    Ok(())
}

fn delete_ids(tx: &rusqlite::Transaction<'_>, table: &str, ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let sql = format!("DELETE FROM {} WHERE id = ?1", table);
    for id in ids {
        tx.execute(&sql, params![id])
            .map_err(|e| format!("Rollback delete {} {}: {}", table, id, e))?;
    }
    Ok(())
}

pub fn rollback_generation_task(db: &Database, job_id: &str, reason: &str) -> Result<bool, String> {
    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let metadata = normalize_metadata(&metadata_raw);
    let completed_at = now_timestamp();
    let Some(snapshot) = load_snapshot(&metadata)? else {
        let failed_metadata = metadata_with_failure(metadata, reason, &completed_at);
        let failed_json = serde_json::to_string(&failed_metadata)
            .map_err(|e| format!("Serialize failed metadata: {}", e))?;
        conn.execute(
            "UPDATE generation_jobs
             SET status = 'failed', completed_at = ?1, error_message = ?2, metadata = ?3, updated_at = datetime('now')
             WHERE id = ?4 AND status NOT IN ('completed','failed','needs_human_review','skipped')",
            params![completed_at, reason, failed_json, job_id],
        )
        .map_err(|e| format!("Mark job failed: {}", e))?;
        return Ok(true);
    };

    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin rollback transaction: {}", e))?;

    for table in [
        "agent_reviews",
        "review_scores",
        "blog_posts",
        "publication_queue",
        "vector_document_metadata",
        "knowledge_graph_edges",
        "character_states",
        "timeline_events",
        "foreshadowing",
        "chapter_versions",
        "chapters",
    ] {
        delete_ids(&tx, table, snapshot.owned_rows.ids(table)?)?;
    }

    tx.execute(
        "UPDATE chapter_plans
         SET status = ?1, metadata = ?2, updated_at = datetime('now')
         WHERE id = ?3 AND project_id = ?4",
        params![
            snapshot.plan_status_before,
            snapshot.plan_metadata_before,
            snapshot.chapter_plan_id,
            snapshot.project_id
        ],
    )
    .map_err(|e| format!("Restore chapter plan: {}", e))?;

    for entry in &snapshot.learning_entry_usage_before {
        tx.execute(
            "UPDATE learning_entries
             SET usage_count = ?1, last_used_at = ?2, updated_at = datetime('now')
             WHERE id = ?3 AND project_id = ?4",
            params![
                entry.usage_count,
                entry.last_used_at,
                entry.id,
                snapshot.project_id
            ],
        )
        .map_err(|e| format!("Restore learning entry usage {}: {}", entry.id, e))?;
    }

    tx.execute(
        "DELETE FROM advisory_locks WHERE holder = ?1",
        params![snapshot.project_id],
    )
    .map_err(|e| format!("Release advisory lock: {}", e))?;

    let failed_metadata = metadata_with_failure(metadata, reason, &completed_at);
    let failed_json = serde_json::to_string(&failed_metadata)
        .map_err(|e| format!("Serialize failed metadata: {}", e))?;
    let changed = tx
        .execute(
            "UPDATE generation_jobs
             SET status = 'failed', completed_at = ?1, error_message = ?2, metadata = ?3, updated_at = datetime('now')
             WHERE id = ?4 AND status NOT IN ('completed','failed','needs_human_review','skipped')",
            params![completed_at, reason, failed_json, job_id],
        )
        .map_err(|e| format!("Mark job failed: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Commit rollback transaction: {}", e))?;
    Ok(changed > 0)
}
