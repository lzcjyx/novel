use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("extension-host.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project_and_job(db: &Database) -> (String, String) {
    let project_id = tauri_app_lib::db::projects::create_project(
        db,
        "Extension Host",
        Some("hook trace fixture"),
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
        "INSERT INTO chapter_plans (id, project_id, sequence, title, status)
         VALUES ('plan-extension', ?1, 1, 'Extension hook', 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        db,
        &project_id,
        "plan-extension",
    )
    .unwrap();
    (project_id, job_id)
}

#[test]
fn extension_manifest_validates_permissions_hooks_and_disabled_default() {
    let manifest = tauri_app_lib::extensions::manifest::ExtensionManifest {
        id: "prompt-pack.red-umbrella".to_string(),
        name: "Red Umbrella Prompt Pack".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Declarative prompt/context package".to_string()),
        enabled_by_default: false,
        permissions: vec!["project_read".to_string(), "project_write".to_string()],
        hooks: vec![
            "before_context_build".to_string(),
            "after_context_build".to_string(),
            "export_target".to_string(),
        ],
        package_kinds: vec!["prompt_pack".to_string(), "context_rule_pack".to_string()],
        metadata: serde_json::json!({}),
    };

    tauri_app_lib::extensions::manifest::validate_extension_manifest(&manifest)
        .expect("declarative manifest should validate");
    assert!(!manifest.enabled_by_default);

    let mut invalid = manifest.clone();
    invalid.hooks.push("run_arbitrary_javascript".to_string());

    let err = tauri_app_lib::extensions::manifest::validate_extension_manifest(&invalid)
        .expect_err("unknown hooks must be rejected");
    assert!(err.contains("run_arbitrary_javascript"));
}

#[test]
fn enabled_extension_contributions_are_visible_as_workflow_inputs() {
    let output = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({"base": true}),
            extensions: vec![tauri_app_lib::extensions::host::ExtensionPackage {
                manifest: tauri_app_lib::extensions::manifest::ExtensionManifest {
                    id: "author.prompt.context".to_string(),
                    name: "Author Prompt Context".to_string(),
                    version: "1.0.0".to_string(),
                    description: None,
                    enabled_by_default: false,
                    permissions: vec!["project_read".to_string()],
                    hooks: vec!["before_context_build".to_string()],
                    package_kinds: vec!["prompt_pack".to_string(), "context_rule_pack".to_string()],
                    metadata: serde_json::json!({}),
                },
                enabled: true,
                contributions: vec![
                    tauri_app_lib::extensions::host::ExtensionContribution {
                        hook: "before_context_build".to_string(),
                        required_permission: Some("project_read".to_string()),
                        package_kind: Some("prompt_pack".to_string()),
                        contribution_id: Some("tight-suspense-style".to_string()),
                        payload: serde_json::json!({
                            "unit_identifier": "extension.tight_suspense",
                            "role": "system",
                            "content": "Use short sensory beats."
                        }),
                        metadata_patch: serde_json::json!({}),
                    },
                    tauri_app_lib::extensions::host::ExtensionContribution {
                        hook: "before_context_build".to_string(),
                        required_permission: Some("project_read".to_string()),
                        package_kind: Some("context_rule_pack".to_string()),
                        contribution_id: Some("ticket-ledger-rule".to_string()),
                        payload: serde_json::json!({
                            "primary_keywords": ["票据"],
                            "content": "Always include the ticket ledger facts."
                        }),
                        metadata_patch: serde_json::json!({}),
                    },
                ],
            }],
        },
    )
    .unwrap();

    let prompt_contributions = output.workflow_metadata["extension_contributions"]["prompt_pack"]
        .as_array()
        .expect("prompt pack contributions should be visible");
    assert_eq!(prompt_contributions.len(), 1);
    assert_eq!(
        prompt_contributions[0]["extension_id"].as_str(),
        Some("author.prompt.context")
    );
    assert_eq!(
        prompt_contributions[0]["contribution_id"].as_str(),
        Some("tight-suspense-style")
    );
    assert_eq!(
        prompt_contributions[0]["payload"]["content"].as_str(),
        Some("Use short sensory beats.")
    );

    let rule_contributions = output.workflow_metadata["extension_contributions"]
        ["context_rule_pack"]
        .as_array()
        .expect("context rule contributions should be visible");
    assert_eq!(rule_contributions.len(), 1);
    assert_eq!(
        output.hook_trace[0].detail.as_deref(),
        Some("prompt_pack:tight-suspense-style")
    );
}

