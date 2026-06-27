use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::db::connection::Database;
use crate::models::ChapterPlan;
use crate::workflow::writing_context::OperatorControls;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextActivationTrace {
    pub activated_rules: Vec<ContextRuleActivation>,
    pub source_keys: Vec<String>,
    #[serde(default)]
    pub source_trace: Vec<ContextSourceTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRuleActivation {
    pub rule_id: String,
    pub name: String,
    pub source_key: String,
    pub priority: i32,
    pub token_estimate: i32,
    pub content: String,
    pub matched_keywords: Vec<String>,
    pub matched_secondary_keywords: Vec<String>,
    pub activation_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSourceTrace {
    pub source_key: String,
    pub source_type: String,
    pub source_id: String,
    pub reason: String,
    pub label: String,
}

fn push_target(targets: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if !normalized.is_empty() {
            targets.push(normalized);
        }
    }
}

fn activation_targets(plan: &ChapterPlan, controls: Option<&OperatorControls>) -> String {
    let mut targets = Vec::new();
    push_target(&mut targets, plan.title.as_deref());
    push_target(&mut targets, plan.outline.as_deref());
    push_target(&mut targets, plan.pov_character_id.as_deref());
    push_target(&mut targets, Some(&plan.required_characters));
    push_target(&mut targets, Some(&plan.required_locations));
    push_target(&mut targets, Some(&plan.plot_goals));
    push_target(&mut targets, Some(&plan.required_foreshadowing));
    if let Some(controls) = controls {
        push_target(&mut targets, controls.chapter_intent.as_deref());
        push_target(&mut targets, controls.must_include_beats.as_deref());
        push_target(&mut targets, controls.forbidden_moves.as_deref());
        push_target(&mut targets, controls.style_emphasis.as_deref());
    }
    targets.join("\n").to_lowercase()
}

fn activation_entity_targets(plan: &ChapterPlan) -> HashSet<String> {
    let mut targets = HashSet::new();
    if let Some(pov) = plan.pov_character_id.as_deref() {
        push_entity_target(&mut targets, "character", pov);
    }
    push_json_or_text_entities(&mut targets, "character", &plan.required_characters);
    push_json_or_text_entities(&mut targets, "location", &plan.required_locations);
    push_json_or_text_entities(&mut targets, "plot_thread", &plan.plot_goals);
    push_json_or_text_entities(&mut targets, "foreshadowing", &plan.required_foreshadowing);
    targets
}

fn push_json_or_text_entities(targets: &mut HashSet<String>, entity_type: &str, raw: &str) {
    if let Ok(values) = serde_json::from_str::<Vec<String>>(raw) {
        for value in values {
            push_entity_target(targets, entity_type, &value);
        }
    } else {
        push_entity_target(targets, entity_type, raw);
    }
}

fn push_entity_target(targets: &mut HashSet<String>, entity_type: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    targets.insert(format!("{}:{}", entity_type, normalize_entity_ref(value)));
    targets.insert(value.to_lowercase());
}

fn normalize_entity_ref(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || !ch.is_ascii() {
                ch
            } else if ch.is_whitespace() {
                '-'
            } else {
                ch
            }
        })
        .collect()
}

fn matched_entity_refs(targets: &HashSet<String>, refs: &[String]) -> Vec<String> {
    refs.iter()
        .filter(|entity_ref| {
            let normalized = normalize_entity_ref(entity_ref);
            targets.contains(&normalized) || targets.contains(&entity_ref.to_lowercase())
        })
        .cloned()
        .collect()
}

fn matched_keywords(haystack: &str, keywords: &[String]) -> Vec<String> {
    keywords
        .iter()
        .filter(|keyword| haystack.contains(&keyword.to_lowercase()))
        .cloned()
        .collect()
}

fn clip_to_token_budget(content: &str, token_budget: i32) -> String {
    if token_budget <= 0 {
        return String::new();
    }
    if crate::db::generation_jobs::estimate_tokens(content) <= token_budget {
        return content.to_string();
    }
    let mut clipped = String::new();
    for ch in content.chars() {
        let candidate = format!("{}{}", clipped, ch);
        if crate::db::generation_jobs::estimate_tokens(&candidate) > token_budget {
            break;
        }
        clipped = candidate;
    }
    clipped
}

