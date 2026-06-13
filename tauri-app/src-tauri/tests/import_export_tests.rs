use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("import-export.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Lorebook Import",
        Some("interop fixture"),
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
fn sillytavern_lorebook_import_creates_context_rules() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let lorebook = r#"{
      "name": "Red Umbrella Lorebook",
      "entries": {
        "0": {
          "uid": 7,
          "comment": "Red umbrella station rule",
          "key": ["red umbrella", "红伞"],
          "keysecondary": ["old station", "旧车站"],
          "content": "红伞只能指向旧案代价，不能直接揭晓凶手。",
          "order": 80,
          "disable": false
        },
        "1": {
          "uid": 8,
          "comment": "Disabled entry",
          "key": ["disabled"],
          "content": "不应导入为启用规则。",
          "disable": true
        }
      }
    }"#;

    let summary = tauri_app_lib::workflow::lorebook_import::import_sillytavern_lorebook(
        &db,
        &project_id,
        lorebook,
    )
    .unwrap();

    assert_eq!(summary.imported_count, 1);
    assert_eq!(summary.skipped_count, 1);

    let rules =
        tauri_app_lib::db::context_rules::list_enabled_context_rules(&db, &project_id).unwrap();

    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "Red umbrella station rule");
    assert_eq!(rules[0].primary_keywords, vec!["red umbrella", "红伞"]);
    assert_eq!(rules[0].secondary_keywords, vec!["old station", "旧车站"]);
    assert_eq!(rules[0].priority, 80);
    assert_eq!(rules[0].source_type, "sillytavern_lorebook");
    assert_eq!(
        rules[0].metadata["original_format"].as_str(),
        Some("sillytavern_world_info")
    );
}

fn insert_bible_fixture(db: &Database, project_id: &str) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO characters
            (id, project_id, name, aliases, role, personality, motivation, speech_style, appearance, backstory, relationship_map, locked_fields, status, metadata)
         VALUES ('char-export', ?1, '林澈', '[\"旧站警探\"]', 'protagonist', '克制', '追查旧案', '短句', '旧风衣', '曾经办错案', '{}', '[]', 'active', '{}')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO world_lore
            (id, project_id, lore_type, title, content, locked, status, metadata)
         VALUES ('lore-export', ?1, 'rule', '红伞规则', '红伞只能指向旧案代价。', 1, 'active', '{}')",
        rusqlite::params![project_id],
    )
    .unwrap();
}

#[test]
fn novel_bible_package_round_trips_stable_fields_with_source_provenance() {
    let db = setup_db();
    let source_project_id = insert_project(&db);
    insert_bible_fixture(&db, &source_project_id);

    let package =
        tauri_app_lib::workflow::package_io::export_novel_bible_package(&db, &source_project_id)
            .unwrap();

    assert_eq!(package.format, "ai_novel_factory.novel_bible");
    assert_eq!(package.format_version, 1);
    assert_eq!(package.source_project_id, source_project_id);
    assert_eq!(package.bible.characters[0].name, "林澈");
    assert_eq!(
        package.bible.world_lore[0].title.as_deref(),
        Some("红伞规则")
    );

    let target_project_id = insert_project(&db);
    let summary = tauri_app_lib::workflow::package_io::import_novel_bible_package(
        &db,
        &target_project_id,
        &package,
    )
    .unwrap();

    assert_eq!(summary.imported_characters, 1);
    assert_eq!(summary.imported_world_lore, 1);

    let imported = tauri_app_lib::db::bible::get_bible(&db, &target_project_id).unwrap();
    assert_eq!(imported.characters.len(), 1);
    assert_eq!(imported.characters[0].name, "林澈");
    assert_eq!(imported.world_lore.len(), 1);
    assert_eq!(imported.world_lore[0].title.as_deref(), Some("红伞规则"));
    let metadata: serde_json::Value =
        serde_json::from_str(&imported.characters[0].metadata).unwrap();
    assert_eq!(
        metadata["source_provenance"]["source_project_id"].as_str(),
        Some(source_project_id.as_str())
    );
    assert_eq!(
        metadata["source_provenance"]["source_id"].as_str(),
        Some("char-export")
    );
}

