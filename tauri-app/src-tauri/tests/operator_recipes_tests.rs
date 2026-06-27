use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("operator-recipes.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_with_plan(db: &Database) -> (String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Recipe Runtime Test",
        Some("recipe execution"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("cold"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id;
    let plan_id = "plan-recipe-runtime".to_string();
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES (?1, ?2, 1, 'Recipe Preview', 'Build context only', 3000, 'planned')",
        rusqlite::params![plan_id, project_id],
    )
    .unwrap();
    drop(conn);
    (project_id, plan_id)
}

#[test]
fn built_in_operator_recipes_are_structured_actions() {
    let recipes = tauri_app_lib::workflow::operator_recipes::built_in_recipes();
    let recipe_ids = recipes
        .iter()
        .map(|recipe| recipe.id.as_str())
        .collect::<Vec<_>>();

    assert!(recipe_ids.contains(&"generate_three_draft_candidates"));
    assert!(recipe_ids.contains(&"rerun_style_reviewer"));
    assert!(recipe_ids.contains(&"build_context_preview"));
    assert!(recipe_ids.contains(&"repair_canon_consistency"));

    for recipe in recipes {
        assert!(!recipe.actions.is_empty());
        for action in recipe.actions {
            assert_ne!(action.kind.as_str(), "arbitrary_js");
            assert!(
                tauri_app_lib::workflow::operator_recipes::is_allowed_action_kind(&action.kind)
            );
        }
    }
}

#[test]
fn build_context_preview_recipe_persists_job_events_without_generating_chapter() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);

    let result = tauri_app_lib::workflow::operator_recipes::execute_builtin_recipe(
        &db,
        tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id.clone(),
            recipe_id: "build_context_preview".to_string(),
        },
    )
    .unwrap();

    assert!(result.ok);
    assert_eq!(result.status, "completed");
    assert_eq!(result.recipe_id, "build_context_preview");
    assert!(result
        .events
        .iter()
        .any(|event| event.step == "build_context_preview" && event.status == "done"));

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "completed");

    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(
        metadata["operator_recipe"]["recipe_id"].as_str(),
        Some("build_context_preview")
    );
    assert_eq!(
        metadata["operator_recipe"]["actions"][0]["kind"].as_str(),
        Some("build_context_preview")
    );
    assert!(
        metadata["operator_recipe"]["context_preview"]["prompt_runtime"]["prompt_name"]
            .as_str()
            .is_some()
    );
    assert!(metadata["phase_events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["step"] == "build_context_preview" && event["status"] == "done"));

    let chapter_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM chapters WHERE chapter_plan_id = ?1",
            rusqlite::params![plan_id],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(chapter_count, 0);
}

#[test]
fn recipe_cancellation_records_cancelled_job_with_reason() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);

    let result =
        tauri_app_lib::workflow::operator_recipes::execute_builtin_recipe_with_cancel_check(
            &db,
            tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
                project_id: project_id.clone(),
                chapter_plan_id: plan_id,
                recipe_id: "generate_three_draft_candidates".to_string(),
            },
            |_action_index, _action| true,
        )
        .unwrap();

    assert!(!result.ok);
    assert_eq!(result.status, "cancelled");
    assert_eq!(
        result.error_message.as_deref(),
        Some("operator cancellation requested")
    );

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "cancelled");
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(
        metadata["phase_summary"]["last_status"].as_str(),
        Some("cancelled")
    );
    assert_eq!(
        metadata["phase_summary"]["failure_reason"].as_str(),
        Some("operator cancellation requested")
    );
}

#[test]
fn generate_three_draft_candidates_recipe_persists_candidates_without_accepting_chapter() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);

    let result = tauri_app_lib::workflow::operator_recipes::execute_builtin_recipe(
        &db,
        tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id.clone(),
            recipe_id: "generate_three_draft_candidates".to_string(),
        },
    )
    .unwrap();

    assert!(result.ok);
    let candidates =
        tauri_app_lib::db::draft_alternatives::list_draft_candidates(&db, &plan_id).unwrap();
    assert_eq!(candidates.len(), 3);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.candidate_number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    for candidate in &candidates {
        assert!(candidate.title.contains("Candidate"));
        assert!(!candidate.prompt_hash.is_empty());
        assert!(!candidate.context_hash.is_empty());
        assert_eq!(
            candidate.metadata["operator_recipe"]["recipe_id"].as_str(),
            Some("generate_three_draft_candidates")
        );
    }

    let chapter_count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM chapters WHERE chapter_plan_id = ?1",
            rusqlite::params![plan_id],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(chapter_count, 0);
}

#[test]
fn recipe_invalid_action_fails_with_persisted_reason() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);
    let recipe = tauri_app_lib::workflow::operator_recipes::OperatorRecipe {
        id: "bad_recipe".to_string(),
        name: "Bad recipe".to_string(),
        description: "Contains a disallowed action".to_string(),
        actions: vec![
            tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                kind: "arbitrary_js".to_string(),
                label: "Run unsafe code".to_string(),
                parameters: json!({}),
            },
        ],
    };

    let result = tauri_app_lib::workflow::operator_recipes::execute_recipe(
        &db,
        tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id,
            recipe_id: recipe.id.clone(),
        },
        recipe,
        |_action_index, _action| false,
    )
    .unwrap();

    assert!(!result.ok);
    assert_eq!(result.status, "failed");
    assert!(result
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("Unsupported recipe action"));

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(jobs[0]
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("Unsupported recipe action"));
}

