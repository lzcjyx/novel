use crate::db::connection::Database;
use crate::models::generation_job::PublicationQueueItem;
use crate::models::PublicationQueueInput;
use rusqlite::{params, OptionalExtension};

pub fn upsert_pending_publication(
    db: &Database,
    input: &PublicationQueueInput,
) -> Result<String, String> {
    let metadata = serde_json::to_string(&input.metadata)
        .map_err(|e| format!("Serialize publication metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM publication_queue
             WHERE project_id = ?1 AND chapter_id = ?2 AND provider = ?3
               AND status IN ('pending','publishing','failed','needs_human_review')
             ORDER BY created_at DESC LIMIT 1",
            params![input.project_id, input.chapter_id, input.provider],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Find publication queue item: {}", e))?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE publication_queue
             SET chapter_version_id = ?1, status = 'pending', scheduled_at = ?2,
                 error_message = NULL, metadata = ?3, updated_at = datetime('now')
             WHERE id = ?4",
            params![input.chapter_version_id, input.scheduled_at, metadata, id],
        )
        .map_err(|e| format!("Update publication queue item: {}", e))?;
        return Ok(id);
    }

    let id = Database::new_uuid();
    conn.execute(
        "INSERT INTO publication_queue
         (id, project_id, chapter_id, chapter_version_id, provider, status, scheduled_at, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?7)",
        params![
            id,
            input.project_id,
            input.chapter_id,
            input.chapter_version_id,
            input.provider,
            input.scheduled_at,
            metadata
        ],
    )
    .map_err(|e| format!("Insert publication queue item: {}", e))?;
    Ok(id)
}

pub fn claim_publication(db: &Database, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let changed = conn
        .execute(
            "UPDATE publication_queue
             SET status = 'publishing', error_message = NULL, updated_at = datetime('now')
             WHERE id = ?1 AND status = 'pending'",
            params![id],
        )
        .map_err(|e| format!("Claim publication queue item: {}", e))?;
    if changed == 0 {
        return Err(format!("Publication queue item is not pending: {}", id));
    }
    Ok(())
}

pub fn recover_interrupted_publications(db: &Database, reason: &str) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE publication_queue
         SET status = 'pending', error_message = ?1, updated_at = datetime('now')
         WHERE status = 'publishing'",
        params![reason],
    )
    .map_err(|e| format!("Recover interrupted publications: {}", e))
}

pub fn retry_publication(db: &Database, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let changed = conn
        .execute(
            "UPDATE publication_queue
             SET status = 'pending', error_message = NULL, updated_at = datetime('now')
             WHERE id = ?1 AND status IN ('failed','needs_human_review','cancelled')",
            params![id],
        )
        .map_err(|e| format!("Retry publication queue item: {}", e))?;
    if changed == 0 {
        return Err(format!("Publication queue item cannot be retried: {}", id));
    }
    Ok(())
}

pub fn mark_publication_published(
    db: &Database,
    id: &str,
    metadata: &serde_json::Value,
) -> Result<(), String> {
    let metadata = serde_json::to_string(metadata)
        .map_err(|e| format!("Serialize publication metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE publication_queue
         SET status = 'published', published_at = datetime('now'), error_message = NULL,
             metadata = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![metadata, id],
    )
    .map_err(|e| format!("Mark publication published: {}", e))?;
    Ok(())
}

pub fn mark_publication_failed(db: &Database, id: &str, error: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE publication_queue
         SET status = 'failed', error_message = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![error, id],
    )
    .map_err(|e| format!("Mark publication failed: {}", e))?;
    Ok(())
}

pub fn list_due_publications(
    db: &Database,
    now: &str,
) -> Result<Vec<PublicationQueueItem>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_id, chapter_version_id, provider, status,
                    scheduled_at, published_at, error_message, metadata, created_at, updated_at
             FROM publication_queue
             WHERE status = 'pending' AND (scheduled_at IS NULL OR scheduled_at <= ?1)
             ORDER BY COALESCE(scheduled_at, created_at), created_at",
        )
        .map_err(|e| format!("Prepare due publications: {}", e))?;
    let items = stmt
        .query_map(params![now], map_queue_item)
        .map_err(|e| format!("Query due publications: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect due publications: {}", e))?;
    Ok(items)
}

fn map_queue_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicationQueueItem> {
    Ok(PublicationQueueItem {
        id: row.get(0)?,
        project_id: row.get(1)?,
        chapter_id: row.get(2)?,
        chapter_version_id: row.get(3)?,
        provider: row.get(4)?,
        status: row.get(5)?,
        scheduled_at: row.get(6)?,
        published_at: row.get(7)?,
        error_message: row.get(8)?,
        metadata: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
