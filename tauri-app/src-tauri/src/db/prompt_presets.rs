use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPresetInput {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub scope: String,
    pub is_builtin: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPreset {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scope: String,
    pub is_builtin: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptUnitInput {
    pub preset_id: String,
    pub identifier: String,
    pub role: String,
    pub order: i32,
    pub enabled: bool,
    pub injection_position: String,
    pub generation_phase: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPresetPackage {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub scope: String,
    pub is_builtin: bool,
    pub metadata: serde_json::Value,
    pub units: Vec<PromptPresetUnitPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPresetUnitPackage {
    pub identifier: String,
    pub role: String,
    pub order: i32,
    pub enabled: bool,
    pub injection_position: String,
    pub generation_phase: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata).map_err(|e| format!("Serialize prompt metadata: {}", e))
}

fn parse_metadata(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

pub fn upsert_prompt_preset(db: &Database, input: &PromptPresetInput) -> Result<String, String> {
    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let metadata = metadata_to_string(&input.metadata)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO prompt_presets
            (id, name, description, scope, is_builtin, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            scope = excluded.scope,
            is_builtin = excluded.is_builtin,
            status = 'active',
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.name,
            input.description,
            input.scope,
            input.is_builtin as i32,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert prompt preset: {}", e))?;
    Ok(id)
}

pub fn upsert_prompt_unit(db: &Database, input: &PromptUnitInput) -> Result<String, String> {
    let id = {
        let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        conn.query_row(
            "SELECT id FROM prompt_preset_units WHERE preset_id = ?1 AND identifier = ?2",
            params![input.preset_id, input.identifier],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| Database::new_uuid())
    };
    let metadata = metadata_to_string(&input.metadata)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO prompt_preset_units
            (id, preset_id, identifier, role, unit_order, enabled, injection_position, generation_phase, content, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
         ON CONFLICT(preset_id, identifier) DO UPDATE SET
            role = excluded.role,
            unit_order = excluded.unit_order,
            enabled = excluded.enabled,
            injection_position = excluded.injection_position,
            generation_phase = excluded.generation_phase,
            content = excluded.content,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.preset_id,
            input.identifier,
            input.role,
            input.order,
            input.enabled as i32,
            input.injection_position,
            input.generation_phase,
            input.content,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert prompt unit: {}", e))?;
    Ok(id)
}

pub fn list_prompt_presets(db: &Database) -> Result<Vec<PromptPreset>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, scope, is_builtin, metadata
             FROM prompt_presets
             WHERE status = 'active'
             ORDER BY name ASC, id ASC",
        )
        .map_err(|e| format!("Prepare prompt presets: {}", e))?;
    let presets = stmt
        .query_map([], |row| {
            Ok(PromptPreset {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                scope: row.get(3)?,
                is_builtin: row.get::<_, i32>(4)? != 0,
                metadata: parse_metadata(row.get(5)?),
            })
        })
        .map_err(|e| format!("Query prompt presets: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect prompt presets: {}", e))?;
    Ok(presets)
}

pub fn export_prompt_preset_package(
    db: &Database,
    preset_id: &str,
) -> Result<PromptPresetPackage, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (id, name, description, scope, is_builtin, metadata): (
        String,
        String,
        Option<String>,
        String,
        i32,
        String,
    ) = conn
        .query_row(
            "SELECT id, name, description, scope, is_builtin, metadata
             FROM prompt_presets WHERE id = ?1 AND status = 'active'",
            params![preset_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .map_err(|e| format!("Load prompt preset: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT identifier, role, unit_order, enabled, injection_position, generation_phase, content, metadata
             FROM prompt_preset_units
             WHERE preset_id = ?1
             ORDER BY unit_order ASC, identifier ASC",
        )
        .map_err(|e| format!("Prepare prompt units: {}", e))?;
    let units = stmt
        .query_map(params![preset_id], |row| {
            Ok(PromptPresetUnitPackage {
                identifier: row.get(0)?,
                role: row.get(1)?,
                order: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                injection_position: row.get(4)?,
                generation_phase: row.get(5)?,
                content: row.get(6)?,
                metadata: parse_metadata(row.get(7)?),
            })
        })
        .map_err(|e| format!("Query prompt units: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect prompt units: {}", e))?;

    Ok(PromptPresetPackage {
        id,
        name,
        description,
        scope,
        is_builtin: is_builtin != 0,
        metadata: parse_metadata(metadata),
        units,
    })
}

pub fn import_prompt_preset_package(
    db: &Database,
    package: &PromptPresetPackage,
) -> Result<String, String> {
    let preset_metadata = metadata_to_string(&package.metadata)?;
    let unit_metadata = package
        .units
        .iter()
        .map(|unit| metadata_to_string(&unit.metadata))
        .collect::<Result<Vec<_>, _>>()?;
    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin prompt preset import: {}", e))?;
    tx.execute(
        "INSERT INTO prompt_presets
            (id, name, description, scope, is_builtin, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            scope = excluded.scope,
            is_builtin = excluded.is_builtin,
            status = 'active',
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            package.id,
            package.name,
            package.description,
            package.scope,
            package.is_builtin as i32,
            preset_metadata
        ],
    )
    .map_err(|e| format!("Import prompt preset: {}", e))?;
    tx.execute(
        "DELETE FROM prompt_preset_units WHERE preset_id = ?1",
        params![package.id],
    )
    .map_err(|e| format!("Clear prompt preset units: {}", e))?;
    for (unit, metadata) in package.units.iter().zip(unit_metadata.iter()) {
        tx.execute(
            "INSERT INTO prompt_preset_units
                (id, preset_id, identifier, role, unit_order, enabled, injection_position, generation_phase, content, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                Database::new_uuid(),
                package.id,
                unit.identifier,
                unit.role,
                unit.order,
                unit.enabled as i32,
                unit.injection_position,
                unit.generation_phase,
                unit.content,
                metadata
            ],
        )
        .map_err(|e| format!("Import prompt unit '{}': {}", unit.identifier, e))?;
    }
    tx.commit()
        .map_err(|e| format!("Commit prompt preset import: {}", e))?;
    Ok(package.id.clone())
}