#[test]
fn invalid_novel_bible_package_rolls_back_without_partial_writes() {
    let db = setup_db();
    let source_project_id = insert_project(&db);
    insert_bible_fixture(&db, &source_project_id);
    let mut package =
        tauri_app_lib::workflow::package_io::export_novel_bible_package(&db, &source_project_id)
            .unwrap();
    package.bible.characters[0].name = "".to_string();

    let target_project_id = insert_project(&db);
    let err = tauri_app_lib::workflow::package_io::import_novel_bible_package(
        &db,
        &target_project_id,
        &package,
    )
    .expect_err("invalid package should fail before writing");

    assert!(err.contains("character name is required"));
    let imported = tauri_app_lib::db::bible::get_bible(&db, &target_project_id).unwrap();
    assert!(imported.characters.is_empty());
    assert!(imported.world_lore.is_empty());
}

#[test]
fn project_package_round_trips_project_plans_and_bible() {
    let db = setup_db();
    let source_project_id = insert_project(&db);
    insert_bible_fixture(&db, &source_project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-export-1', ?1, 1, '旧站开场', '林澈发现红伞。', 3000, 'planned')",
            rusqlite::params![source_project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status, word_count, summary)
             VALUES ('chapter-export-1', ?1, 'plan-export-1', 1, '旧站开场', 'final', 1200, '林澈发现红伞。')",
            rusqlite::params![source_project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapter_versions
             (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count, model_provider, model_name)
             VALUES ('version-export-1', 'chapter-export-1', ?1, 1, 'final', '旧站开场', '正文：红伞被留在旧车站的长椅下。', '林澈发现红伞。', 1200, 'openai_compat', 'fixture-model')",
            rusqlite::params![source_project_id],
        )
        .unwrap();
        conn.execute(
            "UPDATE chapters SET final_version_id = 'version-export-1' WHERE id = 'chapter-export-1'",
            [],
        )
        .unwrap();
    }

    let package =
        tauri_app_lib::workflow::package_io::export_project_package(&db, &source_project_id)
            .unwrap();

    assert_eq!(package.format, "ai_novel_factory.project");
    assert_eq!(package.project.name, "Lorebook Import");
    assert_eq!(package.chapter_plans.len(), 1);
    assert_eq!(package.chapters.len(), 1);
    assert_eq!(package.chapter_versions.len(), 1);
    assert_eq!(package.bible.bible.characters[0].name, "林澈");

    let imported_project_id =
        tauri_app_lib::workflow::package_io::import_project_package(&db, &package).unwrap();
    let imported_project =
        tauri_app_lib::db::projects::get_project(&db, &imported_project_id).unwrap();
    let imported_plans =
        tauri_app_lib::db::chapters::get_chapter_plans(&db, &imported_project_id).unwrap();
    let imported_chapters =
        tauri_app_lib::db::chapters::get_chapters(&db, &imported_project_id).unwrap();
    let imported_bible = tauri_app_lib::db::bible::get_bible(&db, &imported_project_id).unwrap();

    assert_eq!(imported_project.name, "Lorebook Import");
    assert_eq!(imported_project.genre.as_deref(), Some("mystery"));
    assert_eq!(imported_plans.len(), 1);
    assert_eq!(imported_plans[0].title.as_deref(), Some("旧站开场"));
    assert_eq!(imported_chapters.len(), 1);
    assert_eq!(imported_chapters[0].title.as_deref(), Some("旧站开场"));
    let imported_version =
        tauri_app_lib::db::chapters::get_latest_version(&db, &imported_chapters[0].id)
            .unwrap()
            .unwrap();
    assert_eq!(imported_version.version_type, "final");
    assert!(imported_version
        .body_markdown
        .as_deref()
        .unwrap_or("")
        .contains("红伞被留在旧车站"));
    assert_eq!(imported_bible.characters[0].name, "林澈");
    let metadata: serde_json::Value = serde_json::from_str(&imported_project.metadata).unwrap();
    assert_eq!(
        metadata["source_provenance"]["source_project_id"].as_str(),
        Some(source_project_id.as_str())
    );
}