#[test]
fn recipe_run_events_expose_step_io_duration_and_artifacts() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);
    let recipe = tauri_app_lib::workflow::operator_recipes::OperatorRecipe {
        id: "rich_recipe_events".to_string(),
        name: "Rich recipe events".to_string(),
        description: "Exercise non-generating recipe actions with structured run output."
            .to_string(),
        actions: vec![
            tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                kind: "rerun_review_agent".to_string(),
                label: "Style reviewer dry run".to_string(),
                parameters: json!({"agent_name": "style_reviewer"}),
            },
            tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                kind: "repair_canon_consistency".to_string(),
                label: "Canon repair preview".to_string(),
                parameters: json!({"mode": "preview"}),
            },
            tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                kind: "summarize_source_to_canon_candidate".to_string(),
                label: "Canon candidate summary".to_string(),
                parameters: json!({"source_key": "chapter:1"}),
            },
        ],
    };

    let result = tauri_app_lib::workflow::operator_recipes::execute_recipe(
        &db,
        tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id,
            recipe_id: recipe.id.clone(),
        },
        recipe,
        |_action_index, _action| false,
    )
    .unwrap();

    assert!(result.ok);
    let done_events = result
        .events
        .iter()
        .filter(|event| event.status == "done" && event.step != "operator_recipe")
        .collect::<Vec<_>>();
    assert_eq!(done_events.len(), 3);
    for event in &done_events {
        assert!(event.input["parameters"].is_object());
        assert!(event.output["kind"].as_str().is_some());
        assert!(event.duration_ms < 60_000);
        assert!(
            event
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("ready"),
            "recipe actions should expose deterministic ready output instead of queued placeholders"
        );
        assert!(event
            .artifact_refs
            .iter()
            .any(|artifact| artifact.starts_with("recipe_action:")));
    }

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    let step_outputs = metadata["operator_recipe"]["step_outputs"]
        .as_array()
        .expect("recipe step outputs should be persisted");
    assert_eq!(step_outputs.len(), 3);
    assert_eq!(
        step_outputs[0]["output"]["kind"].as_str(),
        Some("review_agent_preview")
    );
}

#[test]
fn user_operator_recipes_persist_and_validate_against_action_whitelist() {
    let db = setup_db();
    let (project_id, _plan_id) = insert_project_with_plan(&db);

    let recipe_id = tauri_app_lib::workflow::operator_recipes::upsert_user_recipe(
        &db,
        &tauri_app_lib::workflow::operator_recipes::UserOperatorRecipeInput {
            id: Some("user-recipe-1".to_string()),
            project_id: project_id.clone(),
            name: "Context then style review".to_string(),
            description: "Build context and queue a style review".to_string(),
            parameter_schema: json!({"type": "object", "properties": {"agent": {"type": "string"}}}),
            actions: vec![
                tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                    kind: "build_context_preview".to_string(),
                    label: "Build context".to_string(),
                    parameters: json!({}),
                },
                tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                    kind: "rerun_review_agent".to_string(),
                    label: "Style review".to_string(),
                    parameters: json!({"agent_name": "style_reviewer"}),
                },
            ],
            enabled: true,
            metadata: json!({"fixture": "user-recipe"}),
        },
    )
    .expect("valid user recipe should persist");

    assert_eq!(recipe_id, "user-recipe-1");
    let recipes =
        tauri_app_lib::workflow::operator_recipes::list_user_recipes(&db, &project_id, true)
            .expect("user recipes should load");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].name, "Context then style review");
    assert_eq!(recipes[0].actions.len(), 2);
    assert_eq!(
        recipes[0].parameter_schema["properties"]["agent"]["type"].as_str(),
        Some("string")
    );

    let err = tauri_app_lib::workflow::operator_recipes::upsert_user_recipe(
        &db,
        &tauri_app_lib::workflow::operator_recipes::UserOperatorRecipeInput {
            id: Some("bad-user-recipe".to_string()),
            project_id,
            name: "Unsafe".to_string(),
            description: "Should fail".to_string(),
            parameter_schema: json!({}),
            actions: vec![
                tauri_app_lib::workflow::operator_recipes::OperatorRecipeAction {
                    kind: "arbitrary_js".to_string(),
                    label: "Unsafe".to_string(),
                    parameters: json!({}),
                },
            ],
            enabled: true,
            metadata: json!({}),
        },
    )
    .expect_err("unknown action must be rejected before persistence");
    assert!(err.contains("Unsupported recipe action"));
}

#[derive(Default)]
struct RecipeProvider {
    calls: Arc<Mutex<usize>>,
}

#[async_trait]
impl ModelClient for RecipeProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        let mut calls = self.calls.lock().unwrap();
        *calls += 1;
        Ok(json!({
            "title": format!("Provider Candidate {}", *calls),
            "body_markdown": format!("Provider generated candidate body {}", *calls),
            "summary": format!("Provider summary {}", *calls),
            "word_count": 240
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok("{}".to_string())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[tokio::test]
async fn generate_three_draft_candidates_recipe_can_use_fake_model_adapter() {
    let db = setup_db();
    let (project_id, plan_id) = insert_project_with_plan(&db);
    let provider = RecipeProvider::default();

    let result = tauri_app_lib::workflow::operator_recipes::execute_builtin_recipe_with_provider(
        &db,
        tauri_app_lib::workflow::operator_recipes::OperatorRecipeRunRequest {
            project_id: project_id.clone(),
            chapter_plan_id: plan_id.clone(),
            recipe_id: "generate_three_draft_candidates".to_string(),
        },
        &provider,
    )
    .await
    .unwrap();

    assert!(result.ok);
    assert_eq!(*provider.calls.lock().unwrap(), 3);
    let candidates =
        tauri_app_lib::db::draft_alternatives::list_draft_candidates(&db, &plan_id).unwrap();
    assert_eq!(candidates.len(), 3);
    assert!(candidates.iter().all(|candidate| candidate
        .body_markdown
        .contains("Provider generated candidate body")));
}
