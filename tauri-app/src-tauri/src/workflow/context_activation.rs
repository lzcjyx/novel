use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::connection::Database;
use crate::models::ChapterPlan;
use crate::workflow::writing_context::OperatorControls;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextActivationTrace {
    pub activated_rules: Vec<ContextRuleActivation>,
    pub source_keys: Vec<String>,
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
        } else if sticky_match {
            "sticky"
        } else {
            continue;
        };
        let content = clip_to_token_budget(&rule.content, rule.token_budget);
        let token_estimate = crate::db::generation_jobs::estimate_tokens(&content);
        let source_id = rule.source_id.clone().unwrap_or_else(|| rule.id.clone());
        activated_rules.push(ContextRuleActivation {
            rule_id: rule.id,
            name: rule.name,
            source_key: format!("{}:{}", rule.source_type, source_id),
            priority: rule.priority,
            token_estimate,
            content,
            matched_keywords: if activation_reason == "sticky" {
                Vec::new()
            } else {
                matched_primary
            },
            matched_secondary_keywords: if activation_reason == "sticky" {
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
    let source_keys = activated_rules
        .iter()
        .map(|activation| activation.source_key.clone())
        .collect();

    Ok(ContextActivationTrace {
        activated_rules,
        source_keys,
    })
}