#[test]
fn invalid_project_package_rolls_back_without_partial_writes() {
    let db = setup_db();
    let source_project_id = insert_project(&db);
    insert_bible_fixture(&db, &source_project_id);
    let mut package =
        tauri_app_lib::workflow::package_io::export_project_package(&db, &source_project_id)
            .unwrap();
    package.source_project_id = "".to_string();

    let before_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .unwrap()
    };
    let err = tauri_app_lib::workflow::package_io::import_project_package(&db, &package)
        .expect_err("invalid project package should fail before writing");

    assert!(err.contains("source_project_id is required"));
    let after_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .unwrap()
    };
    assert_eq!(after_count, before_count);
}

#[test]
fn project_package_round_trips_runtime_assets() {
    let db = setup_db();
    let source_project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
             VALUES ('plan-runtime-assets', ?1, 1, 'Runtime Assets', 'Use runtime assets', 3000, 'planned')",
            rusqlite::params![source_project_id],
        )
        .unwrap();
    }
    tauri_app_lib::db::context_rules::upsert_context_rule(
        &db,
        tauri_app_lib::db::context_rules::ContextRuleInput {
            id: Some("context-runtime-asset".to_string()),
            project_id: source_project_id.clone(),
            name: "Runtime Context Rule".to_string(),
            primary_keywords: vec!["runtime".to_string()],
            secondary_keywords: vec!["asset".to_string()],
            entity_refs: vec!["character:runtime".to_string()],
            chapter_ranges: vec!["1+".to_string()],
            priority: 42,
            token_budget: 120,
            sticky_chapters: 1,
            cooldown_chapters: 2,
            content: "Runtime context must survive project package import.".to_string(),
            source_type: "manual".to_string(),
            source_id: Some("runtime-rule-source".to_string()),
            enabled: true,
            metadata: serde_json::json!({"fixture": "runtime-assets"}),
        },
    )
    .unwrap();
    let preset_id = tauri_app_lib::db::prompt_presets::upsert_prompt_preset(
        &db,
        &tauri_app_lib::db::prompt_presets::PromptPresetInput {
            id: Some("preset-runtime-asset".to_string()),
            name: "Runtime Prompt Preset".to_string(),
            description: Some("project package prompt asset".to_string()),
            scope: "draft".to_string(),
            is_builtin: false,
            metadata: serde_json::json!({"fixture": "runtime-assets"}),
        },
    )
    .unwrap();
    tauri_app_lib::db::prompt_presets::upsert_prompt_unit(
        &db,
        &tauri_app_lib::db::prompt_presets::PromptUnitInput {
            preset_id,
            identifier: "runtime.system".to_string(),
            role: "system".to_string(),
            order: 10,
            enabled: true,
            injection_position: "system".to_string(),
            generation_phase: "draft".to_string(),
            content: "Runtime prompt asset.".to_string(),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    tauri_app_lib::db::model_profiles::upsert_model_profile(
        &db,
        &tauri_app_lib::db::model_profiles::ModelProfileInput {
            id: Some("profile-runtime-asset".to_string()),
            name: "Runtime Model Profile".to_string(),
            provider: "openai_compat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "runtime-model".to_string(),
            context_window: 32000,
            supports_json: true,
            supports_streaming: true,
            supports_embeddings: false,
            input_cost_per_million: Some(0.1),
            output_cost_per_million: Some(0.2),
            intended_use: "draft".to_string(),
            metadata: serde_json::json!({"fixture": "runtime-assets"}),
        },
    )
    .unwrap();
    tauri_app_lib::db::draft_alternatives::create_draft_candidate(
        &db,
        &tauri_app_lib::db::draft_alternatives::DraftCandidateInput {
            project_id: source_project_id.clone(),
            chapter_plan_id: "plan-runtime-assets".to_string(),
            candidate_number: 1,
            title: "Runtime Candidate".to_string(),
            body_markdown: "Candidate body should import under remapped plan.".to_string(),
            summary: Some("candidate summary".to_string()),
            word_count: 120,
            prompt_hash: "prompt-runtime".to_string(),
            context_hash: "context-runtime".to_string(),
            model_profile_id: Some("profile-runtime-asset".to_string()),
            review_notes: serde_json::json!({"score": 88}),
            estimated_cost_usd: Some(0.01),
            metadata: serde_json::json!({"fixture": "runtime-assets"}),
        },
    )
    .unwrap();
    tauri_app_lib::extensions::host::import_extension_package(
        &db,
        &tauri_app_lib::extensions::host::ExtensionPackage {
            manifest: tauri_app_lib::extensions::manifest::ExtensionManifest {
                id: "extension.runtime.asset".to_string(),
                name: "Runtime Extension Asset".to_string(),
                version: "1.0.0".to_string(),
                description: Some("project package extension asset".to_string()),
                enabled_by_default: false,
                permissions: vec!["project_read".to_string()],
                hooks: vec!["before_context_build".to_string()],
                package_kinds: vec!["context_rule_pack".to_string()],
                metadata: serde_json::json!({"fixture": "runtime-assets"}),
            },
            enabled: true,
            contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                hook: "before_context_build".to_string(),
                required_permission: Some("project_read".to_string()),
                metadata_patch: serde_json::json!({"runtime_extension": true}),
            }],
        },
    )
    .unwrap();

    let package =
        tauri_app_lib::workflow::package_io::export_project_package(&db, &source_project_id)
            .unwrap();

    assert_eq!(package.context_rules.len(), 1);
    assert!(package
        .prompt_presets
        .iter()
        .any(|preset| preset.id == "preset-runtime-asset"));
    assert!(package
        .model_profiles
        .iter()
        .any(|profile| profile.id == "profile-runtime-asset"));
    assert_eq!(package.draft_candidates.len(), 1);
    assert!(package
        .extension_packages
        .iter()
        .any(|extension| extension.manifest.id == "extension.runtime.asset"));

    let imported_project_id =
        tauri_app_lib::workflow::package_io::import_project_package(&db, &package).unwrap();
    let imported_rules =
        tauri_app_lib::db::context_rules::list_context_rules(&db, &imported_project_id).unwrap();
    assert_eq!(imported_rules.len(), 1);
    assert_eq!(imported_rules[0].name, "Runtime Context Rule");
    assert_eq!(
        imported_rules[0].primary_keywords,
        vec!["runtime".to_string()]
    );
    assert_eq!(
        imported_rules[0].metadata["source_provenance"]["source_id"].as_str(),
        Some("context-runtime-asset")
    );

    let imported_preset = tauri_app_lib::db::prompt_presets::export_prompt_preset_package(
        &db,
        "preset-runtime-asset",
    )
    .unwrap();
    assert_eq!(imported_preset.units[0].identifier, "runtime.system");

    let imported_profile =
        tauri_app_lib::db::model_profiles::get_model_profile(&db, "profile-runtime-asset").unwrap();
    assert_eq!(imported_profile.model, "runtime-model");

    let imported_plan_id =
        tauri_app_lib::db::chapters::get_chapter_plans(&db, &imported_project_id)
            .unwrap()
            .into_iter()
            .find(|plan| plan.title.as_deref() == Some("Runtime Assets"))
            .unwrap()
            .id;
    let imported_candidates =
        tauri_app_lib::db::draft_alternatives::list_draft_candidates(&db, &imported_plan_id)
            .unwrap();
    assert_eq!(imported_candidates.len(), 1);
    assert_eq!(imported_candidates[0].title, "Runtime Candidate");
    assert_eq!(imported_candidates[0].project_id, imported_project_id);

    let imported_extensions =
        tauri_app_lib::extensions::host::list_extension_packages(&db).unwrap();
    let runtime_extension = imported_extensions
        .iter()
        .find(|extension| extension.manifest.id == "extension.runtime.asset")
        .unwrap();
    assert!(!runtime_extension.enabled);
    assert_eq!(runtime_extension.contributions.len(), 1);
}
