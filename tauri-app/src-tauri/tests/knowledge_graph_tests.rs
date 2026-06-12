use async_trait::async_trait;
use serde_json::{json, Value};
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("knowledge-graph.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "图谱测试",
        Some("测试关系图"),
        Some("悬疑"),
        None,
        Some("成人"),
        Some("冷峻"),
        Some("克制"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

fn seed_nodes(db: &Database, project_id: &str) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO characters (id, project_id, name, role, personality, status)
         VALUES ('char-a', ?1, '林白', '主角', '克制', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO locations (id, project_id, name, type, description, status)
         VALUES ('loc-a', ?1, '旧车站', '地点', '雨夜案发地', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
}

#[test]
fn graph_snapshot_derives_bible_nodes_and_degrees() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);

    tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "appears_at",
        Some("林白在旧车站找到线索"),
    )
    .unwrap();

    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert_eq!(snapshot.edges.len(), 1);
    assert!(snapshot
        .nodes
        .iter()
        .any(|n| n.id == "char-a" && n.node_type == "character" && n.degree == 1));
    assert!(snapshot
        .nodes
        .iter()
        .any(|n| n.id == "loc-a" && n.node_type == "location" && n.degree == 1));
    assert_eq!(snapshot.orphan_count, 0);
}

#[test]
fn graph_edge_creation_validates_and_delete_removes_edge() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);

    let invalid = tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "char-a",
        "character",
        "self",
        None,
    );
    assert!(invalid.is_err());

    let edge = tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "investigates",
        Some("手动添加关系"),
    )
    .unwrap();
    assert_eq!(edge.edge_type, "investigates");
    assert!(!edge.auto_inferred);
    let duplicate = tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "investigates",
        Some("重复关系不应创建第二条边"),
    )
    .unwrap();
    assert_eq!(duplicate.id, edge.id);

    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert_eq!(snapshot.edges.len(), 1);

    tauri_app_lib::db::knowledge_graph::delete_edge(&db, &edge.id).unwrap();
    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert!(snapshot.edges.is_empty());
}

#[test]
fn graph_node_neighborhood_returns_explicit_neighbors_and_retrieval_hints() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO items (id, project_id, name, item_type, description, status)
             VALUES ('item-a', ?1, '旧钥匙', '线索', '打开档案柜', 'active')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "appears_at",
        Some("林白在旧车站找到线索"),
    )
    .unwrap();
    tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "item-a",
        "item",
        "char-a",
        "character",
        "owned_by",
        Some("旧钥匙由林白保管"),
    )
    .unwrap();

    let neighborhood = tauri_app_lib::db::knowledge_graph::get_node_neighborhood(
        &db,
        &project_id,
        "char-a",
        "character",
    )
    .unwrap();

    assert_eq!(neighborhood.center.id, "char-a");
    assert_eq!(neighborhood.center.degree, 2);
    assert_eq!(neighborhood.edges.len(), 2);
    assert_eq!(neighborhood.neighbors.len(), 2);
    assert_eq!(neighborhood.retrieval_hints.source_key, "character:char-a");
    assert!(neighborhood
        .retrieval_hints
        .connected_source_keys
        .contains(&"location:loc-a".to_string()));
    assert!(neighborhood
        .retrieval_hints
        .connected_source_keys
        .contains(&"item:item-a".to_string()));
}

struct CanonGraphEdgeProvider;

