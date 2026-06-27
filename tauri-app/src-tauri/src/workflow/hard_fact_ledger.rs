use crate::db::connection::Database;
use crate::db::hard_facts::{HardFact, HardFactInput};
use crate::models::ChapterPlan;
use crate::workflow::canon_consistency::CanonConsistencyIssue;
use crate::workflow::writing_context::OperatorControls;
use regex::Regex;
use rusqlite::params;
use sha2::{Digest, Sha256};

pub fn materialize_hard_facts_from_chapter_version(
    db: &Database,
    project_id: &str,
    chapter_id: &str,
    chapter_version_id: &str,
) -> Result<Vec<HardFact>, String> {
    let (title, version_type, body_markdown) =
        load_chapter_version_for_hard_facts(db, project_id, chapter_id, chapter_version_id)?;
    if !matches!(version_type.as_str(), "final" | "accepted") {
        return Ok(Vec::new());
    }
    let amount_re = Regex::new(r"[零〇一二两三四五六七八九十百千万亿\d]+(?:枚)?灵石")
        .map_err(|e| format!("Compile hard fact amount regex: {}", e))?;
    let mut ids = Vec::new();
    for matched in amount_re.find_iter(&body_markdown) {
        let amount = matched.as_str();
        let source_quote = source_quote_for_match(&body_markdown, matched.start(), matched.end());
        let fact_id = deterministic_hard_fact_id(chapter_version_id, "amount", amount);
        let id = crate::db::hard_facts::upsert_hard_fact(
            db,
            &HardFactInput {
                id: Some(fact_id),
                project_id: project_id.to_string(),
                chapter_id: Some(chapter_id.to_string()),
                chapter_version_id: Some(chapter_version_id.to_string()),
                fact_type: "amount".to_string(),
                subject: title.clone(),
                predicate: "records_amount".to_string(),
                object: amount.to_string(),
                value_text: format!("{} records {}", title, amount),
                certainty: 0.86,
                source_quote,
                scope: "project".to_string(),
                status: "active".to_string(),
                metadata: serde_json::json!({
                    "source": "chapter_final",
                    "extractor": "deterministic_amount",
                    "chapter_version_id": chapter_version_id,
                }),
            },
        )?;
        ids.push(id);
    }
    let facts = crate::db::hard_facts::list_hard_facts(db, project_id, false)?
        .into_iter()
        .filter(|fact| ids.iter().any(|id| id == &fact.id))
        .collect::<Vec<_>>();
    Ok(facts)
}

pub fn select_relevant_hard_facts(
    db: &Database,
    project_id: &str,
    plan: &ChapterPlan,
    controls: Option<&OperatorControls>,
    limit: usize,
) -> Result<Vec<HardFact>, String> {
    let target = relevance_target(plan, controls);
    let mut facts = crate::db::hard_facts::list_hard_facts(db, project_id, true)?
        .into_iter()
        .filter(|fact| fact_matches_target(fact, &target))
        .collect::<Vec<_>>();
    facts.sort_by(|a, b| {
        a.subject
            .cmp(&b.subject)
            .then(a.predicate.cmp(&b.predicate))
            .then(a.id.cmp(&b.id))
    });
    facts.truncate(limit);
    Ok(facts)
}

pub fn detect_hard_fact_contradictions(
    chapter_text: &str,
    facts: &[HardFact],
) -> Vec<CanonConsistencyIssue> {
    facts
        .iter()
        .filter(|fact| fact.status == "active")
        .filter(|fact| !fact.subject.trim().is_empty() && chapter_text.contains(&fact.subject))
        .filter_map(|fact| {
            let observed = observed_conflicting_value(chapter_text, fact)?;
            Some(CanonConsistencyIssue {
                rule_type: "hard_fact_conflict".to_string(),
                severity: "blocking".to_string(),
                message: format!(
                    "Hard fact conflict for {} / {}",
                    fact.subject, fact.predicate
                ),
                evidence: format!(
                    "expected '{}', observed '{}', source fact {}",
                    fact.object, observed, fact.id
                ),
            })
        })
        .collect()
}

