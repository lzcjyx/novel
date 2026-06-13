use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::connection::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LorebookImportSummary {
    pub imported_count: usize,
    pub skipped_count: usize,
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(item)) => {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn lorebook_entries(root: &Value) -> Vec<Value> {
    match root.get("entries") {
        Some(Value::Array(entries)) => entries.clone(),
        Some(Value::Object(entries)) => entries.values().cloned().collect(),
        _ => Vec::new(),
    }
}

pub fn import_sillytavern_lorebook(
    db: &Database,
    project_id: &str,
    raw_json: &str,
) -> Result<LorebookImportSummary, String> {
    let root = serde_json::from_str::<Value>(raw_json)
        .map_err(|e| format!("Parse SillyTavern lorebook JSON: {}", e))?;
    let lorebook_name = root
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("SillyTavern Lorebook");
    let entries = lorebook_entries(&root);
    let mut imported_count = 0usize;
    let mut skipped_count = 0usize;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    for entry in entries {
        if entry
            .get("disable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            skipped_count += 1;
            continue;
        }
        let primary_keywords = string_array(entry.get("key"));
        let content = entry
            .get("content")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if primary_keywords.is_empty() || content.is_empty() {
            skipped_count += 1;
            continue;
        }
        let secondary_keywords = string_array(entry.get("keysecondary"));
        let uid = entry
            .get("uid")
            .and_then(Value::as_i64)
            .map(|uid| uid.to_string())
            .unwrap_or_else(Database::new_uuid);
        let name = entry
            .get("comment")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("SillyTavern lorebook entry");
        let priority = entry
            .get("order")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let metadata = serde_json::json!({
            "original_format": "sillytavern_world_info",
            "lorebook_name": lorebook_name,
            "uid": uid,
        });

        conn.execute(
            "INSERT INTO context_rules
                (id, project_id, name, primary_keywords, secondary_keywords, priority,
                 token_budget, content, source_type, source_id, enabled, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 320, ?7, 'sillytavern_lorebook', ?8, 1, ?9)",
            rusqlite::params![
                Database::new_uuid(),
                project_id,
                name,
                serde_json::to_string(&primary_keywords)
                    .map_err(|e| format!("Serialize primary keywords: {}", e))?,
                serde_json::to_string(&secondary_keywords)
                    .map_err(|e| format!("Serialize secondary keywords: {}", e))?,
                priority,
                content,
                uid,
                serde_json::to_string(&metadata)
                    .map_err(|e| format!("Serialize lorebook metadata: {}", e))?,
            ],
        )
        .map_err(|e| format!("Insert lorebook context rule: {}", e))?;
        imported_count += 1;
    }

    Ok(LorebookImportSummary {
        imported_count,
        skipped_count,
    })
}
