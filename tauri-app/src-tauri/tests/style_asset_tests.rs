use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::{canon_consistency, style_assets, writing_context};

fn setup_db(name: &str) -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(name);
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Style Asset Fixture",
        Some("style asset fixture"),
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
fn style_asset_round_trips_features_and_anti_ai_rules() {
    let db = setup_db("style-asset-roundtrip.db");
    let project_id = insert_project(&db);

    let asset_id = tauri_app_lib::db::style_assets::upsert_style_asset(
        &db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-asset-1".to_string()),
            project_id: project_id.clone(),
            name: "克制悬疑动作".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: serde_json::json!({"cadence": "short action chains"}),
            positive_examples: vec!["他把杯口转向墙角。".to_string()],
            negative_examples: vec!["他心中充满了复杂情绪。".to_string()],
            anti_ai_rules: serde_json::json!({"forbidden_phrases": ["眼中闪过"]}),
            enabled: true,
            priority: 20,
            metadata: serde_json::json!({"source": "manual"}),
        },
    )
    .expect("style asset should persist");

    let assets = tauri_app_lib::db::style_assets::list_style_assets(&db, &project_id, true)
        .expect("style assets should load");

    assert_eq!(asset_id, "style-asset-1");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].positive_examples[0], "他把杯口转向墙角。");
    assert_eq!(assets[0].negative_examples[0], "他心中充满了复杂情绪。");
    assert_eq!(
        assets[0].anti_ai_rules["forbidden_phrases"][0].as_str(),
        Some("眼中闪过")
    );
    assert!(assets[0].enabled);
}

