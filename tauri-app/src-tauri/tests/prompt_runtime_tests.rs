use std::collections::HashMap;

use tauri_app_lib::db::connection::Database;
use tauri_app_lib::workflow::prompt_runtime::{
    assemble_builtin_draft_prompt, assemble_prompt_runtime, assembled_prompt_preview_payload,
    PromptRuntimeRequest, PromptUnit,
};

fn prompt_unit(
    identifier: &str,
    role: &str,
    order: i32,
    enabled: bool,
    generation_phase: &str,
    content: &str,
) -> PromptUnit {
    PromptUnit {
        identifier: identifier.to_string(),
        role: role.to_string(),
        order,
        enabled,
        injection_position: "main".to_string(),
        generation_phase: generation_phase.to_string(),
        content: content.to_string(),
        metadata: serde_json::json!({}),
    }
}

fn setup_db(name: &str) -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(name);
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

#[test]
fn prompt_runtime_orders_filters_and_estimates_active_units() {
    let request = PromptRuntimeRequest {
        prompt_name: "draft_writer".to_string(),
        generation_phase: "draft".to_string(),
        vars: HashMap::from([
            ("STORY_CONTEXT".to_string(), "Mirror City canon".to_string()),
            (
                "CHAPTER_BRIEF".to_string(),
                "Chapter 3 rooftop clue".to_string(),
            ),
        ]),
        units: vec![
            prompt_unit(
                "user-task",
                "user",
                20,
                true,
                "draft",
                "Write {{CHAPTER_BRIEF}}.",
            ),
            prompt_unit(
                "disabled-rule",
                "system",
                5,
                false,
                "draft",
                "DO NOT INCLUDE {{STORY_CONTEXT}}",
            ),
            prompt_unit(
                "review-only",
                "system",
                7,
                true,
                "review",
                "Review {{CHAPTER_BRIEF}}",
            ),
            prompt_unit(
                "system-context",
                "system",
                10,
                true,
                "draft",
                "Context: {{STORY_CONTEXT}}.",
            ),
        ],
    };

    let assembled = assemble_prompt_runtime(request).expect("prompt runtime should assemble");

    assert_eq!(assembled.system_prompt, "Context: Mirror City canon.");
    assert_eq!(assembled.user_prompt, "Write Chapter 3 rooftop clue.");
    assert!(assembled.token_estimate > 0);
    assert!(!assembled.system_prompt.contains("DO NOT INCLUDE"));
    assert!(!assembled.system_prompt.contains("Review"));
    assert_eq!(
        assembled
            .unit_traces
            .iter()
            .map(|trace| trace.identifier.as_str())
            .collect::<Vec<_>>(),
        vec!["system-context", "user-task"]
    );
    assert!(assembled
        .unit_traces
        .iter()
        .all(|trace| trace.token_estimate > 0));
}

#[test]
fn prompt_runtime_reports_unit_scoped_unresolved_variables() {
    let request = PromptRuntimeRequest {
        prompt_name: "draft_writer".to_string(),
        generation_phase: "draft".to_string(),
        vars: HashMap::from([("KNOWN".to_string(), "value".to_string())]),
        units: vec![prompt_unit(
            "system-context",
            "system",
            10,
            true,
            "draft",
            "{{KNOWN}} {{MISSING_CONTEXT}}",
        )],
    };

    let err = assemble_prompt_runtime(request)
        .expect_err("missing variables should fail before any provider call");

    assert!(err.contains("draft_writer"));
    assert!(err.contains("system-context"));
    assert!(err.contains("MISSING_CONTEXT"));
}

#[test]
fn prompt_runtime_orders_equal_order_units_by_identifier() {
    let request = PromptRuntimeRequest {
        prompt_name: "draft_writer".to_string(),
        generation_phase: "draft".to_string(),
        vars: HashMap::new(),
        units: vec![
            prompt_unit("b-task", "user", 10, true, "all", "B"),
            prompt_unit("a-task", "user", 10, true, "draft", "A"),
        ],
    };

    let assembled = assemble_prompt_runtime(request).expect("equal order should be stable");

    assert_eq!(assembled.user_prompt, "A\n\nB");
    assert_eq!(
        assembled
            .unit_traces
            .iter()
            .map(|trace| trace.identifier.as_str())
            .collect::<Vec<_>>(),
        vec!["a-task", "b-task"]
    );
}

