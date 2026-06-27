use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::writing_context;

fn setup_db(name: &str) -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(name);
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_and_next_plan(db: &Database) -> String {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Compression Fixture",
        Some("compression fixture"),
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
         VALUES ('plan-compression-1', ?1, 1, '雨站旧账', '沈砚核对雨站旧账。', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    project_id
}

#[test]
fn context_compression_requires_approval_before_entering_writing_context() {
    let db = setup_db("context-compression.db");
    let project_id = insert_project_and_next_plan(&db);

    let summary_id = tauri_app_lib::db::context_compression::create_context_compression_summary(
        &db,
        &tauri_app_lib::db::context_compression::ContextCompressionSummaryInput {
            id: Some("compression-1".to_string()),
            project_id: project_id.clone(),
            source_job_id: Some("job-1".to_string()),
            summary_text: "上一轮长任务确认：旧账票据金额必须保持三百枚灵石。".to_string(),
            prompt_hash: Some("prompt-hash-1".to_string()),
            context_hash: Some("context-hash-1".to_string()),
            status: "draft".to_string(),
            metadata: serde_json::json!({"source": "long_task"}),
        },
    )
    .unwrap();
    assert_eq!(summary_id, "compression-1");

    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let canon = tauri_app_lib::db::bible::get_bible(&db, &project_id).unwrap();
    let settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();

    let draft_package = writing_context::build_writing_context(
        &db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    )
    .unwrap();
    let draft_json = serde_json::to_value(&draft_package).unwrap();
    assert_eq!(
        draft_json["continuity"]["context_compression_summaries"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    tauri_app_lib::db::context_compression::set_context_compression_status(
        &db,
        "compression-1",
        "approved",
    )
    .unwrap();
    let approved_package = writing_context::build_writing_context(
        &db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    )
    .unwrap();
    let approved_json = serde_json::to_value(&approved_package).unwrap();

    assert_eq!(
        approved_json["continuity"]["context_compression_summaries"][0]["id"].as_str(),
        Some("compression-1")
    );
    assert!(approved_json["context_activation"]["source_trace"]
        .to_string()
        .contains("context_compression:compression-1"));
}