#[test]
fn extension_contribution_kind_must_be_declared_by_manifest() {
    let err = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({}),
            extensions: vec![tauri_app_lib::extensions::host::ExtensionPackage {
                manifest: manifest(
                    "undeclared.prompt",
                    false,
                    vec!["project_read"],
                    vec!["before_context_build"],
                ),
                enabled: true,
                contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_context_build".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: Some("prompt_pack".to_string()),
                    contribution_id: Some("missing-kind".to_string()),
                    payload: serde_json::json!({"content": "Not declared"}),
                    metadata_patch: serde_json::json!({}),
                }],
            }],
        },
    )
    .expect_err("undeclared contribution kind should be rejected");

    assert!(err.contains("does not declare package kind 'prompt_pack'"));
}

#[test]
fn extension_context_rule_pack_adapts_into_activation_trace() {
    let mut trace = tauri_app_lib::workflow::context_activation::ContextActivationTrace::default();
    let plan = tauri_app_lib::models::ChapterPlan {
        id: "plan-extension-rule".to_string(),
        project_id: "project-extension-rule".to_string(),
        volume_id: None,
        sequence: 1,
        title: Some("票据旧账".to_string()),
        outline: Some("核对票据金额".to_string()),
        target_word_count: Some(3000),
        status: "planned".to_string(),
        pov_character_id: None,
        required_characters: String::new(),
        required_locations: String::new(),
        plot_goals: String::new(),
        required_foreshadowing: String::new(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let metadata = serde_json::json!({
        "extension_contributions": {
            "context_rule_pack": [{
                "extension_id": "rule.pack",
                "contribution_id": "ticket-ledger",
                "payload": {
                    "name": "Ticket Ledger",
                    "primary_keywords": ["票据"],
                    "content": "Always carry ticket ledger facts.",
                    "priority": 88,
                    "token_budget": 64
                }
            }]
        }
    });

    tauri_app_lib::workflow::context_activation::append_extension_context_rules(
        &mut trace, &metadata, &plan, None,
    )
    .expect("extension context rules should adapt");

    assert_eq!(trace.activated_rules.len(), 1);
    assert_eq!(
        trace.activated_rules[0].rule_id,
        "extension:rule.pack:ticket-ledger"
    );
    assert_eq!(
        trace.activated_rules[0].activation_reason,
        "extension_context_rule_pack"
    );
    assert!(trace
        .source_keys
        .contains(&"extension_context_rule:rule.pack:ticket-ledger".to_string()));
}

#[test]
fn extension_recipe_pack_adapts_into_operator_recipes() {
    let metadata = serde_json::json!({
        "extension_contributions": {
            "recipe_pack": [{
                "extension_id": "recipe.pack",
                "contribution_id": "context-only",
                "payload": {
                    "id": "extension.context_only",
                    "name": "Extension Context Only",
                    "description": "Build context from extension recipe",
                    "actions": [{
                        "kind": "build_context_preview",
                        "label": "Build context",
                        "parameters": {}
                    }]
                }
            }]
        }
    });

    let recipes =
        tauri_app_lib::workflow::operator_recipes::extension_recipes_from_metadata(&metadata)
            .expect("extension recipes should adapt");

    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].id, "extension.context_only");
    assert_eq!(recipes[0].actions[0].kind, "build_context_preview");
}

