use serde_json::{json, Value};
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("job-observability.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Job Timeline Test",
        Some("phase observability"),
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

fn insert_plan(db: &Database, project_id: &str, plan_id: &str) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES (?1, ?2, 1, 'Observe', 'Measure job phases', 3000, 'planned')",
        rusqlite::params![plan_id, project_id],
    )
    .unwrap();
}

#[test]
fn model_pricing_settings_are_persisted_and_clearable() {
    let db = setup_db();
    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();

    assert_eq!(settings.input_cost_per_million, None);
    assert_eq!(settings.output_cost_per_million, None);

    settings.input_cost_per_million = Some(1.25);
    settings.output_cost_per_million = Some(5.75);
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    let loaded = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    assert_eq!(loaded.input_cost_per_million, Some(1.25));
    assert_eq!(loaded.output_cost_per_million, Some(5.75));

    let mut cleared = loaded;
    cleared.input_cost_per_million = None;
    cleared.output_cost_per_million = None;
    tauri_app_lib::db::settings::save_settings(&db, &cleared).unwrap();

    let loaded = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    assert_eq!(loaded.input_cost_per_million, None);
    assert_eq!(loaded.output_cost_per_million, None);
}

fn force_phase_started_offset(db: &Database, job_id: &str, offset_ms: i64) {
    let conn = db.conn.lock().unwrap();
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut metadata = serde_json::from_str::<Value>(&metadata_raw).unwrap_or_else(|_| json!({}));
    metadata["phase_started_at_ms"] = json!(chrono::Local::now()
        .timestamp_millis()
        .saturating_sub(offset_ms));
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1 WHERE id = ?2",
        rusqlite::params![metadata.to_string(), job_id],
    )
    .unwrap();
}

#[test]
fn job_phase_events_are_persisted_in_metadata() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-observe");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-observe")
            .unwrap();

    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "load_canon",
        "done",
        Some("3 chars"),
        10.0,
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "retrieve_context",
        "failed",
        Some("embedding timeout"),
        18.0,
    )
    .unwrap();

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let events = metadata["phase_events"].as_array().unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["step"], "load_canon");
    assert_eq!(events[0]["status"], "done");
    assert_eq!(events[1]["step"], "retrieve_context");
    assert_eq!(events[1]["status"], "failed");
    assert_eq!(events[1]["detail"], "embedding timeout");
    assert_eq!(metadata["phase_summary"]["last_step"], "retrieve_context");
    assert_eq!(metadata["phase_summary"]["last_status"], "failed");
    assert_eq!(metadata["phase_summary"]["phase_count"], 2);
    assert!(metadata["phase_summary"]["total_elapsed_ms"]
        .as_u64()
        .is_some());
}

#[test]
fn slow_phase_diagnostics_are_summarized() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-slow");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-slow")
            .unwrap();

    force_phase_started_offset(&db, &job_id, 5_000);
    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "load_canon",
        "done",
        Some("canon loaded"),
        10.0,
    )
    .unwrap();
    force_phase_started_offset(&db, &job_id, 45_000);
    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "generate_draft",
        "done",
        Some("draft complete"),
        50.0,
    )
    .unwrap();

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let events = metadata["phase_events"].as_array().unwrap();
    let summary = &metadata["phase_summary"];

    assert_eq!(events[1]["step"], "generate_draft");
    assert!(events[1]["duration_ms"].as_u64().unwrap() >= 30_000);
    assert_eq!(summary["slowest_step"], "generate_draft");
    assert!(summary["slowest_duration_ms"].as_u64().unwrap() >= 30_000);
    assert_eq!(summary["slow_step_count"], 1);
    assert_eq!(summary["slow_steps"][0]["step"], "generate_draft");
}

#[test]
fn job_status_update_records_failure_summary() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-failure");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-failure")
            .unwrap();

    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "generate_draft",
        "running",
        None,
        30.0,
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(
        &db,
        &job_id,
        "failed",
        Some("draft provider error"),
    )
    .unwrap();

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();

    assert_eq!(jobs[0].status, "failed");
    assert_eq!(metadata["phase_summary"]["last_status"], "failed");
    assert_eq!(
        metadata["phase_summary"]["failure_reason"],
        "draft provider error"
    );
    assert!(metadata["phase_summary"]["completed_at"].as_str().is_some());
}

