use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleAssetInput {
    pub id: Option<String>,
    pub project_id: String,
    pub name: String,
    pub asset_type: String,
    pub scope_type: String,
    pub scope_id: Option<String>,
    pub features: serde_json::Value,
    pub positive_examples: Vec<String>,
    pub negative_examples: Vec<String>,
    pub anti_ai_rules: serde_json::Value,
    pub enabled: bool,
    pub priority: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleAsset {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub asset_type: String,
    pub scope_type: String,
    pub scope_id: Option<String>,
    pub features: serde_json::Value,
    pub positive_examples: Vec<String>,
    pub negative_examples: Vec<String>,
    pub anti_ai_rules: serde_json::Value,
    pub enabled: bool,
    pub priority: i32,
    pub metadata: serde_json::Value,
}

fn value_to_json(value: &serde_json::Value, label: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("Serialize {}: {}", label, e))
}

fn string_array_to_json(values: &[String], label: &str) -> Result<String, String> {
    let cleaned = values
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    serde_json::to_string(&cleaned).map_err(|e| format!("Serialize {}: {}", label, e))
}

fn parse_json(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn parse_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn row_to_style_asset(row: &rusqlite::Row<'_>) -> rusqlite::Result<StyleAsset> {
    Ok(StyleAsset {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        asset_type: row.get(3)?,
        scope_type: row.get(4)?,
        scope_id: row.get(5)?,
        features: parse_json(row.get(6)?),
        positive_examples: parse_string_array(row.get(7)?),
        negative_examples: parse_string_array(row.get(8)?),
        anti_ai_rules: parse_json(row.get(9)?),
        enabled: row.get::<_, i32>(10)? != 0,
        priority: row.get(11)?,
        metadata: parse_json(row.get(12)?),
    })
}

pub fn upsert_style_asset(db: &Database, input: &StyleAssetInput) -> Result<String, String> {
    if input.project_id.trim().is_empty() {
        return Err("Style asset project_id is required".to_string());
    }
    if input.name.trim().is_empty() {
        return Err("Style asset name is required".to_string());
    }
    if input.asset_type.trim().is_empty() {
        return Err("Style asset asset_type is required".to_string());
    }

    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let scope_type = if input.scope_type.trim().is_empty() {
        "project".to_string()
    } else {
        input.scope_type.trim().to_string()
    };
    let features = value_to_json(&input.features, "features")?;
    let positive_examples = string_array_to_json(&input.positive_examples, "positive_examples")?;
    let negative_examples = string_array_to_json(&input.negative_examples, "negative_examples")?;
    let anti_ai_rules = value_to_json(&input.anti_ai_rules, "anti_ai_rules")?;
    let metadata = value_to_json(&input.metadata, "metadata")?;

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO style_assets
            (id, project_id, name, asset_type, scope_type, scope_id, features,
             positive_examples, negative_examples, anti_ai_rules, enabled, priority,
             metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            name = excluded.name,
            asset_type = excluded.asset_type,
            scope_type = excluded.scope_type,
            scope_id = excluded.scope_id,
            features = excluded.features,
            positive_examples = excluded.positive_examples,
            negative_examples = excluded.negative_examples,
            anti_ai_rules = excluded.anti_ai_rules,
            enabled = excluded.enabled,
            priority = excluded.priority,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id.trim(),
            input.name.trim(),
            input.asset_type.trim(),
            scope_type,
            input.scope_id,
            features,
            positive_examples,
            negative_examples,
            anti_ai_rules,
            input.enabled as i32,
            input.priority,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert style asset: {}", e))?;
    Ok(id)
}

pub fn list_style_assets(
    db: &Database,
    project_id: &str,
    enabled_only: bool,
) -> Result<Vec<StyleAsset>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = if enabled_only {
        "SELECT id, project_id, name, asset_type, scope_type, scope_id, features,
                positive_examples, negative_examples, anti_ai_rules, enabled, priority, metadata
         FROM style_assets
         WHERE project_id = ?1 AND enabled = 1
         ORDER BY priority DESC, name ASC, id ASC"
    } else {
        "SELECT id, project_id, name, asset_type, scope_type, scope_id, features,
                positive_examples, negative_examples, anti_ai_rules, enabled, priority, metadata
         FROM style_assets
         WHERE project_id = ?1
         ORDER BY priority DESC, name ASC, id ASC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare style assets: {}", e))?;
    let assets = stmt
        .query_map(params![project_id], row_to_style_asset)
        .map_err(|e| format!("Query style assets: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect style assets: {}", e))?;
    Ok(assets)
}