#[test]
fn extension_review_rubric_and_export_template_have_typed_adapters() {
    let metadata = serde_json::json!({
        "extension_contributions": {
            "review_rubric_pack": [{
                "extension_id": "rubric.pack",
                "contribution_id": "anti-ai",
                "payload": {"rubric_id": "anti-ai", "checks": ["low-information sentences"]}
            }],
            "export_template": [{
                "extension_id": "export.pack",
                "contribution_id": "markdown-audit",
                "payload": {"template_id": "markdown-audit", "format": "markdown"}
            }]
        }
    });

    let rubrics =
        tauri_app_lib::workflow::review_agents::extension_review_rubrics_from_metadata(&metadata);
    let templates =
        tauri_app_lib::workflow::run_artifacts::extension_export_templates_from_metadata(&metadata);

    assert_eq!(rubrics[0]["rubric_id"].as_str(), Some("anti-ai"));
    assert_eq!(templates[0]["template_id"].as_str(), Some("markdown-audit"));
}

fn manifest(
    id: &str,
    enabled_by_default: bool,
    permissions: Vec<&str>,
    hooks: Vec<&str>,
) -> tauri_app_lib::extensions::manifest::ExtensionManifest {
    tauri_app_lib::extensions::manifest::ExtensionManifest {
        id: id.to_string(),
        name: id.to_string(),
        version: "1.0.0".to_string(),
        description: None,
        enabled_by_default,
        permissions: permissions.into_iter().map(str::to_string).collect(),
        hooks: hooks.into_iter().map(str::to_string).collect(),
        package_kinds: vec!["context_rule_pack".to_string()],
        metadata: serde_json::json!({}),
    }
}

#[test]
fn disabled_extensions_do_not_change_hook_output() {
    let output = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({"base": true}),
            extensions: vec![tauri_app_lib::extensions::host::ExtensionPackage {
                manifest: manifest(
                    "disabled.context",
                    false,
                    vec!["project_read"],
                    vec!["before_context_build"],
                ),
                enabled: false,
                contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_context_build".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"changed": true}),
                }],
            }],
        },
    )
    .unwrap();

    assert_eq!(output.workflow_metadata, serde_json::json!({"base": true}));
    assert_eq!(output.hook_trace.len(), 1);
    assert_eq!(output.hook_trace[0].status, "skipped_disabled");
}

#[test]
fn extension_hook_denies_missing_permission() {
    let err = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({}),
            extensions: vec![tauri_app_lib::extensions::host::ExtensionPackage {
                manifest: manifest(
                    "writer.context",
                    false,
                    vec!["project_read"],
                    vec!["before_context_build"],
                ),
                enabled: true,
                contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_context_build".to_string(),
                    required_permission: Some("project_write".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"write": true}),
                }],
            }],
        },
    )
    .expect_err("missing project_write should be rejected");

    assert!(err.contains("permission denied"));
    assert!(err.contains("project_write"));
}

#[test]
fn enabled_extension_hooks_run_in_stable_order_and_write_trace() {
    let output = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({"base": true}),
            extensions: vec![
                tauri_app_lib::extensions::host::ExtensionPackage {
                    manifest: manifest(
                        "z.context",
                        false,
                        vec!["project_read"],
                        vec!["before_context_build"],
                    ),
                    enabled: true,
                    contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                        hook: "before_context_build".to_string(),
                        required_permission: Some("project_read".to_string()),
                        package_kind: None,
                        contribution_id: None,
                        payload: serde_json::json!(null),
                        metadata_patch: serde_json::json!({"z": 2}),
                    }],
                },
                tauri_app_lib::extensions::host::ExtensionPackage {
                    manifest: manifest(
                        "a.context",
                        false,
                        vec!["project_read"],
                        vec!["before_context_build"],
                    ),
                    enabled: true,
                    contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                        hook: "before_context_build".to_string(),
                        required_permission: Some("project_read".to_string()),
                        package_kind: None,
                        contribution_id: None,
                        payload: serde_json::json!(null),
                        metadata_patch: serde_json::json!({"a": 1}),
                    }],
                },
            ],
        },
    )
    .unwrap();

    assert_eq!(output.workflow_metadata["a"], serde_json::json!(1));
    assert_eq!(output.workflow_metadata["z"], serde_json::json!(2));
    let extension_ids = output
        .hook_trace
        .iter()
        .map(|event| event.extension_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(extension_ids, vec!["a.context", "z.context"]);
    assert!(output
        .hook_trace
        .iter()
        .all(|event| event.status == "applied"));
}