#[test]
fn marking_latest_job_failed_records_failure_summary() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-stuck");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-stuck")
            .unwrap();
    tauri_app_lib::db::generation_jobs::record_job_phase_event(
        &db,
        &job_id,
        "aggregate_reviews",
        "running",
        Some("waiting on review agents"),
        65.0,
    )
    .unwrap();

    tauri_app_lib::db::generation_jobs::mark_latest_job_failed(
        &db,
        &project_id,
        "operator reset stuck job",
    )
    .unwrap();

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();

    assert_eq!(jobs[0].status, "failed");
    assert_eq!(metadata["phase_summary"]["last_step"], "aggregate_reviews");
    assert_eq!(metadata["phase_summary"]["last_status"], "failed");
    assert_eq!(
        metadata["phase_summary"]["failure_reason"],
        "operator reset stuck job"
    );
}

#[test]
fn stale_running_jobs_are_marked_failed_on_recovery() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-stale");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-stale")
            .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE generation_jobs
             SET updated_at = datetime('now', '-2 hours')
             WHERE id = ?1",
            rusqlite::params![job_id],
        )
        .unwrap();
    }

    let recovered = tauri_app_lib::db::generation_jobs::recover_stale_running_jobs(
        &db,
        600,
        "Application restarted while this generation job was still running.",
    )
    .unwrap();

    assert_eq!(recovered, 1);
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(jobs[0]
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("Application restarted"));
    assert_eq!(metadata["phase_summary"]["last_status"], "failed");
    assert!(metadata["phase_summary"]["failure_reason"]
        .as_str()
        .unwrap_or("")
        .contains("Application restarted"));
}

