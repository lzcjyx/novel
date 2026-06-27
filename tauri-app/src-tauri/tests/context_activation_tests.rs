use tauri_app_lib::db::connection::Database;
use tauri_app_lib::db::context_rules::{
    list_context_rules, list_enabled_context_rules, upsert_context_rule, ContextRuleInput,
};
use tauri_app_lib::workflow::context_activation::activate_context_rules;
use tauri_app_lib::workflow::writing_context::OperatorControls;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("context-activation.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Context Activation",
        Some("rule fixture"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("quiet"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn context_activation_rules_fire_from_plan_keywords_with_trace() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, required_characters, required_locations, plot_goals, status)
             VALUES
             ('plan-activation', ?1, 1, '红伞回到旧车站', '主角在旧车站发现红伞收据。', '林白', '旧车站', '红伞线索必须付出代价', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO context_rules
             (id, project_id, name, primary_keywords, secondary_keywords, priority, token_budget, content, source_type, source_id, enabled)
             VALUES
             ('rule-red-umbrella', ?1, '红伞旧案', '[\"红伞\"]', '[\"旧车站\"]', 80, 120, '红伞只能作为旧案代价线索，不能直接揭晓凶手。', 'canon_rule', 'canon-red-umbrella', 1)",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO context_rules
             (id, project_id, name, primary_keywords, secondary_keywords, priority, token_budget, content, source_type, source_id, enabled)
             VALUES
             ('rule-secondary-miss', ?1, '未命中的红伞规则', '[\"红伞\"]', '[\"天台\"]', 90, 120, '不应进入上下文。', 'canon_rule', 'canon-miss', 1)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

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

    assert_eq!(package.context_activation.activated_rules.len(), 1);
    let activation = &package.context_activation.activated_rules[0];
    assert_eq!(activation.rule_id, "rule-red-umbrella");
    assert_eq!(activation.source_key, "canon_rule:canon-red-umbrella");
    assert!(activation.content.contains("不能直接揭晓凶手"));
    assert!(activation.matched_keywords.contains(&"红伞".to_string()));
    assert!(activation
        .matched_secondary_keywords
        .contains(&"旧车站".to_string()));
}

#[test]
fn manual_context_rule_upsert_preserves_activation_fields_and_enabled_filter() {
    let db = setup_db();
    let project_id = insert_project(&db);

    let rule_id = upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("manual-red-umbrella".to_string()),
            project_id: project_id.clone(),
            name: "手工红伞规则".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec!["旧车站".to_string()],
            entity_refs: vec!["character:lin-bai".to_string()],
            chapter_ranges: vec!["1-3".to_string()],
            priority: 55,
            token_budget: 88,
            sticky_chapters: 2,
            cooldown_chapters: 1,
            content: "红伞线索必须保持间接，不能直接揭晓凶手。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({ "operator_note": "pin when old station appears" }),
        },
    )
    .unwrap();

    assert_eq!(rule_id, "manual-red-umbrella");

    let rules = list_enabled_context_rules(&db, &project_id).unwrap();
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "manual-red-umbrella");
    assert_eq!(rule.name, "手工红伞规则");
    assert_eq!(rule.primary_keywords, vec!["红伞".to_string()]);
    assert_eq!(rule.secondary_keywords, vec!["旧车站".to_string()]);
    assert_eq!(rule.entity_refs, vec!["character:lin-bai".to_string()]);
    assert_eq!(rule.chapter_ranges, vec!["1-3".to_string()]);
    assert_eq!(rule.priority, 55);
    assert_eq!(rule.token_budget, 88);
    assert_eq!(rule.sticky_chapters, 2);
    assert_eq!(rule.cooldown_chapters, 1);
    assert_eq!(rule.source_type, "manual");
    assert_eq!(
        rule.metadata["operator_note"],
        "pin when old station appears"
    );

    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some(rule_id),
            project_id: project_id.clone(),
            name: "禁用红伞规则".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec![],
            priority: 10,
            token_budget: 40,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "disabled".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: false,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    assert!(list_enabled_context_rules(&db, &project_id)
        .unwrap()
        .is_empty());
}