fn chapter_range_matches(sequence: i32, ranges: &[String]) -> bool {
    if ranges.is_empty() {
        return true;
    }

    ranges.iter().any(|range| {
        let normalized = range.trim();
        if normalized.is_empty() {
            return false;
        }
        if let Ok(single) = normalized.parse::<i32>() {
            return sequence == single;
        }
        if let Some(start) = normalized.strip_suffix('+') {
            return start
                .trim()
                .parse::<i32>()
                .map(|start| sequence >= start)
                .unwrap_or(false);
        }
        let separator = if normalized.contains("..") {
            ".."
        } else if normalized.contains('-') {
            "-"
        } else {
            return false;
        };
        let mut parts = normalized.splitn(2, separator);
        let start = parts
            .next()
            .and_then(|part| part.trim().parse::<i32>().ok());
        let end = parts
            .next()
            .and_then(|part| part.trim().parse::<i32>().ok());
        match (start, end) {
            (Some(start), Some(end)) if start <= end => sequence >= start && sequence <= end,
            (Some(start), Some(end)) => sequence >= end && sequence <= start,
            _ => false,
        }
    })
}

fn load_last_rule_activation_sequences(
    db: &Database,
    project_id: &str,
    before_sequence: i32,
) -> Result<HashMap<String, i32>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT c.sequence, cv.metadata
             FROM chapter_versions cv
             JOIN chapters c ON c.id = cv.chapter_id
             WHERE cv.project_id = ?1 AND c.sequence < ?2
             ORDER BY c.sequence DESC, cv.created_at DESC",
        )
        .map_err(|e| format!("Prepare context rule history: {}", e))?;
    let rows = stmt
        .query_map(rusqlite::params![project_id, before_sequence], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Query context rule history: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect context rule history: {}", e))?;

    let mut history = HashMap::new();
    for (sequence, metadata) in rows {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&metadata) else {
            continue;
        };
        let Some(activated_rules) = value
            .get("context_activation")
            .and_then(|trace| trace.get("activated_rules"))
            .and_then(|rules| rules.as_array())
        else {
            continue;
        };
        for activation in activated_rules {
            if let Some(rule_id) = activation.get("rule_id").and_then(|id| id.as_str()) {
                history.entry(rule_id.to_string()).or_insert(sequence);
            }
        }
    }

    Ok(history)
}

fn within_recent_window(
    history: &HashMap<String, i32>,
    rule_id: &str,
    current_sequence: i32,
    window: i32,
) -> bool {
    if window <= 0 {
        return false;
    }
    history
        .get(rule_id)
        .map(|last_sequence| {
            let distance = current_sequence - last_sequence;
            distance > 0 && distance <= window
        })
        .unwrap_or(false)
}

pub fn activate_context_rules(
    db: &Database,
    project_id: &str,
    plan: &ChapterPlan,
    controls: Option<&OperatorControls>,
) -> Result<ContextActivationTrace, String> {
    let target_text = activation_targets(plan, controls);
    let entity_targets = activation_entity_targets(plan);
    let unpinned = controls
        .map(|controls| {
            controls
                .unpinned_source_keys
                .iter()
                .cloned()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let rules = crate::db::context_rules::list_enabled_context_rules(db, project_id)?;
    let recent_activations = load_last_rule_activation_sequences(db, project_id, plan.sequence)?;
    let mut activated_rules = Vec::new();

    for rule in rules {
        if !chapter_range_matches(plan.sequence, &rule.chapter_ranges) {
            continue;
        }
        let matched_primary = matched_keywords(&target_text, &rule.primary_keywords);
        let matched_secondary = matched_keywords(&target_text, &rule.secondary_keywords);
        let keyword_match = !matched_primary.is_empty()
            && (rule.secondary_keywords.is_empty() || !matched_secondary.is_empty());
        let matched_entities = matched_entity_refs(&entity_targets, &rule.entity_refs);
        let entity_ref_match = !matched_entities.is_empty();
        let sticky_match = within_recent_window(
            &recent_activations,
            &rule.id,
            plan.sequence,
            rule.sticky_chapters,
        );
        let activation_reason = if keyword_match {
            if within_recent_window(
                &recent_activations,
                &rule.id,
                plan.sequence,
                rule.cooldown_chapters,
            ) {
                continue;
            }
            "keyword"
        } else if entity_ref_match {
            "entity_ref"
        } else if sticky_match {
            "sticky"
        } else {
            continue;
        };
        let content = clip_to_token_budget(&rule.content, rule.token_budget);
        let token_estimate = crate::db::generation_jobs::estimate_tokens(&content);
        let source_id = rule.source_id.clone().unwrap_or_else(|| rule.id.clone());
        let source_key = format!("{}:{}", rule.source_type, source_id);
        if unpinned.contains(&source_key) {
            continue;
        }
        activated_rules.push(ContextRuleActivation {
            rule_id: rule.id,
            name: rule.name,
            source_key,
            priority: rule.priority,
            token_estimate,
            content,
            matched_keywords: if activation_reason == "sticky" || activation_reason == "entity_ref"
            {
                Vec::new()
            } else {
                matched_primary
            },
            matched_secondary_keywords: if activation_reason == "sticky"
                || activation_reason == "entity_ref"
            {
                Vec::new()
            } else {
                matched_secondary
            },
            activation_reason: activation_reason.to_string(),
        });
    }

    activated_rules.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.name.cmp(&b.name))
            .then(a.rule_id.cmp(&b.rule_id))
    });
    let mut source_keys = activated_rules
        .iter()
        .map(|activation| activation.source_key.clone())
        .collect::<Vec<_>>();
    let mut source_trace = Vec::new();
    if let Some(controls) = controls {
        for source_key in &controls.pinned_source_keys {
            let source_key = source_key.trim();
            if source_key.is_empty() || source_keys.iter().any(|existing| existing == source_key) {
                continue;
            }
            source_keys.push(source_key.to_string());
            let (source_type, source_id) = source_key
                .split_once(':')
                .map(|(source_type, source_id)| (source_type.to_string(), source_id.to_string()))
                .unwrap_or_else(|| ("manual".to_string(), source_key.to_string()));
            source_trace.push(ContextSourceTrace {
                source_key: source_key.to_string(),
                source_type,
                source_id,
                reason: "manual_pin".to_string(),
                label: source_key.to_string(),
            });
        }
    }

    Ok(ContextActivationTrace {
        activated_rules,
        source_keys,
        source_trace,
    })
}