fn relevance_target(plan: &ChapterPlan, controls: Option<&OperatorControls>) -> String {
    let mut parts = Vec::new();
    push_part(&mut parts, plan.title.as_deref());
    push_part(&mut parts, plan.outline.as_deref());
    push_part(&mut parts, plan.pov_character_id.as_deref());
    push_part(&mut parts, Some(&plan.required_characters));
    push_part(&mut parts, Some(&plan.required_locations));
    push_part(&mut parts, Some(&plan.plot_goals));
    push_part(&mut parts, Some(&plan.required_foreshadowing));
    if let Some(controls) = controls {
        push_part(&mut parts, controls.chapter_intent.as_deref());
        push_part(&mut parts, controls.must_include_beats.as_deref());
        push_part(&mut parts, controls.forbidden_moves.as_deref());
        push_part(&mut parts, controls.style_emphasis.as_deref());
    }
    parts.join("\n").to_lowercase()
}

fn push_part(parts: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        let value = value.trim();
        if !value.is_empty() {
            parts.push(value.to_string());
        }
    }
}

fn load_chapter_version_for_hard_facts(
    db: &Database,
    project_id: &str,
    chapter_id: &str,
    chapter_version_id: &str,
) -> Result<(String, String, String), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.query_row(
        "SELECT COALESCE(cv.title, c.title, 'Untitled'), cv.version_type, COALESCE(cv.body_markdown, '')
         FROM chapter_versions cv
         JOIN chapters c ON c.id = cv.chapter_id
         WHERE cv.id = ?1 AND cv.chapter_id = ?2 AND cv.project_id = ?3",
        params![chapter_version_id, chapter_id, project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|e| format!("Load chapter version for hard facts: {}", e))
}

fn deterministic_hard_fact_id(version_id: &str, fact_type: &str, object: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(version_id.as_bytes());
    hasher.update(b":");
    hasher.update(fact_type.as_bytes());
    hasher.update(b":");
    hasher.update(object.as_bytes());
    format!(
        "hardfact-{}",
        hex::encode(hasher.finalize())[..24].to_string()
    )
}

fn source_quote_for_match(text: &str, start: usize, end: usize) -> Option<String> {
    let quote_start = text[..start]
        .char_indices()
        .rev()
        .find(|(_, ch)| ['。', '！', '？', '\n'].contains(ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let quote_end = text[end..]
        .char_indices()
        .find(|(_, ch)| ['。', '！', '？', '\n'].contains(ch))
        .map(|(index, ch)| end + index + ch.len_utf8())
        .unwrap_or_else(|| text.len());
    let quote = text[quote_start..quote_end].trim();
    (!quote.is_empty()).then(|| quote.to_string())
}

fn fact_matches_target(fact: &HardFact, target: &str) -> bool {
    [
        fact.subject.as_str(),
        fact.object.as_str(),
        fact.value_text.as_str(),
    ]
    .iter()
    .any(|needle| {
        let needle = needle.trim().to_lowercase();
        !needle.is_empty() && target.contains(&needle)
    })
}

fn observed_conflicting_value(chapter_text: &str, fact: &HardFact) -> Option<String> {
    if chapter_text.contains(&fact.object) || chapter_text.contains(&fact.value_text) {
        return None;
    }
    if fact.fact_type == "amount" || fact.predicate.contains("amount") {
        let amount_re = Regex::new(r"[零〇一二两三四五六七八九十百千万亿\d]+(?:枚)?灵石").ok()?;
        return amount_re
            .find_iter(chapter_text)
            .map(|matched| matched.as_str().to_string())
            .find(|value| value != &fact.object);
    }
    if !fact.object.trim().is_empty() && chapter_text.contains(&fact.subject) {
        return Some("different value implied in chapter text".to_string());
    }
    None
}