#[test]
fn style_assets_list_enabled_assets_by_priority_then_name() {
    let db = setup_db("style-asset-list.db");
    let project_id = insert_project(&db);

    for (id, name, priority, enabled) in [
        ("style-low", "低优先级", 10, true),
        ("style-disabled", "禁用高优先级", 100, false),
        ("style-alpha", "A 高优先级", 50, true),
        ("style-beta", "B 高优先级", 50, true),
    ] {
        tauri_app_lib::db::style_assets::upsert_style_asset(
            &db,
            &tauri_app_lib::db::style_assets::StyleAssetInput {
                id: Some(id.to_string()),
                project_id: project_id.clone(),
                name: name.to_string(),
                asset_type: "prose_rule".to_string(),
                scope_type: "project".to_string(),
                scope_id: None,
                features: serde_json::json!({}),
                positive_examples: vec![],
                negative_examples: vec![],
                anti_ai_rules: serde_json::json!({}),
                enabled,
                priority,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let assets = tauri_app_lib::db::style_assets::list_style_assets(&db, &project_id, true)
        .expect("enabled style assets should load");
    let ids = assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["style-alpha", "style-beta", "style-low"]);
}

#[test]
fn compile_style_assets_builds_deterministic_prompt_payload() {
    let db = setup_db("style-asset-compile.db");
    let project_id = insert_project(&db);

    for (id, name, priority) in [
        ("style-low", "低优先级", 10),
        ("style-alpha", "A 高优先级", 50),
    ] {
        tauri_app_lib::db::style_assets::upsert_style_asset(
            &db,
            &tauri_app_lib::db::style_assets::StyleAssetInput {
                id: Some(id.to_string()),
                project_id: project_id.clone(),
                name: name.to_string(),
                asset_type: "prose_rule".to_string(),
                scope_type: "project".to_string(),
                scope_id: None,
                features: serde_json::json!({"cadence": name}),
                positive_examples: vec![format!("{name}正例")],
                negative_examples: vec![format!("{name}反例")],
                anti_ai_rules: serde_json::json!({"forbidden_phrases": [format!("{name}禁句")]}),
                enabled: true,
                priority,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let payload = style_assets::compile_style_assets(
        &db,
        &project_id,
        style_assets::StyleAssetScope::Project,
    )
    .expect("style assets should compile");

    assert_eq!(payload.asset_ids, vec!["style-alpha", "style-low"]);
    assert!(payload.prompt_instructions.contains("A 高优先级"));
    assert_eq!(payload.positive_examples[0], "A 高优先级正例");
    assert_eq!(
        payload.anti_ai_rules["forbidden_phrases"][0].as_str(),
        Some("A 高优先级禁句")
    );
}

#[test]
fn writing_context_includes_compiled_style_assets() {
    let db = setup_db("style-asset-context.db");
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-style-context', ?1, 1, '旧站', '用克制动作推进悬疑。', 3000, 'planned')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }
    tauri_app_lib::db::style_assets::upsert_style_asset(
        &db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-context".to_string()),
            project_id: project_id.clone(),
            name: "克制动作".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: serde_json::json!({"cadence": "short"}),
            positive_examples: vec!["他把杯口转向墙角。".to_string()],
            negative_examples: vec![],
            anti_ai_rules: serde_json::json!({"forbidden_phrases": ["眼中闪过"]}),
            enabled: true,
            priority: 10,
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

    assert!(context_json["style"]["style_assets"]["asset_ids"]
        .to_string()
        .contains("style-context"));
    assert!(context_json["style"]["style_assets"]["prompt_instructions"]
        .as_str()
        .unwrap_or("")
        .contains("克制动作"));
}

#[test]
fn review_precheck_flags_style_asset_forbidden_phrase() {
    let writing_context_json = serde_json::json!({
        "style": {
            "style_assets": {
                "asset_ids": ["style-1"],
                "anti_ai_rules": {
                    "forbidden_phrases": ["眼中闪过"]
                }
            }
        }
    })
    .to_string();

    let issues = canon_consistency::detect_review_precheck_issues_from_json(
        "他眼中闪过一丝复杂情绪。",
        &writing_context_json,
        "[]",
        "[]",
        "[]",
        "[]",
        "[]",
        "[]",
        1,
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].rule_type, "style_asset_forbidden_phrase");
    assert_eq!(issues[0].severity, "blocking");
}

#[test]
fn review_precheck_flags_style_asset_required_phrase_missing() {
    let writing_context_json = serde_json::json!({
        "style": {
            "style_assets": {
                "asset_ids": ["style-required"],
                "anti_ai_rules": {
                    "required_phrases": ["金属触感"]
                }
            }
        }
    })
    .to_string();

    let issues = canon_consistency::detect_review_precheck_issues_from_json(
        "他把杯口转向墙角，没有解释自己的恐惧。",
        &writing_context_json,
        "[]",
        "[]",
        "[]",
        "[]",
        "[]",
        "[]",
        1,
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].rule_type, "style_asset_required_phrase_missing");
    assert_eq!(issues[0].severity, "blocking");
}

#[test]
fn learning_intake_creates_disabled_draft_style_asset_until_author_enables_it() {
    let db = setup_db("style-asset-learning-draft.db");
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO learning_entries
             (id, project_id, source_type, source_title, category, pattern_name,
              pattern_description, example_text, application_notes, confidence, metadata)
             VALUES ('learn-style-draft', ?1, 'manual', '样章', 'style_pattern',
                     '冷硬物件', '用物件动作承载压力，避免解释情绪。',
                     '他把杯口转向墙角。', '用于高压对话', 0.92, '{\"source\":\"sample\"}')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    let asset_id = style_assets::create_draft_style_asset_from_learning_entry(
        &db,
        &project_id,
        "learn-style-draft",
    )
    .expect("learning entry should produce a draft style asset");

    let all_assets = tauri_app_lib::db::style_assets::list_style_assets(&db, &project_id, false)
        .expect("style assets should load");
    let draft = all_assets
        .iter()
        .find(|asset| asset.id == asset_id)
        .expect("draft style asset should exist");

    assert!(!draft.enabled);
    assert_eq!(draft.asset_type, "learned_style_pattern");
    assert_eq!(
        draft.metadata["source_learning_entry_id"].as_str(),
        Some("learn-style-draft")
    );
    assert_eq!(draft.metadata["approval_required"].as_bool(), Some(true));
    assert_eq!(draft.positive_examples, vec!["他把杯口转向墙角。"]);

    let disabled_payload = style_assets::compile_style_assets(
        &db,
        &project_id,
        style_assets::StyleAssetScope::Project,
    )
    .expect("disabled style assets should compile as empty");
    assert!(!disabled_payload.asset_ids.contains(&asset_id));

    tauri_app_lib::db::style_assets::upsert_style_asset(
        &db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some(draft.id.clone()),
            project_id: draft.project_id.clone(),
            name: draft.name.clone(),
            asset_type: draft.asset_type.clone(),
            scope_type: draft.scope_type.clone(),
            scope_id: draft.scope_id.clone(),
            features: draft.features.clone(),
            positive_examples: draft.positive_examples.clone(),
            negative_examples: draft.negative_examples.clone(),
            anti_ai_rules: draft.anti_ai_rules.clone(),
            enabled: true,
            priority: draft.priority,
            metadata: draft.metadata.clone(),
        },
    )
    .expect("author should be able to enable the draft style asset");

    let enabled_payload = style_assets::compile_style_assets(
        &db,
        &project_id,
        style_assets::StyleAssetScope::Project,
    )
    .expect("enabled style assets should compile");
    assert_eq!(enabled_payload.asset_ids, vec![asset_id]);
}

#[test]
fn persisted_style_learning_entries_create_disabled_draft_style_assets() {
    let db = setup_db("style-asset-learning-persist.db");
    let project_id = insert_project(&db);
    let entry = tauri_app_lib::models::LearningEntry {
        id: "learn-style-persist".to_string(),
        project_id: String::new(),
        source_type: "manual".to_string(),
        source_url: None,
        source_title: Some("样章".to_string()),
        category: "style_pattern".to_string(),
        pattern_name: "冷硬动作".to_string(),
        pattern_description: "用短促动作承载压力。".to_string(),
        example_text: Some("他把伞沿压低。".to_string()),
        application_notes: Some("用于高压对话".to_string()),
        confidence: 0.93,
        usage_count: 0,
        last_used_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let saved_ids = tauri_app_lib::workflow::learning::save_learning_entries_with_style_drafts(
        &db,
        &project_id,
        &[entry],
    )
    .expect("learning entries should persist with style drafts");

    assert_eq!(saved_ids, vec!["learn-style-persist"]);
    let all_assets = tauri_app_lib::db::style_assets::list_style_assets(&db, &project_id, false)
        .expect("style assets should load");
    let draft = all_assets
        .iter()
        .find(|asset| asset.metadata["source_learning_entry_id"] == "learn-style-persist")
        .expect("style learning entry should create a draft style asset");
    assert!(!draft.enabled);
    assert_eq!(draft.asset_type, "learned_style_pattern");
}
