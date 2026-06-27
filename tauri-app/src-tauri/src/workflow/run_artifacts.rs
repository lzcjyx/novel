use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArtifactPayload {
    pub system_prompt: String,
    pub user_prompt: String,
    pub context_package: Value,
    pub context_trace: Value,
    pub draft_markdown: String,
    pub reviews: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArtifactManifest {
    pub id: String,
    pub project_id: String,
    pub job_id: String,
    pub dir_path: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub export_templates: Vec<Value>,
}

pub fn extension_export_templates_from_metadata(metadata: &Value) -> Vec<Value> {
    crate::extensions::host::extension_contribution_payloads(metadata, "export_template")
}

pub fn write_run_artifacts(
    db: &Database,
    job_id: &str,
    base_dir: &Path,
    payload: &RunArtifactPayload,
) -> Result<RunArtifactManifest, String> {
    let (project_id, job_metadata) = load_job_for_artifacts(db, job_id)?;
    let run_dir = base_dir.join(job_id);
    if let Err(err) = write_artifact_files(&run_dir, &job_metadata, payload) {
        let _ = persist_artifact_failure(db, job_id, &project_id, &run_dir, &err);
        return Err(err);
    }

    let files = vec![
        "status.json".to_string(),
        "prompt/system.md".to_string(),
        "prompt/user.md".to_string(),
        "context/package.json".to_string(),
        "context/trace.json".to_string(),
        "output/draft.md".to_string(),
        "usage.json".to_string(),
        "events.jsonl".to_string(),
    ];
    let review_files = (0..payload.reviews.len())
        .map(|index| format!("reviews/review-{:03}.json", index + 1))
        .collect::<Vec<_>>();
    let mut all_files = files;
    all_files.extend(review_files);

    let manifest = RunArtifactManifest {
        id: Database::new_uuid(),
        project_id,
        job_id: job_id.to_string(),
        dir_path: run_dir.to_string_lossy().to_string(),
        files: all_files,
        export_templates: extension_export_templates_from_metadata(&job_metadata),
    };
    persist_artifact_manifest(db, &manifest)?;
    Ok(manifest)
}

fn persist_artifact_failure(
    db: &Database,
    job_id: &str,
    project_id: &str,
    run_dir: &Path,
    error_message: &str,
) -> Result<(), String> {
    let dir_path = run_dir.to_string_lossy().to_string();
    let manifest = json!({
        "project_id": project_id,
        "job_id": job_id,
        "dir_path": dir_path,
        "status": "failed",
        "error_message": error_message,
    });
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO run_artifacts
            (id, project_id, job_id, dir_path, manifest, status, error_message, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'failed', ?6, datetime('now'))
         ON CONFLICT(job_id) DO UPDATE SET
            dir_path = excluded.dir_path,
            manifest = excluded.manifest,
            status = 'failed',
            error_message = excluded.error_message,
            updated_at = datetime('now')",
        params![
            Database::new_uuid(),
            project_id,
            job_id,
            dir_path,
            manifest.to_string(),
            error_message,
        ],
    )
    .map_err(|e| format!("Persist run artifact failure: {}", e))?;

    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load failed artifact job metadata: {}", e))?;
    let mut metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata["run_artifacts"] = json!({
        "status": "failed",
        "dir_path": dir_path,
        "error_message": error_message,
        "updated_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    });
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata.to_string(), job_id],
    )
    .map_err(|e| format!("Update failed artifact metadata: {}", e))?;
    Ok(())
}

fn load_job_for_artifacts(db: &Database, job_id: &str) -> Result<(String, Value), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (project_id, metadata_raw): (String, String) = conn
        .query_row(
            "SELECT project_id, metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Load job for artifacts: {}", e))?;
    let metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    Ok((project_id, metadata))
}

fn write_artifact_files(
    run_dir: &Path,
    job_metadata: &Value,
    payload: &RunArtifactPayload,
) -> Result<(), String> {
    create_dir(run_dir)?;
    create_dir(&run_dir.join("prompt"))?;
    create_dir(&run_dir.join("context"))?;
    create_dir(&run_dir.join("output"))?;
    create_dir(&run_dir.join("reviews"))?;

    write_string(
        &run_dir.join("status.json"),
        &serde_json::to_string_pretty(job_metadata)
            .map_err(|e| format!("Serialize status artifact: {}", e))?,
    )?;
    write_string(&run_dir.join("prompt/system.md"), &payload.system_prompt)?;
    write_string(&run_dir.join("prompt/user.md"), &payload.user_prompt)?;
    write_string(
        &run_dir.join("context/package.json"),
        &serde_json::to_string_pretty(&payload.context_package)
            .map_err(|e| format!("Serialize context package artifact: {}", e))?,
    )?;
    write_string(
        &run_dir.join("context/trace.json"),
        &serde_json::to_string_pretty(&payload.context_trace)
            .map_err(|e| format!("Serialize context trace artifact: {}", e))?,
    )?;
    write_string(&run_dir.join("output/draft.md"), &payload.draft_markdown)?;
    for (index, review) in payload.reviews.iter().enumerate() {
        write_string(
            &run_dir.join(format!("reviews/review-{:03}.json", index + 1)),
            &serde_json::to_string_pretty(review)
                .map_err(|e| format!("Serialize review artifact: {}", e))?,
        )?;
    }
    write_string(
        &run_dir.join("usage.json"),
        &serde_json::to_string_pretty(job_metadata.get("usage_summary").unwrap_or(&Value::Null))
            .map_err(|e| format!("Serialize usage artifact: {}", e))?,
    )?;
    let events = job_metadata
        .get("phase_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|event| event.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    write_string(&run_dir.join("events.jsonl"), &events)?;
    Ok(())
}

fn persist_artifact_manifest(db: &Database, manifest: &RunArtifactManifest) -> Result<(), String> {
    let manifest_json = serde_json::to_string(manifest)
        .map_err(|e| format!("Serialize artifact manifest: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO run_artifacts
            (id, project_id, job_id, dir_path, manifest, status, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'written', datetime('now'))
         ON CONFLICT(job_id) DO UPDATE SET
            dir_path = excluded.dir_path,
            manifest = excluded.manifest,
            status = 'written',
            error_message = NULL,
            updated_at = datetime('now')",
        params![
            manifest.id,
            manifest.project_id,
            manifest.job_id,
            manifest.dir_path,
            manifest_json,
        ],
    )
    .map_err(|e| format!("Persist run artifact manifest: {}", e))?;

    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![manifest.job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load artifact job metadata: {}", e))?;
    let mut metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata["run_artifacts"] = json!({
        "artifact_id": manifest.id,
        "dir_path": manifest.dir_path,
        "files": manifest.files,
        "export_templates": manifest.export_templates,
        "updated_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    });
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata.to_string(), manifest.job_id],
    )
    .map_err(|e| format!("Update job artifact metadata: {}", e))?;
    Ok(())
}

fn create_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Create artifact dir '{}': {}", display(path), e))
}

fn write_string(path: &PathBuf, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| format!("Write artifact '{}': {}", display(path), e))
}

fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
