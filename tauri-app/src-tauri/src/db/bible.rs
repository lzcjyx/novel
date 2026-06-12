use crate::db::connection::Database;
use crate::models::*;
use rusqlite::params;

pub fn get_bible(db: &Database, project_id: &str) -> Result<BibleData, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, aliases, role, personality, motivation, speech_style,
                appearance, backstory, relationship_map, locked_fields, status, metadata, created_at, updated_at
         FROM characters WHERE project_id = ?1 ORDER BY name"
    ).map_err(|e| format!("Prepare chars: {}", e))?;
    let characters = stmt
        .query_map(params![project_id], char_row)
        .map_err(|e| format!("Query chars: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect chars: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, type, description, rules, connected_locations, status, metadata, created_at, updated_at
         FROM locations WHERE project_id = ?1 ORDER BY name"
    ).map_err(|e| format!("Prepare locs: {}", e))?;
    let locations = stmt
        .query_map(params![project_id], loc_row)
        .map_err(|e| format!("Query locs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect locs: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, description, hierarchy, goals, relationship_map, status, metadata, created_at, updated_at
         FROM organizations WHERE project_id = ?1 ORDER BY name"
    ).map_err(|e| format!("Prepare orgs: {}", e))?;
    let organizations = stmt
        .query_map(params![project_id], org_row)
        .map_err(|e| format!("Query orgs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect orgs: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, item_type, owner_character_id, location_id, description, abilities, limitations, status, metadata, created_at, updated_at
         FROM items WHERE project_id = ?1 ORDER BY name"
    ).map_err(|e| format!("Prepare items: {}", e))?;
    let items = stmt
        .query_map(params![project_id], item_row)
        .map_err(|e| format!("Query items: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect items: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, lore_type, title, content, locked, status, metadata, created_at, updated_at
         FROM world_lore WHERE project_id = ?1 ORDER BY title"
    ).map_err(|e| format!("Prepare lore: {}", e))?;
    let world_lore = stmt
        .query_map(params![project_id], lore_row)
        .map_err(|e| format!("Query lore: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect lore: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, description, rules, limitations, progression, locked, status, metadata, created_at, updated_at
         FROM magic_or_power_systems WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare magic: {}", e))?;
    let magic_systems = stmt
        .query_map(params![project_id], magic_row)
        .map_err(|e| format!("Query magic: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect magic: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, rule_type, rule_text, severity, locked, status, metadata, created_at, updated_at
         FROM canon_rules WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare rules: {}", e))?;
    let canon_rules = stmt
        .query_map(params![project_id], rule_row)
        .map_err(|e| format!("Query rules: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect rules: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, description, priority, arc_status, introduced_chapter_id,
                expected_resolution_chapter_id, resolved_chapter_id, related_characters, related_chapters, metadata, created_at, updated_at
         FROM plot_threads WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare threads: {}", e))?;
    let plot_threads = stmt
        .query_map(params![project_id], thread_row)
        .map_err(|e| format!("Query threads: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect threads: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, clue_text, intended_payoff, introduced_chapter_id,
                expected_resolution_chapter_id, resolved_chapter_id, status, importance, metadata, created_at, updated_at
         FROM foreshadowing WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare foreshadow: {}", e))?;
    let foreshadowing = stmt
        .query_map(params![project_id], foreshadow_row)
        .map_err(|e| format!("Query foreshadow: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect foreshadow: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, style_text, positive_examples, negative_examples, status, metadata, created_at, updated_at
         FROM style_guides WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare style: {}", e))?;
    let style_guides = stmt
        .query_map(params![project_id], style_row)
        .map_err(|e| format!("Query style: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect style: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, chapter_id, event_time_label, sequence, event_summary,
                involved_characters, involved_locations, consequences, status, metadata, created_at, updated_at
         FROM timeline_events WHERE project_id = ?1 ORDER BY sequence"
    ).map_err(|e| format!("Prepare timeline: {}", e))?;
    let timeline_events = stmt
        .query_map(params![project_id], timeline_row)
        .map_err(|e| format!("Query timeline: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect timeline: {}", e))?;

    Ok(BibleData {
        characters,
        locations,
        organizations,
        items,
        world_lore,
        magic_systems,
        canon_rules,
        plot_threads,
        foreshadowing,
        style_guides,
        timeline_events,
    })
}

/// Get the latest character state for each character in a project
pub fn get_character_states(
    db: &Database,
    project_id: &str,
) -> Result<Vec<CharacterState>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT cs.id, cs.project_id, cs.character_id, cs.after_chapter_id,
                cs.physical_state, cs.emotional_state, cs.knowledge_state,
                cs.relationship_state, cs.location_id, cs.inventory,
                cs.open_conflicts, cs.metadata, cs.created_at, cs.updated_at
         FROM character_states cs
         INNER JOIN (
             SELECT character_id, MAX(after_chapter_id) as max_chapter
             FROM character_states WHERE project_id = ?1 GROUP BY character_id
         ) latest ON cs.character_id = latest.character_id AND cs.after_chapter_id = latest.max_chapter
         WHERE cs.project_id = ?1"
    ).map_err(|e| format!("Prepare char_states: {}", e))?;

    let states = stmt
        .query_map(rusqlite::params![project_id], state_row)
        .map_err(|e| format!("Query char_states: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect char_states: {}", e))?;
    Ok(states)
}

// Row mapping helpers
fn char_row(row: &rusqlite::Row) -> rusqlite::Result<Character> {
    Ok(Character {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        aliases: row.get(3)?,
        role: row.get(4)?,
        personality: row.get(5)?,
        motivation: row.get(6)?,
        speech_style: row.get(7)?,
        appearance: row.get(8)?,
        backstory: row.get(9)?,
        relationship_map: row.get(10)?,
        locked_fields: row.get(11)?,
        status: row.get(12)?,
        metadata: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn loc_row(row: &rusqlite::Row) -> rusqlite::Result<Location> {
    Ok(Location {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        r#type: row.get(3)?,
        description: row.get(4)?,
        rules: row.get(5)?,
        connected_locations: row.get(6)?,
        status: row.get(7)?,
        metadata: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

// Additional row mappers
fn org_row(row: &rusqlite::Row) -> rusqlite::Result<Organization> {
    Ok(Organization {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        hierarchy: row.get(4)?,
        goals: row.get(5)?,
        relationship_map: row.get(6)?,
        status: row.get(7)?,
        metadata: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn item_row(row: &rusqlite::Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        item_type: row.get(3)?,
        owner_character_id: row.get(4)?,
        location_id: row.get(5)?,
        description: row.get(6)?,
        abilities: row.get(7)?,
        limitations: row.get(8)?,
        status: row.get(9)?,
        metadata: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn lore_row(row: &rusqlite::Row) -> rusqlite::Result<WorldLore> {
    Ok(WorldLore {
        id: row.get(0)?,
        project_id: row.get(1)?,
        lore_type: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        locked: row.get::<_, i32>(5)? != 0,
        status: row.get(6)?,
        metadata: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn magic_row(row: &rusqlite::Row) -> rusqlite::Result<MagicSystem> {
    Ok(MagicSystem {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        rules: row.get(4)?,
        limitations: row.get(5)?,
        progression: row.get(6)?,
        locked: row.get::<_, i32>(7)? != 0,
        status: row.get(8)?,
        metadata: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn rule_row(row: &rusqlite::Row) -> rusqlite::Result<CanonRule> {
    Ok(CanonRule {
        id: row.get(0)?,
        project_id: row.get(1)?,
        rule_type: row.get(2)?,
        rule_text: row.get(3)?,
        severity: row.get(4)?,
        locked: row.get::<_, i32>(5)? != 0,
        status: row.get(6)?,
        metadata: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn thread_row(row: &rusqlite::Row) -> rusqlite::Result<PlotThread> {
    Ok(PlotThread {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        priority: row.get(4)?,
        arc_status: row.get(5)?,
        introduced_chapter_id: row.get(6)?,
        expected_resolution_chapter_id: row.get(7)?,
        resolved_chapter_id: row.get(8)?,
        related_characters: row.get(9)?,
        related_chapters: row.get(10)?,
        metadata: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn foreshadow_row(row: &rusqlite::Row) -> rusqlite::Result<Foreshadowing> {
    Ok(Foreshadowing {
        id: row.get(0)?,
        project_id: row.get(1)?,
        clue_text: row.get(2)?,
        intended_payoff: row.get(3)?,
        introduced_chapter_id: row.get(4)?,
        expected_resolution_chapter_id: row.get(5)?,
        resolved_chapter_id: row.get(6)?,
        status: row.get(7)?,
        importance: row.get(8)?,
        metadata: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn style_row(row: &rusqlite::Row) -> rusqlite::Result<StyleGuide> {
    Ok(StyleGuide {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        style_text: row.get(3)?,
        positive_examples: row.get(4)?,
        negative_examples: row.get(5)?,
        status: row.get(6)?,
        metadata: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn timeline_row(row: &rusqlite::Row) -> rusqlite::Result<TimelineEvent> {
    Ok(TimelineEvent {
        id: row.get(0)?,
        project_id: row.get(1)?,
        chapter_id: row.get(2)?,
        event_time_label: row.get(3)?,
        sequence: row.get(4)?,
        event_summary: row.get(5)?,
        involved_characters: row.get(6)?,
        involved_locations: row.get(7)?,
        consequences: row.get(8)?,
        status: row.get(9)?,
        metadata: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn state_row(row: &rusqlite::Row) -> rusqlite::Result<CharacterState> {
    Ok(CharacterState {
        id: row.get(0)?,
        project_id: row.get(1)?,
        character_id: row.get(2)?,
        after_chapter_id: row.get(3)?,
        physical_state: row.get(4)?,
        emotional_state: row.get(5)?,
        knowledge_state: row.get(6)?,
        relationship_state: row.get(7)?,
        location_id: row.get(8)?,
        inventory: row.get(9)?,
        open_conflicts: row.get(10)?,
        metadata: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}
