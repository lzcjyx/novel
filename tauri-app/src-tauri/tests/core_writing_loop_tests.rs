use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{json, Value};
use tauri_app_lib::ai::client::{ModelClient, ModelUsageReport};
use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::prompt_rendering::{
    find_unresolved_placeholders, render_prompt_strict,
};

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("core-loop.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "核心闭环测试",
        Some("测试项目"),
        Some("悬疑"),
        None,
        Some("成人"),
        Some("冷峻"),
        Some("克制、具体、少套话"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn strict_prompt_rendering_rejects_unresolved_placeholders() {
    let vars = HashMap::from([("KNOWN", "value".to_string())]);

    let err = render_prompt_strict("demo", "A {{KNOWN}} and {{MISSING}}", &vars)
        .expect_err("unresolved placeholders must fail before model calls");

    assert!(err.contains("demo"));
    assert!(err.contains("MISSING"));
}

#[test]
fn strict_prompt_rendering_replaces_all_known_placeholders() {
    let vars = HashMap::from([
        ("PROJECT", "镜城".to_string()),
        ("CHAPTER", "第七章".to_string()),
    ]);

    let rendered = render_prompt_strict("demo", "{{PROJECT}} / {{CHAPTER}}", &vars)
        .expect("all placeholders are supplied");

    assert_eq!(rendered, "镜城 / 第七章");
    assert!(find_unresolved_placeholders(&rendered).is_empty());
}

#[test]
fn weekly_planner_prompt_is_registered_and_dedicated() {
    let prompt = tauri_app_lib::prompts::load_prompt("weekly_planner")
        .expect("weekly planner prompt should be registered");

    assert!(prompt.contains("weekly_planner"));
    assert!(prompt.contains("WEEKLY_PLANNER_CONTEXT_JSON"));
    assert!(!prompt.contains("style_reviewer"));
    assert!(!prompt.contains("review_arbiter"));
}

#[test]
fn learning_entries_can_be_selected_and_marked_used() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence, usage_count)
         VALUES
         ('learn-1', ?1, 'manual', '样章', 'style', '冷处理冲突', '用克制动作替代情绪解释', '他把杯口转向墙角。', '用于高压对话', 0.95, 0),
         ('learn-2', ?1, 'manual', '样章', 'dialogue', '半句台词', '角色说一半留一半', '你来晚了。', '用于悬念揭示', 0.90, 0)",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let entries = tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
        .expect("learning entries should load");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "learn-1");

    tauri_app_lib::workflow::learning::mark_learning_entries_used(&db, &["learn-1".to_string()])
        .expect("usage metadata should update");

    let updated = tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
        .expect("updated entries should load");
    let used = updated.iter().find(|entry| entry.id == "learn-1").unwrap();
    assert_eq!(used.usage_count, 1);
    assert!(used.last_used_at.is_some());
}

#[test]
fn writing_context_includes_recent_bodies_and_learning_patterns() {
    let db = setup_db();
    let project_id = insert_project(&db);

    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-1', ?1, 1, '门后的人', '主角发现旧案线索', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-prev', ?1, 0, '雨夜旧案', 'final', 1200, '上一章主角找到带血钥匙')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-prev', 'chapter-prev', ?1, 1, 'final', '雨夜旧案', '雨水敲在铁门上。钥匙在掌心发冷。最后，他听见门后有人叫出了他的旧名。', '上一章主角找到带血钥匙', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = 'version-prev' WHERE id = 'chapter-prev'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
         VALUES ('learn-style', ?1, 'manual', '样章', 'style', '冷硬细节', '用物件触感承载压力', 0.92)",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let canon = tauri_app_lib::db::bible::get_bible(&db, &project_id).unwrap();
    let settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();

    let package = tauri_app_lib::workflow::writing_context::build_writing_context(
        &db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    )
    .unwrap();

    let json = serde_json::to_value(&package).unwrap();
    assert!(json["continuity"]["recent_body_excerpts"]
        .to_string()
        .contains("门后有人叫出了他的旧名"));
    assert!(json["continuity"]["previous_ending_hook"]
        .as_str()
        .unwrap_or("")
        .contains("旧名"));
    assert!(json["learned_patterns"].to_string().contains("冷硬细节"));
}

