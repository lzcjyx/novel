use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("provider-capability.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

#[test]
fn model_profile_round_trips_and_reports_workflow_warnings() {
    let db = setup_db();
    let profile_id = tauri_app_lib::db::model_profiles::upsert_model_profile(
        &db,
        &tauri_app_lib::db::model_profiles::ModelProfileInput {
            id: Some("profile-small-json-risk".to_string()),
            name: "Small JSON Risk".to_string(),
            provider: "openai_compat".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "local-small".to_string(),
            context_window: 4096,
            supports_json: false,
            supports_streaming: true,
            supports_embeddings: false,
            input_cost_per_million: Some(0.2),
            output_cost_per_million: Some(0.4),
            intended_use: "draft".to_string(),
            metadata: serde_json::json!({"fixture": "provider-capability"}),
        },
    )
    .unwrap();

    let profile = tauri_app_lib::db::model_profiles::get_model_profile(&db, &profile_id).unwrap();

    assert_eq!(profile.name, "Small JSON Risk");
    assert_eq!(profile.provider, "openai_compat");
    assert_eq!(profile.context_window, 4096);
    assert_eq!(profile.input_cost_per_million, Some(0.2));

    let warnings = tauri_app_lib::ai::provider_capabilities::validate_model_profile_for_workflow(
        &profile,
        tauri_app_lib::ai::provider_capabilities::ModelWorkflow::GraphRagDraft,
    );
    let warning_codes = warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect::<Vec<_>>();

    assert!(warning_codes.contains(&"context_window_too_small"));
    assert!(warning_codes.contains(&"json_not_guaranteed"));
    assert!(warning_codes.contains(&"embeddings_unsupported"));
}

#[test]
fn model_profiles_are_listed_as_named_active_profiles() {
    let db = setup_db();
    for (id, name, intended_use) in [
        ("profile-z", "Zeta Review", "review"),
        ("profile-a", "Alpha Draft", "draft"),
    ] {
        tauri_app_lib::db::model_profiles::upsert_model_profile(
            &db,
            &tauri_app_lib::db::model_profiles::ModelProfileInput {
                id: Some(id.to_string()),
                name: name.to_string(),
                provider: "openai_compat".to_string(),
                base_url: "http://localhost:11434/v1".to_string(),
                model: "local".to_string(),
                context_window: 16000,
                supports_json: true,
                supports_streaming: true,
                supports_embeddings: false,
                input_cost_per_million: None,
                output_cost_per_million: None,
                intended_use: intended_use.to_string(),
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    let profiles = tauri_app_lib::db::model_profiles::list_model_profiles(&db).unwrap();
    let names = profiles
        .iter()
        .map(|profile| profile.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["Alpha Draft", "Zeta Review"]);
    assert_eq!(profiles[0].id, "profile-a");
    assert_eq!(profiles[0].intended_use, "draft");
}

#[test]
fn provider_config_uses_workflow_profile_identity_with_settings_fallback() {
    let mut settings = tauri_app_lib::models::AppSettings::default();
    settings.provider = "deepseek".to_string();
    settings.base_url = "https://base-provider.test".to_string();
    settings.model = "base-model".to_string();
    settings.embedding_model = "base-embedding".to_string();

    let profile = tauri_app_lib::db::model_profiles::ModelProfile {
        id: "profile-draft".to_string(),
        name: "Draft Profile".to_string(),
        provider: "openai_compat".to_string(),
        base_url: "http://localhost:11434/v1".to_string(),
        model: "profile-draft-model".to_string(),
        context_window: 32000,
        supports_json: true,
        supports_streaming: true,
        supports_embeddings: false,
        input_cost_per_million: Some(1.0),
        output_cost_per_million: Some(2.0),
        intended_use: "draft".to_string(),
        metadata: serde_json::json!({}),
    };

    let profiled = tauri_app_lib::ai::factory::provider_config_for_model_profile(
        &settings,
        Some(&profile),
        "profile-key".to_string(),
    );
    assert_eq!(profiled.provider_type, "openai_compat");
    assert_eq!(profiled.base_url, "http://localhost:11434/v1");
    assert_eq!(profiled.model, "profile-draft-model");
    assert_eq!(profiled.embedding_model, "base-embedding");
    assert_eq!(profiled.api_key, "profile-key");

    let fallback = tauri_app_lib::ai::factory::provider_config_for_model_profile(
        &settings,
        None,
        "base-key".to_string(),
    );
    assert_eq!(fallback.provider_type, "deepseek");
    assert_eq!(fallback.base_url, "https://base-provider.test");
    assert_eq!(fallback.model, "base-model");
    assert_eq!(fallback.embedding_model, "base-embedding");
    assert_eq!(fallback.api_key, "base-key");
}

#[test]
fn summarization_workflow_requires_reliable_json_profile() {
    let profile = tauri_app_lib::db::model_profiles::ModelProfile {
        id: "profile-summarization-risk".to_string(),
        name: "Summarization Risk".to_string(),
        provider: "openai_compat".to_string(),
        base_url: "http://localhost:11434/v1".to_string(),
        model: "summary-model".to_string(),
        context_window: 32000,
        supports_json: false,
        supports_streaming: true,
        supports_embeddings: false,
        input_cost_per_million: None,
        output_cost_per_million: None,
        intended_use: "summarization".to_string(),
        metadata: serde_json::json!({}),
    };

    let warnings = tauri_app_lib::ai::provider_capabilities::validate_model_profile_for_workflow(
        &profile,
        tauri_app_lib::ai::provider_capabilities::ModelWorkflow::Summarization,
    );

    assert!(warnings
        .iter()
        .any(|warning| warning.code == "json_not_guaranteed" && warning.severity == "error"));
}
