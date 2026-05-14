use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::chapters;
use crate::prompts;

pub async fn update_canon_after_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    chapter_id: &str,
    chapter_draft: &serde_json::Value,
) -> Result<(), String> {
    let chapter = chapters::get_chapter(db, chapter_id)?;
    let canon_template = prompts::load_prompt("canon_extractor")?;

    let context = serde_json::json!({
        "chapter": {
            "title": chapter.title,
            "sequence": chapter.sequence,
            "content": chapter_draft["body_markdown"].as_str().unwrap_or(""),
            "summary": chapter_draft["summary"].as_str().unwrap_or(""),
        },
        "major_events": chapter_draft.get("major_events"),
        "character_state_changes": chapter_draft.get("character_state_changes"),
        "timeline_events": chapter_draft.get("timeline_events"),
        "foreshadowing_used": chapter_draft.get("foreshadowing_used"),
        "foreshadowing_planted": chapter_draft.get("foreshadowing_planted"),
        "new_canon_candidates": chapter_draft.get("new_canon_candidates"),
    });

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "chapter_summary": {"type": "string"},
            "character_state_updates": {"type": "array"},
            "timeline_events": {"type": "array"},
            "new_lore": {"type": "array"},
            "foreshadowing_updates": {"type": "array"},
            "vector_documents": {"type": "array"},
            "human_review_required": {"type": "array"}
        }
    });

    // Try to extract canon, but don't block on failure
    match provider.generate_json(&canon_template, &serde_json::to_string(&context).unwrap_or_default(), &schema, 8192).await {
        Ok(extracted) => {
            let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

            // Update chapter summary
            if let Some(summary) = extracted["chapter_summary"].as_str() {
                let _ = conn.execute(
                    "UPDATE chapters SET summary = ?1, updated_at = datetime('now') WHERE id = ?2",
                    rusqlite::params![summary, chapter_id],
                );
            }

            // Insert character states
            if let Some(arr) = extracted["character_state_updates"].as_array() {
                for state in arr {
                    let id = Database::new_uuid();
                    let char_id = state["character_id"].as_str().unwrap_or("");
                    let _ = conn.execute(
                        "INSERT INTO character_states (id, project_id, character_id, after_chapter_id, physical_state, emotional_state)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            id, project_id, char_id, chapter_id,
                            state.get("physical_state").and_then(|v| v.as_str()),
                            state.get("emotional_state").and_then(|v| v.as_str()),
                        ],
                    );
                }
            }

            // Insert timeline events
            if let Some(arr) = extracted["timeline_events"].as_array() {
                for event in arr {
                    let id = Database::new_uuid();
                    let _ = conn.execute(
                        "INSERT INTO timeline_events (id, project_id, chapter_id, sequence, event_summary)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![
                            id, project_id, chapter_id,
                            event.get("sequence").and_then(|v| v.as_i64()),
                            event.get("event_summary").and_then(|v| v.as_str()),
                        ],
                    );
                }
            }

            // Update foreshadowing
            if let Some(arr) = extracted["foreshadowing_updates"].as_array() {
                for f in arr {
                    let action = f.get("action").and_then(|v| v.as_str()).unwrap_or("");
                    match action {
                        "introduced" => {
                            let id = Database::new_uuid();
                            let _ = conn.execute(
                                "INSERT INTO foreshadowing (id, project_id, clue_text, introduced_chapter_id, status)
                                 VALUES (?1, ?2, ?3, ?4, 'open')",
                                rusqlite::params![
                                    id, project_id,
                                    f.get("clue_text").and_then(|v| v.as_str()),
                                    chapter_id,
                                ],
                            );
                        }
                        "resolved" => {
                            if let Some(f_id) = f.get("foreshadowing_id").and_then(|v| v.as_str()) {
                                let _ = conn.execute(
                                    "UPDATE foreshadowing SET resolved_chapter_id = ?1, status = 'resolved', updated_at = datetime('now') WHERE id = ?2",
                                    rusqlite::params![chapter_id, f_id],
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            drop(conn);
        }
        Err(e) => {
            // Canon extraction is non-blocking
            log::warn!("Canon extraction failed (non-blocking): {}", e);
        }
    }

    Ok(())
}
