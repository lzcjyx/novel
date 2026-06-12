use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{bible, chapters};
use crate::prompts;
use crate::workflow::prompt_rendering;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct InsertedTimelineEvent {
    id: String,
    event: serde_json::Value,
}

#[derive(Debug, Clone)]
struct ForeshadowingGraphUpdate {
    id: String,
    edge_type: String,
    clue_text: Option<String>,
}

#[derive(Debug, Clone)]
struct GraphNodeRef {
    id: String,
    node_type: String,
}

struct GraphNodeIndex {
    by_id: HashMap<String, GraphNodeRef>,
    by_label: HashMap<String, Option<GraphNodeRef>>,
}

pub async fn update_canon_after_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    chapter_id: &str,
    chapter_draft: &serde_json::Value,
    generation_job_id: Option<&str>,
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
            let mut inserted_character_states = Vec::new();
            let mut inserted_timeline_events = Vec::new();
            let mut foreshadowing_graph_updates = Vec::new();
            let mut inserted_foreshadowing = Vec::new();
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
                    let changed = conn.execute(
                        "INSERT INTO character_states (id, project_id, character_id, after_chapter_id, physical_state, emotional_state)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            id, project_id, char_id, chapter_id,
                            state.get("physical_state").and_then(|v| v.as_str()),
                            state.get("emotional_state").and_then(|v| v.as_str()),
                        ],
                    );
                    if matches!(changed, Ok(count) if count > 0) {
                        inserted_character_states.push(id);
                    }
                }
            }

            // Insert timeline events
            if let Some(arr) = extracted["timeline_events"].as_array() {
                for event in arr {
                    let id = Database::new_uuid();
                    let changed = conn.execute(
                        "INSERT INTO timeline_events (id, project_id, chapter_id, event_time_label, sequence, event_summary, involved_characters, involved_locations, consequences)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            id, project_id, chapter_id,
                            event.get("event_time_label").and_then(|v| v.as_str()),
                            event_sequence(event),
                            event.get("event_summary").and_then(|v| v.as_str()),
                            json_array_text(event.get("involved_characters")),
                            json_array_text(event.get("involved_locations")),
                            json_array_text(event.get("consequences")),
                        ],
                    );
                    if matches!(changed, Ok(count) if count > 0) {
                        inserted_timeline_events.push(InsertedTimelineEvent {
                            id,
                            event: event.clone(),
                        });
                    }
                }
            }

            // Update foreshadowing
            if let Some(arr) = extracted["foreshadowing_updates"].as_array() {
                for f in arr {
                    let action = f
                        .get("action")
                        .or_else(|| f.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    match action {
                        "introduced" => {
                            let id = Database::new_uuid();
                            let changed = conn.execute(
                                "INSERT INTO foreshadowing (id, project_id, clue_text, introduced_chapter_id, status)
                                 VALUES (?1, ?2, ?3, ?4, 'open')",
                                rusqlite::params![
                                    id, project_id,
                                    f.get("clue_text").and_then(|v| v.as_str()),
                                    chapter_id,
                                ],
                            );
                            if matches!(changed, Ok(count) if count > 0) {
                                inserted_foreshadowing.push(id.clone());
                                foreshadowing_graph_updates.push(ForeshadowingGraphUpdate {
                                    id,
                                    edge_type: "introduces".to_string(),
                                    clue_text: f
                                        .get("clue_text")
                                        .and_then(|value| value.as_str())
                                        .map(|value| value.to_string()),
                                });
                            }
                        }
                        "resolved" => {
                            if let Some(f_id) = f
                                .get("foreshadowing_id")
                                .or_else(|| f.get("related_existing_id"))
                                .and_then(|v| v.as_str())
                            {
                                let _ = conn.execute(
                                    "UPDATE foreshadowing SET resolved_chapter_id = ?1, status = 'resolved', updated_at = datetime('now') WHERE id = ?2",
                                    rusqlite::params![chapter_id, f_id],
                                );
                                foreshadowing_graph_updates.push(ForeshadowingGraphUpdate {
                                    id: f_id.to_string(),
                                    edge_type: "resolves".to_string(),
                                    clue_text: f
                                        .get("clue_text")
                                        .and_then(|value| value.as_str())
                                        .map(|value| value.to_string()),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }

            drop(conn);
            record_task_rows(
                db,
                generation_job_id,
                "character_states",
                &inserted_character_states,
            )?;
            record_task_rows(
                db,
                generation_job_id,
                "timeline_events",
                inserted_timeline_events
                    .iter()
                    .map(|event| event.id.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            )?;
            record_task_rows(
                db,
                generation_job_id,
                "foreshadowing",
                &inserted_foreshadowing,
            )?;
            persist_deterministic_timeline_graph_edges(
                db,
                project_id,
                &inserted_timeline_events,
                &foreshadowing_graph_updates,
                generation_job_id,
            )?;
            persist_ai_inferred_graph_edges(db, project_id, &extracted, generation_job_id)?;
        }
        Err(e) => {
            // Canon extraction is non-blocking
            log::warn!("Canon extraction failed (non-blocking): {}", e);
        }
    }

    Ok(())
}

fn record_task_rows<S: AsRef<str>>(
    db: &Database,
    generation_job_id: Option<&str>,
    table: &str,
    row_ids: &[S],
) -> Result<(), String> {
    let Some(job_id) = generation_job_id else {
        return Ok(());
    };
    for row_id in row_ids {
        crate::workflow::task_transaction::record_task_owned_row(
            db,
            job_id,
            table,
            row_id.as_ref(),
        )?;
    }
    Ok(())
}

fn event_sequence(event: &serde_json::Value) -> Option<i64> {
    event
        .get("sequence")
        .or_else(|| event.get("sequence_hint"))
        .and_then(|value| value.as_i64())
}

fn json_array_text(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(value @ serde_json::Value::Array(_)) => {
            serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
        }
        Some(serde_json::Value::String(text)) if !text.trim().is_empty() => {
            serde_json::to_string(&vec![text.trim()]).unwrap_or_else(|_| "[]".to_string())
        }
        _ => "[]".to_string(),
    }
}

fn string_values(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(serde_json::Value::String(text)) if !text.trim().is_empty() => {
            vec![text.trim().to_string()]
        }
        _ => Vec::new(),
    }
}

fn persist_deterministic_timeline_graph_edges(
    db: &Database,
    project_id: &str,
    timeline_events: &[InsertedTimelineEvent],
    foreshadowing_updates: &[ForeshadowingGraphUpdate],
    generation_job_id: Option<&str>,
) -> Result<(), String> {
    if timeline_events.is_empty() {
        return Ok(());
    }

    let snapshot = crate::db::knowledge_graph::get_snapshot(db, project_id)?;
    let node_index = GraphNodeIndex::from_snapshot(&snapshot);

    for timeline_event in timeline_events {
        let timeline_ref = GraphNodeRef {
            id: timeline_event.id.clone(),
            node_type: "timeline_event".to_string(),
        };
        let event_summary = timeline_event
            .event
            .get("event_summary")
            .and_then(|value| value.as_str())
            .unwrap_or("timeline event");
        let confidence = timeline_event
            .event
            .get("confidence")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.85);

        for character in string_values(timeline_event.event.get("involved_characters")) {
            if let Some(character_ref) =
                resolve_node_ref(&node_index, "character", None, Some(&character))
            {
                let description = format!("参与事件：{}", event_summary);
                if let Err(err) = insert_graph_edge(
                    db,
                    project_id,
                    &character_ref,
                    &timeline_ref,
                    "participates_in",
                    Some(description.as_str()),
                    confidence,
                    generation_job_id,
                ) {
                    log::warn!("Skipping deterministic timeline character edge: {}", err);
                }
            }
        }

        for location in string_values(timeline_event.event.get("involved_locations")) {
            if let Some(location_ref) =
                resolve_node_ref(&node_index, "location", None, Some(&location))
            {
                let description = format!("事件发生地：{}", event_summary);
                if let Err(err) = insert_graph_edge(
                    db,
                    project_id,
                    &timeline_ref,
                    &location_ref,
                    "occurs_at",
                    Some(description.as_str()),
                    confidence,
                    generation_job_id,
                ) {
                    log::warn!("Skipping deterministic timeline location edge: {}", err);
                }
            }
        }

        for update in foreshadowing_updates {
            if let Some(foreshadowing_ref) =
                resolve_node_ref(&node_index, "foreshadowing", Some(&update.id), None)
            {
                let description = match update.clue_text.as_deref() {
                    Some(text) if !text.trim().is_empty() => {
                        format!("事件{}伏笔：{}", edge_action_label(&update.edge_type), text)
                    }
                    _ => format!("事件{}伏笔", edge_action_label(&update.edge_type)),
                };
                if let Err(err) = insert_graph_edge(
                    db,
                    project_id,
                    &timeline_ref,
                    &foreshadowing_ref,
                    &update.edge_type,
                    Some(description.as_str()),
                    confidence,
                    generation_job_id,
                ) {
                    log::warn!(
                        "Skipping deterministic timeline foreshadowing edge: {}",
                        err
                    );
                }
            }
        }
    }

    Ok(())
}

fn edge_action_label(edge_type: &str) -> &'static str {
    match edge_type {
        "resolves" => "回收",
        _ => "引入",
    }
}

fn persist_ai_inferred_graph_edges(
    db: &Database,
    project_id: &str,
    extracted: &serde_json::Value,
    generation_job_id: Option<&str>,
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
    let node_index = GraphNodeIndex::from_snapshot(&snapshot);

    for edge in edges {
        let source_type = edge
            .get("source_node_type")
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
        let Some(source_node) = resolve_node_ref(
            &node_index,
            source_type,
            edge.get("source_node_id").and_then(|value| value.as_str()),
            edge.get("source_label")
                .or_else(|| edge.get("source_name"))
                .and_then(|value| value.as_str()),
        ) else {
            log::warn!("Skipping inferred graph edge with unknown source node");
            continue;
        };
        let Some(target_node) = resolve_node_ref(
            &node_index,
            target_type,
            edge.get("target_node_id").and_then(|value| value.as_str()),
            edge.get("target_label")
                .or_else(|| edge.get("target_name"))
                .and_then(|value| value.as_str()),
        ) else {
            log::warn!("Skipping inferred graph edge with unknown target node");
            continue;
        };

        let confidence = edge
            .get("confidence")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.7);
        let description = edge.get("description").and_then(|value| value.as_str());
        if let Err(err) = insert_graph_edge(
            db,
            project_id,
            &source_node,
            &target_node,
            edge_type,
            description,
            confidence,
            generation_job_id,
        ) {
            log::warn!("Skipping inferred knowledge graph edge: {}", err);
        }
    }

    Ok(())
}

