use tauri_app_lib::db::connection::Database;
use tauri_app_lib::models::VectorDocument;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("graph-rag.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Graph RAG Test",
        Some("graph guided retrieval"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("cold"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

fn seed_plan_and_graph(db: &Database, project_id: &str) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans
         (id, project_id, sequence, title, outline, target_word_count, required_characters, required_locations, plot_goals, status)
         VALUES ('plan-graph-rag', ?1, 3, '旧车站回声', '林白回到旧车站追查失踪怀表', 3000, '林白', '', '找到失踪怀表', 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
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
    drop(conn);

    tauri_app_lib::db::knowledge_graph::create_edge(
        db,
        project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "investigates",
        Some("林白在旧车站追查失踪怀表"),
    )
    .unwrap();
}

fn retrieval_doc(
    id: &str,
    source_type: &str,
    source_id: &str,
    title: &str,
    similarity: f64,
) -> VectorDocument {
    VectorDocument {
        id: id.into(),
        project_id: "project-a".into(),
        source_type: source_type.into(),
        source_id: Some(source_id.into()),
        title: Some(title.into()),
        content: format!("{} context", title),
        content_hash: format!("hash-{id}"),
        metadata: "{}".into(),
        created_at: String::new(),
        updated_at: String::new(),
        embedding: None,
        similarity: Some(similarity),
    }
}

#[test]
fn graph_context_matches_plan_entities_and_summarizes_neighbors() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_plan_and_graph(&db, &project_id);
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();

    let graph_context =
        tauri_app_lib::workflow::writing_context::build_graph_context(&db, &project_id, &plan)
            .unwrap();

    assert_eq!(graph_context.seeds.len(), 1);
    assert_eq!(graph_context.seeds[0].id, "char-a");
    assert_eq!(graph_context.neighbors.len(), 1);
    assert_eq!(graph_context.neighbors[0].id, "loc-a");
    assert!(graph_context
        .source_keys
        .contains(&"character:char-a".to_string()));
    assert!(graph_context
        .source_keys
        .contains(&"location:loc-a".to_string()));
    assert!(graph_context.summary.contains("林白"));
    assert!(graph_context.summary.contains("旧车站"));
}

#[test]
fn graph_context_expands_two_hops_with_token_budget() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_plan_and_graph(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO items (id, project_id, name, item_type, description, status)
             VALUES ('item-watch', ?1, '失踪怀表', '线索', '藏在旧车站的关键物件', 'active')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "loc-a",
        "location",
        "item-watch",
        "item",
        "contains",
        Some("旧车站藏着失踪怀表"),
    )
    .unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();

    let graph_context =
        tauri_app_lib::workflow::writing_context::build_graph_context(&db, &project_id, &plan)
            .unwrap();

    let direct_neighbor = graph_context
        .neighbors
        .iter()
        .find(|neighbor| neighbor.id == "loc-a")
        .expect("one-hop location should remain included");
    let second_hop = graph_context
        .neighbors
        .iter()
        .find(|neighbor| neighbor.id == "item-watch")
        .expect("two-hop item should be included");

    assert_eq!(direct_neighbor.depth, 1);
    assert_eq!(second_hop.depth, 2);
    assert_eq!(second_hop.via_id, "loc-a");
    assert!(graph_context
        .source_keys
        .contains(&"item:item-watch".to_string()));
    assert!(graph_context.summary.contains("失踪怀表"));
    assert!(
        graph_context.neighbors.len()
            <= tauri_app_lib::workflow::writing_context::GRAPH_CONTEXT_MAX_NEIGHBORS
    );
    assert!(
        tauri_app_lib::db::generation_jobs::estimate_tokens(&graph_context.summary)
            <= tauri_app_lib::workflow::writing_context::GRAPH_CONTEXT_SUMMARY_TOKEN_BUDGET
    );
}

#[test]
fn writing_context_includes_graph_context_and_boosts_connected_sources() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_plan_and_graph(&db, &project_id);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let canon = tauri_app_lib::db::bible::get_bible(&db, &project_id).unwrap();
    let settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    let retrieval = vec![
        retrieval_doc(
            "doc-unrelated",
            "world_lore",
            "lore-x",
            "Unrelated ritual",
            0.91,
        ),
        retrieval_doc(
            "doc-location",
            "location",
            "loc-a",
            "Old station clue",
            0.84,
        ),
    ];

    let package = tauri_app_lib::workflow::writing_context::build_writing_context(
        &db, &project, &plan, &canon, &settings, retrieval, None,
    )
    .unwrap();

    assert_eq!(package.graph_context.neighbors[0].id, "loc-a");
    assert!(package.graph_context.summary.contains("investigates"));
    assert_eq!(package.retrieval[0].source_id.as_deref(), Some("loc-a"));
    assert_eq!(
        package.retrieval_trace.sources[0].source_id.as_deref(),
        Some("loc-a")
    );
}
