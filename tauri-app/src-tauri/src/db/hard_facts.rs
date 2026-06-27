use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFactInput {
    pub id: Option<String>,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub chapter_version_id: Option<String>,
    pub fact_type: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub value_text: String,
    pub certainty: f64,
    pub source_quote: Option<String>,
    pub scope: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardFact {
    pub id: String,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub chapter_version_id: Option<String>,
    pub fact_type: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub value_text: String,
    pub certainty: f64,
    pub source_quote: Option<String>,
    pub scope: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata).map_err(|e| format!("Serialize hard fact metadata: {}", e))
}

fn parse_metadata(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn row_to_hard_fact(row: &rusqlite::Row<'_>) -> rusqlite::Result<HardFact> {
    Ok(HardFact {
        id: row.get(0)?,
        project_id: row.get(1)?,
        chapter_id: row.get(2)?,
        chapter_version_id: row.get(3)?,
        fact_type: row.get(4)?,
        subject: row.get(5)?,
        predicate: row.get(6)?,
        object: row.get(7)?,
        value_text: row.get(8)?,
        certainty: row.get(9)?,
        source_quote: row.get(10)?,
        scope: row.get(11)?,
        status: row.get(12)?,
        metadata: parse_metadata(row.get(13)?),
    })
}

pub fn upsert_hard_fact(db: &Database, input: &HardFactInput) -> Result<String, String> {
    if input.project_id.trim().is_empty() {
        return Err("Hard fact project_id is required".to_string());
    }
    if input.fact_type.trim().is_empty() {
        return Err("Hard fact fact_type is required".to_string());
    }
    if input.subject.trim().is_empty() {
        return Err("Hard fact subject is required".to_string());
    }
    if input.predicate.trim().is_empty() {
        return Err("Hard fact predicate is required".to_string());
    }
    if input.object.trim().is_empty() {
        return Err("Hard fact object is required".to_string());
    }

    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let metadata = metadata_to_string(&input.metadata)?;
    let scope = if input.scope.trim().is_empty() {
        "project".to_string()
    } else {
        input.scope.trim().to_string()
    };
    let status = if input.status.trim().is_empty() {
        "active".to_string()
    } else {
        input.status.trim().to_string()
    };

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO hard_facts
            (id, project_id, chapter_id, chapter_version_id, fact_type, subject, predicate,
             object, value_text, certainty, source_quote, scope, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            chapter_id = excluded.chapter_id,
            chapter_version_id = excluded.chapter_version_id,
            fact_type = excluded.fact_type,
            subject = excluded.subject,
            predicate = excluded.predicate,
            object = excluded.object,
            value_text = excluded.value_text,
            certainty = excluded.certainty,
            source_quote = excluded.source_quote,
            scope = excluded.scope,
            status = excluded.status,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id.trim(),
            input.chapter_id,
            input.chapter_version_id,
            input.fact_type.trim(),
            input.subject.trim(),
            input.predicate.trim(),
            input.object.trim(),
            input.value_text.trim(),
            input.certainty.clamp(0.0, 1.0),
            input.source_quote,
            scope,
            status,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert hard fact: {}", e))?;
    Ok(id)
}

pub fn list_hard_facts(
    db: &Database,
    project_id: &str,
    active_only: bool,
) -> Result<Vec<HardFact>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = if active_only {
        "SELECT id, project_id, chapter_id, chapter_version_id, fact_type, subject, predicate,
                object, value_text, certainty, source_quote, scope, status, metadata
         FROM hard_facts
         WHERE project_id = ?1 AND status = 'active'
         ORDER BY created_at ASC, id ASC"
    } else {
        "SELECT id, project_id, chapter_id, chapter_version_id, fact_type, subject, predicate,
                object, value_text, certainty, source_quote, scope, status, metadata
         FROM hard_facts
         WHERE project_id = ?1
         ORDER BY created_at ASC, id ASC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare hard facts: {}", e))?;
    let facts = stmt
        .query_map(params![project_id], row_to_hard_fact)
        .map_err(|e| format!("Query hard facts: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect hard facts: {}", e))?;
    Ok(facts)
}