impl GraphNodeIndex {
    fn from_snapshot(snapshot: &crate::db::knowledge_graph::KnowledgeGraphSnapshot) -> Self {
        let mut by_id = HashMap::new();
        let mut by_label: HashMap<String, Option<GraphNodeRef>> = HashMap::new();

        for node in &snapshot.nodes {
            let node_ref = GraphNodeRef {
                id: node.id.clone(),
                node_type: node.node_type.clone(),
            };
            by_id.insert(graph_node_key(&node.node_type, &node.id), node_ref.clone());

            if !node.label.trim().is_empty() {
                let label_key = graph_label_key(&node.node_type, &node.label);
                by_label
                    .entry(label_key)
                    .and_modify(|existing| *existing = None)
                    .or_insert_with(|| Some(node_ref));
            }
        }

        Self { by_id, by_label }
    }
}

fn resolve_node_ref(
    index: &GraphNodeIndex,
    node_type: &str,
    node_id: Option<&str>,
    label: Option<&str>,
) -> Option<GraphNodeRef> {
    let node_type = node_type.trim();
    if node_type.is_empty() {
        return None;
    }

    if let Some(id) = node_id.map(str::trim).filter(|value| !value.is_empty()) {
        return index.by_id.get(&graph_node_key(node_type, id)).cloned();
    }

    let label = label.map(str::trim).filter(|value| !value.is_empty())?;
    index
        .by_label
        .get(&graph_label_key(node_type, label))
        .and_then(|node| node.clone())
}

