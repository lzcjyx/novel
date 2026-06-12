use crate::db::connection::Database;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub degree: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphSnapshot {
    pub nodes: Vec<KnowledgeGraphNode>,
    pub edges: Vec<KnowledgeGraphEdge>,
    pub orphan_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphRetrievalHints {
    pub source_key: String,
    pub connected_source_keys: Vec<String>,
    pub query_terms: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphNeighborhood {
    pub center: KnowledgeGraphNode,
    pub neighbors: Vec<KnowledgeGraphNode>,
    pub edges: Vec<KnowledgeGraphEdge>,
    pub retrieval_hints: KnowledgeGraphRetrievalHints,
}

pub fn get_edges(db: &Database, project_id: &str) -> Result<Vec<KnowledgeGraphEdge>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_node_id, source_node_type, target_node_id, target_node_type,
                edge_type, description, auto_inferred, confidence, metadata, created_at, updated_at
         FROM knowledge_graph_edges WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let edges = stmt
        .query_map(params![project_id], |row| {
            Ok(KnowledgeGraphEdge {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_node_id: row.get(2)?,
                source_node_type: row.get(3)?,
                target_node_id: row.get(4)?,
                target_node_type: row.get(5)?,
                edge_type: row.get(6)?,
                description: row.get(7)?,
                auto_inferred: row.get::<_, i32>(8)? != 0,
                confidence: row.get(9).unwrap_or(1.0),
                metadata: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect: {}", e))?;
    Ok(edges)
}

pub fn get_edge(db: &Database, edge_id: &str) -> Result<KnowledgeGraphEdge, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT id, project_id, source_node_id, source_node_type, target_node_id, target_node_type,
                edge_type, description, auto_inferred, confidence, metadata, created_at, updated_at
         FROM knowledge_graph_edges WHERE id = ?1",
        params![edge_id],
        |row| {
            Ok(KnowledgeGraphEdge {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_node_id: row.get(2)?,
                source_node_type: row.get(3)?,
                target_node_id: row.get(4)?,
                target_node_type: row.get(5)?,
                edge_type: row.get(6)?,
                description: row.get(7)?,
                auto_inferred: row.get::<_, i32>(8)? != 0,
                confidence: row.get(9).unwrap_or(1.0),
                metadata: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        },
    )
    .map_err(|e| format!("Get edge: {}", e))
}

pub fn get_snapshot(db: &Database, project_id: &str) -> Result<KnowledgeGraphSnapshot, String> {
    let bible = crate::db::bible::get_bible(db, project_id)?;
    let edges = get_edges(db, project_id)?;
    let mut degrees: HashMap<String, i32> = HashMap::new();

    for edge in &edges {
        *degrees
            .entry(node_key(&edge.source_node_type, &edge.source_node_id))
            .or_insert(0) += 1;
        *degrees
            .entry(node_key(&edge.target_node_type, &edge.target_node_id))
            .or_insert(0) += 1;
    }

    let mut nodes = Vec::new();
    for c in bible.characters {
        nodes.push(node(
            c.id,
            "character",
            c.name,
            c.role,
            c.personality.or(c.backstory),
            c.status,
            &degrees,
        ));
    }
    for l in bible.locations {
        nodes.push(node(
            l.id,
            "location",
            l.name,
            l.r#type,
            l.description,
            l.status,
            &degrees,
        ));
    }
    for o in bible.organizations {
        nodes.push(node(
            o.id,
            "organization",
            o.name,
            o.goals,
            o.description,
            o.status,
            &degrees,
        ));
    }
    for i in bible.items {
        nodes.push(node(
            i.id,
            "item",
            i.name,
            i.item_type,
            i.description.or(i.abilities),
            i.status,
            &degrees,
        ));
    }
    for l in bible.world_lore {
        nodes.push(node(
            l.id,
            "world_lore",
            l.title.unwrap_or_else(|| "World Lore".into()),
            l.lore_type,
            l.content,
            l.status,
            &degrees,
        ));
    }
    for m in bible.magic_systems {
        nodes.push(node(
            m.id,
            "magic_system",
            m.name.unwrap_or_else(|| "Magic System".into()),
            None,
            m.description.or(m.rules),
            m.status,
            &degrees,
        ));
    }
    for r in bible.canon_rules {
        nodes.push(node(
            r.id,
            "canon_rule",
            r.rule_text.unwrap_or_else(|| "Canon Rule".into()),
            Some(r.severity),
            r.rule_type,
            r.status,
            &degrees,
        ));
    }
    for t in bible.plot_threads {
        nodes.push(node(
            t.id,
            "plot_thread",
            t.name.unwrap_or_else(|| "Plot Thread".into()),
            Some(format!("priority {}", t.priority)),
            t.description,
            t.arc_status,
            &degrees,
        ));
    }
    for f in bible.foreshadowing {
        nodes.push(node(
            f.id,
            "foreshadowing",
            f.clue_text.unwrap_or_else(|| "Foreshadowing".into()),
            Some(format!("importance {}", f.importance)),
            f.intended_payoff,
            f.status,
            &degrees,
        ));
    }
    for s in bible.style_guides {
        nodes.push(node(
            s.id,
            "style_guide",
            s.name.unwrap_or_else(|| "Style Guide".into()),
            None,
            s.style_text,
            s.status,
            &degrees,
        ));
    }
    for t in bible.timeline_events {
        nodes.push(node(
            t.id,
            "timeline_event",
            t.event_summary.unwrap_or_else(|| "Timeline Event".into()),
            t.event_time_label,
            Some(t.consequences),
            t.status,
            &degrees,
        ));
    }

    nodes.sort_by(|a, b| a.node_type.cmp(&b.node_type).then(a.label.cmp(&b.label)));
    let orphan_count = nodes.iter().filter(|n| n.degree == 0).count();
    Ok(KnowledgeGraphSnapshot {
        nodes,
        edges,
        orphan_count,
    })
}

pub fn get_node_neighborhood(
    db: &Database,
    project_id: &str,
    node_id: &str,
    node_type: &str,
) -> Result<KnowledgeGraphNeighborhood, String> {
    if node_id.trim().is_empty() || node_type.trim().is_empty() {
        return Err("Node id and type are required".into());
    }

    let snapshot = get_snapshot(db, project_id)?;
    let center_key = node_key(node_type, node_id);
    let node_by_key = snapshot
        .nodes
        .iter()
        .cloned()
        .map(|node| (node_key(&node.node_type, &node.id), node))
        .collect::<HashMap<_, _>>();
    let center = node_by_key
        .get(&center_key)
        .cloned()
        .ok_or_else(|| format!("Graph node not found: {}", center_key))?;

    let mut connected_source_keys = Vec::new();
    let mut neighbor_map = HashMap::<String, KnowledgeGraphNode>::new();
    let edges = snapshot
        .edges
        .into_iter()
        .filter(|edge| {
            node_key(&edge.source_node_type, &edge.source_node_id) == center_key
                || node_key(&edge.target_node_type, &edge.target_node_id) == center_key
        })
        .inspect(|edge| {
            let other_key = if node_key(&edge.source_node_type, &edge.source_node_id) == center_key
            {
                node_key(&edge.target_node_type, &edge.target_node_id)
            } else {
                node_key(&edge.source_node_type, &edge.source_node_id)
            };
            if let Some(neighbor) = node_by_key.get(&other_key) {
                connected_source_keys.push(other_key.clone());
                neighbor_map.insert(other_key, neighbor.clone());
            }
        })
        .collect::<Vec<_>>();

    connected_source_keys.sort();
    connected_source_keys.dedup();
    let mut neighbors = neighbor_map.into_values().collect::<Vec<_>>();
    neighbors.sort_by(|a, b| a.node_type.cmp(&b.node_type).then(a.label.cmp(&b.label)));
    let mut query_terms = vec![center.label.clone()];
    query_terms.extend(neighbors.iter().map(|node| node.label.clone()));
    query_terms.sort();
    query_terms.dedup();

    Ok(KnowledgeGraphNeighborhood {
        center,
        neighbors,
        edges,
        retrieval_hints: KnowledgeGraphRetrievalHints {
            source_key: center_key,
            connected_source_keys,
            query_terms,
        },
    })
}

pub fn create_edge(
    db: &Database,
    project_id: &str,
    source_id: &str,
    source_type: &str,
    target_id: &str,
    target_type: &str,
    edge_type: &str,
    description: Option<&str>,
) -> Result<KnowledgeGraphEdge, String> {
    validate_edge(
        project_id,
        source_id,
        source_type,
        target_id,
        target_type,
        edge_type,
    )?;
    let edge_id = insert_edge(
        db,
        project_id,
        source_id,
        source_type,
        target_id,
        target_type,
        edge_type,
        description,
        false,
        1.0,
    )?;
    get_edge(db, &edge_id)
}

pub fn insert_edge(
    db: &Database,
    project_id: &str,
    source_id: &str,
    source_type: &str,
    target_id: &str,
    target_type: &str,
    edge_type: &str,
    description: Option<&str>,
    auto_inferred: bool,
    confidence: f64,
) -> Result<String, String> {
    insert_edge_with_metadata(
        db,
        project_id,
        source_id,
        source_type,
        target_id,
        target_type,
        edge_type,
        description,
        auto_inferred,
        confidence,
        &serde_json::json!({}),
    )
}

pub fn insert_edge_with_metadata(
    db: &Database,
    project_id: &str,
    source_id: &str,
    source_type: &str,
    target_id: &str,
    target_type: &str,
    edge_type: &str,
    description: Option<&str>,
    auto_inferred: bool,
    confidence: f64,
    metadata: &serde_json::Value,
) -> Result<String, String> {
    validate_edge(
        project_id,
        source_id,
        source_type,
        target_id,
        target_type,
        edge_type,
    )?;
    let normalized_confidence = if confidence.is_finite() {
        confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let metadata_json =
        serde_json::to_string(metadata).map_err(|e| format!("Serialize edge metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id
             FROM knowledge_graph_edges
             WHERE project_id = ?1
               AND source_node_id = ?2
               AND source_node_type = ?3
               AND target_node_id = ?4
               AND target_node_type = ?5
               AND edge_type = ?6
             ORDER BY created_at ASC
             LIMIT 1",
            params![
                project_id,
                source_id,
                source_type,
                target_id,
                target_type,
                edge_type
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Find existing edge: {}", e))?;

    if let Some(existing_id) = existing_id {
        return Ok(existing_id);
    }

    let id = Database::new_uuid();
    conn.execute(
        "INSERT OR IGNORE INTO knowledge_graph_edges (id, project_id, source_node_id, source_node_type, target_node_id, target_node_type, edge_type, description, auto_inferred, confidence, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            project_id,
            source_id,
            source_type,
            target_id,
            target_type,
            edge_type,
            description,
            auto_inferred as i32,
            normalized_confidence,
            metadata_json
        ],
    ).map_err(|e| format!("Insert edge: {}", e))?;
    Ok(id)
}

pub fn delete_edge(db: &Database, edge_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "DELETE FROM knowledge_graph_edges WHERE id = ?1",
        params![edge_id],
    )
    .map_err(|e| format!("Delete edge: {}", e))?;
    Ok(())
}

fn node_key(node_type: &str, id: &str) -> String {
    format!("{}:{}", node_type, id)
}

fn node(
    id: String,
    node_type: &str,
    label: String,
    subtitle: Option<String>,
    description: Option<String>,
    status: String,
    degrees: &HashMap<String, i32>,
) -> KnowledgeGraphNode {
    let degree = degrees.get(&node_key(node_type, &id)).copied().unwrap_or(0);
    KnowledgeGraphNode {
        id,
        node_type: node_type.into(),
        label,
        subtitle,
        description,
        status,
        degree,
    }
}

fn validate_edge(
    project_id: &str,
    source_id: &str,
    source_type: &str,
    target_id: &str,
    target_type: &str,
    edge_type: &str,
) -> Result<(), String> {
    if project_id.trim().is_empty() {
        return Err("Project id is required".into());
    }
    if source_id.trim().is_empty() || source_type.trim().is_empty() {
        return Err("Source node is required".into());
    }
    if target_id.trim().is_empty() || target_type.trim().is_empty() {
        return Err("Target node is required".into());
    }
    if edge_type.trim().is_empty() {
        return Err("Edge type is required".into());
    }
    if source_id == target_id && source_type == target_type {
        return Err("Self edges are not supported".into());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphEdge {
    pub id: String,
    pub project_id: String,
    pub source_node_id: String,
    pub source_node_type: String,
    pub target_node_id: String,
    pub target_node_type: String,
    pub edge_type: String,
    pub description: Option<String>,
    pub auto_inferred: bool,
    pub confidence: f64,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}
