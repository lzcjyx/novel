use crate::db::connection::Database;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorMemoryBanksSnapshot {
    pub project_id: String,
    pub banks: Vec<AuthorMemoryBank>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorMemoryBank {
    pub id: String,
    pub label: String,
    pub entries: Vec<AuthorMemoryBankEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorMemoryBankEntry {
    pub id: String,
    pub source_key: String,
    pub title: String,
    pub summary: String,
    pub status: Option<String>,
    pub edit_command: String,
    pub metadata: Value,
}

pub fn build_author_memory_banks(
    db: &Database,
    project_id: &str,
) -> Result<AuthorMemoryBanksSnapshot, String> {
    let bible = crate::db::bible::get_bible(db, project_id)?;
    let character_states = crate::db::bible::get_character_states(db, project_id)?;
    let hard_facts = crate::db::hard_facts::list_hard_facts(db, project_id, false)?;
    let style_assets = crate::db::style_assets::list_style_assets(db, project_id, false)?;
    let learning_entries = crate::workflow::learning::get_top_learning_entries(db, project_id, 50)?;

    let mut canon_entries = Vec::new();
    for character in bible.characters {
        canon_entries.push(AuthorMemoryBankEntry {
            id: character.id.clone(),
            source_key: format!("character:{}", character.id),
            title: character.name,
            summary: character.role.unwrap_or_else(|| "character".to_string()),
            status: Some(character.status),
            edit_command: "update_bible_entry".to_string(),
            metadata: json!({"table": "characters"}),
        });
    }
    for rule in bible.canon_rules {
        canon_entries.push(AuthorMemoryBankEntry {
            id: rule.id.clone(),
            source_key: format!("canon_rule:{}", rule.id),
            title: rule.rule_type.unwrap_or_else(|| "canon rule".to_string()),
            summary: rule.rule_text.unwrap_or_default(),
            status: Some(rule.status),
            edit_command: "update_bible_entry".to_string(),
            metadata: json!({"table": "canon_rules"}),
        });
    }

    let hard_fact_entries = hard_facts
        .into_iter()
        .map(|fact| AuthorMemoryBankEntry {
            id: fact.id.clone(),
            source_key: format!("hard_fact:{}", fact.id),
            title: format!("{} {}", fact.subject, fact.predicate),
            summary: fact.value_text,
            status: Some(fact.status),
            edit_command: "upsert_hard_fact".to_string(),
            metadata: json!({
                "fact_type": fact.fact_type,
                "object": fact.object,
                "chapter_id": fact.chapter_id,
                "chapter_version_id": fact.chapter_version_id,
            }),
        })
        .collect::<Vec<_>>();

    let character_state_entries = character_states
        .into_iter()
        .map(|state| AuthorMemoryBankEntry {
            id: state.id.clone(),
            source_key: format!("character_state:{}", state.id),
            title: state.character_id,
            summary: [
                state.physical_state,
                state.emotional_state,
                state.knowledge_state,
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" / "),
            status: None,
            edit_command: "update_bible_entry".to_string(),
            metadata: json!({
                "table": "character_states",
                "after_chapter_id": state.after_chapter_id,
                "location_id": state.location_id,
            }),
        })
        .collect::<Vec<_>>();

    let timeline_entries = bible
        .timeline_events
        .into_iter()
        .map(|event| AuthorMemoryBankEntry {
            id: event.id.clone(),
            source_key: format!("timeline_event:{}", event.id),
            title: event
                .event_time_label
                .unwrap_or_else(|| format!("Sequence {}", event.sequence.unwrap_or_default())),
            summary: event.event_summary.unwrap_or_default(),
            status: Some(event.status),
            edit_command: "update_bible_entry".to_string(),
            metadata: json!({
                "table": "timeline_events",
                "chapter_id": event.chapter_id,
                "sequence": event.sequence,
            }),
        })
        .collect::<Vec<_>>();

    let learning_entries = learning_entries
        .into_iter()
        .map(|entry| AuthorMemoryBankEntry {
            id: entry.id.clone(),
            source_key: format!("learning_entry:{}", entry.id),
            title: entry.pattern_name,
            summary: entry.pattern_description,
            status: None,
            edit_command: "delete_learning_entry".to_string(),
            metadata: json!({
                "category": entry.category,
                "source_type": entry.source_type,
                "confidence": entry.confidence,
            }),
        })
        .collect::<Vec<_>>();

    let style_asset_entries = style_assets
        .into_iter()
        .map(|asset| AuthorMemoryBankEntry {
            id: asset.id.clone(),
            source_key: format!("style_asset:{}", asset.id),
            title: asset.name,
            summary: asset.asset_type,
            status: Some(if asset.enabled { "enabled" } else { "disabled" }.to_string()),
            edit_command: "upsert_style_asset".to_string(),
            metadata: json!({
                "scope_type": asset.scope_type,
                "scope_id": asset.scope_id,
                "priority": asset.priority,
            }),
        })
        .collect::<Vec<_>>();

    Ok(AuthorMemoryBanksSnapshot {
        project_id: project_id.to_string(),
        banks: vec![
            bank("canon", "Canon", canon_entries),
            bank("hard_facts", "Hard Facts", hard_fact_entries),
            bank(
                "character_state",
                "Character State",
                character_state_entries,
            ),
            bank("timeline", "Timeline", timeline_entries),
            bank("learning_entries", "Learning Entries", learning_entries),
            bank("style_assets", "Style Assets", style_asset_entries),
        ],
    })
}

fn bank(id: &str, label: &str, entries: Vec<AuthorMemoryBankEntry>) -> AuthorMemoryBank {
    AuthorMemoryBank {
        id: id.to_string(),
        label: label.to_string(),
        entries,
    }
}
