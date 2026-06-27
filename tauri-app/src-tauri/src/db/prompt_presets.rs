use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

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
pub struct PromptPresetSnapshot {
    pub id: String,
    pub preset_id: String,
    pub version: i32,
    pub prompt_hash: String,
    pub note: Option<String>,
    pub package: PromptPresetPackage,
    pub created_at: String,
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

fn prompt_package_hash(package_json: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(package_json.as_bytes());
    format!("{:x}", hasher.finalize())
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

pub fn create_prompt_preset_snapshot(
    db: &Database,
    preset_id: &str,
    note: Option<&str>,
) -> Result<PromptPresetSnapshot, String> {
    let package = export_prompt_preset_package(db, preset_id)?;
    let package_json = serde_json::to_string(&package)
        .map_err(|e| format!("Serialize prompt preset snapshot: {}", e))?;
    let prompt_hash = prompt_package_hash(&package_json);
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let version = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM prompt_preset_snapshots WHERE preset_id = ?1",
            params![preset_id],
            |row| row.get::<_, i32>(0),
        )
        .map_err(|e| format!("Next prompt preset snapshot version: {}", e))?;
    conn.execute(
        "INSERT INTO prompt_preset_snapshots
            (id, preset_id, version, prompt_hash, note, package_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, preset_id, version, prompt_hash, note, package_json],
    )
    .map_err(|e| format!("Create prompt preset snapshot: {}", e))?;
    let created_at = conn
        .query_row(
            "SELECT created_at FROM prompt_preset_snapshots WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| format!("Load prompt preset snapshot timestamp: {}", e))?;
    Ok(PromptPresetSnapshot {
        id,
        preset_id: preset_id.to_string(),
        version,
        prompt_hash,
        note: note.map(str::to_string),
        package,
        created_at,
    })
}

pub fn list_prompt_preset_snapshots(
    db: &Database,
    preset_id: &str,
) -> Result<Vec<PromptPresetSnapshot>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, preset_id, version, prompt_hash, note, package_json, created_at
             FROM prompt_preset_snapshots
             WHERE preset_id = ?1
             ORDER BY version ASC",
        )
        .map_err(|e| format!("Prepare prompt preset snapshots: {}", e))?;
    let snapshots = stmt
        .query_map(params![preset_id], |row| {
            let package_json: String = row.get(5)?;
            let package =
                serde_json::from_str::<PromptPresetPackage>(&package_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            Ok(PromptPresetSnapshot {
                id: row.get(0)?,
                preset_id: row.get(1)?,
                version: row.get(2)?,
                prompt_hash: row.get(3)?,
                note: row.get(4)?,
                package,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query prompt preset snapshots: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect prompt preset snapshots: {}", e))?;
    Ok(snapshots)
}

pub fn clone_prompt_preset(
    db: &Database,
    source_preset_id: &str,
    new_id: Option<String>,
    new_name: &str,
) -> Result<String, String> {
    if new_name.trim().is_empty() {
        return Err("Cloned prompt preset name is required".to_string());
    }
    let mut package = export_prompt_preset_package(db, source_preset_id)?;
    let cloned_id = new_id.unwrap_or_else(Database::new_uuid);
    let mut metadata = package.metadata;
    if !metadata.is_object() {
        metadata = serde_json::json!({});
    }
    metadata["cloned_from"] = serde_json::json!(package.id);
    metadata["clone_source_builtin"] = serde_json::json!(package.is_builtin);
    package.id = cloned_id.clone();
    package.name = new_name.to_string();
    package.is_builtin = false;
    package.metadata = metadata;
    import_prompt_preset_package(db, &package)?;
    Ok(cloned_id)
}

pub fn dry_run_prompt_preset(
    db: &Database,
    preset_id: &str,
    generation_phase: &str,
    temporary_overrides: HashMap<String, String>,
) -> Result<crate::workflow::prompt_runtime::AssembledPrompt, String> {
    let package = export_prompt_preset_package(db, preset_id)?;
    let units = package
        .units
        .into_iter()
        .map(prompt_package_unit_to_runtime_unit)
        .collect::<Vec<_>>();
    let vars = prompt_default_vars_from_units(&units)
        .into_iter()
        .chain(temporary_overrides)
        .collect::<HashMap<_, _>>();
    crate::workflow::prompt_runtime::assemble_prompt_runtime(
        crate::workflow::prompt_runtime::PromptRuntimeRequest {
            prompt_name: package.name,
            generation_phase: generation_phase.to_string(),
            vars,
            units,
        },
    )
}

fn prompt_package_unit_to_runtime_unit(
    unit: PromptPresetUnitPackage,
) -> crate::workflow::prompt_runtime::PromptUnit {
    let content = append_few_shot_examples(&unit.content, &unit.metadata);
    crate::workflow::prompt_runtime::PromptUnit {
        identifier: unit.identifier,
        role: unit.role,
        order: unit.order,
        enabled: unit.enabled,
        injection_position: unit.injection_position,
        generation_phase: unit.generation_phase,
        content,
        metadata: unit.metadata,
    }
}

fn prompt_default_vars_from_units(
    units: &[crate::workflow::prompt_runtime::PromptUnit],
) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for unit in units {
        if let Some(parameters) = unit
            .metadata
            .get("parameters")
            .and_then(|value| value.as_object())
        {
            for (key, spec) in parameters {
                if let Some(default) = spec.get("default").and_then(|value| value.as_str()) {
                    vars.entry(key.clone())
                        .or_insert_with(|| default.to_string());
                }
            }
        }
    }
    vars
}

fn append_few_shot_examples(content: &str, metadata: &serde_json::Value) -> String {
    let Some(examples) = metadata
        .get("few_shot_examples")
        .and_then(|value| value.as_array())
    else {
        return content.to_string();
    };
    if examples.is_empty() {
        return content.to_string();
    }
    let mut rendered = content.to_string();
    rendered.push_str("\n\nFew-shot examples:");
    for example in examples {
        let label = example
            .get("label")
            .and_then(|value| value.as_str())
            .unwrap_or("example");
        let input = example
            .get("input")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let output = example
            .get("output")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        rendered.push_str(&format!(
            "\n- {} input: {}\n  output: {}",
            label, input, output
        ));
    }
    rendered
}