#[async_trait]
impl ModelClient for CanonGraphEdgeProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({
            "chapter_summary": "林白在旧车站发现新线索。",
            "character_state_updates": [],
            "timeline_events": [],
            "new_lore": [],
            "foreshadowing_updates": [],
            "vector_documents": [],
            "knowledge_graph_edges": [
                {
                    "source_node_id": "char-a",
                    "source_node_type": "character",
                    "target_node_id": "loc-a",
                    "target_node_type": "location",
                    "edge_type": "investigates_at",
                    "description": "林白在旧车站调查雨夜案。",
                    "confidence": 0.72
                },
                {
                    "source_node_id": "char-a",
                    "source_node_type": "character",
                    "target_node_id": "missing-loc",
                    "target_node_type": "location",
                    "edge_type": "invalid_missing_target",
                    "description": "模型幻觉出的地点不应写入。",
                    "confidence": 0.95
                }
            ],
            "human_review_required": []
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

struct CanonGraphLabelEdgeProvider;

#[async_trait]
impl ModelClient for CanonGraphLabelEdgeProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({
            "chapter_summary": "林白在旧车站发现新线索。",
            "character_state_updates": [],
            "timeline_events": [],
            "new_lore": [],
            "foreshadowing_updates": [],
            "vector_documents": [],
            "knowledge_graph_edges": [
                {
                    "source_label": "林白",
                    "source_node_type": "character",
                    "target_label": "旧车站",
                    "target_node_type": "location",
                    "edge_type": "investigates_at",
                    "description": "林白在旧车站调查雨夜案。",
                    "confidence": 0.82
                }
            ],
            "human_review_required": []
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

struct CanonTimelineGraphProvider;

#[async_trait]
impl ModelClient for CanonTimelineGraphProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({
            "chapter_summary": "林白在旧车站发现怀表线索。",
            "character_state_updates": [],
            "timeline_events": [
                {
                    "event_time_label": "第一夜",
                    "sequence_hint": 1,
                    "event_summary": "林白在旧车站发现怀表线索。",
                    "involved_characters": ["林白"],
                    "involved_locations": ["旧车站"],
                    "consequences": ["旧案重新打开"],
                    "confidence": 0.9
                }
            ],
            "new_lore": [],
            "foreshadowing_updates": [],
            "vector_documents": [],
            "knowledge_graph_edges": [],
            "human_review_required": []
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[tokio::test]
async fn canon_update_persists_valid_ai_inferred_graph_edges() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, sequence, title, status, word_count)
             VALUES ('chapter-edge', ?1, 1, '旧车站回访', 'draft', 1200)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::workflow::canon_updater::update_canon_after_chapter(
        &db,
        &CanonGraphEdgeProvider,
        &project_id,
        "chapter-edge",
        &json!({
            "title": "旧车站回访",
            "body_markdown": "林白在旧车站调查雨夜案。",
            "summary": "发现旧车站线索。",
            "major_events": []
        }),
    )
    .await
    .unwrap();

    let edges = tauri_app_lib::db::knowledge_graph::get_edges(&db, &project_id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].source_node_id, "char-a");
    assert_eq!(edges[0].target_node_id, "loc-a");
    assert_eq!(edges[0].edge_type, "investigates_at");
    assert!(edges[0].auto_inferred);
    assert!((edges[0].confidence - 0.72).abs() < 0.000001);
}

#[tokio::test]
async fn canon_update_resolves_graph_edges_from_unique_node_labels() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, sequence, title, status, word_count)
             VALUES ('chapter-label-edge', ?1, 1, '旧车站回访', 'draft', 1200)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::workflow::canon_updater::update_canon_after_chapter(
        &db,
        &CanonGraphLabelEdgeProvider,
        &project_id,
        "chapter-label-edge",
        &json!({
            "title": "旧车站回访",
            "body_markdown": "林白在旧车站调查雨夜案。",
            "summary": "发现旧车站线索。",
            "major_events": []
        }),
    )
    .await
    .unwrap();

    let edges = tauri_app_lib::db::knowledge_graph::get_edges(&db, &project_id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].source_node_id, "char-a");
    assert_eq!(edges[0].target_node_id, "loc-a");
    assert_eq!(edges[0].edge_type, "investigates_at");
    assert!((edges[0].confidence - 0.82).abs() < 0.000001);
}

#[tokio::test]
async fn canon_update_creates_deterministic_edges_from_timeline_events() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, sequence, title, status, word_count)
             VALUES ('chapter-timeline-edge', ?1, 1, '旧车站回访', 'draft', 1200)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::workflow::canon_updater::update_canon_after_chapter(
        &db,
        &CanonTimelineGraphProvider,
        &project_id,
        "chapter-timeline-edge",
        &json!({
            "title": "旧车站回访",
            "body_markdown": "林白在旧车站发现怀表线索。",
            "summary": "发现怀表线索。",
            "major_events": []
        }),
    )
    .await
    .unwrap();

    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    let timeline = snapshot
        .nodes
        .iter()
        .find(|node| node.node_type == "timeline_event" && node.label.contains("怀表线索"))
        .expect("timeline event node should exist");
    assert!(snapshot.edges.iter().any(|edge| {
        edge.source_node_id == "char-a"
            && edge.source_node_type == "character"
            && edge.target_node_id == timeline.id
            && edge.target_node_type == "timeline_event"
            && edge.edge_type == "participates_in"
    }));
    assert!(snapshot.edges.iter().any(|edge| {
        edge.source_node_id == timeline.id
            && edge.source_node_type == "timeline_event"
            && edge.target_node_id == "loc-a"
            && edge.target_node_type == "location"
            && edge.edge_type == "occurs_at"
    }));
}
