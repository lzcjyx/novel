use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::db::connection::Database;
use crate::db::vector_store::RetrievalTrace;
use crate::models::{AppSettings, BibleData, ChapterPlan, LearningEntry, Project, VectorDocument};

pub const GRAPH_CONTEXT_MAX_HOPS: usize = 2;
pub const GRAPH_CONTEXT_MAX_NEIGHBORS: usize = 12;
pub const GRAPH_CONTEXT_SUMMARY_TOKEN_BUDGET: i32 = 180;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperatorControls {
    pub generation_mode: Option<String>,
    pub chapter_intent: Option<String>,
    pub must_include_beats: Option<String>,
    pub forbidden_moves: Option<String>,
    pub style_emphasis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingContextPackage {
    pub project: serde_json::Value,
    pub chapter_plan: serde_json::Value,
    pub continuity: serde_json::Value,
    pub canon: serde_json::Value,
    pub graph_context: GraphContext,
    pub retrieval: Vec<VectorDocument>,
    pub retrieval_trace: RetrievalTrace,
    pub style: serde_json::Value,
    pub learned_patterns: Vec<LearningEntry>,
    pub learning_entry_context_ids: Vec<String>,
    pub operator_controls: OperatorControls,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphContext {
    pub seeds: Vec<GraphContextNode>,
    pub neighbors: Vec<GraphContextNeighbor>,
    pub source_keys: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphContextNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphContextNeighbor {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub edge_type: String,
    pub direction: String,
    #[serde(default)]
    pub depth: usize,
    pub via_id: String,
    pub via_type: String,
    pub via_label: String,
    pub description: Option<String>,
}

fn push_query_part(parts: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if !normalized.is_empty() {
            parts.push(normalized);
        }
    }
}

pub fn build_retrieval_query(plan: &ChapterPlan, controls: Option<&OperatorControls>) -> String {
    let mut parts = Vec::new();

    push_query_part(&mut parts, plan.title.as_deref());
    push_query_part(&mut parts, plan.outline.as_deref());
    push_query_part(&mut parts, plan.pov_character_id.as_deref());
    push_query_part(&mut parts, Some(&plan.required_characters));
    push_query_part(&mut parts, Some(&plan.required_locations));
    push_query_part(&mut parts, Some(&plan.plot_goals));
    push_query_part(&mut parts, Some(&plan.required_foreshadowing));

    if let Some(controls) = controls {
        push_query_part(&mut parts, controls.chapter_intent.as_deref());
        push_query_part(&mut parts, controls.must_include_beats.as_deref());
        push_query_part(&mut parts, controls.forbidden_moves.as_deref());
        push_query_part(&mut parts, controls.style_emphasis.as_deref());
    }

    parts.join(" ")
}

fn graph_source_key(node_type: &str, id: &str) -> String {
    format!("{}:{}", node_type, id)
}

fn plan_field_contains(field: &str, needle: &str) -> bool {
    if needle.trim().is_empty() {
        return false;
    }
    field.to_lowercase().contains(&needle.to_lowercase())
}

fn node_matches_plan(
    node: &crate::db::knowledge_graph::KnowledgeGraphNode,
    plan: &ChapterPlan,
) -> bool {
    match node.node_type.as_str() {
        "character" => {
            plan.pov_character_id
                .as_deref()
                .is_some_and(|pov| pov == node.id || plan_field_contains(pov, &node.label))
                || plan_field_contains(&plan.required_characters, &node.id)
                || plan_field_contains(&plan.required_characters, &node.label)
        }
        "location" => {
            plan_field_contains(&plan.required_locations, &node.id)
                || plan_field_contains(&plan.required_locations, &node.label)
        }
        "plot_thread" => {
            plan_field_contains(&plan.plot_goals, &node.id)
                || plan_field_contains(&plan.plot_goals, &node.label)
        }
        "foreshadowing" => {
            plan_field_contains(&plan.required_foreshadowing, &node.id)
                || plan_field_contains(&plan.required_foreshadowing, &node.label)
        }
        _ => false,
    }
}

pub fn build_graph_context(
    db: &Database,
    project_id: &str,
    plan: &ChapterPlan,
) -> Result<GraphContext, String> {
    let snapshot = crate::db::knowledge_graph::get_snapshot(db, project_id)?;
    let node_map = snapshot
        .nodes
        .iter()
        .map(|node| (graph_source_key(&node.node_type, &node.id), node.clone()))
        .collect::<HashMap<_, _>>();

    let seeds = snapshot
        .nodes
        .iter()
        .filter(|node| node_matches_plan(node, plan))
        .map(|node| GraphContextNode {
            id: node.id.clone(),
            node_type: node.node_type.clone(),
            label: node.label.clone(),
        })
        .collect::<Vec<_>>();
    let seed_keys = seeds
        .iter()
        .map(|seed| graph_source_key(&seed.node_type, &seed.id))
        .collect::<HashSet<_>>();

    let mut neighbors = Vec::new();
    let mut source_keys = seed_keys.iter().cloned().collect::<HashSet<_>>();
    let mut visited = seed_keys.clone();
    let mut edges = snapshot.edges;
    edges.sort_by(|a, b| {
        graph_source_key(&a.source_node_type, &a.source_node_id)
            .cmp(&graph_source_key(&b.source_node_type, &b.source_node_id))
            .then(
                graph_source_key(&a.target_node_type, &a.target_node_id)
                    .cmp(&graph_source_key(&b.target_node_type, &b.target_node_id)),
            )
            .then(a.edge_type.cmp(&b.edge_type))
    });
    let mut frontier = seed_keys.iter().cloned().collect::<Vec<_>>();
    frontier.sort();

    'expand: for depth in 1..=GRAPH_CONTEXT_MAX_HOPS {
        let mut next_frontier = Vec::new();
        for current_key in &frontier {
            let Some(via_node) = node_map.get(current_key) else {
                continue;
            };
            for edge in &edges {
                if neighbors.len() >= GRAPH_CONTEXT_MAX_NEIGHBORS {
                    break 'expand;
                }

                let source_key = graph_source_key(&edge.source_node_type, &edge.source_node_id);
                let target_key = graph_source_key(&edge.target_node_type, &edge.target_node_id);
                let (neighbor_key, direction) = if source_key == *current_key {
                    (target_key, "outgoing")
                } else if target_key == *current_key {
                    (source_key, "incoming")
                } else {
                    continue;
                };

                if visited.contains(&neighbor_key) {
                    continue;
                }
                let Some(neighbor_node) = node_map.get(&neighbor_key) else {
                    continue;
                };
                visited.insert(neighbor_key.clone());
                source_keys.insert(neighbor_key.clone());
                next_frontier.push(neighbor_key);
                neighbors.push(GraphContextNeighbor {
                    id: neighbor_node.id.clone(),
                    node_type: neighbor_node.node_type.clone(),
                    label: neighbor_node.label.clone(),
                    edge_type: edge.edge_type.clone(),
                    direction: direction.into(),
                    depth,
                    via_id: via_node.id.clone(),
                    via_type: via_node.node_type.clone(),
                    via_label: via_node.label.clone(),
                    description: edge.description.clone(),
                });
            }
        }
        next_frontier.sort();
        next_frontier.dedup();
        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    neighbors.sort_by(|a, b| {
        a.depth.cmp(&b.depth).then(
            a.via_label
                .cmp(&b.via_label)
                .then(a.edge_type.cmp(&b.edge_type))
                .then(a.label.cmp(&b.label)),
        )
    });
    let mut source_keys = source_keys.into_iter().collect::<Vec<_>>();
    source_keys.sort();

    let mut summary = String::new();
    for neighbor in &neighbors {
        let item = format!(
            "{} {} {} (hop {}, {})",
            neighbor.via_label,
            neighbor.edge_type,
            neighbor.label,
            neighbor.depth,
            neighbor.direction
        );
        let candidate = if summary.is_empty() {
            item
        } else {
            format!("{}; {}", summary, item)
        };
        if crate::db::generation_jobs::estimate_tokens(&candidate)
            > GRAPH_CONTEXT_SUMMARY_TOKEN_BUDGET
        {
            break;
        }
        summary = candidate;
    }

    Ok(GraphContext {
        seeds,
        neighbors,
        source_keys,
        summary,
    })
}

pub fn rerank_retrieval_with_graph_context(
    mut retrieval: Vec<VectorDocument>,
    graph_context: &GraphContext,
) -> Vec<VectorDocument> {
    let graph_keys = graph_context
        .source_keys
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    for doc in &mut retrieval {
        let Some(source_id) = doc.source_id.as_deref() else {
            continue;
        };
        let key = graph_source_key(&doc.source_type, source_id);
        if graph_keys.contains(&key) {
            let base = doc.similarity.unwrap_or(0.0);
            doc.similarity = Some((base + 0.18).min(1.0));
        }
    }

    retrieval.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    retrieval
}

pub fn build_writing_context(
    db: &Database,
    project: &Project,
    plan: &ChapterPlan,
    canon: &BibleData,
    settings: &AppSettings,
    retrieval: Vec<VectorDocument>,
    controls: Option<OperatorControls>,
) -> Result<WritingContextPackage, String> {
    let chapters = crate::db::chapters::get_chapters(db, &project.id)?;
    let mut recent_summaries = Vec::new();
    let mut recent_body_excerpts = Vec::new();

    for chapter in chapters.iter().rev().take(5) {
        recent_summaries.push(json!({
            "sequence": chapter.sequence,
            "title": chapter.title,
            "summary": chapter.summary,
            "status": chapter.status,
        }));
    }

    for chapter in chapters.iter().rev().take(2) {
        if let Some(version) = crate::db::chapters::get_latest_version(db, &chapter.id)? {
            if let Some(body) = version.body_markdown {
                let excerpt = body.chars().take(2400).collect::<String>();
                recent_body_excerpts.push(json!({
                    "sequence": chapter.sequence,
                    "title": chapter.title,
                    "excerpt": excerpt,
                }));
            }
        }
    }

    let previous_ending_hook = recent_body_excerpts
        .first()
        .and_then(|item| item["excerpt"].as_str())
        .map(|text| {
            let chars = text.chars().collect::<Vec<_>>();
            let start = chars.len().saturating_sub(160);
            chars[start..].iter().collect::<String>()
        })
        .unwrap_or_default();

    let character_states =
        crate::db::bible::get_character_states(db, &project.id).unwrap_or_default();
    let learned_patterns =
        crate::workflow::learning::get_top_learning_entries(db, &project.id, 8).unwrap_or_default();
    let learning_entry_context_ids = learned_patterns
        .iter()
        .map(|entry| format!("learning_entry:{}", entry.id))
        .collect::<Vec<_>>();

    let graph_context = build_graph_context(db, &project.id, plan).unwrap_or_default();
    let retrieval = rerank_retrieval_with_graph_context(retrieval, &graph_context);
    let retrieval_trace = crate::db::vector_store::build_retrieval_trace(&retrieval);

    Ok(WritingContextPackage {
        project: json!({
            "id": project.id,
            "name": project.name,
            "genre": project.genre,
            "target_audience": project.target_audience,
            "quality_threshold": project.quality_threshold,
        }),
        chapter_plan: json!({
            "id": plan.id,
            "sequence": plan.sequence,
            "title": plan.title,
            "outline": plan.outline,
            "target_word_count": plan.target_word_count.unwrap_or(settings.daily_target_words),
            "pov_character": plan.pov_character_id,
            "required_characters": plan.required_characters,
            "required_locations": plan.required_locations,
            "plot_goals": plan.plot_goals,
            "required_foreshadowing": plan.required_foreshadowing,
        }),
        continuity: json!({
            "recent_summaries": recent_summaries,
            "recent_body_excerpts": recent_body_excerpts,
            "previous_ending_hook": previous_ending_hook,
            "timeline_events": &canon.timeline_events,
            "character_states": character_states,
        }),
        canon: json!({
            "characters": &canon.characters,
            "locations": &canon.locations,
            "organizations": &canon.organizations,
            "items": &canon.items,
            "magic_systems": &canon.magic_systems,
            "canon_rules": &canon.canon_rules,
            "active_plot_threads": &canon.plot_threads,
            "unresolved_foreshadowing": &canon.foreshadowing,
            "world_lore": &canon.world_lore,
        }),
        graph_context,
        retrieval,
        retrieval_trace,
        style: json!({
            "project_style_profile": project.style_profile,
            "style_guides": &canon.style_guides,
        }),
        learned_patterns,
        learning_entry_context_ids,
        operator_controls: controls.unwrap_or_default(),
    })
}
