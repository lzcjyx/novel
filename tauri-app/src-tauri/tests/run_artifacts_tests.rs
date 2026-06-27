use serde_json::json;
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("run-artifacts.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_plan_job(db: &Database) -> (String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Run Artifact Fixture",
        Some("inspectable runs"),
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
         VALUES ('plan-artifact', ?1, 1, 'Artifact', 'Export run artifacts', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(db, &project_id, "plan-artifact")
            .unwrap();
    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        db,
        &job_id,
        "generate_draft",
        "done",
        Some("draft created"),
        40.0,
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::record_job_model_usage(
        db,
        &job_id,
        "generate_draft",
        "openai_compat",
        "fixture-model",
        1200,
        800,
        Some(0.1),
        Some(0.2),
    )
    .unwrap();
    (project_id, job_id)
}

fn insert_audit_sidecar_fixture(db: &Database) -> String {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Audit Sidecar Fixture",
        Some("human readable sidecar"),
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
        "INSERT INTO characters (id, project_id, name, role, personality, status)
         VALUES ('char-audit', ?1, '沈砚', '主角', '查账人', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-audit-1', ?1, 1, '雨站旧账', '核对旧账', 3000, 'completed')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-audit-1', ?1, 'plan-audit-1', 1, '雨站旧账', 'final', 1200, '票据金额确定。')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
            (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-audit-1', 'chapter-audit-1', ?1, 1, 'final', '雨站旧账', '票面写着三百枚灵石。', '票据金额确定。', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO timeline_events
            (id, project_id, chapter_id, event_time_label, sequence, event_summary, involved_characters, involved_locations, consequences, status)
         VALUES ('timeline-audit-1', ?1, 'chapter-audit-1', '第一夜', 1, '沈砚确认旧账票据。', '[\"沈砚\"]', '[\"雨站\"]', '[\"票据金额锁定\"]', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);
    tauri_app_lib::db::hard_facts::upsert_hard_fact(
        db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-audit-1".to_string()),
            project_id: project_id.clone(),
            chapter_id: Some("chapter-audit-1".to_string()),
            chapter_version_id: Some("version-audit-1".to_string()),
            fact_type: "amount".to_string(),
            subject: "旧账票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "旧账票据金额为三百枚灵石".to_string(),
            certainty: 0.98,
            source_quote: Some("票面写着三百枚灵石。".to_string()),
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: json!({}),
        },
    )
    .unwrap();
    tauri_app_lib::db::style_assets::upsert_style_asset(
        db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-audit-1".to_string()),
            project_id: project_id.clone(),
            name: "克制悬疑".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: json!({"cadence": "short"}),
            positive_examples: vec!["他把伞沿压低。".to_string()],
            negative_examples: vec!["他心中五味杂陈。".to_string()],
            anti_ai_rules: json!({"forbidden_phrases": ["眼中闪过"]}),
            enabled: true,
            priority: 10,
            metadata: json!({}),
        },
    )
    .unwrap();
    project_id
}

#[test]
fn run_artifact_writer_exports_inspectable_files_and_records_metadata() {
    let db = setup_db();
    let (_project_id, job_id) = insert_project_plan_job(&db);
    let dir = tempfile::tempdir().unwrap();

    let manifest = tauri_app_lib::workflow::run_artifacts::write_run_artifacts(
        &db,
        &job_id,
        dir.path(),
        &tauri_app_lib::workflow::run_artifacts::RunArtifactPayload {
            system_prompt: "system prompt".to_string(),
            user_prompt: "user prompt".to_string(),
            context_package: json!({"chapter": "Artifact"}),
            context_trace: json!({"sources": ["hard_fact:1"]}),
            draft_markdown: "draft body".to_string(),
            reviews: vec![json!({"agent": "style", "score": 91})],
        },
    )
    .expect("artifacts should write");

    assert!(manifest.dir_path.ends_with(&job_id));
    for relative in [
        "status.json",
        "prompt/system.md",
        "prompt/user.md",
        "context/package.json",
        "context/trace.json",
        "output/draft.md",
        "reviews/review-001.json",
        "usage.json",
        "events.jsonl",
    ] {
        assert!(
            dir.path().join(&job_id).join(relative).exists(),
            "missing {relative}"
        );
    }

    let status = std::fs::read_to_string(dir.path().join(&job_id).join("status.json")).unwrap();
    assert!(status.contains("generate_draft"));
    let draft = std::fs::read_to_string(dir.path().join(&job_id).join("output/draft.md")).unwrap();
    assert_eq!(draft, "draft body");

    let jobs =
        tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &manifest.project_id).unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(
        metadata["run_artifacts"]["dir_path"].as_str(),
        Some(manifest.dir_path.as_str())
    );

    let artifact_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM run_artifacts WHERE job_id = ?1",
            [job_id],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(artifact_count, 1);
}

