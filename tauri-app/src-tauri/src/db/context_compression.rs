use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCompressionSummaryInput {
    pub id: Option<String>,
    pub project_id: String,
    pub source_job_id: Option<String>,
    pub summary_text: String,
    pub prompt_hash: Option<String>,
    pub context_hash: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCompressionSummary {
    pub id: String,
    pub project_id: String,
    pub source_job_id: Option<String>,
    pub summary_text: String,
    pub prompt_hash: Option<String>,
    pub context_hash: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata)
        .map_err(|e| format!("Serialize context compression metadata: {}", e))
}

fn parse_metadata(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn validate_status(status: &str) -> Result<(), String> {
    match status {
        "draft" | "approved" | "rejected" => Ok(()),
        other => Err(format!(
            "Unsupported context compression status '{}'",
            other
        )),
    }
}

pub fn create_context_compression_summary(
    db: &Database,
    input: &ContextCompressionSummaryInput,
) -> Result<String, String> {
    if input.project_id.trim().is_empty() {
        return Err("context compression project_id is required".to_string());
    }
    if input.summary_text.trim().is_empty() {
        return Err("context compression summary_text is required".to_string());
    }
    validate_status(&input.status)?;
    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let metadata = metadata_to_string(&input.metadata)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO context_compression_summaries
            (id, project_id, source_job_id, summary_text, prompt_hash, context_hash, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            source_job_id = excluded.source_job_id,
            summary_text = excluded.summary_text,
            prompt_hash = excluded.prompt_hash,
            context_hash = excluded.context_hash,
            status = excluded.status,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id,
            input.source_job_id,
            input.summary_text,
            input.prompt_hash,
            input.context_hash,
            input.status,
            metadata
        ],
    )
    .map_err(|e| format!("Create context compression summary: {}", e))?;
    Ok(id)
}

pub fn set_context_compression_status(
    db: &Database,
    summary_id: &str,
    status: &str,
) -> Result<(), String> {
    validate_status(status)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let rows = conn
        .execute(
            "UPDATE context_compression_summaries
             SET status = ?1, updated_at = datetime('now')
             WHERE id = ?2",
            params![status, summary_id],
        )
        .map_err(|e| format!("Set context compression status: {}", e))?;
    if rows == 0 {
        return Err(format!(
            "Context compression summary '{}' was not found",
            summary_id
        ));
    }
    Ok(())
}

pub fn list_context_compression_summaries(
    db: &Database,
    project_id: &str,
    approved_only: bool,
) -> Result<Vec<ContextCompressionSummary>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = if approved_only {
        "SELECT id, project_id, source_job_id, summary_text, prompt_hash, context_hash, status, metadata, created_at, updated_at
         FROM context_compression_summaries
         WHERE project_id = ?1 AND status = 'approved'
         ORDER BY updated_at DESC, id ASC
         LIMIT 8"
    } else {
        "SELECT id, project_id, source_job_id, summary_text, prompt_hash, context_hash, status, metadata, created_at, updated_at
         FROM context_compression_summaries
         WHERE project_id = ?1
         ORDER BY updated_at DESC, id ASC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare context compression summaries: {}", e))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok(ContextCompressionSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_job_id: row.get(2)?,
                summary_text: row.get(3)?,
                prompt_hash: row.get(4)?,
                context_hash: row.get(5)?,
                status: row.get(6)?,
                metadata: parse_metadata(row.get(7)?),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Query context compression summaries: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect context compression summaries: {}", e))?;
    Ok(rows)
}
