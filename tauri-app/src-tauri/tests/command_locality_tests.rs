use std::fs;
use std::path::Path;

#[test]
fn runtime_commands_are_registered_from_domain_module() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("lib.rs should be readable");
    let runtime_source = fs::read_to_string(manifest_dir.join("src/commands/runtime.rs"))
        .expect("runtime commands should live in src/commands/runtime.rs");

    assert!(
        lib_source.contains("pub mod commands;"),
        "lib.rs should expose command modules"
    );

    for command in [
        "get_context_rules",
        "upsert_context_rule",
        "import_sillytavern_lorebook",
        "export_project_package",
        "upsert_prompt_preset",
        "upsert_model_profile",
        "run_operator_recipe",
        "select_draft_candidate",
        "import_extension_package",
        "get_author_memory_banks",
    ] {
        assert!(
            lib_source.contains(&format!("commands::runtime::{command}")),
            "{command} should be registered through commands::runtime"
        );
        assert!(
            runtime_source.contains(&format!("pub async fn {command}")),
            "{command} should be implemented in commands::runtime"
        );
        assert!(
            !lib_source.contains(&format!("async fn {command}")),
            "{command} implementation should not remain in lib.rs"
        );
    }
}