pub fn append_extension_context_rules(
    trace: &mut ContextActivationTrace,
    workflow_metadata: &serde_json::Value,
    plan: &ChapterPlan,
    controls: Option<&OperatorControls>,
) -> Result<(), String> {
    let target_text = activation_targets(plan, controls);
    let payloads = crate::extensions::host::extension_contribution_payloads(
        workflow_metadata,
        "context_rule_pack",
    );
    for (index, payload) in payloads.into_iter().enumerate() {
        let extension_id = extension_contribution_field(
            workflow_metadata,
            "context_rule_pack",
            index,
            "extension_id",
        )
        .unwrap_or_else(|| "extension".to_string());
        let contribution_id = extension_contribution_field(
            workflow_metadata,
            "context_rule_pack",
            index,
            "contribution_id",
        )
        .unwrap_or_else(|| format!("context-rule-{}", index + 1));
        let primary_keywords = payload
            .get("primary_keywords")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let matched_primary = if primary_keywords.is_empty() {
            Vec::new()
        } else {
            matched_keywords(&target_text, &primary_keywords)
        };
        if !primary_keywords.is_empty() && matched_primary.is_empty() {
            continue;
        }
        let content = payload
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        if content.is_empty() {
            continue;
        }
        let token_budget = payload
            .get("token_budget")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(256) as i32;
        let content = clip_to_token_budget(content, token_budget);
        let source_key = format!(
            "extension_context_rule:{}:{}",
            extension_id, contribution_id
        );
        trace.activated_rules.push(ContextRuleActivation {
            rule_id: format!("extension:{}:{}", extension_id, contribution_id),
            name: payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&contribution_id)
                .to_string(),
            source_key: source_key.clone(),
            priority: payload
                .get("priority")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(50) as i32,
            token_estimate: crate::db::generation_jobs::estimate_tokens(&content),
            content,
            matched_keywords: matched_primary,
            matched_secondary_keywords: Vec::new(),
            activation_reason: "extension_context_rule_pack".to_string(),
        });
        if !trace.source_keys.contains(&source_key) {
            trace.source_keys.push(source_key.clone());
        }
        trace.source_trace.push(ContextSourceTrace {
            source_key,
            source_type: "extension_context_rule".to_string(),
            source_id: format!("{}:{}", extension_id, contribution_id),
            reason: "extension_context_rule_pack".to_string(),
            label: payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&contribution_id)
                .to_string(),
        });
    }
    trace.activated_rules.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.name.cmp(&b.name))
            .then(a.rule_id.cmp(&b.rule_id))
    });
    Ok(())
}

fn extension_contribution_field(
    workflow_metadata: &serde_json::Value,
    package_kind: &str,
    index: usize,
    field: &str,
) -> Option<String> {
    workflow_metadata
        .get("extension_contributions")
        .and_then(|value| value.get(package_kind))
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.get(index))
        .and_then(|item| item.get(field))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}
