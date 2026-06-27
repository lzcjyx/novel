use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("memory-banks.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_memory_bank_fixture(db: &Database) -> String {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Memory Bank Test",
        Some("memory banks"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("cold"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id;

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO characters (id, project_id, name, role, status)
             VALUES ('char-memory-1', ?1, '沈砚', 'protagonist', 'active')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, sequence, title, status)
             VALUES ('chapter-memory-1', ?1, 1, '雨站旧账', 'final')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO character_states
                (id, project_id, character_id, after_chapter_id, physical_state, emotional_state, knowledge_state)
             VALUES ('state-memory-1', ?1, 'char-memory-1', 'chapter-memory-1', '外套湿透', '警觉', '知道票据金额')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO timeline_events
                (id, project_id, event_time_label, sequence, event_summary, involved_characters, involved_locations, consequences, status)
             VALUES ('timeline-memory-1', ?1, '第一夜', 1, '沈砚确认旧账票据。', '[\"沈砚\"]', '[\"雨站\"]', '[\"票据金额锁定\"]', 'active')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO learning_entries
                (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
             VALUES ('learn-memory-1', ?1, 'manual', '样章', 'style', '克制动作', '用动作替代解释', 0.91)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::db::hard_facts::upsert_hard_fact(
        db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-memory-1".to_string()),
            project_id: project_id.clone(),
            chapter_id: None,
            chapter_version_id: None,
            fact_type: "amount".to_string(),
            subject: "旧账票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "旧账票据金额为三百枚灵石".to_string(),
            certainty: 0.97,
            source_quote: Some("票面写着三百枚灵石。".to_string()),
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    tauri_app_lib::db::style_assets::upsert_style_asset(
        db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-memory-1".to_string()),
            project_id: project_id.clone(),
            name: "克制悬疑".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: serde_json::json!({"cadence": "short"}),
            positive_examples: vec!["他把杯口转向墙角。".to_string()],
            negative_examples: vec![],
            anti_ai_rules: serde_json::json!({}),
            enabled: true,
            priority: 10,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    project_id
}

#[test]
fn memory_banks_snapshot_exposes_author_editable_sources() {
    let db = setup_db();
    let project_id = insert_memory_bank_fixture(&db);

    let snapshot =
        tauri_app_lib::workflow::memory_banks::build_author_memory_banks(&db, &project_id)
            .expect("memory banks should build");
    let snapshot_json = serde_json::to_value(&snapshot).unwrap();

    for bank_id in [
        "canon",
        "hard_facts",
        "character_state",
        "timeline",
        "learning_entries",
        "style_assets",
    ] {
        let bank = snapshot_json["banks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|bank| bank["id"] == bank_id)
            .unwrap_or_else(|| panic!("missing memory bank {bank_id}"));
        assert!(bank["entries"].as_array().unwrap().len() >= 1);
        assert!(bank["entries"][0]["source_key"]
            .as_str()
            .unwrap()
            .contains(':'));
        assert!(
            bank["entries"][0]["edit_command"].as_str().is_some(),
            "bank {bank_id} should expose the backend command used for edits"
        );
    }

    let hard_fact_entry = snapshot_json["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == "hard_facts")
        .and_then(|bank| bank["entries"].as_array())
        .unwrap()
        .iter()
        .find(|entry| entry["source_key"] == "hard_fact:fact-memory-1")
        .unwrap();
    assert_eq!(
        hard_fact_entry["edit_command"].as_str(),
        Some("upsert_hard_fact")
    );

    let character_state_entry = snapshot_json["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == "character_state")
        .and_then(|bank| bank["entries"].as_array())
        .unwrap()
        .iter()
        .find(|entry| entry["source_key"] == "character_state:state-memory-1")
        .unwrap();
    assert_eq!(
        character_state_entry["edit_command"].as_str(),
        Some("update_bible_entry")
    );
    assert_eq!(
        character_state_entry["metadata"]["table"].as_str(),
        Some("character_states")
    );

    let timeline_entry = snapshot_json["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == "timeline")
        .and_then(|bank| bank["entries"].as_array())
        .unwrap()
        .iter()
        .find(|entry| entry["source_key"] == "timeline_event:timeline-memory-1")
        .unwrap();
    assert_eq!(
        timeline_entry["edit_command"].as_str(),
        Some("update_bible_entry")
    );
    assert_eq!(
        timeline_entry["metadata"]["table"].as_str(),
        Some("timeline_events")
    );

    let style_entry = snapshot_json["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == "style_assets")
        .and_then(|bank| bank["entries"].as_array())
        .unwrap()
        .iter()
        .find(|entry| entry["source_key"] == "style_asset:style-memory-1")
        .unwrap();
    assert_eq!(
        style_entry["edit_command"].as_str(),
        Some("upsert_style_asset")
    );
}