#[test]
fn run_artifact_manifest_records_extension_export_templates() {
    let db = setup_db();
    let (_project_id, job_id) = insert_project_plan_job(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE generation_jobs SET metadata = ?1 WHERE id = ?2",
            rusqlite::params![
                json!({
                    "extension_contributions": {
                        "export_template": [{
                            "extension_id": "export.pack",
                            "contribution_id": "markdown-audit",
                            "payload": {
                                "template_id": "markdown-audit",
                                "format": "markdown"
                            }
                        }]
                    }
                })
                .to_string(),
                job_id
            ],
        )
        .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();

    tauri_app_lib::workflow::run_artifacts::write_run_artifacts(
        &db,
        &job_id,
        dir.path(),
        &tauri_app_lib::workflow::run_artifacts::RunArtifactPayload {
            system_prompt: "system prompt".to_string(),
            user_prompt: "user prompt".to_string(),
            context_package: json!({}),
            context_trace: json!({}),
            draft_markdown: "draft body".to_string(),
            reviews: vec![],
        },
    )
    .expect("artifacts should write");

    let manifest: serde_json::Value = {
        let conn = db.conn.lock().unwrap();
        let raw: String = conn
            .query_row(
                "SELECT manifest FROM run_artifacts WHERE job_id = ?1",
                [job_id],
                |row| row.get(0),
            )
            .unwrap();
        serde_json::from_str(&raw).unwrap()
    };

    assert_eq!(
        manifest["export_templates"][0]["template_id"].as_str(),
        Some("markdown-audit")
    );
}

#[test]
fn run_artifact_writer_records_metadata_when_file_export_fails() {
    let db = setup_db();
    let (project_id, job_id) = insert_project_plan_job(&db);
    let dir = tempfile::tempdir().unwrap();
    let file_base = dir.path().join("not-a-directory");
    std::fs::write(&file_base, "blocking file").unwrap();

    let err = tauri_app_lib::workflow::run_artifacts::write_run_artifacts(
        &db,
        &job_id,
        &file_base,
        &tauri_app_lib::workflow::run_artifacts::RunArtifactPayload {
            system_prompt: "system prompt".to_string(),
            user_prompt: "user prompt".to_string(),
            context_package: json!({}),
            context_trace: json!({}),
            draft_markdown: "draft body".to_string(),
            reviews: vec![],
        },
    )
    .expect_err("artifact export should fail when base path is a file");
    assert!(err.contains("Create artifact dir"));

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(metadata["run_artifacts"]["status"].as_str(), Some("failed"));
    assert!(metadata["run_artifacts"]["error_message"]
        .as_str()
        .unwrap_or("")
        .contains("Create artifact dir"));
}

#[test]
fn audit_sidecar_exports_human_readable_project_state() {
    let db = setup_db();
    let project_id = insert_audit_sidecar_fixture(&db);
    let dir = tempfile::tempdir().unwrap();

    let manifest = tauri_app_lib::export::audit::export_audit_sidecar(&db, &project_id, dir.path())
        .expect("audit sidecar should export");

    for relative in [
        "bible/project.md",
        "bible/characters.md",
        "state/chapter-01/final.md",
        "state/chapter-01/summary.md",
        "timeline/history.md",
        "memory/hard-facts.md",
        "memory/style-assets.md",
    ] {
        assert!(
            dir.path().join(&manifest.dir_name).join(relative).exists(),
            "missing {relative}"
        );
    }

    let project =
        std::fs::read_to_string(dir.path().join(&manifest.dir_name).join("bible/project.md"))
            .unwrap();
    assert!(project.contains("Audit Sidecar Fixture"));

    let chapter = std::fs::read_to_string(
        dir.path()
            .join(&manifest.dir_name)
            .join("state/chapter-01/final.md"),
    )
    .unwrap();
    assert!(chapter.contains("票面写着三百枚灵石"));

    let hard_facts = std::fs::read_to_string(
        dir.path()
            .join(&manifest.dir_name)
            .join("memory/hard-facts.md"),
    )
    .unwrap();
    assert!(hard_facts.contains("旧账票据"));
    assert!(hard_facts.contains("三百枚灵石"));
}