#[test]
fn builtin_draft_prompt_preview_renders_context_and_user_instruction() {
    let assembled =
        assemble_builtin_draft_prompt(r#"{"chapter_plan":{"title":"Door Behind the Rain"}}"#)
            .expect("built-in draft prompt should assemble");

    assert_eq!(assembled.prompt_name, "draft_writer");
    assert_eq!(assembled.generation_phase, "draft");
    assert!(assembled.system_prompt.contains("Door Behind the Rain"));
    assert!(!assembled.system_prompt.contains("WRITING_CONTEXT_JSON"));
    assert!(assembled.user_prompt.contains("只输出合法 JSON"));
    assert_eq!(
        assembled
            .unit_traces
            .iter()
            .map(|trace| trace.identifier.as_str())
            .collect::<Vec<_>>(),
        vec!["draft_writer.system", "draft_writer.user"]
    );
}

#[test]
fn assembled_prompt_preview_payload_includes_exact_prompts_and_trace() {
    let assembled = assemble_builtin_draft_prompt(r#"{"project":{"name":"Mirror City"}}"#)
        .expect("built-in draft prompt should assemble");

    let payload = assembled_prompt_preview_payload(&assembled);

    assert_eq!(payload["prompt_name"].as_str(), Some("draft_writer"));
    assert!(payload["system_prompt"]
        .as_str()
        .unwrap_or("")
        .contains("Mirror City"));
    assert!(payload["user_prompt"]
        .as_str()
        .unwrap_or("")
        .contains("只输出合法 JSON"));
    assert!(payload["token_estimate"].as_i64().unwrap_or(0) > 0);
    assert_eq!(
        payload["unit_traces"][0]["identifier"].as_str(),
        Some("draft_writer.system")
    );
}

#[test]
fn prompt_preset_package_round_trips_units_in_stable_order() {
    let source_db = setup_db("prompt-preset-source.db");
    let preset_id = tauri_app_lib::db::prompt_presets::upsert_prompt_preset(
        &source_db,
        &tauri_app_lib::db::prompt_presets::PromptPresetInput {
            id: Some("preset-draft-default".to_string()),
            name: "Draft Default".to_string(),
            description: Some("Built-in draft prompt override".to_string()),
            scope: "project".to_string(),
            is_builtin: false,
            metadata: serde_json::json!({"fixture": "prompt-runtime"}),
        },
    )
    .unwrap();
    tauri_app_lib::db::prompt_presets::upsert_prompt_unit(
        &source_db,
        &tauri_app_lib::db::prompt_presets::PromptUnitInput {
            preset_id: preset_id.clone(),
            identifier: "draft.user".to_string(),
            role: "user".to_string(),
            order: 20,
            enabled: true,
            injection_position: "user".to_string(),
            generation_phase: "draft".to_string(),
            content: "Write the chapter.".to_string(),
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();
    tauri_app_lib::db::prompt_presets::upsert_prompt_unit(
        &source_db,
        &tauri_app_lib::db::prompt_presets::PromptUnitInput {
            preset_id: preset_id.clone(),
            identifier: "draft.system".to_string(),
            role: "system".to_string(),
            order: 10,
            enabled: true,
            injection_position: "system".to_string(),
            generation_phase: "draft".to_string(),
            content: "Use {{WRITING_CONTEXT_JSON}}.".to_string(),
            metadata: serde_json::json!({"source": "test"}),
        },
    )
    .unwrap();

    let package =
        tauri_app_lib::db::prompt_presets::export_prompt_preset_package(&source_db, &preset_id)
            .unwrap();

    assert_eq!(package.id, "preset-draft-default");
    assert_eq!(
        package
            .units
            .iter()
            .map(|unit| unit.identifier.as_str())
            .collect::<Vec<_>>(),
        vec!["draft.system", "draft.user"]
    );

    let target_db = setup_db("prompt-preset-target.db");
    let imported_id =
        tauri_app_lib::db::prompt_presets::import_prompt_preset_package(&target_db, &package)
            .unwrap();
    let imported =
        tauri_app_lib::db::prompt_presets::export_prompt_preset_package(&target_db, &imported_id)
            .unwrap();

    assert_eq!(imported.id, package.id);
    assert_eq!(imported.name, package.name);
    assert_eq!(
        imported
            .units
            .iter()
            .map(|unit| (unit.identifier.as_str(), unit.order, unit.enabled))
            .collect::<Vec<_>>(),
        vec![("draft.system", 10, true), ("draft.user", 20, true)]
    );
    assert_eq!(imported.units[0].content, "Use {{WRITING_CONTEXT_JSON}}.");
}

#[test]
fn prompt_presets_are_listed_by_scope_and_name() {
    let db = setup_db("prompt-preset-list.db");
    for (id, name, scope) in [
        ("preset-z", "Zeta Review", "review"),
        ("preset-a", "Alpha Draft", "draft"),
    ] {
        tauri_app_lib::db::prompt_presets::upsert_prompt_preset(
            &db,
            &tauri_app_lib::db::prompt_presets::PromptPresetInput {
                id: Some(id.to_string()),
                name: name.to_string(),
                description: None,
                scope: scope.to_string(),
                is_builtin: false,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let presets = tauri_app_lib::db::prompt_presets::list_prompt_presets(&db).unwrap();
    let ids = presets
        .iter()
        .map(|preset| preset.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["preset-a", "preset-z"]);
    assert_eq!(presets[0].scope, "draft");
}
