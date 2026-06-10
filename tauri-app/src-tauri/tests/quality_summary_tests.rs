use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("quality-summary.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Quality Test",
        Some("quality dashboard"),
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

fn seed_chapter(db: &Database, project_id: &str, sequence: i32) -> (String, String) {
    let plan_id = format!("plan-{}", sequence);
    let chapter_id = format!("chapter-{}", sequence);
    let version_id = format!("version-{}", sequence);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, status)
         VALUES (?1, ?2, ?3, 'completed')",
        rusqlite::params![plan_id, project_id, sequence],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES (?1, ?2, ?3, ?4, ?5, 'final', 3000, 'summary')",
        rusqlite::params![
            chapter_id,
            project_id,
            plan_id,
            sequence,
            format!("Chapter {}", sequence)
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count)
         VALUES (?1, ?2, ?3, 1, 'final', ?4, 'body', 3000)",
        rusqlite::params![version_id, chapter_id, project_id, format!("Chapter {}", sequence)],
    )
    .unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = ?1 WHERE id = ?2",
        rusqlite::params![version_id, chapter_id],
    )
    .unwrap();
    (chapter_id, version_id)
}

#[test]
fn project_quality_summary_aggregates_scores_and_decisions() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let (chapter_a, version_a) = seed_chapter(&db, &project_id, 1);
    let (chapter_b, version_b) = seed_chapter(&db, &project_id, 2);

    tauri_app_lib::db::reviews::save_review_scores(
        &db,
        &project_id,
        &chapter_a,
        &version_a,
        91.0,
        93.0,
        "publish_ready",
        true,
        0,
    )
    .unwrap();
    tauri_app_lib::db::reviews::save_review_scores(
        &db,
        &project_id,
        &chapter_b,
        &version_b,
        72.0,
        74.0,
        "revise",
        false,
        2,
    )
    .unwrap();

    let summary =
        tauri_app_lib::db::reviews::get_project_quality_summary(&db, &project_id).unwrap();

    assert_eq!(summary.reviewed_chapter_count, 2);
    assert_eq!(summary.publish_ready_count, 1);
    assert_eq!(summary.revise_count, 1);
    assert_eq!(summary.needs_human_review_count, 0);
    assert_eq!(summary.total_blocking_issues, 2);
    assert_eq!(summary.average_final_score.unwrap().round(), 84.0);
    assert_eq!(summary.latest_decision.as_deref(), Some("revise"));
    assert_eq!(summary.latest_final_score.unwrap().round(), 74.0);
}

#[test]
fn project_quality_summary_groups_agent_scores() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let (chapter_a, version_a) = seed_chapter(&db, &project_id, 1);
    let (chapter_b, version_b) = seed_chapter(&db, &project_id, 2);

    tauri_app_lib::db::reviews::save_agent_review(
        &db,
        &project_id,
        &chapter_a,
        &version_a,
        "continuity_reviewer",
        90,
        true,
        "[]",
        "[]",
        "[]",
        "{}",
    )
    .unwrap();
    tauri_app_lib::db::reviews::save_agent_review(
        &db,
        &project_id,
        &chapter_b,
        &version_b,
        "continuity_reviewer",
        70,
        false,
        r#"[{"issue":"timeline mismatch"}]"#,
        "[]",
        "[]",
        "{}",
    )
    .unwrap();
    tauri_app_lib::db::reviews::save_agent_review(
        &db,
        &project_id,
        &chapter_b,
        &version_b,
        "style_reviewer",
        88,
        true,
        "[]",
        "[]",
        "[]",
        "{}",
    )
    .unwrap();

    let summary =
        tauri_app_lib::db::reviews::get_project_quality_summary(&db, &project_id).unwrap();
    let continuity = summary
        .agent_scores
        .iter()
        .find(|agent| agent.agent_name == "continuity_reviewer")
        .unwrap();
    let style = summary
        .agent_scores
        .iter()
        .find(|agent| agent.agent_name == "style_reviewer")
        .unwrap();

    assert_eq!(continuity.review_count, 2);
    assert_eq!(continuity.average_score.unwrap().round(), 80.0);
    assert_eq!(continuity.pass_rate.unwrap(), 0.5);
    assert_eq!(continuity.blocking_issue_count, 1);
    assert_eq!(style.review_count, 1);
    assert_eq!(style.pass_rate.unwrap(), 1.0);
}
