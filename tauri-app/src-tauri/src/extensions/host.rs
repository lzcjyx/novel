use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::connection::Database;
use crate::extensions::manifest::{validate_extension_manifest, ExtensionManifest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionContribution {
    pub hook: String,
    pub required_permission: Option<String>,
    pub metadata_patch: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPackage {
    pub manifest: ExtensionManifest,
    pub enabled: bool,
    pub contributions: Vec<ExtensionContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionHookRequest {
    pub hook: String,
    pub workflow_metadata: Value,
    pub extensions: Vec<ExtensionPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionHookTrace {
    pub extension_id: String,
    pub hook: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionHookOutput {
    pub workflow_metadata: Value,
    pub hook_trace: Vec<ExtensionHookTrace>,
}

pub fn execute_extension_hook(
    request: ExtensionHookRequest,
) -> Result<ExtensionHookOutput, String> {
    let mut workflow_metadata = request.workflow_metadata;
    ensure_object(&mut workflow_metadata);

    let mut packages = request.extensions;
    packages.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));

    let mut hook_trace = Vec::new();
    for package in packages {
        validate_extension_manifest(&package.manifest)?;
        let matching_contributions = package
            .contributions
            .iter()
            .filter(|contribution| contribution.hook == request.hook)
            .collect::<Vec<_>>();

        if matching_contributions.is_empty() {
            continue;
        }

        if !package.enabled {
            hook_trace.push(ExtensionHookTrace {
                extension_id: package.manifest.id.clone(),
                hook: request.hook.clone(),
                status: "skipped_disabled".to_string(),
                detail: None,
            });
            continue;
        }

        if !package
            .manifest
            .hooks
            .iter()
            .any(|hook| hook == &request.hook)
        {
            return Err(format!(
                "Extension '{}' does not declare hook '{}'",
                package.manifest.id, request.hook
            ));
        }

        for contribution in matching_contributions {
            if let Some(permission) = contribution.required_permission.as_deref() {
                if !package
                    .manifest
                    .permissions
                    .iter()
                    .any(|declared| declared == permission)
                {
                    return Err(format!(
                        "Extension '{}' permission denied: missing '{}'",
                        package.manifest.id, permission
                    ));
                }
            }
            merge_metadata_patch(&mut workflow_metadata, &contribution.metadata_patch);
            hook_trace.push(ExtensionHookTrace {
                extension_id: package.manifest.id.clone(),
                hook: request.hook.clone(),
                status: "applied".to_string(),
                detail: contribution.required_permission.clone(),
            });
        }
    }

    Ok(ExtensionHookOutput {
        workflow_metadata,
        hook_trace,
    })
}

pub fn import_extension_package(
    db: &Database,
    package: &ExtensionPackage,
) -> Result<String, String> {
    validate_extension_manifest(&package.manifest)?;
    let manifest_json = serde_json::to_string(&package.manifest)
        .map_err(|e| format!("Serialize extension manifest: {}", e))?;
    let contributions_json = serde_json::to_string(&package.contributions)
        .map_err(|e| format!("Serialize extension contributions: {}", e))?;
    let metadata_json = serde_json::to_string(&package.manifest.metadata)
        .map_err(|e| format!("Serialize extension metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO extension_packages
            (id, name, version, description, manifest, contributions, enabled, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'installed', ?7, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            version = excluded.version,
            description = excluded.description,
            manifest = excluded.manifest,
            contributions = excluded.contributions,
            enabled = 0,
            status = 'installed',
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        rusqlite::params![
            &package.manifest.id,
            &package.manifest.name,
            &package.manifest.version,
            &package.manifest.description,
            manifest_json,
            contributions_json,
            metadata_json
        ],
    )
    .map_err(|e| format!("Import extension package: {}", e))?;
    Ok(package.manifest.id.clone())
}

pub fn list_extension_packages(db: &Database) -> Result<Vec<ExtensionPackage>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT manifest, contributions, enabled
             FROM extension_packages
             WHERE status = 'installed'
             ORDER BY id ASC",
        )
        .map_err(|e| format!("Prepare extension packages: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })
        .map_err(|e| format!("Query extension packages: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect extension packages: {}", e))?;

    rows.into_iter()
        .map(|(manifest_raw, contributions_raw, enabled)| {
            let manifest = serde_json::from_str::<ExtensionManifest>(&manifest_raw)
                .map_err(|e| format!("Parse extension manifest: {}", e))?;
            validate_extension_manifest(&manifest)?;
            let contributions =
                serde_json::from_str::<Vec<ExtensionContribution>>(&contributions_raw)
                    .map_err(|e| format!("Parse extension contributions: {}", e))?;
            Ok(ExtensionPackage {
                manifest,
                enabled: enabled != 0,
                contributions,
            })
        })
        .collect()
}

pub fn set_extension_enabled(
    db: &Database,
    extension_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let rows = conn
        .execute(
            "UPDATE extension_packages
             SET enabled = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND status = 'installed'",
            rusqlite::params![enabled as i32, extension_id],
        )
        .map_err(|e| format!("Set extension enabled: {}", e))?;
    if rows == 0 {
        return Err(format!("Extension '{}' is not installed", extension_id));
    }
    Ok(())
}

pub fn execute_extension_hook_for_job(
    db: &Database,
    job_id: &str,
    request: ExtensionHookRequest,
) -> Result<ExtensionHookOutput, String> {
    let hook = request.hook.clone();
    let output = execute_extension_hook(request)?;
    record_extension_hook_trace(db, job_id, &hook, &output)?;
    Ok(output)
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
}

fn merge_metadata_patch(target: &mut Value, patch: &Value) {
    ensure_object(target);
    let Some(target_object) = target.as_object_mut() else {
        return;
    };
    if let Some(patch_object) = patch.as_object() {
        for (key, value) in patch_object {
            target_object.insert(key.clone(), value.clone());
        }
    }
}

fn record_extension_hook_trace(
    db: &Database,
    job_id: &str,
    hook: &str,
    output: &ExtensionHookOutput,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load extension hook job metadata: {}", e))?;
    let mut metadata =
        serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| serde_json::json!({}));
    if !metadata.is_object() {
        metadata = serde_json::json!({});
    }
    if !metadata.get("extension_hooks").is_some_and(Value::is_array) {
        metadata["extension_hooks"] = serde_json::json!([]);
    }
    metadata["extension_hooks"]
        .as_array_mut()
        .ok_or_else(|| "Job metadata extension_hooks is not an array".to_string())?
        .push(serde_json::json!({
            "hook": hook,
            "events": output.hook_trace,
            "workflow_metadata": output.workflow_metadata,
            "timestamp": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        }));
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![metadata.to_string(), job_id],
    )
    .map_err(|e| format!("Record extension hook trace: {}", e))?;
    Ok(())
}
