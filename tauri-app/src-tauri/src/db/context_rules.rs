use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRuleInput {
    pub id: Option<String>,
    pub project_id: String,
    pub name: String,
    pub primary_keywords: Vec<String>,
    pub secondary_keywords: Vec<String>,
    pub entity_refs: Vec<String>,
    pub chapter_ranges: Vec<String>,
    pub priority: i32,
    pub token_budget: i32,
    pub sticky_chapters: i32,
    pub cooldown_chapters: i32,
    pub content: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRule {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub primary_keywords: Vec<String>,
    pub secondary_keywords: Vec<String>,
    pub entity_refs: Vec<String>,
    pub chapter_ranges: Vec<String>,
    pub priority: i32,
    pub token_budget: i32,
    pub sticky_chapters: i32,
    pub cooldown_chapters: i32,
    pub content: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

fn string_array_to_json(values: &[String]) -> Result<String, String> {
    let cleaned = values
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    serde_json::to_string(&cleaned).map_err(|e| format!("Serialize context rule array: {}", e))
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata).map_err(|e| format!("Serialize context rule metadata: {}", e))
}

fn parse_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_metadata(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn row_to_context_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContextRule> {
    Ok(ContextRule {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        primary_keywords: parse_string_array(row.get(3)?),
        secondary_keywords: parse_string_array(row.get(4)?),
        entity_refs: parse_string_array(row.get(5)?),
        chapter_ranges: parse_string_array(row.get(6)?),
        priority: row.get(7)?,
        token_budget: row.get(8)?,
        sticky_chapters: row.get(9)?,
        cooldown_chapters: row.get(10)?,
        content: row.get(11)?,
        source_type: row.get(12)?,
        source_id: row.get(13)?,
        enabled: row.get::<_, i32>(14)? != 0,
        metadata: parse_metadata(row.get(15)?),
    })
}

fn query_context_rules(
    db: &Database,
    project_id: &str,
    enabled_only: bool,
) -> Result<Vec<ContextRule>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = if enabled_only {
        "SELECT id, project_id, name, primary_keywords, secondary_keywords, entity_refs,
                chapter_ranges, priority, token_budget, sticky_chapters, cooldown_chapters,
                content, source_type, source_id, enabled, metadata
         FROM context_rules
         WHERE project_id = ?1 AND enabled = 1
         ORDER BY priority DESC, name ASC, id ASC"
    } else {
        "SELECT id, project_id, name, primary_keywords, secondary_keywords, entity_refs,
                chapter_ranges, priority, token_budget, sticky_chapters, cooldown_chapters,
                content, source_type, source_id, enabled, metadata
         FROM context_rules
         WHERE project_id = ?1
         ORDER BY priority DESC, name ASC, id ASC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare context rules: {}", e))?;

    let rules = stmt
        .query_map(params![project_id], row_to_context_rule)
        .map_err(|e| format!("Query context rules: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect context rules: {}", e))?;
    Ok(rules)
}

pub fn upsert_context_rule(db: &Database, input: ContextRuleInput) -> Result<String, String> {
    if input.project_id.trim().is_empty() {
        return Err("Context rule project_id is required".to_string());
    }
    if input.name.trim().is_empty() {
        return Err("Context rule name is required".to_string());
    }
    if input.content.trim().is_empty() {
        return Err("Context rule content is required".to_string());
    }

    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let primary_keywords = string_array_to_json(&input.primary_keywords)?;
    let secondary_keywords = string_array_to_json(&input.secondary_keywords)?;
    let entity_refs = string_array_to_json(&input.entity_refs)?;
    let chapter_ranges = string_array_to_json(&input.chapter_ranges)?;
    let metadata = metadata_to_string(&input.metadata)?;
    let source_type = if input.source_type.trim().is_empty() {
        "manual".to_string()
    } else {
        input.source_type.trim().to_string()
    };

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO context_rules
            (id, project_id, name, primary_keywords, secondary_keywords, entity_refs,
             chapter_ranges, priority, token_budget, sticky_chapters, cooldown_chapters,
             content, source_type, source_id, enabled, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            name = excluded.name,
            primary_keywords = excluded.primary_keywords,
            secondary_keywords = excluded.secondary_keywords,
            entity_refs = excluded.entity_refs,
            chapter_ranges = excluded.chapter_ranges,
            priority = excluded.priority,
            token_budget = excluded.token_budget,
            sticky_chapters = excluded.sticky_chapters,
            cooldown_chapters = excluded.cooldown_chapters,
            content = excluded.content,
            source_type = excluded.source_type,
            source_id = excluded.source_id,
            enabled = excluded.enabled,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id,
            input.name.trim(),
            primary_keywords,
            secondary_keywords,
            entity_refs,
            chapter_ranges,
            input.priority,
            input.token_budget.max(0),
            input.sticky_chapters.max(0),
            input.cooldown_chapters.max(0),
            input.content.trim(),
            source_type,
            input.source_id,
            input.enabled as i32,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert context rule: {}", e))?;
    Ok(id)
}

pub fn list_context_rules(db: &Database, project_id: &str) -> Result<Vec<ContextRule>, String> {
    query_context_rules(db, project_id, false)
}

pub fn list_enabled_context_rules(
    db: &Database,
    project_id: &str,
) -> Result<Vec<ContextRule>, String> {
    query_context_rules(db, project_id, true)
}