#[test]
fn extension_hook_execution_persists_trace_to_job_metadata() {
    let db = setup_db();
    let (project_id, job_id) = insert_project_and_job(&db);

    let output = tauri_app_lib::extensions::host::execute_extension_hook_for_job(
        &db,
        &job_id,
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({"base": true}),
            extensions: vec![tauri_app_lib::extensions::host::ExtensionPackage {
                manifest: manifest(
                    "trace.context",
                    false,
                    vec!["project_read"],
                    vec!["before_context_build"],
                ),
                enabled: true,
                contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
                    hook: "before_context_build".to_string(),
                    required_permission: Some("project_read".to_string()),
                    package_kind: None,
                    contribution_id: None,
                    payload: serde_json::json!(null),
                    metadata_patch: serde_json::json!({"trace": true}),
                }],
            }],
        },
    )
    .unwrap();

    assert_eq!(output.workflow_metadata["trace"], serde_json::json!(true));

    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&jobs[0].metadata).unwrap();
    assert_eq!(
        metadata["extension_hooks"][0]["hook"].as_str(),
        Some("before_context_build")
    );
    assert_eq!(
        metadata["extension_hooks"][0]["events"][0]["extension_id"].as_str(),
        Some("trace.context")
    );
    assert_eq!(
        metadata["extension_hooks"][0]["events"][0]["status"].as_str(),
        Some("applied")
    );
}

#[test]
fn imported_extension_packages_are_disabled_until_explicitly_enabled() {
    let db = setup_db();
    let package = tauri_app_lib::extensions::host::ExtensionPackage {
        manifest: manifest(
            "persisted.context",
            false,
            vec!["project_read"],
            vec!["before_context_build"],
        ),
        enabled: true,
        contributions: vec![tauri_app_lib::extensions::host::ExtensionContribution {
            hook: "before_context_build".to_string(),
            required_permission: Some("project_read".to_string()),
            package_kind: None,
            contribution_id: None,
            payload: serde_json::json!(null),
            metadata_patch: serde_json::json!({"persisted": true}),
        }],
    };

    let extension_id =
        tauri_app_lib::extensions::host::import_extension_package(&db, &package).unwrap();
    assert_eq!(extension_id, "persisted.context");

    let imported = tauri_app_lib::extensions::host::list_extension_packages(&db).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].manifest.id, "persisted.context");
    assert!(!imported[0].enabled);
    assert_eq!(imported[0].contributions.len(), 1);

    let disabled_output = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({}),
            extensions: imported,
        },
    )
    .unwrap();
    assert_eq!(disabled_output.workflow_metadata, serde_json::json!({}));
    assert_eq!(disabled_output.hook_trace[0].status, "skipped_disabled");

    tauri_app_lib::extensions::host::set_extension_enabled(&db, "persisted.context", true).unwrap();
    let enabled = tauri_app_lib::extensions::host::list_extension_packages(&db).unwrap();
    assert!(enabled[0].enabled);

    let enabled_output = tauri_app_lib::extensions::host::execute_extension_hook(
        tauri_app_lib::extensions::host::ExtensionHookRequest {
            hook: "before_context_build".to_string(),
            workflow_metadata: serde_json::json!({}),
            extensions: enabled,
        },
    )
    .unwrap();
    assert_eq!(
        enabled_output.workflow_metadata["persisted"],
        serde_json::json!(true)
    );
    assert_eq!(enabled_output.hook_trace[0].status, "applied");
}
