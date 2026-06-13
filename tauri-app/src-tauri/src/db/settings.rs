use crate::db::connection::Database;
use crate::models::AppSettings;
use rusqlite::params;

fn parse_optional_cost(value: &str) -> Option<f64> {
    let trimmed = value.trim_matches('"').trim();
    if trimmed.is_empty() {
        return None;
    }

    trimmed
        .parse::<f64>()
        .ok()
        .filter(|cost| cost.is_finite() && *cost >= 0.0)
}

fn parse_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim_matches('"').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn get_settings(db: &Database) -> Result<AppSettings, String> {
    let mut settings = AppSettings::default();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    let pairs: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT key, value FROM system_settings WHERE status = 'active' AND project_id IS NULL"
        ).map_err(|e| format!("Prepare: {}", e))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Query: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    for (key, value) in pairs {
        let v = value.trim_matches('"');
        match key.as_str() {
            "provider" => settings.provider = v.to_string(),
            "model" => settings.model = v.to_string(),
            "base_url" => settings.base_url = v.to_string(),
            "embedding_model" => settings.embedding_model = v.to_string(),
            "embedding_dimension" => {
                if let Ok(d) = v.parse() {
                    settings.embedding_dim = d;
                }
            }
            "embedding_provider" => settings.embedding_provider = v.to_string(),
            "embedding_base_url" => settings.embedding_base_url = v.to_string(),
            "quality_threshold" => {
                if let Ok(d) = v.parse() {
                    settings.quality_threshold = d;
                }
            }
            "auto_publish" => {
                settings.auto_publish = v == "true";
            }
            "max_revise_count" => {
                if let Ok(d) = v.parse() {
                    settings.max_revise_count = d;
                }
            }
            "daily_target_words" => {
                if let Ok(d) = v.parse() {
                    settings.daily_target_words = d;
                }
            }
            "data_dir" => settings.data_dir = v.to_string(),
            "debug_mode" => {
                settings.debug_mode = v == "true";
            }
            "blog_provider" => settings.blog_provider = v.to_string(),
            "blog_url" => settings.blog_url = Some(v.to_string()),
            "blog_username" => settings.blog_username = Some(v.to_string()),
            "input_cost_per_million" => settings.input_cost_per_million = parse_optional_cost(v),
            "output_cost_per_million" => settings.output_cost_per_million = parse_optional_cost(v),
            "draft_model_profile_id" => settings.draft_model_profile_id = parse_optional_string(v),
            "review_model_profile_id" => {
                settings.review_model_profile_id = parse_optional_string(v)
            }
            "repair_model_profile_id" => {
                settings.repair_model_profile_id = parse_optional_string(v)
            }
            "embedding_model_profile_id" => {
                settings.embedding_model_profile_id = parse_optional_string(v)
            }
            "summarization_model_profile_id" => {
                settings.summarization_model_profile_id = parse_optional_string(v)
            }
            _ => {}
        }
    }
    Ok(settings)
}

pub fn save_setting(db: &Database, key: &str, value: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO system_settings (id, key, value, status, updated_at)
         VALUES (COALESCE((SELECT id FROM system_settings WHERE key = ?1 AND project_id IS NULL), ?2), ?1, ?3, 'active', datetime('now'))",
        params![key, Database::new_uuid(), value],
    ).map_err(|e| format!("Save setting: {}", e))?;
    Ok(())
}

fn save_optional_cost_setting(db: &Database, key: &str, value: Option<f64>) -> Result<(), String> {
    let value = value
        .filter(|cost| cost.is_finite() && *cost >= 0.0)
        .map(|cost| cost.to_string())
        .unwrap_or_default();
    save_setting(db, key, &value)
}

fn save_optional_string_setting(
    db: &Database,
    key: &str,
    value: Option<&str>,
) -> Result<(), String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    save_setting(db, key, &format!("\"{}\"", value))
}

pub fn save_settings(db: &Database, settings: &AppSettings) -> Result<(), String> {
    save_setting(db, "provider", &format!("\"{}\"", settings.provider))?;
    save_setting(db, "model", &format!("\"{}\"", settings.model))?;
    save_setting(db, "base_url", &format!("\"{}\"", settings.base_url))?;
    save_setting(
        db,
        "embedding_model",
        &format!("\"{}\"", settings.embedding_model),
    )?;
    save_setting(
        db,
        "embedding_dimension",
        &settings.embedding_dim.to_string(),
    )?;
    save_setting(
        db,
        "embedding_provider",
        &format!("\"{}\"", settings.embedding_provider),
    )?;
    save_setting(
        db,
        "embedding_base_url",
        &format!("\"{}\"", settings.embedding_base_url),
    )?;
    save_setting(
        db,
        "quality_threshold",
        &settings.quality_threshold.to_string(),
    )?;
    save_setting(
        db,
        "auto_publish",
        if settings.auto_publish {
            "true"
        } else {
            "false"
        },
    )?;
    save_setting(
        db,
        "max_revise_count",
        &settings.max_revise_count.to_string(),
    )?;
    save_setting(
        db,
        "daily_target_words",
        &settings.daily_target_words.to_string(),
    )?;
    save_setting(db, "data_dir", &format!("\"{}\"", settings.data_dir))?;
    save_setting(
        db,
        "debug_mode",
        if settings.debug_mode { "true" } else { "false" },
    )?;
    save_setting(
        db,
        "blog_provider",
        &format!("\"{}\"", settings.blog_provider),
    )?;
    if let Some(ref url) = settings.blog_url {
        save_setting(db, "blog_url", &format!("\"{}\"", url))?;
    }
    if let Some(ref user) = settings.blog_username {
        save_setting(db, "blog_username", &format!("\"{}\"", user))?;
    }
    save_optional_cost_setting(
        db,
        "input_cost_per_million",
        settings.input_cost_per_million,
    )?;
    save_optional_cost_setting(
        db,
        "output_cost_per_million",
        settings.output_cost_per_million,
    )?;
    save_optional_string_setting(
        db,
        "draft_model_profile_id",
        settings.draft_model_profile_id.as_deref(),
    )?;
    save_optional_string_setting(
        db,
        "review_model_profile_id",
        settings.review_model_profile_id.as_deref(),
    )?;
    save_optional_string_setting(
        db,
        "repair_model_profile_id",
        settings.repair_model_profile_id.as_deref(),
    )?;
    save_optional_string_setting(
        db,
        "embedding_model_profile_id",
        settings.embedding_model_profile_id.as_deref(),
    )?;
    save_optional_string_setting(
        db,
        "summarization_model_profile_id",
        settings.summarization_model_profile_id.as_deref(),
    )?;
    Ok(())
}
