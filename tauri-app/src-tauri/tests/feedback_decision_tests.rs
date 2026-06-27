use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("feedback-decisions.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_chapter_feedback(db: &Database) -> (String, String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Feedback Fixture",
        Some("feedback decision loop"),
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
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-feedback', ?1, 1, '反馈章节', '读者反馈修订', 3000, 'completed')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-feedback', ?1, 'plan-feedback', 1, '反馈章节', 'final', 1200, '原章节摘要')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-feedback-1', 'chapter-feedback', ?1, 1, 'final', '反馈章节', '原正文。', '原章节摘要', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = 'version-feedback-1' WHERE id = 'chapter-feedback'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO reader_feedback
         (id, project_id, chapter_id, source, external_id, rating, comment_text, sentiment, metadata)
         VALUES ('feedback-1', ?1, 'chapter-feedback', 'reader', 'comment-1', 4.0, '第二段动机不清楚。', 'mixed', '{}')",
        rusqlite::params![project_id],
    )
    .unwrap();
    (
        project_id,
        "chapter-feedback".to_string(),
        "feedback-1".to_string(),
    )
}

#[test]
fn feedback_revision_candidate_stays_pending_until_approved() {
    let db = setup_db();
    let (project_id, chapter_id, feedback_id) = insert_project_chapter_feedback(&db);

    let decision_id =
        tauri_app_lib::workflow::feedback_decisions::create_feedback_revision_candidate(
            &db,
            &tauri_app_lib::workflow::feedback_decisions::FeedbackRevisionCandidateInput {
                id: Some("decision-1".to_string()),
                project_id: project_id.clone(),
                feedback_id: feedback_id.clone(),
                chapter_id: chapter_id.clone(),
                title: "反馈章节 修订候选".to_string(),
                body_markdown: "修订候选正文。".to_string(),
                summary: Some("候选摘要".to_string()),
                metadata: serde_json::json!({"fixture": "feedback"}),
            },
        )
        .expect("candidate should persist");

    assert_eq!(decision_id, "decision-1");
    let decisions =
        tauri_app_lib::workflow::feedback_decisions::list_feedback_decisions(&db, &project_id)
            .unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].status, "pending");
    assert!(decisions[0].resulting_chapter_version_id.is_none());

    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, "version-feedback-1");
    assert_eq!(latest.body_markdown.as_deref(), Some("原正文。"));
}

#[test]
fn approving_feedback_revision_promotes_candidate_to_chapter_version() {
    let db = setup_db();
    let (project_id, chapter_id, feedback_id) = insert_project_chapter_feedback(&db);
    let decision_id =
        tauri_app_lib::workflow::feedback_decisions::create_feedback_revision_candidate(
            &db,
            &tauri_app_lib::workflow::feedback_decisions::FeedbackRevisionCandidateInput {
                id: Some("decision-approve".to_string()),
                project_id: project_id.clone(),
                feedback_id,
                chapter_id: chapter_id.clone(),
                title: "反馈章节 修订候选".to_string(),
                body_markdown: "修订后动机更清楚。".to_string(),
                summary: Some("修订摘要".to_string()),
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();

    let decision = tauri_app_lib::workflow::feedback_decisions::decide_feedback_revision(
        &db,
        &decision_id,
        tauri_app_lib::workflow::feedback_decisions::FeedbackDecisionAction::Approve,
        Some("accepted reader clarity fix"),
    )
    .expect("approval should create a chapter version");

    assert_eq!(decision.status, "approved");
    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.version_type, "accepted_candidate");
    assert_eq!(latest.body_markdown.as_deref(), Some("修订后动机更清楚。"));
    let metadata: serde_json::Value = serde_json::from_str(&latest.metadata).unwrap();
    assert_eq!(
        metadata["feedback_decision"]["decision_id"].as_str(),
        Some("decision-approve")
    );
    assert_eq!(
        decision.resulting_chapter_version_id.as_deref(),
        Some(latest.id.as_str())
    );
}

#[test]
fn rejected_feedback_revision_remains_searchable_without_promoting_content() {
    let db = setup_db();
    let (project_id, chapter_id, feedback_id) = insert_project_chapter_feedback(&db);
    let decision_id =
        tauri_app_lib::workflow::feedback_decisions::create_feedback_revision_candidate(
            &db,
            &tauri_app_lib::workflow::feedback_decisions::FeedbackRevisionCandidateInput {
                id: Some("decision-reject".to_string()),
                project_id: project_id.clone(),
                feedback_id,
                chapter_id: chapter_id.clone(),
                title: "反馈章节 被拒候选".to_string(),
                body_markdown: "不采用的修订。".to_string(),
                summary: None,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();

    let decision = tauri_app_lib::workflow::feedback_decisions::decide_feedback_revision(
        &db,
        &decision_id,
        tauri_app_lib::workflow::feedback_decisions::FeedbackDecisionAction::Reject,
        Some("does not fit canon"),
    )
    .unwrap();

    assert_eq!(decision.status, "rejected");
    assert!(decision.resulting_chapter_version_id.is_none());
    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, "version-feedback-1");
    let decisions =
        tauri_app_lib::workflow::feedback_decisions::list_feedback_decisions(&db, &project_id)
            .unwrap();
    assert_eq!(decisions[0].status, "rejected");
    assert_eq!(
        decisions[0].decision_note.as_deref(),
        Some("does not fit canon")
    );
}

#[test]
fn deferred_feedback_revision_remains_searchable_without_promoting_content() {
    let db = setup_db();
    let (project_id, chapter_id, feedback_id) = insert_project_chapter_feedback(&db);
    let decision_id =
        tauri_app_lib::workflow::feedback_decisions::create_feedback_revision_candidate(
            &db,
            &tauri_app_lib::workflow::feedback_decisions::FeedbackRevisionCandidateInput {
                id: Some("decision-defer".to_string()),
                project_id: project_id.clone(),
                feedback_id,
                chapter_id: chapter_id.clone(),
                title: "反馈章节 待定候选".to_string(),
                body_markdown: "稍后再判断的修订。".to_string(),
                summary: None,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();

    let decision = tauri_app_lib::workflow::feedback_decisions::decide_feedback_revision(
        &db,
        &decision_id,
        tauri_app_lib::workflow::feedback_decisions::FeedbackDecisionAction::Defer,
        Some("needs editor review"),
    )
    .unwrap();

    assert_eq!(decision.status, "deferred");
    assert!(decision.resulting_chapter_version_id.is_none());
    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, "version-feedback-1");
    let decisions =
        tauri_app_lib::workflow::feedback_decisions::list_feedback_decisions(&db, &project_id)
            .unwrap();
    assert_eq!(decisions[0].status, "deferred");
    assert_eq!(
        decisions[0].decision_note.as_deref(),
        Some("needs editor review")
    );
}
