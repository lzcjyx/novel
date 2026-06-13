use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub enabled_by_default: bool,
    pub permissions: Vec<String>,
    pub hooks: Vec<String>,
    pub package_kinds: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

const ALLOWED_PERMISSIONS: &[&str] = &[
    "filesystem",
    "network",
    "model_calls",
    "project_read",
    "project_write",
];

const ALLOWED_HOOKS: &[&str] = &[
    "before_context_build",
    "after_context_build",
    "before_review",
    "after_review",
    "export_target",
];

const ALLOWED_PACKAGE_KINDS: &[&str] = &[
    "prompt_pack",
    "context_rule_pack",
    "review_rubric",
    "recipe_pack",
    "export_target",
];

fn validate_ident(value: &str, field: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("Extension manifest field '{}' is required", field));
    }
    if trimmed.len() > 128 {
        return Err(format!("Extension manifest field '{}' is too long", field));
    }
    Ok(())
}

fn validate_allowed(values: &[String], allowed: &[&str], field: &str) -> Result<(), String> {
    for value in values {
        if !allowed.contains(&value.as_str()) {
            return Err(format!(
                "Extension manifest has unsupported {} '{}'",
                field, value
            ));
        }
    }
    Ok(())
}

pub fn validate_extension_manifest(manifest: &ExtensionManifest) -> Result<(), String> {
    validate_ident(&manifest.id, "id")?;
    validate_ident(&manifest.name, "name")?;
    validate_ident(&manifest.version, "version")?;
    validate_allowed(&manifest.permissions, ALLOWED_PERMISSIONS, "permission")?;
    validate_allowed(&manifest.hooks, ALLOWED_HOOKS, "hook")?;
    validate_allowed(
        &manifest.package_kinds,
        ALLOWED_PACKAGE_KINDS,
        "package kind",
    )?;

    if manifest.enabled_by_default {
        return Err("Extensions must be disabled by default after import".to_string());
    }

    Ok(())
}
