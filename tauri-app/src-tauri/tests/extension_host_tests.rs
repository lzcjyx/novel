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
