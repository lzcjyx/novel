use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::bible::{
    CanonRule, Character, CharacterState, Location, StyleGuide, TimelineEvent,
};
use crate::models::BibleData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonConsistencyIssue {
    pub rule_type: String,
    pub severity: String,
    pub message: String,
    pub evidence: String,
}

pub fn detect_canon_consistency_issues(
    chapter_text: &str,
    bible: &BibleData,
    character_states: &[CharacterState],
) -> Vec<CanonConsistencyIssue> {
    let mut issues = Vec::new();
    issues.extend(detect_forbidden_terms(chapter_text, bible));
    issues.extend(detect_dead_character_appearances(
        chapter_text,
        bible,
        character_states,
    ));
    issues
}

pub fn detect_canon_consistency_issues_from_json(
    chapter_text: &str,
    characters_json: &str,
    character_states_json: &str,
    canon_rules_json: &str,
) -> Vec<CanonConsistencyIssue> {
    let mut bible = BibleData::empty();
    bible.characters = parse_json_vec::<Character>(characters_json);
    bible.canon_rules = parse_json_vec::<CanonRule>(canon_rules_json);
    let states = parse_json_vec::<CharacterState>(character_states_json);
    detect_canon_consistency_issues(chapter_text, &bible, &states)
}

pub fn detect_review_precheck_issues_from_json(
    chapter_text: &str,
    writing_context_json: &str,
    characters_json: &str,
    character_states_json: &str,
    canon_rules_json: &str,
    locations_json: &str,
    timeline_json: &str,
    style_guide_json: &str,
    current_chapter_sequence: i32,
) -> Vec<CanonConsistencyIssue> {
    let mut issues = detect_canon_consistency_issues_from_json(
        chapter_text,
        characters_json,
        character_states_json,
        canon_rules_json,
    );
    let characters = parse_json_vec::<Character>(characters_json);
    let states = parse_json_vec::<CharacterState>(character_states_json);
    let locations = parse_json_vec::<Location>(locations_json);
    let timeline = parse_json_vec::<TimelineEvent>(timeline_json);
    let style_guides = parse_json_vec::<StyleGuide>(style_guide_json);
    issues.extend(detect_repeated_previous_ending(
        chapter_text,
        writing_context_json,
    ));
    issues.extend(detect_location_continuity_conflicts(
        chapter_text,
        &characters,
        &states,
        &locations,
    ));
    issues.extend(detect_future_timeline_events(
        chapter_text,
        &timeline,
        current_chapter_sequence,
    ));
    issues.extend(detect_style_drift_issues(chapter_text, &style_guides));
    issues
}

fn parse_json_vec<T: DeserializeOwned>(raw: &str) -> Vec<T> {
    serde_json::from_str::<Vec<T>>(raw).unwrap_or_default()
}