fn insert_graph_edge(
    db: &Database,
    project_id: &str,
    source: &GraphNodeRef,
    target: &GraphNodeRef,
    edge_type: &str,
    description: Option<&str>,
    confidence: f64,
    generation_job_id: Option<&str>,
) -> Result<String, String> {
    let metadata = generation_job_id
        .map(|job_id| serde_json::json!({ "generation_job_id": job_id }))
        .unwrap_or_else(|| serde_json::json!({}));
    let edge_id = crate::db::knowledge_graph::insert_edge_with_metadata(
        db,
        project_id,
        &source.id,
        &source.node_type,
        &target.id,
        &target.node_type,
        edge_type,
        description,
        true,
        confidence,
        &metadata,
    )?;
    if let Some(job_id) = generation_job_id {
        crate::workflow::task_transaction::record_task_owned_row(
            db,
            job_id,
            "knowledge_graph_edges",
            &edge_id,
        )?;
    }
    Ok(edge_id)
}

fn graph_node_key(node_type: &str, id: &str) -> String {
    format!("{}:{}", node_type.trim(), id.trim())
}

fn graph_label_key(node_type: &str, label: &str) -> String {
    format!("{}:{}", node_type.trim(), normalize_label(label))
}

fn normalize_label(label: &str) -> String {
    label.trim().to_lowercase()
}
