use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{json, Value};
use tauri_app_lib::ai::client::{EmbeddingInputKind, ModelClient, ModelUsageReport};
use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::prompt_rendering::{
    find_unresolved_placeholders, render_prompt_strict,
};
use tauri_app_lib::workflow::review_agents::{self, CanonContext};

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("core-loop.db");
    let export_dir = dir.path().join("exports").to_string_lossy().to_string();
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    settings.data_dir = export_dir;
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();
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

fn review_chapter(project_id: &str) -> tauri_app_lib::models::Chapter {
    tauri_app_lib::models::Chapter {
        id: "chapter-review".to_string(),
        project_id: project_id.to_string(),
        chapter_plan_id: Some("plan-review".to_string()),
        sequence: 1,
        title: Some("评审章节".to_string()),
        final_version_id: Some("version-review".to_string()),
        status: "draft".to_string(),
        word_count: Some(1200),
        summary: Some("主角发现第一个线索。".to_string()),
        published_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn review_version(project_id: &str) -> tauri_app_lib::models::ChapterVersion {
    tauri_app_lib::models::ChapterVersion {
        id: "version-review".to_string(),
        chapter_id: "chapter-review".to_string(),
        project_id: project_id.to_string(),
        version_number: 1,
        version_type: "draft".to_string(),
        title: Some("评审章节".to_string()),
        body_markdown: Some(
            "月光落在旧站台上，主角没有立刻解释恐惧，只把钥匙攥进掌心。".repeat(40),
        ),
        summary: Some("主角发现第一个线索。".to_string()),
        word_count: Some(1200),
        model_provider: None,
        model_name: None,
        prompt_hash: None,
        context_hash: None,
        created_by_agent: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn empty_review_canon() -> CanonContext {
    CanonContext {
        writing_brief_json: "{}".to_string(),
        characters_json: "[]".to_string(),
        character_states_json: "[]".to_string(),
        previous_chapters_json: "[]".to_string(),
        active_plot_threads_json: "[]".to_string(),
        unresolved_foreshadowing_json: "[]".to_string(),
        world_lore_json: "[]".to_string(),
        locations_json: "[]".to_string(),
        organizations_json: "[]".to_string(),
        items_json: "[]".to_string(),
        magic_systems_json: "[]".to_string(),
        canon_rules_json: "[]".to_string(),
        timeline_json: "[]".to_string(),
        style_guide_json: "[]".to_string(),
        extension_review_rubrics_json: "[]".to_string(),
        blog_config_json: "{}".to_string(),
        project_policy_json: "{}".to_string(),
    }
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
fn weekly_planner_prompt_preserves_longform_pacing() {
    let prompt = tauri_app_lib::prompts::load_prompt("weekly_planner")
        .expect("weekly planner prompt should be registered");

    assert!(prompt.contains("longform pacing"));
    assert!(prompt.contains("must not resolve"));
    assert!(prompt.contains("next local movement"));
    assert!(prompt.contains("story_phase"));
    assert!(prompt.contains("story_progress_percent"));
    assert!(prompt.contains("endgame"));
}

#[test]
fn weekly_planner_context_includes_story_progress_and_recent_summaries() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    for sequence in 1..=5 {
        conn.execute(
            "INSERT INTO chapters (id, project_id, sequence, title, status, word_count, summary)
             VALUES (?1, ?2, ?3, ?4, 'final', 3000, ?5)",
            rusqlite::params![
                format!("chapter-{sequence}"),
                project_id,
                sequence,
                format!("第{sequence}章"),
                format!("第{sequence}章摘要，主角只推进局部线索。")
            ],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-next', ?1, 6, '下一步', '继续局部推进，不解决终局。', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let context =
        tauri_app_lib::workflow::weekly_planner::build_weekly_planner_context(&db, &project_id)
            .expect("weekly planner context should build");

    assert_eq!(context["estimated_total_chapters"].as_i64(), Some(167));
    assert_eq!(context["chapters_written"].as_u64(), Some(5));
    assert_eq!(context["next_sequence"].as_i64(), Some(6));
    assert_eq!(context["story_phase"].as_str(), Some("opening"));
    assert!(context["story_progress_percent"].as_f64().unwrap() > 2.9);
    assert!(context["story_progress_percent"].as_f64().unwrap() < 3.1);
    assert!(context["recent_chapter_summaries"]
        .to_string()
        .contains("第5章摘要"));
    assert!(context["pacing_directive"]
        .as_str()
        .unwrap_or("")
        .contains("next local movement"));
}

#[test]
fn continuity_prompt_does_not_treat_missing_rag_as_blocking() {
    let prompt = tauri_app_lib::prompts::load_prompt("continuity_reviewer")
        .expect("continuity reviewer prompt should be registered");

    assert!(prompt.contains("RAG"));
    assert!(prompt.contains("not a blocking"));
}

#[test]
fn style_reviewer_prompt_defines_consistent_score_rubric() {
    let prompt = tauri_app_lib::prompts::load_prompt("style_reviewer")
        .expect("style reviewer prompt should be registered");

    assert!(prompt.contains("90-100"));
    assert!(prompt.contains("75-89"));
    assert!(prompt.contains("score >= 75"));
    assert!(prompt.contains("pass=true"));
    assert!(prompt.contains("0-20"));
}

#[test]
fn bible_generation_prompt_treats_first_ten_plans_as_opening_arc() {
    let prompt = tauri_app_lib::prompts::load_prompt("bible_generation")
        .expect("bible generation prompt should be registered");

    assert!(prompt.contains("first 10 immediate chapter plans"));
    assert!(prompt.contains("opening movement"));
    assert!(prompt.contains("2-5%"));
    assert!(prompt.contains("must not resolve"));
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

#[test]
fn project_stats_plans_left_counts_only_unwritten_planned_plans() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, status)
         VALUES ('plan-written', ?1, 1, '已写章节', '已经产出章节但等待人工复核', 'in_progress')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-written', ?1, 'plan-written', 1, '已写章节', 'needs_human_review', 1200, '等待人工复核')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let stats = tauri_app_lib::db::projects::get_project_stats(&db, &project_id).unwrap();

    assert_eq!(stats.chapter_count, 1);
    assert_eq!(stats.plans_left, 0);
}

#[derive(Default)]
struct CapturingProvider {
    systems: Arc<Mutex<Vec<String>>>,
    users: Arc<Mutex<Vec<String>>>,
    embed_calls: Arc<Mutex<usize>>,
    embed_kinds: Arc<Mutex<Vec<EmbeddingInputKind>>>,
    canon_graph_edges: bool,
    canon_task_rows: bool,
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
            if self.canon_task_rows {
                return Ok(json!({
                    "chapter_summary": "最终修订稿进入圣经",
                    "character_state_updates": [{
                        "character_id": "char-pipe",
                        "physical_state": "雨水浸透外套",
                        "emotional_state": "警觉"
                    }],
                    "timeline_events": [{
                        "event_time_label": "雨夜",
                        "sequence": 1,
                        "event_summary": "主角发现墙面旧名",
                        "involved_characters": ["主角"],
                        "involved_locations": [],
                        "consequences": ["旧名线索启动"],
                        "confidence": 0.9
                    }],
                    "new_lore": [],
                    "foreshadowing_updates": [{
                        "action": "introduced",
                        "clue_text": "潮湿墙面上的旧名"
                    }],
                    "vector_documents": [],
                    "knowledge_graph_edges": [],
                    "human_review_required": []
                }));
            }
            if self.canon_graph_edges {
                return Ok(json!({
                    "chapter_summary": "最终修订稿进入圣经",
                    "character_state_updates": [],
                    "timeline_events": [],
                    "new_lore": [],
                    "foreshadowing_updates": [],
                    "vector_documents": [],
                    "knowledge_graph_edges": [{
                        "source_node_id": "missing-source",
                        "source_node_type": "character",
                        "target_node_id": "missing-target",
                        "target_node_type": "location",
                        "edge_type": "visited",
                        "confidence": 0.7
                    }],
                    "human_review_required": []
                }));
            }
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
            "body_markdown": "最终稿正文。门后的人没有现身，只把他的旧名写在潮湿墙面上。票面写着三百枚灵石。这个版本必须进入 canon。",
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
        system: &str,
        user: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        self.systems.lock().unwrap().push(system.to_string());
        self.users.lock().unwrap().push(user.to_string());
        if let Some(text) = &self.review_text {
            return Ok(text.clone());
        }
        Ok(r#"{"score":92,"pass":true,"blocking_issues":[],"minor_issues":[],"recommendations":[]}"#.into())
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Err("legacy embed should not be used for RAG retrieval".into())
    }

    async fn embed_with_kind(
        &self,
        texts: &[String],
        kind: EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        *self.embed_calls.lock().unwrap() += 1;
        self.embed_kinds.lock().unwrap().push(kind);
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

fn ids_for(db: &Database, sql: &str, value: &str) -> Vec<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.query_map(rusqlite::params![value], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn assert_owned_rows_include(owned_rows: &Value, table: &str, row_ids: &[String]) {
    let owned = owned_rows[table]
        .as_array()
        .unwrap_or_else(|| panic!("owned_rows.{} should be an array", table))
        .iter()
        .filter_map(|id| id.as_str())
        .collect::<Vec<_>>();
    assert!(
        !row_ids.is_empty(),
        "test setup should create at least one {} row",
        table
    );
    for row_id in row_ids {
        assert!(
            owned.iter().any(|owned_id| owned_id == row_id),
            "owned_rows.{} should include {}",
            table,
            row_id
        );
    }
}

#[tokio::test]
async fn reviewer_wrapped_chinese_low_score_does_not_panic_or_default_to_zero() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let chapter = review_chapter(&project_id);
    let version = review_version(&project_id);
    let canon = empty_review_canon();
    let prefix = format!("x{}\n", "级".repeat(140));
    let provider = CapturingProvider {
        review_text: Some(format!(
            "{}{}",
            prefix,
            r#"{"score":9,"pass":false,"blocking_issues":[],"minor_issues":[{"id":"S001","issue":"语言质量低","evidence":"级别描述重复","recommendation":"重写句群"}],"recommendations":[]}"#
        )),
        ..Default::default()
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .expect("wrapped Chinese review output should be parsed safely");
    let style = reviews
        .iter()
        .find(|review| review.agent_name == "style_reviewer")
        .expect("style review should be present");

    assert_eq!(style.score, Some(9));
    assert_eq!(style.pass, Some(false));
    assert!(style.raw_output.contains("\"score\":9"));
}

#[tokio::test]
async fn reviewer_preserves_high_score_from_wrapped_json_output() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let chapter = review_chapter(&project_id);
    let version = review_version(&project_id);
    let canon = empty_review_canon();
    let provider = CapturingProvider {
        review_text: Some(
            "模型评审如下：\n```json\n{\"score\":95,\"pass\":true,\"blocking_issues\":[],\"minor_issues\":[],\"recommendations\":[]}\n```\n请查收。"
                .to_string(),
        ),
        ..Default::default()
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .expect("fenced review output should parse");
    let continuity = reviews
        .iter()
        .find(|review| review.agent_name == "continuity_reviewer")
        .expect("continuity review should be present");

    assert_eq!(continuity.score, Some(95));
    assert_eq!(continuity.pass, Some(true));
}

#[tokio::test]
async fn publication_reviewer_metadata_is_preserved_for_publish_drafts() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let chapter = review_chapter(&project_id);
    let version = review_version(&project_id);
    let canon = empty_review_canon();
    let provider = CapturingProvider {
        review_text: Some(
            r#"{"score":91,"pass":true,"blocking_issues":[],"minor_issues":[],"blog_metadata":{"title":"雨夜旧案","slug":"rain-night-case","excerpt":"钥匙与旧站台引出第一条线索。","tags":["悬疑","连载"],"category":"小说连载","seo_description":"雨夜旧案章节发布草稿。","status_recommendation":"draft"},"recommendations":[]}"#
                .to_string(),
        ),
        ..Default::default()
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .expect("publication metadata review should parse");
    let publication = reviews
        .iter()
        .find(|review| review.agent_name == "publication_reviewer")
        .expect("publication review should be present");
    let metadata: Value =
        serde_json::from_str(&publication.metadata).expect("metadata should be json");

    assert_eq!(
        metadata["blog_metadata"]["slug"].as_str(),
        Some("rain-night-case")
    );
    assert_eq!(
        metadata["publication_interface"]["provider_kind"].as_str(),
        Some("local_draft")
    );
}

#[tokio::test]
async fn style_reviewer_prompt_includes_compiled_style_assets() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let chapter = review_chapter(&project_id);
    let version = review_version(&project_id);
    let mut canon = empty_review_canon();
    canon.writing_brief_json = serde_json::json!({
        "style": {
            "style_assets": {
                "asset_ids": ["style-asset-1"],
                "prompt_instructions": "- 冷硬物件 [learned_style_pattern]: priority=92",
                "positive_examples": ["他把杯口转向墙角。"],
                "anti_ai_rules": {
                    "forbidden_phrases": ["眼中闪过"],
                    "required_phrases": ["金属触感"]
                }
            }
        }
    })
    .to_string();
    let provider = CapturingProvider::default();

    review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .expect("review agents should run");

    let systems = provider.systems.lock().unwrap();
    let style_prompt = systems
        .iter()
        .find(|prompt| prompt.contains("你是 style_reviewer"))
        .expect("style reviewer prompt should be captured");
    assert!(style_prompt.contains("冷硬物件"));
    assert!(style_prompt.contains("required_phrases"));
}

#[tokio::test]
async fn extension_review_rubrics_are_injected_into_reviewer_prompts() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let chapter = review_chapter(&project_id);
    let version = review_version(&project_id);
    let mut canon = empty_review_canon();
    canon.extension_review_rubrics_json = serde_json::json!([{
        "rubric_id": "anti-ai",
        "checks": ["低信息密度句", "过度解释人物心理"]
    }])
    .to_string();
    let provider = CapturingProvider::default();

    review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .expect("review agents should run");

    let systems = provider.systems.lock().unwrap();
    let style_prompt = systems
        .iter()
        .find(|prompt| prompt.contains("你是 style_reviewer"))
        .expect("style reviewer prompt should be captured");
    assert!(style_prompt.contains("低信息密度句"));
    assert!(style_prompt.contains("过度解释人物心理"));
}

#[test]
fn local_blog_draft_uses_latest_publication_review_metadata() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-publish', ?1, 1, '雨夜旧案', 'final', 1200, '章节摘要')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-publish', 'chapter-publish', ?1, 1, 'final', '雨夜旧案', '正文。', '章节摘要', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = 'version-publish' WHERE id = 'chapter-publish'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO agent_reviews
         (id, project_id, chapter_id, chapter_version_id, agent_name, score, pass, blocking_issues, minor_issues, recommendations, raw_output)
         VALUES ('review-publication', ?1, 'chapter-publish', 'version-publish', 'publication_reviewer', 91, 1, '[]', '[]', '[]', ?2)",
        rusqlite::params![
            project_id,
            r#"{"score":91,"pass":true,"blocking_issues":[],"minor_issues":[],"blog_metadata":{"title":"雨夜旧案","slug":"rain-night-case","excerpt":"钥匙与旧站台引出第一条线索。","tags":["悬疑","连载"],"category":"小说连载","seo_description":"雨夜旧案章节发布草稿。","status_recommendation":"draft"},"recommendations":[]}"#
        ],
    )
    .unwrap();
    drop(conn);

    let post_id = tauri_app_lib::create_local_blog_draft(&db, "chapter-publish")
        .expect("local blog draft should be created");
    let posts = tauri_app_lib::db::blog_posts::get_blog_posts(&db, &project_id).unwrap();
    let post = posts.iter().find(|post| post.id == post_id).unwrap();
    let metadata: Value = serde_json::from_str(&post.metadata).unwrap();

    assert_eq!(post.title.as_deref(), Some("雨夜旧案"));
    assert_eq!(post.slug.as_deref(), Some("rain-night-case"));
    assert_eq!(
        metadata["publication_metadata"]["excerpt"].as_str(),
        Some("钥匙与旧站台引出第一条线索。")
    );
    assert_eq!(
        metadata["publication_interface"]["target"].as_str(),
        Some("blog")
    );
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
    conn.execute(
        "INSERT INTO characters (id, project_id, name, role)
         VALUES ('char-pipe', ?1, '主角', 'protagonist')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE projects SET auto_publish = 1, blog_provider = 'local' WHERE id = ?1",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);
    tauri_app_lib::db::style_assets::upsert_style_asset(
        &db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-pipe".to_string()),
            project_id: project_id.clone(),
            name: "冷硬物件资产".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: serde_json::json!({"cadence": "short object beats"}),
            positive_examples: vec!["他把杯口转向墙角。".to_string()],
            negative_examples: vec![],
            anti_ai_rules: serde_json::json!({"required_phrases": ["票面"]}),
            enabled: true,
            priority: 30,
            metadata: serde_json::json!({"fixture": "core-loop"}),
        },
    )
    .unwrap();
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
    let draft_profile_id = tauri_app_lib::db::model_profiles::upsert_model_profile(
        &db,
        &tauri_app_lib::db::model_profiles::ModelProfileInput {
            id: Some("profile-draft-paid".to_string()),
            name: "Paid Draft Profile".to_string(),
            provider: "openai_compat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "draft-profile-model".to_string(),
            context_window: 128000,
            supports_json: true,
            supports_streaming: true,
            supports_embeddings: false,
            input_cost_per_million: Some(9.0),
            output_cost_per_million: Some(12.0),
            intended_use: "draft".to_string(),
            metadata: serde_json::json!({"fixture": "core-loop"}),
        },
    )
    .unwrap();
    let review_profile_id = tauri_app_lib::db::model_profiles::upsert_model_profile(
        &db,
        &tauri_app_lib::db::model_profiles::ModelProfileInput {
            id: Some("profile-review-paid".to_string()),
            name: "Paid Review Profile".to_string(),
            provider: "openai_compat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "review-profile-model".to_string(),
            context_window: 64000,
            supports_json: true,
            supports_streaming: true,
            supports_embeddings: false,
            input_cost_per_million: Some(3.0),
            output_cost_per_million: Some(4.0),
            intended_use: "review".to_string(),
            metadata: serde_json::json!({"fixture": "core-loop"}),
        },
    )
    .unwrap();
    settings.draft_model_profile_id = Some(draft_profile_id.clone());
    settings.review_model_profile_id = Some(review_profile_id.clone());
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    let provider = CapturingProvider {
        canon_task_rows: true,
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
        Some(&provider),
        &project_id,
        true,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .unwrap();

    assert!(result.ok);
    assert_eq!(*provider.embed_calls.lock().unwrap(), 1);
    assert_eq!(
        *provider.embed_kinds.lock().unwrap(),
        vec![EmbeddingInputKind::Query]
    );

    let systems = provider.systems.lock().unwrap().join("\n---\n");
    assert!(!systems.contains("WRITING_CONTEXT_JSON"));
    assert!(systems.contains("克制悬疑"));
    assert!(systems.contains("冷硬物件资产"));
    assert!(systems.contains("learning_entry:learn-pipe"));
    assert!(systems.contains("门后旧名"));
    assert!(systems.contains("旧名伏笔"));
    assert!(!systems.contains("{{"));

    let chapters = tauri_app_lib::db::chapters::get_chapters(&db, &project_id).unwrap();
    let chapter_id = chapters[0].id.clone();
    let latest_version = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
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
    assert_eq!(
        version_metadata["selected_learning_entry_ids"],
        serde_json::json!(["learn-pipe"])
    );
    assert_eq!(
        version_metadata["selected_learning_entries"][0]["pattern_name"].as_str(),
        Some("克制悬疑")
    );
    assert_eq!(
        version_metadata["style_asset_ids"],
        serde_json::json!(["style-pipe"])
    );
    assert!(version_metadata["style_assets"]["prompt_instructions"]
        .as_str()
        .unwrap_or("")
        .contains("冷硬物件资产"));
    let context_source_trace = version_metadata["context_activation"]["source_trace"]
        .as_array()
        .expect("context source trace should be present");
    assert!(context_source_trace.iter().any(|source| {
        source["source_key"].as_str() == Some("learning_entry:learn-pipe")
            && source["reason"].as_str() == Some("learning_entry_selection")
    }));
    assert!(context_source_trace.iter().any(|source| {
        source["source_key"].as_str() == Some("style_asset:style-pipe")
            && source["reason"].as_str() == Some("style_asset_enabled")
    }));
    assert_eq!(
        version_metadata["prompt_runtime"]["prompt_name"].as_str(),
        Some("draft_writer")
    );
    assert_eq!(
        version_metadata["prompt_runtime"]["generation_phase"].as_str(),
        Some("draft")
    );
    assert_eq!(
        version_metadata["prompt_runtime"]["unit_traces"][0]["identifier"].as_str(),
        Some("draft_writer.system")
    );
    assert!(
        version_metadata["prompt_runtime"]["token_estimate"]
            .as_i64()
            .unwrap_or(0)
            > 0
    );
    assert!(
        version_metadata["learning_context_hash"]
            .as_str()
            .unwrap_or("")
            .len()
            >= 16
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
    assert_eq!(
        job_metadata["task_snapshot"]["chapter_plan_id"].as_str(),
        Some("plan-pipe")
    );
    assert!(job_metadata["task_snapshot"]["owned_rows"]["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id.as_str() == Some(chapter_id.as_str())));
    assert!(
        job_metadata["task_snapshot"]["owned_rows"]["chapter_versions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id.as_str() == Some(latest_version.id.as_str()))
    );
    let owned_rows = &job_metadata["task_snapshot"]["owned_rows"];
    let review_ids = ids_for(
        &db,
        "SELECT id FROM agent_reviews WHERE chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let score_ids = ids_for(
        &db,
        "SELECT id FROM review_scores WHERE chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let blog_ids = ids_for(
        &db,
        "SELECT id FROM blog_posts WHERE chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let character_state_ids = ids_for(
        &db,
        "SELECT id FROM character_states WHERE after_chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let timeline_event_ids = ids_for(
        &db,
        "SELECT id FROM timeline_events WHERE chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let foreshadowing_ids = ids_for(
        &db,
        "SELECT id FROM foreshadowing WHERE introduced_chapter_id = ?1 ORDER BY id",
        &chapter_id,
    );
    let graph_edge_ids = ids_for(
        &db,
        "SELECT id FROM knowledge_graph_edges WHERE project_id = ?1 ORDER BY id",
        &project_id,
    );
    let hard_fact_ids = ids_for(
        &db,
        "SELECT id FROM hard_facts WHERE project_id = ?1 ORDER BY id",
        &project_id,
    );
    let hard_facts = tauri_app_lib::db::hard_facts::list_hard_facts(&db, &project_id, true)
        .expect("hard facts should load after final chapter");
    assert!(
        hard_facts.iter().any(|fact| fact.object == "三百枚灵石"
            && fact.chapter_version_id.as_deref() == Some(latest_version.id.as_str())),
        "final chapter should materialize amount hard facts"
    );
    assert_owned_rows_include(owned_rows, "agent_reviews", &review_ids);
    assert_owned_rows_include(owned_rows, "review_scores", &score_ids);
    assert_owned_rows_include(owned_rows, "blog_posts", &blog_ids);
    assert_owned_rows_include(owned_rows, "character_states", &character_state_ids);
    assert_owned_rows_include(owned_rows, "timeline_events", &timeline_event_ids);
    assert_owned_rows_include(owned_rows, "foreshadowing", &foreshadowing_ids);
    assert_owned_rows_include(owned_rows, "knowledge_graph_edges", &graph_edge_ids);
    assert_owned_rows_include(owned_rows, "hard_facts", &hard_fact_ids);
    assert_eq!(
        job_metadata["learning_context"]["selected_learning_entry_ids"],
        serde_json::json!(["learn-pipe"])
    );
    let usage_events = job_metadata["model_usage_events"].as_array().unwrap();
    let draft_usage = usage_events
        .iter()
        .find(|event| event["phase"].as_str() == Some("generate_draft"))
        .expect("draft usage should be recorded");
    assert_eq!(draft_usage["usage_source"].as_str(), Some("provider"));
    assert_eq!(
        draft_usage["model_profile"]["id"].as_str(),
        Some(draft_profile_id.as_str())
    );
    assert_eq!(
        draft_usage["model_profile"]["name"].as_str(),
        Some("Paid Draft Profile")
    );
    assert_eq!(draft_usage["provider"].as_str(), Some("openai_compat"));
    assert_eq!(draft_usage["model"].as_str(), Some("draft-profile-model"));
    assert_eq!(draft_usage["prompt_tokens"].as_i64(), Some(1111));
    assert_eq!(draft_usage["completion_tokens"].as_i64(), Some(222));
    assert_eq!(draft_usage["total_tokens"].as_i64(), Some(1333));
    assert_eq!(draft_usage["input_cost_per_million"].as_f64(), Some(9.0));
    assert_eq!(draft_usage["output_cost_per_million"].as_f64(), Some(12.0));
    assert!((draft_usage["estimated_cost_usd"].as_f64().unwrap() - 0.012663).abs() < 0.000001);
    let review_usage = usage_events
        .iter()
        .find(|event| event["phase"].as_str() == Some("review_agents"))
        .expect("review usage should be recorded");
    assert_eq!(review_usage["usage_source"].as_str(), Some("estimated"));
    assert_eq!(
        review_usage["model_profile"]["id"].as_str(),
        Some(review_profile_id.as_str())
    );
    assert_eq!(
        review_usage["model_profile"]["name"].as_str(),
        Some("Paid Review Profile")
    );
    assert_eq!(review_usage["provider"].as_str(), Some("openai_compat"));
    assert_eq!(review_usage["model"].as_str(), Some("review-profile-model"));
    assert_eq!(review_usage["input_cost_per_million"].as_f64(), Some(3.0));
    assert_eq!(review_usage["output_cost_per_million"].as_f64(), Some(4.0));
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

#[test]
fn markdown_export_uses_persisted_project_paper_dir_after_settings_change() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let fallback_dir = tempfile::tempdir().unwrap();
    let persisted_parent = tempfile::tempdir().unwrap();
    let persisted_dir = persisted_parent
        .path()
        .join("chosen-storage")
        .to_string_lossy()
        .to_string();
    tauri_app_lib::db::projects::set_project_paper_dir(&db, &project_id, &persisted_dir).unwrap();

    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-export', ?1, 1, '导出章节', 'final', 1200, '导出摘要')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-export', 'chapter-export', ?1, 1, 'final', '导出章节', '正文需要写入用户选择的位置。', '导出摘要', 1200)",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = 'version-export' WHERE id = 'chapter-export'",
        [],
    )
    .unwrap();
    drop(conn);

    let exported = tauri_app_lib::export::markdown::export_chapter_markdown(
        &db,
        "chapter-export",
        fallback_dir.path().to_str().unwrap(),
    )
    .unwrap();

    assert!(Path::new(&exported).starts_with(&persisted_dir));
    assert!(Path::new(&exported).exists());
    assert!(!fallback_dir
        .path()
        .join(tauri_app_lib::db::projects::slugify(&project_id))
        .exists());

    let content = tauri_app_lib::db::chapters::read_chapter_file_content(
        &db,
        fallback_dir.path().to_str().unwrap(),
        &project_id,
        "ch001.md",
    )
    .unwrap();
    assert!(content.contains("正文需要写入用户选择的位置。"));

    let files = tauri_app_lib::db::chapters::list_chapter_files(
        &db,
        &project_id,
        fallback_dir.path().to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "ch001.md");
}

#[test]
fn project_cleanup_dirs_include_persisted_and_current_fallback_dirs() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let fallback_dir = tempfile::tempdir().unwrap();
    let persisted_parent = tempfile::tempdir().unwrap();
    let persisted_dir = persisted_parent
        .path()
        .join("original-storage")
        .to_string_lossy()
        .to_string();
    tauri_app_lib::db::projects::set_project_paper_dir(&db, &project_id, &persisted_dir).unwrap();

    let cleanup_dirs = tauri_app_lib::db::projects::project_paper_dirs_for_cleanup(
        &db,
        &project_id,
        fallback_dir.path().to_str().unwrap(),
    )
    .unwrap();
    let fallback_project_dir =
        tauri_app_lib::db::projects::paper_dir(fallback_dir.path().to_str().unwrap(), &project_id);

    assert!(cleanup_dirs.contains(&persisted_dir));
    assert!(cleanup_dirs.contains(&fallback_project_dir));
    assert_eq!(cleanup_dirs.len(), 2);
}

#[tokio::test]
async fn chapter_pipeline_completes_when_canon_update_fails_after_content_save() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-canon-noncritical', ?1, 1, '后处理失败', '章节已经保存后 canon 更新失败', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute("DROP TABLE knowledge_graph_edges", [])
        .unwrap();
    drop(conn);

    let provider = CapturingProvider {
        canon_graph_edges: true,
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
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "completed");

    let mut saw_noncritical_canon_failure = false;
    while let Ok(event) = event_rx.try_recv() {
        if event.step == "update_canon" && event.status == "failed_noncritical" {
            saw_noncritical_canon_failure = true;
        }
    }
    assert!(saw_noncritical_canon_failure);
}

#[tokio::test]
async fn chapter_pipeline_does_not_use_main_provider_for_rag_when_embeddings_are_disabled() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-no-rag', ?1, 1, '无向量上下文', '不配置 embedding 时也要正常写作', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let provider = CapturingProvider::default();
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
    assert_eq!(*provider.embed_calls.lock().unwrap(), 0);

    let mut retrieve_detail = String::new();
    while let Ok(event) = event_rx.try_recv() {
        if event.step == "retrieve_context" {
            retrieve_detail = event.detail.unwrap_or_default();
        }
    }
    assert!(retrieve_detail.contains("RAG disabled"));
}

#[tokio::test]
async fn chapter_pipeline_routes_draft_review_and_repair_to_stage_providers() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-stage-providers', ?1, 1, '阶段 Provider', '触发修订以验证 provider 路由', 3000, 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    settings.max_revise_count = 1;
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    let draft_provider = CapturingProvider::default();
    let review_provider = CapturingProvider {
        review_text: Some(
            r#"{"score":40,"pass":true,"blocking_issues":[],"minor_issues":[{"issue":"needs repair"}],"recommendations":["revise"]}"#
                .to_string(),
        ),
        ..Default::default()
    };
    let repair_provider = CapturingProvider::default();
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(100);
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(100);

    let result =
        tauri_app_lib::workflow::chapter_production::generate_next_chapter_with_stage_providers(
            &db,
            tauri_app_lib::workflow::chapter_production::ChapterPipelineProviders {
                draft: &draft_provider,
                review: &review_provider,
                repair: &repair_provider,
                postprocess: &draft_provider,
            },
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
    assert!(!draft_provider.systems.lock().unwrap().is_empty());
    assert!(review_provider
        .users
        .lock()
        .unwrap()
        .iter()
        .any(|prompt| prompt.contains("请评审以下章节内容")));
    assert!(repair_provider
        .systems
        .lock()
        .unwrap()
        .iter()
        .any(|prompt| prompt.contains("资深中文网文修订编辑")));
    assert!(repair_provider
        .users
        .lock()
        .unwrap()
        .iter()
        .any(|prompt| prompt.contains("needs repair")));
}

#[tokio::test]
async fn chapter_pipeline_runs_context_extension_hooks_and_persists_trace() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-extension-hooks', ?1, 1, '扩展 Hook', '验证 context hook trace', 3000, 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    tauri_app_lib::extensions::host::import_extension_package(
        &db,
        &tauri_app_lib::extensions::host::ExtensionPackage {
            manifest: tauri_app_lib::extensions::manifest::ExtensionManifest {
                id: "pipeline.context.trace".to_string(),
                name: "Pipeline Context Trace".to_string(),
                version: "1.0.0".to_string(),
                description: Some("trace pipeline context hooks".to_string()),
                enabled_by_default: false,
                permissions: vec!["project_read".to_string()],
                hooks: vec![
                    "before_context_build".to_string(),
                    "after_context_build".to_string(),
                    "before_review".to_string(),
                    "after_review".to_string(),
                    "export_target".to_string(),
                ],
                package_kinds: vec!["context_rule_pack".to_string(), "prompt_pack".to_string()],
                metadata: serde_json::json!({}),
            },
            enabled: true,
            contributions: vec![
                tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_context_build".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"before_context_extension": true}),
                },
                tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "after_context_build".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: Some("prompt_pack".to_string()),
                    contribution_id: Some("pipeline-prompt-style".to_string()),
                    payload: serde_json::json!({
                        "unit_identifier": "extension.pipeline_prompt_style",
                        "role": "system",
                        "order": 15,
                        "generation_phase": "draft",
                        "content": "EXTENSION PROMPT STYLE: keep the rain ticket motif visible."
                    }),
                    metadata_patch: serde_json::json!({"after_context_extension": true}),
                },
                tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_review".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"before_review_extension": true}),
                },
                tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "after_review".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"after_review_extension": true}),
                },
                tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "export_target".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"export_extension": true}),
                },
            ],
        },
    )
    .unwrap();
    tauri_app_lib::extensions::host::set_extension_enabled(&db, "pipeline.context.trace", true)
        .unwrap();

    let provider = CapturingProvider::default();
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

    assert!(result.ok);
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let hooks = metadata["extension_hooks"].as_array().unwrap();
    let hook_names = hooks
        .iter()
        .filter_map(|hook| hook["hook"].as_str())
        .collect::<Vec<_>>();
    assert!(hook_names.contains(&"before_context_build"));
    assert!(hook_names.contains(&"after_context_build"));
    assert!(hook_names.contains(&"before_review"));
    assert!(hook_names.contains(&"after_review"));
    assert!(hook_names.contains(&"export_target"));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "before_context_build"
            && hook["events"][0]["extension_id"] == "pipeline.context.trace"
            && hook["workflow_metadata"]["before_context_extension"] == true
    }));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "after_context_build"
            && hook["events"][0]["status"] == "applied"
            && hook["workflow_metadata"]["after_context_extension"] == true
    }));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "after_context_build"
            && hook["workflow_metadata"]["extension_contributions"]["prompt_pack"][0]
                ["contribution_id"]
                == "pipeline-prompt-style"
    }));
    assert!(provider
        .systems
        .lock()
        .unwrap()
        .iter()
        .any(|prompt| prompt.contains("EXTENSION PROMPT STYLE")));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "before_review"
            && hook["workflow_metadata"]["before_review_extension"] == true
    }));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "after_review"
            && hook["workflow_metadata"]["after_review_extension"] == true
    }));
    assert!(hooks.iter().any(|hook| {
        hook["hook"] == "export_target" && hook["workflow_metadata"]["export_extension"] == true
    }));
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

