use crate::db::connection::Database;
use crate::models::LearningEntry;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleAssetScope {
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledStyleAssetPayload {
    pub asset_ids: Vec<String>,
    pub prompt_instructions: String,
    pub positive_examples: Vec<String>,
    pub negative_examples: Vec<String>,
    pub anti_ai_rules: Value,
}

pub fn compile_style_assets(
    db: &Database,
    project_id: &str,
    _scope: StyleAssetScope,
) -> Result<CompiledStyleAssetPayload, String> {
    let assets = crate::db::style_assets::list_style_assets(db, project_id, true)?;
    let asset_ids = assets
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<Vec<_>>();
    let prompt_instructions = assets
        .iter()
        .map(|asset| {
            let features =
                if asset.features.is_object() && !asset.features.as_object().unwrap().is_empty() {
                    format!(" features={}", asset.features)
                } else {
                    String::new()
                };
            format!(
                "- {} [{}]:{} priority={}",
                asset.name, asset.asset_type, features, asset.priority
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let positive_examples = assets
        .iter()
        .flat_map(|asset| asset.positive_examples.clone())
        .collect::<Vec<_>>();
    let negative_examples = assets
        .iter()
        .flat_map(|asset| asset.negative_examples.clone())
        .collect::<Vec<_>>();
    let forbidden_phrases = assets
        .iter()
        .flat_map(|asset| {
            phrases_from_rule_keys(
                &asset.anti_ai_rules,
                &["forbidden_phrases", "forbidden_patterns"],
            )
        })
        .collect::<Vec<_>>();
    let required_phrases = assets
        .iter()
        .flat_map(|asset| {
            phrases_from_rule_keys(
                &asset.anti_ai_rules,
                &["required_phrases", "required_patterns"],
            )
        })
        .collect::<Vec<_>>();

    Ok(CompiledStyleAssetPayload {
        asset_ids,
        prompt_instructions,
        positive_examples,
        negative_examples,
        anti_ai_rules: json!({
            "forbidden_phrases": dedup(forbidden_phrases),
            "required_phrases": dedup(required_phrases),
        }),
    })
}

pub fn create_draft_style_asset_from_learning_entry(
    db: &Database,
    project_id: &str,
    learning_entry_id: &str,
) -> Result<String, String> {
    let asset_id = format!("style-draft-{}", learning_entry_id.trim());
    if asset_id.trim() == "style-draft-" {
        return Err("Learning entry id is required".to_string());
    }

    if crate::db::style_assets::list_style_assets(db, project_id, false)?
        .iter()
        .any(|asset| asset.id == asset_id)
    {
        return Ok(asset_id);
    }

    let entry = load_learning_entry(db, project_id, learning_entry_id)?;
    if !is_style_learning_category(&entry.category) {
        return Err(format!(
            "Learning entry {} is not a style pattern",
            learning_entry_id
        ));
    }

    crate::db::style_assets::upsert_style_asset(
        db,
        &crate::db::style_assets::StyleAssetInput {
            id: Some(asset_id.clone()),
            project_id: project_id.to_string(),
            name: entry.pattern_name.clone(),
            asset_type: "learned_style_pattern".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: json!({
                "category": entry.category,
                "pattern_description": entry.pattern_description,
                "application_notes": entry.application_notes,
                "confidence": entry.confidence,
            }),
            positive_examples: entry.example_text.into_iter().collect(),
            negative_examples: Vec::new(),
            anti_ai_rules: json!({}),
            enabled: false,
            priority: confidence_priority(entry.confidence),
            metadata: json!({
                "source": "learning_entry",
                "source_learning_entry_id": entry.id,
                "approval_required": true,
            }),
        },
    )?;

    Ok(asset_id)
}

pub fn forbidden_phrases_from_payload(payload: &Value) -> Vec<String> {
    phrases_from_rule_keys(payload, &["forbidden_phrases", "forbidden_patterns"])
}

pub fn required_phrases_from_payload(payload: &Value) -> Vec<String> {
    phrases_from_rule_keys(payload, &["required_phrases", "required_patterns"])
}

fn phrases_from_rule_keys(rules: &Value, keys: &[&str]) -> Vec<String> {
    dedup(
        keys.iter()
            .flat_map(|key| phrases_from_rules(rules, key))
            .collect::<Vec<_>>(),
    )
}

fn phrases_from_rules(rules: &Value, key: &str) -> Vec<String> {
    rules
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|phrase| !phrase.is_empty())
        .map(str::to_string)
        .collect()
}

fn dedup(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if !output.iter().any(|existing| existing == &value) {
            output.push(value);
        }
    }
    output
}

fn load_learning_entry(
    db: &Database,
    project_id: &str,
    learning_entry_id: &str,
) -> Result<LearningEntry, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT id, project_id, source_type, source_url, source_title, category,
                pattern_name, pattern_description, example_text, application_notes,
                confidence, usage_count, last_used_at, metadata, created_at, updated_at
         FROM learning_entries
         WHERE project_id = ?1 AND id = ?2",
        rusqlite::params![project_id, learning_entry_id],
        |row| {
            Ok(LearningEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_type: row.get(2)?,
                source_url: row.get(3)?,
                source_title: row.get(4)?,
                category: row.get(5)?,
                pattern_name: row.get(6)?,
                pattern_description: row.get(7)?,
                example_text: row.get(8)?,
                application_notes: row.get(9)?,
                confidence: row.get(10)?,
                usage_count: row.get(11)?,
                last_used_at: row.get(12)?,
                metadata: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("Load learning entry: {}", e))?
    .ok_or_else(|| format!("Learning entry {} not found", learning_entry_id))
}

fn is_style_learning_category(category: &str) -> bool {
    matches!(
        category.trim().to_lowercase().as_str(),
        "style" | "style_pattern" | "prose" | "language"
    )
}

fn confidence_priority(confidence: f64) -> i32 {
    (confidence.clamp(0.0, 1.0) * 100.0).round() as i32
}
