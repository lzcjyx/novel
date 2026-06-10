use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{bible, chapters};
use crate::prompts;
use crate::workflow::prompt_rendering;
use std::collections::{HashMap, HashSet};

pub async fn update_canon_after_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    chapter_id: &str,
    chapter_draft: &serde_json::Value,
) -> Result<(), String> {
    let chapter = chapters::get_chapter(db, chapter_id)?;
    let canon_template = prompts::load_prompt("canon_extractor")?;
    let existing_canon = bible::get_bible(db, project_id)?;
    let chapter_text = serde_json::json!({
        "title": chapter_draft["title"].as_str().or(chapter.title.as_deref()).unwrap_or(""),
        "sequence": chapter.sequence,
        "body_markdown": chapter_draft["body_markdown"].as_str().unwrap_or(""),
        "summary": chapter_draft["summary"].as_str().unwrap_or(""),
        "major_events": chapter_draft.get("major_events"),
        "character_state_changes": chapter_draft.get("character_state_changes"),
        "timeline_events": chapter_draft.get("timeline_events"),
        "foreshadowing_used": chapter_draft.get("foreshadowing_used"),
        "foreshadowing_planted": chapter_draft.get("foreshadowing_planted"),
        "new_canon_candidates": chapter_draft.get("new_canon_candidates"),
    });
    let vars = HashMap::from([
        ("PROJECT_ID", project_id.to_string()),
        ("CHAPTER_ID", chapter_id.to_string()),
        (
            "CHAPTER_TEXT",
            serde_json::to_string_pretty(&chapter_text).unwrap_or_default(),
        ),
        (
            "EXISTING_CANON_JSON",
            serde_json::to_string_pretty(&existing_canon).unwrap_or_default(),
        ),
    ]);
    let canon_prompt =
        prompt_rendering::render_prompt_strict("canon_extractor", &canon_template, &vars)?;

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "chapter_summary": {"type": "string"},
            "character_state_updates": {"type": "array"},
            "timeline_events": {"type": "array"},
            "new_lore": {"type": "array"},
            "foreshadowing_updates": {"type": "array"},
            "vector_documents": {"type": "array"},
            "knowledge_graph_edges": {"type": "array"},
            "human_review_required": {"type": "array"}
        }
    });

    // Try to extract canon, but don't block on failure
    match provider
        .generate_json(
            &canon_prompt,
            "请从已完成章节中提取 canon 更新，只输出 JSON。",
            &schema,
            8192,
        )
        .await
    {
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
            persist_ai_inferred_graph_edges(db, project_id, &extracted)?;
        }
        Err(e) => {
            // Canon extraction is non-blocking
            log::warn!("Canon extraction failed (non-blocking): {}", e);
        }
    }

    Ok(())
}

fn persist_ai_inferred_graph_edges(
    db: &Database,
    project_id: &str,
    extracted: &serde_json::Value,
) -> Result<(), String> {
    let Some(edges) = extracted
        .get("knowledge_graph_edges")
        .and_then(|value| value.as_array())
    else {
        return Ok(());
    };
    if edges.is_empty() {
        return Ok(());
    }

    let snapshot = crate::db::knowledge_graph::get_snapshot(db, project_id)?;
    let valid_nodes = snapshot
        .nodes
        .iter()
        .map(|node| format!("{}:{}", node.node_type, node.id))
        .collect::<HashSet<_>>();

    for edge in edges {
        let source_id = edge
            .get("source_node_id")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let source_type = edge
            .get("source_node_type")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let target_id = edge
            .get("target_node_id")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let target_type = edge
            .get("target_node_type")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let edge_type = edge
            .get("edge_type")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if !valid_nodes.contains(&format!("{}:{}", source_type, source_id))
            || !valid_nodes.contains(&format!("{}:{}", target_type, target_id))
        {
            continue;
        }

        let confidence = edge
            .get("confidence")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.7);
        let description = edge.get("description").and_then(|value| value.as_str());
        if let Err(err) = crate::db::knowledge_graph::insert_edge(
            db,
            project_id,
            source_id,
            source_type,
            target_id,
            target_type,
            edge_type,
            description,
            true,
            confidence,
        ) {
            log::warn!("Skipping inferred knowledge graph edge: {}", err);
        }
    }

    Ok(())
}