#[test]
fn context_rule_management_list_includes_disabled_rules() {
    let db = setup_db();
    let project_id = insert_project(&db);

    for (id, enabled, priority) in [("rule-enabled", true, 10), ("rule-disabled", false, 80)] {
        upsert_context_rule(
            &db,
            ContextRuleInput {
                id: Some(id.to_string()),
                project_id: project_id.clone(),
                name: id.to_string(),
                primary_keywords: vec!["红伞".to_string()],
                secondary_keywords: vec![],
                entity_refs: vec![],
                chapter_ranges: vec![],
                priority,
                token_budget: 80,
                sticky_chapters: 0,
                cooldown_chapters: 0,
                content: format!("{} content", id),
                source_type: "manual".to_string(),
                source_id: None,
                enabled,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let managed = list_context_rules(&db, &project_id).unwrap();
    assert_eq!(
        managed
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule-disabled", "rule-enabled"]
    );
    assert!(!managed[0].enabled);

    let enabled = list_enabled_context_rules(&db, &project_id).unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, "rule-enabled");
}

#[test]
fn context_activation_respects_chapter_ranges() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, required_characters, required_locations, plot_goals, status)
             VALUES
             ('plan-range', ?1, 5, '红伞第五章', '红伞线索回到旧车站。', '林白', '旧车站', '延迟揭示', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-range-hit".to_string()),
            project_id: project_id.clone(),
            name: "第五章范围内".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec!["4-6".to_string()],
            priority: 20,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "第五章可以使用的红伞限制。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-range-miss".to_string()),
            project_id: project_id.clone(),
            name: "第三章范围外".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec!["1-3".to_string()],
            priority: 80,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "第五章不能使用这条规则。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let trace = activate_context_rules(&db, &project_id, &plan, None).unwrap();

    assert_eq!(trace.activated_rules.len(), 1);
    assert_eq!(trace.activated_rules[0].rule_id, "rule-range-hit");
    assert_eq!(trace.activated_rules[0].activation_reason, "keyword");
}

fn insert_previous_activation(db: &Database, project_id: &str, sequence: i32, rule_id: &str) {
    let plan_id = format!("prior-plan-{}-{}", sequence, rule_id);
    let chapter_id = format!("prior-chapter-{}-{}", sequence, rule_id);
    let version_id = format!("prior-version-{}-{}", sequence, rule_id);
    let metadata = serde_json::json!({
        "context_activation": {
            "activated_rules": [{
                "rule_id": rule_id,
                "name": "prior",
                "source_key": format!("manual:{}", rule_id),
                "priority": 10,
                "token_estimate": 8,
                "content": "prior content",
                "matched_keywords": ["红伞"],
                "matched_secondary_keywords": [],
                "activation_reason": "keyword"
            }],
            "source_keys": [format!("manual:{}", rule_id)]
        }
    })
    .to_string();
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans
         (id, project_id, sequence, title, outline, required_characters, required_locations, plot_goals, status)
         VALUES (?1, ?2, ?3, 'prior', 'prior', '', '', '', 'completed')",
        rusqlite::params![plan_id, project_id, sequence],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters
         (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
         VALUES (?1, ?2, ?3, ?4, 'prior', 'draft', 10, 'prior')",
        rusqlite::params![chapter_id, project_id, plan_id, sequence],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count, metadata)
         VALUES (?1, ?2, ?3, 1, 'draft', 'prior', 'prior', 'prior', 10, ?4)",
        rusqlite::params![version_id, chapter_id, project_id, metadata],
    )
    .unwrap();
}

#[test]
fn context_activation_extends_recent_rule_with_sticky_chapters() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_previous_activation(&db, &project_id, 1, "rule-sticky");
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, required_characters, required_locations, plot_goals, status)
             VALUES
             ('plan-sticky', ?1, 2, '雨夜车站', '这一章没有直接提到目标关键词。', '林白', '旧车站', '追踪收据', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-sticky".to_string()),
            project_id: project_id.clone(),
            name: "红伞 sticky".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec![],
            priority: 30,
            token_budget: 80,
            sticky_chapters: 2,
            cooldown_chapters: 0,
            content: "红伞限制应在后续两章继续保留。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let trace = activate_context_rules(&db, &project_id, &plan, None).unwrap();

    assert_eq!(trace.activated_rules.len(), 1);
    assert_eq!(trace.activated_rules[0].rule_id, "rule-sticky");
    assert_eq!(trace.activated_rules[0].activation_reason, "sticky");
    assert!(trace.activated_rules[0].matched_keywords.is_empty());
}

