use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("draft-alternatives.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_and_plan(db: &Database) -> (String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Draft Alternatives",
        Some("candidate fixture"),
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
    let plan_id = "plan-candidates".to_string();
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES (?1, ?2, 1, '候选章节', '同一上下文生成多个候选', 3000, 'planned')",
        rusqlite::params![plan_id, project_id],
    )
    .unwrap();
    (project_id, plan_id)
}

#[test]
fn selecting_draft_candidate_promotes_it_to_accepted_chapter_version() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_and_plan(&db);

    let first = tauri_app_lib::db::draft_alternatives::create_draft_candidate(
        &db,
        &tauri_app_lib::db::draft_alternatives::DraftCandidateInput {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id.clone(),
            candidate_number: 1,
            title: "冷开场".to_string(),
            body_markdown: "第一版候选正文，不应该被自动写入 chapters。".to_string(),
            summary: Some("第一版".to_string()),
            word_count: 1200,
            prompt_hash: "prompt-a".to_string(),
            context_hash: "context-a".to_string(),
            model_profile_id: Some("profile-draft".to_string()),
            review_notes: serde_json::json!({"style": "quiet"}),
            estimated_cost_usd: Some(0.12),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    let second = tauri_app_lib::db::draft_alternatives::create_draft_candidate(
        &db,
        &tauri_app_lib::db::draft_alternatives::DraftCandidateInput {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id.clone(),
            candidate_number: 2,
            title: "强冲突开场".to_string(),
            body_markdown: "第二版候选正文，等待操作者选择。".to_string(),
            summary: Some("第二版".to_string()),
            word_count: 1300,
            prompt_hash: "prompt-a".to_string(),
            context_hash: "context-a".to_string(),
            model_profile_id: Some("profile-draft".to_string()),
            review_notes: serde_json::json!({"style": "tense"}),
            estimated_cost_usd: Some(0.13),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    tauri_app_lib::db::draft_alternatives::select_draft_candidate(&db, &second, "更符合本章意图")
        .unwrap();

    let candidates =
        tauri_app_lib::db::draft_alternatives::list_draft_candidates(&db, &plan_id).unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, first);
    assert_eq!(candidates[0].status, "rejected");
    assert_eq!(candidates[1].id, second);
    assert_eq!(candidates[1].status, "selected");
    assert_eq!(
        candidates[1].selection_reason.as_deref(),
        Some("更符合本章意图")
    );

    let chapters = tauri_app_lib::db::chapters::get_chapters(&db, &project_id).unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(
        chapters[0].chapter_plan_id.as_deref(),
        Some(plan_id.as_str())
    );
    assert_eq!(chapters[0].title.as_deref(), Some("强冲突开场"));
    assert_eq!(chapters[0].final_version_id.as_deref().is_some(), true);

    let accepted_version = tauri_app_lib::db::chapters::get_latest_version(&db, &chapters[0].id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted_version.version_type, "accepted_candidate");
    assert_eq!(accepted_version.prompt_hash.as_deref(), Some("prompt-a"));
    assert_eq!(accepted_version.context_hash.as_deref(), Some("context-a"));
    assert_eq!(
        accepted_version.created_by_agent.as_deref(),
        Some("draft_candidate_selector")
    );
    assert!(accepted_version
        .body_markdown
        .as_deref()
        .unwrap_or("")
        .contains("第二版候选正文"));
    let metadata: serde_json::Value = serde_json::from_str(&accepted_version.metadata).unwrap();
    assert_eq!(
        metadata["draft_candidate_id"].as_str(),
        Some(second.as_str())
    );
    assert_eq!(
        metadata["selection_reason"].as_str(),
        Some("更符合本章意图")
    );
}
