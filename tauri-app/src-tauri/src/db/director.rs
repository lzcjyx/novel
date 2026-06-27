use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionCandidateInput {
    pub id: Option<String>,
    pub project_id: Option<String>,
    pub inspiration: String,
    pub title_options: Vec<String>,
    pub positioning: String,
    pub target_reader: String,
    pub core_hook: String,
    pub series_promise: String,
    pub first_30_chapter_promise: String,
    pub world_seed: serde_json::Value,
    pub character_seed: serde_json::Value,
    pub volume_strategy: serde_json::Value,
    pub golden_three_chapters: serde_json::Value,
    pub checkpoint_status: String,
    pub revision_note: Option<String>,
    pub selected: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionCandidate {
    pub id: String,
    pub project_id: Option<String>,
    pub inspiration: String,
    pub title_options: Vec<String>,
    pub positioning: String,
    pub target_reader: String,
    pub core_hook: String,
    pub series_promise: String,
    pub first_30_chapter_promise: String,
    pub world_seed: serde_json::Value,
    pub character_seed: serde_json::Value,
    pub volume_strategy: serde_json::Value,
    pub golden_three_chapters: serde_json::Value,
    pub checkpoint_status: String,
    pub revision_note: Option<String>,
    pub selected: bool,
    pub metadata: serde_json::Value,
}

fn to_json(value: &serde_json::Value, label: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("Serialize {}: {}", label, e))
}

fn string_array_to_json(values: &[String]) -> Result<String, String> {
    let cleaned = values
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    serde_json::to_string(&cleaned).map_err(|e| format!("Serialize title options: {}", e))
}

fn parse_json(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn parse_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn row_to_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<DirectionCandidate> {
    Ok(DirectionCandidate {
        id: row.get(0)?,
        project_id: row.get(1)?,
        inspiration: row.get(2)?,
        title_options: parse_string_array(row.get(3)?),
        positioning: row.get(4)?,
        target_reader: row.get(5)?,
        core_hook: row.get(6)?,
        series_promise: row.get(7)?,
        first_30_chapter_promise: row.get(8)?,
        world_seed: parse_json(row.get(9)?),
        character_seed: parse_json(row.get(10)?),
        volume_strategy: parse_json(row.get(11)?),
        golden_three_chapters: parse_json(row.get(12)?),
        checkpoint_status: row.get(13)?,
        revision_note: row.get(14)?,
        selected: row.get::<_, i32>(15)? != 0,
        metadata: parse_json(row.get(16)?),
    })
}

pub fn upsert_direction_candidate(
    db: &Database,
    input: &DirectionCandidateInput,
) -> Result<String, String> {
    if input.inspiration.trim().is_empty() {
        return Err("Direction candidate inspiration is required".to_string());
    }
    if input.positioning.trim().is_empty() {
        return Err("Direction candidate positioning is required".to_string());
    }
    if input.target_reader.trim().is_empty() {
        return Err("Direction candidate target_reader is required".to_string());
    }
    if input.core_hook.trim().is_empty() {
        return Err("Direction candidate core_hook is required".to_string());
    }

    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let title_options = string_array_to_json(&input.title_options)?;
    let world_seed = to_json(&input.world_seed, "world_seed")?;
    let character_seed = to_json(&input.character_seed, "character_seed")?;
    let volume_strategy = to_json(&input.volume_strategy, "volume_strategy")?;
    let golden_three_chapters = to_json(&input.golden_three_chapters, "golden_three_chapters")?;
    let metadata = to_json(&input.metadata, "metadata")?;
    let checkpoint_status = if input.checkpoint_status.trim().is_empty() {
        "draft".to_string()
    } else {
        input.checkpoint_status.trim().to_string()
    };

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO direction_candidates
            (id, project_id, inspiration, title_options, positioning, target_reader,
             core_hook, series_promise, first_30_chapter_promise, world_seed,
             character_seed, volume_strategy, golden_three_chapters, checkpoint_status,
             revision_note, selected, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            inspiration = excluded.inspiration,
            title_options = excluded.title_options,
            positioning = excluded.positioning,
            target_reader = excluded.target_reader,
            core_hook = excluded.core_hook,
            series_promise = excluded.series_promise,
            first_30_chapter_promise = excluded.first_30_chapter_promise,
            world_seed = excluded.world_seed,
            character_seed = excluded.character_seed,
            volume_strategy = excluded.volume_strategy,
            golden_three_chapters = excluded.golden_three_chapters,
            checkpoint_status = excluded.checkpoint_status,
            revision_note = excluded.revision_note,
            selected = excluded.selected,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id,
            input.inspiration.trim(),
            title_options,
            input.positioning.trim(),
            input.target_reader.trim(),
            input.core_hook.trim(),
            input.series_promise.trim(),
            input.first_30_chapter_promise.trim(),
            world_seed,
            character_seed,
            volume_strategy,
            golden_three_chapters,
            checkpoint_status,
            input.revision_note,
            input.selected as i32,
            metadata
        ],
    )
    .map_err(|e| format!("Upsert direction candidate: {}", e))?;
    Ok(id)
}

pub fn list_direction_candidates(
    db: &Database,
    project_id: Option<&str>,
) -> Result<Vec<DirectionCandidate>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let sql = "SELECT id, project_id, inspiration, title_options, positioning, target_reader,
                      core_hook, series_promise, first_30_chapter_promise, world_seed,
                      character_seed, volume_strategy, golden_three_chapters, checkpoint_status,
                      revision_note, selected, metadata
               FROM direction_candidates
               WHERE (?1 IS NULL OR project_id = ?1)
               ORDER BY created_at ASC, id ASC";
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Prepare direction candidates: {}", e))?;
    let candidates = stmt
        .query_map(params![project_id], row_to_candidate)
        .map_err(|e| format!("Query direction candidates: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect direction candidates: {}", e))?;
    Ok(candidates)
}

pub fn get_direction_candidate(
    db: &Database,
    candidate_id: &str,
) -> Result<DirectionCandidate, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, inspiration, title_options, positioning, target_reader,
                    core_hook, series_promise, first_30_chapter_promise, world_seed,
                    character_seed, volume_strategy, golden_three_chapters, checkpoint_status,
                    revision_note, selected, metadata
             FROM direction_candidates
             WHERE id = ?1",
        )
        .map_err(|e| format!("Prepare direction candidate: {}", e))?;
    let candidate = stmt
        .query_row(params![candidate_id], row_to_candidate)
        .map_err(|e| format!("Load direction candidate: {}", e))?;
    Ok(candidate)
}

pub fn select_direction_candidate(
    db: &Database,
    candidate_id: &str,
    revision_note: Option<&str>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (project_id, inspiration): (Option<String>, String) = conn
        .query_row(
            "SELECT project_id, inspiration FROM direction_candidates WHERE id = ?1",
            params![candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Load direction candidate: {}", e))?;

    conn.execute(
        "UPDATE direction_candidates
         SET selected = 0, updated_at = datetime('now')
         WHERE ((project_id IS NULL AND ?1 IS NULL) OR project_id = ?1)
           AND inspiration = ?2",
        params![project_id, inspiration],
    )
    .map_err(|e| format!("Clear sibling direction selections: {}", e))?;

    conn.execute(
        "UPDATE direction_candidates
         SET selected = 1,
             revision_note = COALESCE(?2, revision_note),
             updated_at = datetime('now')
         WHERE id = ?1",
        params![candidate_id, revision_note],
    )
    .map_err(|e| format!("Select direction candidate: {}", e))?;
    Ok(())
}
