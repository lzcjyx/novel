use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::{hard_fact_ledger, writing_context};

fn setup_db(name: &str) -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(name);
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_with_version(db: &Database) -> (String, String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Hard Fact Fixture",
        Some("fact fixture"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("quiet"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id;
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans
         (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-fact-1', ?1, 1, '旧票据', '发现灵税票据。', 3000, 'completed')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters
         (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-fact-1', ?1, 'plan-fact-1', 1, '旧票据', 'final', 1200, '票据金额确定。')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-fact-1', 'chapter-fact-1', ?1, 1, 'final', '旧票据', '票面写着三百枚灵石。', '票据金额确定。', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    (
        project_id,
        "chapter-fact-1".to_string(),
        "version-fact-1".to_string(),
    )
}

#[test]
fn hard_fact_round_trips_status_and_source_quote() {
    let db = setup_db("hard-fact-roundtrip.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);

    let fact_id = tauri_app_lib::db::hard_facts::upsert_hard_fact(
        &db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-1".to_string()),
            project_id: project_id.clone(),
            chapter_id: Some(chapter_id),
            chapter_version_id: Some(version_id),
            fact_type: "amount".to_string(),
            subject: "灵税票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "灵税票据金额为三百枚灵石".to_string(),
            certainty: 0.97,
            source_quote: Some("票面写着三百枚灵石。".to_string()),
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({"source": "chapter_final"}),
        },
    )
    .expect("hard fact should persist");

    let facts = tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, true)
        .expect("hard facts should load");

    assert_eq!(fact_id, "fact-1");
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].object, "三百枚灵石");
    assert_eq!(facts[0].status, "active");
    assert_eq!(
        facts[0].source_quote.as_deref(),
        Some("票面写着三百枚灵石。")
    );
    assert_eq!(facts[0].metadata["source"].as_str(), Some("chapter_final"));
}

#[test]
fn hard_fact_active_filter_excludes_superseded_facts() {
    let db = setup_db("hard-fact-active-filter.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);

    for (id, status) in [("fact-active", "active"), ("fact-old", "superseded")] {
        tauri_app_lib::db::hard_facts::upsert_hard_fact(
            &db,
            &tauri_app_lib::db::hard_facts::HardFactInput {
                id: Some(id.to_string()),
                project_id: project_id.clone(),
                chapter_id: Some(chapter_id.clone()),
                chapter_version_id: Some(version_id.clone()),
                fact_type: "ownership".to_string(),
                subject: "红伞".to_string(),
                predicate: "owned_by".to_string(),
                object: "林白".to_string(),
                value_text: "红伞归林白持有".to_string(),
                certainty: 0.9,
                source_quote: None,
                scope: "project".to_string(),
                status: status.to_string(),
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let active = tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, true).unwrap();
    let all = tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, false).unwrap();

    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, "fact-active");
    assert_eq!(all.len(), 2);
}

#[test]
fn select_relevant_hard_facts_matches_plan_and_operator_notes() {
    let db = setup_db("hard-fact-relevance.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, target_word_count, required_characters, required_locations, status)
             VALUES ('plan-next-fact', ?1, 2, '追查灵税票据', '沈砚去旧站核对票据金额。', 3000, '[\"沈砚\"]', '[\"旧站\"]', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    for (id, subject, object) in [
        ("fact-ticket", "灵税票据", "三百枚灵石"),
        ("fact-umbrella", "红伞", "林白"),
    ] {
        tauri_app_lib::db::hard_facts::upsert_hard_fact(
            &db,
            &tauri_app_lib::db::hard_facts::HardFactInput {
                id: Some(id.to_string()),
                project_id: project_id.clone(),
                chapter_id: Some(chapter_id.clone()),
                chapter_version_id: Some(version_id.clone()),
                fact_type: "amount".to_string(),
                subject: subject.to_string(),
                predicate: "records_amount".to_string(),
                object: object.to_string(),
                value_text: format!("{subject}金额为{object}"),
                certainty: 0.95,
                source_quote: None,
                scope: "project".to_string(),
                status: "active".to_string(),
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let controls = writing_context::OperatorControls {
        must_include_beats: Some("红伞必须作为线索出现".to_string()),
        ..Default::default()
    };
    let facts =
        hard_fact_ledger::select_relevant_hard_facts(&db, &project_id, &plan, Some(&controls), 8)
            .expect("facts should select");
    let ids = facts
        .iter()
        .map(|fact| fact.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["fact-ticket", "fact-umbrella"]);
}

#[test]
fn writing_context_includes_hard_facts_and_trace_entries() {
    let db = setup_db("hard-fact-context.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-context-fact', ?1, 2, '追查灵税票据', '核对三百枚灵石票据。', 3000, 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    tauri_app_lib::db::hard_facts::upsert_hard_fact(
        &db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-context".to_string()),
            project_id: project_id.clone(),
            chapter_id: Some(chapter_id),
            chapter_version_id: Some(version_id),
            fact_type: "amount".to_string(),
            subject: "灵税票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "灵税票据金额为三百枚灵石".to_string(),
            certainty: 0.97,
            source_quote: Some("票面写着三百枚灵石。".to_string()),
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let canon = tauri_app_lib::db::bible::get_bible(&db, &project_id).unwrap();
    let settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    let package = writing_context::build_writing_context(
        &db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    )
    .unwrap();
    let context_json = serde_json::to_value(package).unwrap();

    assert_eq!(
        context_json["hard_facts"][0]["id"].as_str(),
        Some("fact-context")
    );
    assert!(context_json["context_activation"]["source_trace"]
        .to_string()
        .contains("hard_fact:fact-context"));
}

#[test]
fn detect_hard_fact_contradictions_blocks_changed_amounts() {
    let db = setup_db("hard-fact-contradiction.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);
    tauri_app_lib::db::hard_facts::upsert_hard_fact(
        &db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-conflict".to_string()),
            project_id: project_id.clone(),
            chapter_id: Some(chapter_id),
            chapter_version_id: Some(version_id),
            fact_type: "amount".to_string(),
            subject: "灵税票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "灵税票据金额为三百枚灵石".to_string(),
            certainty: 0.97,
            source_quote: None,
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    let facts = tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, true).unwrap();

    let issues = hard_fact_ledger::detect_hard_fact_contradictions(
        "沈砚低头看见灵税票据上写着五百枚灵石。",
        &facts,
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].rule_type, "hard_fact_conflict");
    assert_eq!(issues[0].severity, "blocking");
    assert!(issues[0].evidence.contains("三百枚灵石"));
    assert!(issues[0].evidence.contains("五百枚灵石"));
}

#[test]
fn finalized_chapter_version_materializes_amount_hard_facts() {
    let db = setup_db("hard-fact-extract-final.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);

    let facts = hard_fact_ledger::materialize_hard_facts_from_chapter_version(
        &db,
        &project_id,
        &chapter_id,
        &version_id,
    )
    .expect("final version should materialize hard facts");

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].project_id, project_id);
    assert_eq!(facts[0].chapter_id.as_deref(), Some(chapter_id.as_str()));
    assert_eq!(
        facts[0].chapter_version_id.as_deref(),
        Some(version_id.as_str())
    );
    assert_eq!(facts[0].fact_type, "amount");
    assert_eq!(facts[0].subject, "旧票据");
    assert_eq!(facts[0].predicate, "records_amount");
    assert_eq!(facts[0].object, "三百枚灵石");
    assert_eq!(facts[0].status, "active");
    assert_eq!(facts[0].metadata["source"].as_str(), Some("chapter_final"));
}

#[test]
fn draft_chapter_version_does_not_materialize_hard_facts() {
    let db = setup_db("hard-fact-extract-draft.db");
    let (project_id, chapter_id, version_id) = insert_project_with_version(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE chapter_versions SET version_type = 'draft' WHERE id = ?1",
            rusqlite::params![version_id],
        )
        .unwrap();
    }

    let facts = hard_fact_ledger::materialize_hard_facts_from_chapter_version(
        &db,
        &project_id,
        &chapter_id,
        "version-fact-1",
    )
    .expect("draft version should be ignored");

    assert!(facts.is_empty());
    assert!(
        tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, false)
            .unwrap()
            .is_empty()
    );
}
