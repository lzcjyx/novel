use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: String,
    pub role: Option<String>,
    pub personality: Option<String>,
    pub motivation: Option<String>,
    pub speech_style: Option<String>,
    pub appearance: Option<String>,
    pub backstory: Option<String>,
    #[serde(default)]
    pub relationship_map: String,
    #[serde(default)]
    pub locked_fields: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterState {
    pub id: String,
    pub project_id: String,
    pub character_id: String,
    pub after_chapter_id: Option<String>,
    pub physical_state: Option<String>,
    pub emotional_state: Option<String>,
    pub knowledge_state: Option<String>,
    #[serde(default)]
    pub relationship_state: String,
    pub location_id: Option<String>,
    #[serde(default)]
    pub inventory: String,
    #[serde(default)]
    pub open_conflicts: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub r#type: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    #[serde(default)]
    pub connected_locations: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub hierarchy: String,
    pub goals: Option<String>,
    #[serde(default)]
    pub relationship_map: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub item_type: Option<String>,
    pub owner_character_id: Option<String>,
    pub location_id: Option<String>,
    pub description: Option<String>,
    pub abilities: Option<String>,
    pub limitations: Option<String>,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldLore {
    pub id: String,
    pub project_id: String,
    pub lore_type: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub locked: bool,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicSystem {
    pub id: String,
    pub project_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub limitations: Option<String>,
    #[serde(default)]
    pub progression: String,
    pub locked: bool,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub event_time_label: Option<String>,
    pub sequence: Option<i32>,
    pub event_summary: Option<String>,
    #[serde(default)]
    pub involved_characters: String,
    #[serde(default)]
    pub involved_locations: String,
    #[serde(default)]
    pub consequences: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotThread {
    pub id: String,
    pub project_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub priority: i32,
    pub arc_status: String,
    pub introduced_chapter_id: Option<String>,
    pub expected_resolution_chapter_id: Option<String>,
    pub resolved_chapter_id: Option<String>,
    #[serde(default)]
    pub related_characters: String,
    #[serde(default)]
    pub related_chapters: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foreshadowing {
    pub id: String,
    pub project_id: String,
    pub clue_text: Option<String>,
    pub intended_payoff: Option<String>,
    pub introduced_chapter_id: Option<String>,
    pub expected_resolution_chapter_id: Option<String>,
    pub resolved_chapter_id: Option<String>,
    pub status: String,
    pub importance: i32,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonRule {
    pub id: String,
    pub project_id: String,
    pub rule_type: Option<String>,
    pub rule_text: Option<String>,
    pub severity: String,
    pub locked: bool,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleGuide {
    pub id: String,
    pub project_id: String,
    pub name: Option<String>,
    pub style_text: Option<String>,
    #[serde(default)]
    pub positive_examples: String,
    #[serde(default)]
    pub negative_examples: String,
    pub status: String,
    #[serde(default)]
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}
