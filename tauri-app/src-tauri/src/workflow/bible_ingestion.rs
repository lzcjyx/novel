use crate::db::connection::Database;
use crate::ai::client::ModelClient;

pub async fn ingest_bible_note(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    note: &str,
) -> Result<(), String> {
    let system = "You are a canon extraction agent. Extract structured world-building information from the author's notes. Output valid JSON only.";
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "new_characters": {"type": "array"},
            "new_locations": {"type": "array"},
            "world_lore_updates": {"type": "array"},
            "canon_rule_updates": {"type": "array"},
            "plot_thread_updates": {"type": "array"},
            "human_review_required": {"type": "array"}
        }
    });

    let extracted = provider.generate_json(system, note, &schema, 8192).await?;

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    if let Some(arr) = extracted["new_characters"].as_array() {
        for c in arr {
            let id = Database::new_uuid();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO characters (id, project_id, name, role, personality, backstory)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    id, project_id,
                    c["name"].as_str().unwrap_or(""),
                    c.get("role").and_then(|v| v.as_str()),
                    c.get("personality").and_then(|v| v.as_str()),
                    c.get("backstory").and_then(|v| v.as_str()),
                ],
            );
        }
    }

    if let Some(arr) = extracted["new_locations"].as_array() {
        for loc in arr {
            let id = Database::new_uuid();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO locations (id, project_id, name, description, type)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    id, project_id,
                    loc["name"].as_str().unwrap_or(""),
                    loc.get("description").and_then(|v| v.as_str()),
                    loc.get("type").and_then(|v| v.as_str()),
                ],
            );
        }
    }

    drop(conn);
    Ok(())
}