#[test]
fn context_activation_suppresses_keyword_hit_during_cooldown() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_previous_activation(&db, &project_id, 2, "rule-cooldown");
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, required_characters, required_locations, plot_goals, status)
             VALUES
             ('plan-cooldown', ?1, 3, '红伞再次出现', '红伞线索再次回到旧车站。', '林白', '旧车站', '追踪收据', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-cooldown".to_string()),
            project_id: project_id.clone(),
            name: "红伞 cooldown".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec![],
            priority: 90,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 2,
            content: "红伞规则不能连续重复注入。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-normal".to_string()),
            project_id: project_id.clone(),
            name: "普通红伞".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec![],
            priority: 10,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "没有冷却限制。".to_string(),
            source_type: "manual".to_string(),
            source_id: None,
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let trace = activate_context_rules(&db, &project_id, &plan, None).unwrap();
    let ids = trace
        .activated_rules
        .iter()
        .map(|rule| rule.rule_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["rule-normal"]);
    assert_eq!(trace.activated_rules[0].activation_reason, "keyword");
}

#[test]
fn context_activation_fires_from_entity_refs_without_keywords() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, pov_character_id, required_characters, required_locations, plot_goals, status)
             VALUES
             ('plan-entity-ref', ?1, 1, '旧站低语', '没有关键词。', 'lin-bai', '林白', '旧车站', '调查', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-entity-ref".to_string()),
            project_id: project_id.clone(),
            name: "林白实体规则".to_string(),
            primary_keywords: vec!["不会出现的关键词".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec!["character:lin-bai".to_string()],
            chapter_ranges: vec![],
            priority: 30,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "林白在旧站不能直接说出真相。".to_string(),
            source_type: "manual".to_string(),
            source_id: Some("entity-rule".to_string()),
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let trace = activate_context_rules(&db, &project_id, &plan, None).unwrap();

    assert_eq!(trace.activated_rules.len(), 1);
    assert_eq!(trace.activated_rules[0].rule_id, "rule-entity-ref");
    assert_eq!(trace.activated_rules[0].activation_reason, "entity_ref");
}

#[test]
fn operator_controls_can_pin_and_unpin_source_keys() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans
             (id, project_id, sequence, title, outline, status)
             VALUES ('plan-pins', ?1, 1, '红伞', '红伞线索。', 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    upsert_context_rule(
        &db,
        ContextRuleInput {
            id: Some("rule-pin-target".to_string()),
            project_id: project_id.clone(),
            name: "红伞规则".to_string(),
            primary_keywords: vec!["红伞".to_string()],
            secondary_keywords: vec![],
            entity_refs: vec![],
            chapter_ranges: vec![],
            priority: 30,
            token_budget: 80,
            sticky_chapters: 0,
            cooldown_chapters: 0,
            content: "红伞规则。".to_string(),
            source_type: "manual".to_string(),
            source_id: Some("red-umbrella".to_string()),
            enabled: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    let controls = OperatorControls {
        pinned_source_keys: vec!["hard_fact:fact-pinned".to_string()],
        unpinned_source_keys: vec!["manual:red-umbrella".to_string()],
        ..Default::default()
    };
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id)
        .unwrap()
        .unwrap();
    let trace = activate_context_rules(&db, &project_id, &plan, Some(&controls)).unwrap();

    assert!(trace.activated_rules.is_empty());
    assert!(trace
        .source_keys
        .contains(&"hard_fact:fact-pinned".to_string()));
    assert!(!trace
        .source_keys
        .contains(&"manual:red-umbrella".to_string()));
    assert_eq!(trace.source_trace[0].reason, "manual_pin");
}