#[test]
fn human_edit_save_keeps_runtime_read_paths_available() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-human-save', ?1, 1, '待审旧名', '生成后进入人工复核', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-after-save', ?1, 2, '下一章', '继续旧名后果', 3000, 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
         VALUES ('learn-after-save', ?1, 'manual', '样章', 'style_pattern', '冷硬物件', '用物件承载压力', 0.91)",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let (chapter_id, _version_id) = tauri_app_lib::db::chapters::save_draft_version(
        &db,
        &project_id,
        "plan-human-save",
        1,
        "待审旧名",
        "初稿正文。门后旧名需要人工确认。",
        18,
        "初稿摘要",
        "test",
        "test-model",
        "prompt",
        "context",
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-after-save")
        .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE chapters SET status = 'needs_human_review' WHERE id = ?1",
            rusqlite::params![chapter_id],
        )
        .unwrap();
    }

    tauri_app_lib::save_human_edited_chapter(
        &db,
        &chapter_id,
        "待审旧名 修订",
        "人工保存后的正文仍然可被后续上下文读取。",
    )
    .expect("human edit should save without blocking read paths");

    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.version_type, "revised");
    assert!(latest
        .body_markdown
        .as_deref()
        .unwrap_or("")
        .contains("人工保存后的正文"));

    assert_eq!(
        tauri_app_lib::db::chapters::get_chapters(&db, &project_id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id)
            .unwrap()
            .len(),
        1
    );
    assert!(tauri_app_lib::db::bible::get_bible(&db, &project_id)
        .unwrap()
        .characters
        .is_empty());
    let graph = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert!(graph.nodes.is_empty());
    assert_eq!(
        tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
            .unwrap()
            .len(),
        1
    );

    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .expect("next planned chapter should remain available");
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
    let context_json = serde_json::to_value(package).unwrap();
    assert!(context_json["continuity"]["recent_body_excerpts"]
        .to_string()
        .contains("人工保存后的正文"));
}