#[test]
fn conflicting_generation_job_returns_existing_job_id() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, status)
         VALUES ('plan-job', ?1, 1, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let first =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-job")
            .unwrap();
    let second =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-job")
            .unwrap();

    assert_eq!(first, second);
}

#[test]
fn chapter_plan_can_be_marked_completed() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, status)
         VALUES ('plan-complete', ?1, 1, 'in_progress')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    tauri_app_lib::db::chapters::mark_chapter_plan_completed(&db, "plan-complete").unwrap();

    let plans = tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id).unwrap();
    assert_eq!(plans[0].status, "completed");
}

#[derive(Default)]
struct CapturingProvider {
    systems: Arc<Mutex<Vec<String>>>,
    users: Arc<Mutex<Vec<String>>>,
    review_text: Option<String>,
    usage: Option<ModelUsageReport>,
}

#[async_trait]
impl ModelClient for CapturingProvider {
    async fn generate_json(
        &self,
        system: &str,
        user: &str,
        _schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        self.systems.lock().unwrap().push(system.to_string());
        self.users.lock().unwrap().push(user.to_string());

        if system.contains("自我批评文学导师") {
            return Ok(json!([{
                "category": "improvement_note",
                "pattern_name": "减少解释",
                "pattern_description": "下一章减少动机解释，用动作和物件推进情绪。",
                "application_notes": "在关键对话后加入可见代价。"
            }]));
        }

        if system.contains("canon_extractor") {
            return Ok(json!({
                "chapter_summary": "最终修订稿进入圣经",
                "character_state_updates": [],
                "timeline_events": [],
                "new_lore": [],
                "foreshadowing_updates": [],
                "vector_documents": [],
                "human_review_required": []
            }));
        }

        Ok(json!({
            "title": "门后旧名",
            "body_markdown": "最终稿正文。门后的人没有现身，只把他的旧名写在潮湿墙面上。这个版本必须进入 canon。",
            "summary": "最终稿摘要",
            "word_count": 120,
            "pov_character": "主角",
            "major_events": ["旧名出现"],
            "character_state_changes": [],
            "timeline_events": [],
            "foreshadowing_used": [],
            "foreshadowing_planted": [],
            "new_canon_candidates": [],
            "continuity_notes": "下一章回应旧名",
            "used_context_ids": []
        }))
    }

    async fn generate_json_with_usage(
        &self,
        system: &str,
        user: &str,
        schema: &Value,
        max_tokens: u32,
    ) -> Result<(Value, Option<ModelUsageReport>), String> {
        let output = self.generate_json(system, user, schema, max_tokens).await?;
        Ok((output, self.usage.clone()))
    }