#[test]
fn interrupted_generation_recovery_rolls_back_task_owned_rows() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-rollback");
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-rollback",
    )
    .unwrap();

    tauri_app_lib::workflow::task_transaction::begin_generation_task_snapshot(
        &db,
        &job_id,
        &project_id,
        "plan-rollback",
    )
    .unwrap();

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE chapter_plans SET status = 'in_progress' WHERE id = 'plan-rollback'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status)
             VALUES ('chapter-owned', ?1, 'plan-rollback', 1, 'Owned Draft', 'draft')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title)
             VALUES ('version-owned', 'chapter-owned', ?1, 1, 'draft', 'Owned Draft')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::workflow::task_transaction::record_task_owned_row(
        &db,
        &job_id,
        "chapters",
        "chapter-owned",
    )
    .unwrap();
    tauri_app_lib::workflow::task_transaction::record_task_owned_row(
        &db,
        &job_id,
        "chapter_versions",
        "version-owned",
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    let recovered = tauri_app_lib::db::generation_jobs::recover_interrupted_generation_jobs(
        &db,
        0,
        "Application quit before this generation job completed.",
    )
    .unwrap();

    assert_eq!(recovered, 1);
    let conn = db.conn.lock().unwrap();
    let chapter_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM chapters WHERE id = 'chapter-owned'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let version_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM chapter_versions WHERE id = 'version-owned'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let plan_status: String = conn
        .query_row(
            "SELECT status FROM chapter_plans WHERE id = 'plan-rollback'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let job_status: String = conn
        .query_row(
            "SELECT status FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(chapter_count, 0);
    assert_eq!(version_count, 0);
    assert_eq!(plan_status, "planned");
    assert_eq!(job_status, "failed");
}

#[test]
fn reset_stuck_job_uses_recovery_and_restores_plan() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-reset-rollback");
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-reset-rollback",
    )
    .unwrap();
    tauri_app_lib::workflow::task_transaction::begin_generation_task_snapshot(
        &db,
        &job_id,
        &project_id,
        "plan-reset-rollback",
    )
    .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE chapter_plans SET status = 'in_progress' WHERE id = 'plan-reset-rollback'",
            [],
        )
        .unwrap();
    }
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    tauri_app_lib::db::generation_jobs::recover_project_interrupted_jobs(
        &db,
        &project_id,
        "operator reset stuck job",
    )
    .unwrap();

    let conn = db.conn.lock().unwrap();
    let plan_status: String = conn
        .query_row(
            "SELECT status FROM chapter_plans WHERE id = 'plan-reset-rollback'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let job_status: String = conn
        .query_row(
            "SELECT status FROM generation_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(plan_status, "planned");
    assert_eq!(job_status, "failed");
}

#[test]
fn interrupted_generation_recovery_restores_learning_usage_counters() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-learning-rollback");
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-learning-rollback",
    )
    .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO learning_entries
             (id, project_id, source_type, category, pattern_name, pattern_description, usage_count, last_used_at)
             VALUES ('learn-rollback', ?1, 'manual', 'style', 'before', 'before', 5, '2026-01-01 00:00:00')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    tauri_app_lib::workflow::task_transaction::begin_generation_task_snapshot(
        &db,
        &job_id,
        &project_id,
        "plan-learning-rollback",
    )
    .unwrap();
    tauri_app_lib::workflow::task_transaction::record_learning_entry_usage_snapshot(
        &db,
        &job_id,
        &["learn-rollback".to_string()],
    )
    .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE learning_entries
             SET usage_count = 6, last_used_at = '2026-06-12 12:00:00'
             WHERE id = 'learn-rollback'",
            [],
        )
        .unwrap();
    }
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    let recovered = tauri_app_lib::db::generation_jobs::recover_project_interrupted_jobs(
        &db,
        &project_id,
        "operator reset stuck job",
    )
    .unwrap();

    assert_eq!(recovered, 1);
    let conn = db.conn.lock().unwrap();
    let (usage_count, last_used_at): (i64, Option<String>) = conn
        .query_row(
            "SELECT usage_count, last_used_at FROM learning_entries WHERE id = 'learn-rollback'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(usage_count, 5);
    assert_eq!(last_used_at.as_deref(), Some("2026-01-01 00:00:00"));
}

#[test]
fn fresh_running_jobs_are_not_recovered_as_stale() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-fresh");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-fresh")
            .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    let recovered = tauri_app_lib::db::generation_jobs::recover_stale_running_jobs(
        &db,
        600,
        "Application restarted while this generation job was still running.",
    )
    .unwrap();

    assert_eq!(recovered, 0);
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "reviewing");
    assert_eq!(jobs[0].error_message, None);
}

#[test]
fn project_running_state_includes_active_sqlite_jobs() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-status");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-status")
            .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    assert!(tauri_app_lib::project_is_running(&db, false, &project_id).unwrap());

    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "completed", None).unwrap();

    assert!(!tauri_app_lib::project_is_running(&db, false, &project_id).unwrap());
    assert!(tauri_app_lib::project_is_running(&db, true, &project_id).unwrap());
}

#[test]
fn job_model_usage_events_are_persisted_and_summarized() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-usage");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-usage")
            .unwrap();

    tauri_app_lib::db::generation_jobs::record_job_model_usage(
        &db,
        &job_id,
        "generate_draft",
        "openai",
        "gpt-test",
        1200,
        800,
        Some(2.0),
        Some(8.0),
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::record_job_model_usage(
        &db,
        &job_id,
        "revise",
        "openai",
        "gpt-test",
        600,
        400,
        Some(2.0),
        Some(8.0),
    )
    .unwrap();

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let events = metadata["model_usage_events"].as_array().unwrap();
    let summary = &metadata["usage_summary"];

    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["phase"], "generate_draft");
    assert_eq!(summary["prompt_tokens"], 1800);
    assert_eq!(summary["completion_tokens"], 1200);
    assert_eq!(summary["total_tokens"], 3000);
    assert_eq!(summary["call_count"], 2);
    assert!((summary["estimated_cost_usd"].as_f64().unwrap() - 0.0132).abs() < 0.0001);
}