fn detect_forbidden_terms(chapter_text: &str, bible: &BibleData) -> Vec<CanonConsistencyIssue> {
    bible
        .canon_rules
        .iter()
        .filter(|rule| rule.status == "active")
        .flat_map(|rule| {
            let metadata = serde_json::from_str::<Value>(&rule.metadata).unwrap_or(Value::Null);
            metadata
                .get("forbidden_terms")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|term| term.as_str().map(str::to_string))
                .filter(|term| !term.trim().is_empty() && contains_text(chapter_text, term))
                .map(|term| CanonConsistencyIssue {
                    rule_type: "forbidden_term".to_string(),
                    severity: severity_for_rule(&rule.severity, rule.locked),
                    message: format!("Forbidden canon term appears in chapter: {term}"),
                    evidence: term,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn detect_dead_character_appearances(
    chapter_text: &str,
    bible: &BibleData,
    character_states: &[CharacterState],
) -> Vec<CanonConsistencyIssue> {
    bible
        .characters
        .iter()
        .filter(|character| {
            is_dead_status(&character.status)
                || latest_state_is_dead(&character.id, character_states)
        })
        .filter(|character| contains_text(chapter_text, &character.name))
        .map(|character| CanonConsistencyIssue {
            rule_type: "dead_character_appearance".to_string(),
            severity: "blocking".to_string(),
            message: format!(
                "Dead character appears as present action: {}",
                character.name
            ),
            evidence: character.name.clone(),
        })
        .collect()
}

fn detect_repeated_previous_ending(
    chapter_text: &str,
    writing_context_json: &str,
) -> Vec<CanonConsistencyIssue> {
    let Ok(context) = serde_json::from_str::<Value>(writing_context_json) else {
        return Vec::new();
    };
    let Some(hook) = context
        .get("continuity")
        .and_then(|continuity| continuity.get("previous_ending_hook"))
        .and_then(Value::as_str)
    else {
        return Vec::new();
    };
    let hook = normalize_for_overlap(hook);
    if hook.chars().count() < 48 {
        return Vec::new();
    }
    let repeated_span = tail_chars(&hook, 120);
    if repeated_span.chars().count() < 48 {
        return Vec::new();
    }
    let chapter = normalize_for_overlap(chapter_text);
    if !chapter.contains(&repeated_span) {
        return Vec::new();
    }

    vec![CanonConsistencyIssue {
        rule_type: "repeated_previous_ending".to_string(),
        severity: "blocking".to_string(),
        message: "Chapter repeats the previous ending hook verbatim instead of continuing it"
            .to_string(),
        evidence: tail_chars(&repeated_span, 80),
    }]
}

fn detect_location_continuity_conflicts(
    chapter_text: &str,
    characters: &[Character],
    character_states: &[CharacterState],
    locations: &[Location],
) -> Vec<CanonConsistencyIssue> {
    characters
        .iter()
        .filter(|character| contains_text(chapter_text, &character.name))
        .flat_map(|character| {
            let Some(state) = latest_located_state(&character.id, character_states) else {
                return Vec::new();
            };
            let Some(expected_location_id) = state.location_id.as_deref() else {
                return Vec::new();
            };
            let Some(expected_location) = locations
                .iter()
                .find(|location| location.id == expected_location_id)
            else {
                return Vec::new();
            };
            let locked = metadata_bool(&state.metadata, &["locked_location", "location_locked"]);
            let expected_location_is_present = contains_text(chapter_text, &expected_location.name);

            locations
                .iter()
                .filter(|location| location.id != expected_location_id)
                .filter(|location| !location.name.trim().is_empty())
                .filter(|location| contains_text(chapter_text, &location.name))
                .filter(|_| locked || !expected_location_is_present)
                .map(|location| CanonConsistencyIssue {
                    rule_type: "location_continuity_conflict".to_string(),
                    severity: if locked { "blocking" } else { "warning" }.to_string(),
                    message: if locked {
                        format!(
                            "Character {} appears at {} while locked to {}",
                            character.name, location.name, expected_location.name
                        )
                    } else {
                        format!(
                            "Character {} appears at {} while latest known location is {}",
                            character.name, location.name, expected_location.name
                        )
                    },
                    evidence: format!(
                        "{}: expected {}, found {}",
                        character.name, expected_location.name, location.name
                    ),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn detect_future_timeline_events(
    chapter_text: &str,
    timeline_events: &[TimelineEvent],
    current_chapter_sequence: i32,
) -> Vec<CanonConsistencyIssue> {
    timeline_events
        .iter()
        .filter(|event| {
            event
                .sequence
                .is_some_and(|sequence| sequence > current_chapter_sequence)
        })
        .filter(|event| {
            !event.status.eq_ignore_ascii_case("resolved")
                && !event.status.eq_ignore_ascii_case("abandoned")
                && !event.status.eq_ignore_ascii_case("inactive")
        })
        .filter_map(|event| {
            let summary = event.event_summary.as_deref()?.trim();
            if summary.chars().count() < 8 || !contains_text(chapter_text, summary) {
                return None;
            }

            Some(CanonConsistencyIssue {
                rule_type: "future_timeline_event".to_string(),
                severity: "blocking".to_string(),
                message: format!(
                    "Future scheduled timeline event appears before chapter {}",
                    event.sequence.unwrap_or_default()
                ),
                evidence: summary.to_string(),
            })
        })
        .collect()
}

fn detect_style_drift_issues(
    chapter_text: &str,
    style_guides: &[StyleGuide],
) -> Vec<CanonConsistencyIssue> {
    let mut issues = Vec::new();
    for guide in style_guides
        .iter()
        .filter(|guide| guide.status.is_empty() || guide.status == "active")
    {
        let rule_values = style_rule_values(guide);
        let plain_rules = plain_text_style_rules(guide.style_text.as_deref());
        let mut forbidden_phrases = collect_phrases(
            &rule_values,
            &["forbidden_phrases", "forbidden_terms", "avoid_phrases"],
        );
        forbidden_phrases.extend(plain_rules.forbidden_phrases.clone());
        let mut required_phrases = collect_phrases(
            &rule_values,
            &["required_phrases", "required_terms", "must_include_phrases"],
        );
        required_phrases.extend(plain_rules.required_phrases.clone());
        let forbidden_default = plain_rules.severity.as_deref().unwrap_or("blocking");
        let required_default = plain_rules.severity.as_deref().unwrap_or("warning");
        let forbidden_severity = configured_style_severity(&rule_values, forbidden_default);
        let required_severity = configured_style_severity(&rule_values, required_default);

        issues.extend(
            forbidden_phrases
                .into_iter()
                .filter(|phrase| contains_text(chapter_text, phrase))
                .map(|phrase| CanonConsistencyIssue {
                    rule_type: "style_forbidden_phrase".to_string(),
                    severity: forbidden_severity.clone(),
                    message: format!("Style guide forbids phrase: {phrase}"),
                    evidence: phrase,
                }),
        );

        if style_bool(
            &rule_values,
            &["enforce_required_phrases", "strict_required_phrases"],
        ) || plain_rules.enforce_required_phrases
        {
            issues.extend(
                required_phrases
                    .into_iter()
                    .filter(|phrase| !contains_text(chapter_text, phrase))
                    .map(|phrase| CanonConsistencyIssue {
                        rule_type: "style_required_phrase_missing".to_string(),
                        severity: required_severity.clone(),
                        message: format!("Style guide requires missing phrase: {phrase}"),
                        evidence: phrase,
                    }),
            );
        }
    }
    issues
}

fn latest_state_is_dead(character_id: &str, character_states: &[CharacterState]) -> bool {
    character_states
        .iter()
        .find(|state| state.character_id == character_id)
        .and_then(|state| state.physical_state.as_deref())
        .is_some_and(is_dead_status)
}

fn latest_located_state<'a>(
    character_id: &str,
    character_states: &'a [CharacterState],
) -> Option<&'a CharacterState> {
    character_states
        .iter()
        .rev()
        .find(|state| state.character_id == character_id && state.location_id.is_some())
}

fn is_dead_status(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.contains("dead")
        || normalized.contains("deceased")
        || normalized.contains("死亡")
        || normalized.contains("已死")
        || normalized.contains("阵亡")
}

fn severity_for_rule(severity: &str, locked: bool) -> String {
    if locked || severity.eq_ignore_ascii_case("hard") {
        "blocking".to_string()
    } else {
        "warning".to_string()
    }
}

fn contains_text(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle) || haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn metadata_bool(raw: &str, keys: &[&str]) -> bool {
    let Ok(metadata) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    keys.iter()
        .any(|key| metadata.get(*key).and_then(Value::as_bool).unwrap_or(false))
}

fn style_rule_values(guide: &StyleGuide) -> Vec<Value> {
    let mut values = Vec::new();
    if let Some(value) = parse_json_value(guide.style_text.as_deref()) {
        values.push(value);
    }
    if let Some(value) = parse_json_value(Some(&guide.metadata)) {
        values.push(value);
    }
    values
}

#[derive(Debug, Clone, Default)]
struct PlainTextStyleRules {
    forbidden_phrases: Vec<String>,
    required_phrases: Vec<String>,
    enforce_required_phrases: bool,
    severity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlainTextStyleSection {
    Forbidden,
    Required,
}

fn plain_text_style_rules(raw: Option<&str>) -> PlainTextStyleRules {
    let Some(raw) = raw else {
        return PlainTextStyleRules::default();
    };
    let mut rules = PlainTextStyleRules::default();
    let mut section: Option<PlainTextStyleSection> = None;

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some((label, value)) = line.split_once(':') {
            let normalized_label = label.trim().to_lowercase();
            let value = value.trim();
            if is_forbidden_style_label(&normalized_label) {
                section = Some(PlainTextStyleSection::Forbidden);
                rules.forbidden_phrases.extend(split_phrase_list(value));
                continue;
            }
            if is_required_style_label(&normalized_label) {
                section = Some(PlainTextStyleSection::Required);
                rules.required_phrases.extend(split_phrase_list(value));
                continue;
            }
            if matches!(
                normalized_label.as_str(),
                "enforce required phrases"
                    | "strict required phrases"
                    | "enforce_required_phrases"
                    | "strict_required_phrases"
            ) {
                rules.enforce_required_phrases = parse_plain_bool(value);
                section = None;
                continue;
            }
            if matches!(
                normalized_label.as_str(),
                "style precheck severity"
                    | "precheck severity"
                    | "style_precheck_severity"
                    | "precheck_severity"
                    | "severity"
            ) {
                if !value.is_empty() {
                    rules.severity = Some(value.to_string());
                }
                section = None;
                continue;
            }
        }

        let phrase = strip_plain_phrase_bullet(line);
        if phrase.is_empty() {
            continue;
        }
        match section {
            Some(PlainTextStyleSection::Forbidden) => {
                rules.forbidden_phrases.push(phrase.to_string())
            }
            Some(PlainTextStyleSection::Required) => {
                rules.required_phrases.push(phrase.to_string())
            }
            None => {}
        }
    }

    rules.forbidden_phrases = dedup_strings(rules.forbidden_phrases);
    rules.required_phrases = dedup_strings(rules.required_phrases);
    rules
}

fn parse_json_value(raw: Option<&str>) -> Option<Value> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(raw).ok()
}

fn collect_phrases(values: &[Value], keys: &[&str]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| keys.iter().filter_map(|key| value.get(*key)))
        .filter_map(Value::as_array)
        .flat_map(|phrases| phrases.iter())
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|phrase| !phrase.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_forbidden_style_label(label: &str) -> bool {
    matches!(
        label,
        "forbidden phrases" | "forbidden terms" | "avoid phrases" | "avoid terms" | "avoid"
    )
}

fn is_required_style_label(label: &str) -> bool {
    matches!(
        label,
        "required phrases"
            | "required terms"
            | "must include phrases"
            | "must include terms"
            | "must include"
    )
}

fn split_phrase_list(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split([',', '，', ';', '；', '|'])
        .map(strip_plain_phrase_bullet)
        .map(str::trim)
        .filter(|phrase| !phrase.is_empty())
        .map(str::to_string)
        .collect()
}

fn strip_plain_phrase_bullet(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix("- ")
        .or_else(|| raw.strip_prefix("* "))
        .unwrap_or(raw);
    let raw = raw
        .split_once(". ")
        .filter(|(prefix, _)| prefix.chars().all(|ch| ch.is_ascii_digit()))
        .map(|(_, rest)| rest)
        .unwrap_or(raw);
    raw.trim_matches(['"', '\'', '“', '”', '‘', '’']).trim()
}

fn parse_plain_bool(raw: &str) -> bool {
    matches!(
        raw.trim().to_lowercase().as_str(),
        "true" | "yes" | "y" | "1" | "on" | "enabled"
    )
}

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if !output.iter().any(|existing| existing == &value) {
            output.push(value);
        }
    }
    output
}

fn style_bool(values: &[Value], keys: &[&str]) -> bool {
    values.iter().any(|value| {
        keys.iter()
            .any(|key| value.get(*key).and_then(Value::as_bool).unwrap_or(false))
    })
}

fn configured_style_severity(values: &[Value], default: &str) -> String {
    values
        .iter()
        .find_map(|value| {
            value
                .get("style_precheck_severity")
                .or_else(|| value.get("precheck_severity"))
                .or_else(|| value.get("severity"))
                .and_then(Value::as_str)
        })
        .map(|severity| {
            if severity.eq_ignore_ascii_case("blocking") || severity.eq_ignore_ascii_case("hard") {
                "blocking"
            } else {
                "warning"
            }
        })
        .unwrap_or(default)
        .to_string()
}

fn normalize_for_overlap(text: &str) -> String {
    text.split_whitespace().collect::<String>().to_lowercase()
}

fn tail_chars(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}