    async fn generate_text(
        &self,
        _system: &str,
        _user: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        if let Some(text) = &self.review_text {
            return Ok(text.clone());
        }
        Ok(r#"{"score":92,"pass":true,"blocking_issues":[],"minor_issues":[],"recommendations":[]}"#.into())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[tokio::test]
async fn chapter_pipeline_uses_writing_context_and_finalizes_plan() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-pipe', ?1, 1, '门后旧名', '调查门后声音', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
         VALUES ('learn-pipe', ?1, 'manual', '样章', 'style', '克制悬疑', '少解释，多用动作和物件', 0.91)",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);
    tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("ctx-chapter-1"),
        "旧名伏笔",
        "门后旧名相关伏笔，潮湿墙面上有人写下主角旧名。",
        r#"{"fixture":"core-writing-loop"}"#,
        &[0.1; 8],
    )
    .unwrap();

    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    settings.input_cost_per_million = Some(1.5);
    settings.output_cost_per_million = Some(6.0);
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    let provider = CapturingProvider {
        usage: Some(ModelUsageReport {
            prompt_tokens: Some(1111),
            completion_tokens: Some(222),
            total_tokens: Some(1333),
        }),
        ..Default::default()
    };
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(100);
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    let result = tauri_app_lib::workflow::chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project_id,
        true,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .unwrap();

    assert!(result.ok);

    let systems = provider.systems.lock().unwrap().join("\n---\n");
    assert!(!systems.contains("WRITING_CONTEXT_JSON"));
    assert!(systems.contains("克制悬疑"));
    assert!(systems.contains("门后旧名"));
    assert!(systems.contains("旧名伏笔"));
    assert!(!systems.contains("{{"));

    let latest_version = {
        let chapters = tauri_app_lib::db::chapters::get_chapters(&db, &project_id).unwrap();
        tauri_app_lib::db::chapters::get_latest_version(&db, &chapters[0].id)
            .unwrap()
            .unwrap()
    };
    let version_metadata: serde_json::Value = serde_json::from_str(&latest_version.metadata)
        .expect("chapter version metadata should be json");
    assert!(version_metadata["selected_retrieval_source_keys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key.as_str() == Some("chapter:ctx-chapter-1")));
    assert_eq!(
        version_metadata["retrieval_trace"]["sources"][0]["source_id"].as_str(),
        Some("ctx-chapter-1")
    );

    let plans = tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id).unwrap();
    assert_eq!(plans[0].status, "completed");

    let reflection_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM learning_entries WHERE project_id = ?1 AND source_type = 'self_reflection'",
            rusqlite::params![project_id],
            |row| row.get(0),
        ).unwrap()
    };
    assert_eq!(reflection_count, 1);

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let job_metadata: serde_json::Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let usage_events = job_metadata["model_usage_events"].as_array().unwrap();
    let draft_usage = usage_events
        .iter()
        .find(|event| event["phase"].as_str() == Some("generate_draft"))
        .expect("draft usage should be recorded");
    assert_eq!(draft_usage["usage_source"].as_str(), Some("provider"));
    assert_eq!(draft_usage["prompt_tokens"].as_i64(), Some(1111));
    assert_eq!(draft_usage["completion_tokens"].as_i64(), Some(222));
    assert_eq!(draft_usage["total_tokens"].as_i64(), Some(1333));
    assert_eq!(draft_usage["input_cost_per_million"].as_f64(), Some(1.5));
    assert_eq!(draft_usage["output_cost_per_million"].as_f64(), Some(6.0));
    assert!((draft_usage["estimated_cost_usd"].as_f64().unwrap() - 0.0029985).abs() < 0.000001);
    assert_eq!(
        job_metadata["usage_summary"]["provider_reported_call_count"].as_u64(),
        Some(1)
    );

    let mut events = Vec::new();
    let mut steps = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        steps.push(event.step.clone());
        events.push(event);
    }
    for expected in [
        "acquire_lock",
        "load_canon",
        "retrieve_context",
        "generate_draft",
        "aggregate_reviews",
        "export",
        "update_canon",
        "complete",
    ] {
        assert!(
            steps.iter().any(|step| step == expected),
            "missing pipeline step {expected}; got {steps:?}"
        );
    }

    let preview_event = events
        .iter()
        .find(|event| event.preview_kind.as_deref() == Some("draft"))
        .expect("draft preview event should be emitted");
    assert_eq!(preview_event.preview_title.as_deref(), Some("门后旧名"));
    assert!(preview_event
        .preview_text
        .as_deref()
        .unwrap_or("")
        .contains("最终稿正文"));
}

#[tokio::test]
async fn human_review_generation_keeps_plan_in_progress() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-human', ?1, 1, '待审旧名', '生成后进入人工复核', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let provider = CapturingProvider {
        review_text: Some(r#"{"score":92,"pass":false,"blocking_issues":[],"minor_issues":[],"recommendations":[]}"#.into()),
        ..Default::default()
    };
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(100);
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);

    let result = tauri_app_lib::workflow::chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project_id,
        true,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .unwrap();

    assert_eq!(result.decision.as_deref(), Some("needs_human_review"));
    let plans = tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id).unwrap();
    assert_eq!(plans[0].status, "in_progress");
}
