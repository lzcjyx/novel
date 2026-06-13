use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub context_window: i32,
    pub supports_json: bool,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
    pub input_cost_per_million: Option<f64>,
    pub output_cost_per_million: Option<f64>,
    pub intended_use: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub context_window: i32,
    pub supports_json: bool,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
    pub input_cost_per_million: Option<f64>,
    pub output_cost_per_million: Option<f64>,
    pub intended_use: String,
    pub metadata: serde_json::Value,
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata).map_err(|e| format!("Serialize model profile metadata: {}", e))
}

fn parse_metadata(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

pub fn upsert_model_profile(db: &Database, input: &ModelProfileInput) -> Result<String, String> {
    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let metadata = metadata_to_string(&input.metadata)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO model_profiles
            (id, name, provider, base_url, model, context_window, supports_json,
             supports_streaming, supports_embeddings, input_cost_per_million,
             output_cost_per_million, intended_use, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'active', ?13, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            provider = excluded.provider,
            base_url = excluded.base_url,
            model = excluded.model,
            context_window = excluded.context_window,
            supports_json = excluded.supports_json,
            supports_streaming = excluded.supports_streaming,
            supports_embeddings = excluded.supports_embeddings,
            input_cost_per_million = excluded.input_cost_per_million,
            output_cost_per_million = excluded.output_cost_per_million,
            intended_use = excluded.intended_use,
            status = 'active',
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.name,
            input.provider,
            input.base_url,
            input.model,
            input.context_window,
            input.supports_json as i32,
            input.supports_streaming as i32,
            input.supports_embeddings as i32,
            input.input_cost_per_million,
            input.output_cost_per_million,
            input.intended_use,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert model profile: {}", e))?;
    Ok(id)
}

pub fn get_model_profile(db: &Database, profile_id: &str) -> Result<ModelProfile, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT id, name, provider, base_url, model, context_window, supports_json,
                supports_streaming, supports_embeddings, input_cost_per_million,
                output_cost_per_million, intended_use, metadata
         FROM model_profiles
         WHERE id = ?1 AND status = 'active'",
        params![profile_id],
        |row| {
            Ok(ModelProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                base_url: row.get(3)?,
                model: row.get(4)?,
                context_window: row.get(5)?,
                supports_json: row.get::<_, i32>(6)? != 0,
                supports_streaming: row.get::<_, i32>(7)? != 0,
                supports_embeddings: row.get::<_, i32>(8)? != 0,
                input_cost_per_million: row.get(9)?,
                output_cost_per_million: row.get(10)?,
                intended_use: row.get(11)?,
                metadata: parse_metadata(row.get(12)?),
            })
        },
    )
    .map_err(|e| format!("Get model profile: {}", e))
}

pub fn list_model_profiles(db: &Database) -> Result<Vec<ModelProfile>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, provider, base_url, model, context_window, supports_json,
                    supports_streaming, supports_embeddings, input_cost_per_million,
                    output_cost_per_million, intended_use, metadata
             FROM model_profiles
             WHERE status = 'active'
             ORDER BY name ASC, id ASC",
        )
        .map_err(|e| format!("Prepare model profiles: {}", e))?;
    let profiles = stmt
        .query_map([], |row| {
            Ok(ModelProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                base_url: row.get(3)?,
                model: row.get(4)?,
                context_window: row.get(5)?,
                supports_json: row.get::<_, i32>(6)? != 0,
                supports_streaming: row.get::<_, i32>(7)? != 0,
                supports_embeddings: row.get::<_, i32>(8)? != 0,
                input_cost_per_million: row.get(9)?,
                output_cost_per_million: row.get(10)?,
                intended_use: row.get(11)?,
                metadata: parse_metadata(row.get(12)?),
            })
        })
        .map_err(|e| format!("Query model profiles: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect model profiles: {}", e))?;
    Ok(profiles)
}
