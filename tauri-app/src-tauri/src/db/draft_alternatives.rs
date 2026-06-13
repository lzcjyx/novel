use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftCandidateInput {
    pub project_id: String,
    pub chapter_plan_id: String,
    pub candidate_number: i32,
    pub title: String,
    pub body_markdown: String,
    pub summary: Option<String>,
    pub word_count: i32,
    pub prompt_hash: String,
    pub context_hash: String,
    pub model_profile_id: Option<String>,
    pub review_notes: serde_json::Value,
    pub estimated_cost_usd: Option<f64>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftCandidate {
    pub id: String,
    pub project_id: String,
    pub chapter_plan_id: String,
    pub candidate_number: i32,
    pub title: String,
    pub body_markdown: String,
    pub summary: Option<String>,
    pub word_count: i32,
    pub prompt_hash: String,
    pub context_hash: String,
    pub model_profile_id: Option<String>,
    pub review_notes: serde_json::Value,
    pub estimated_cost_usd: Option<f64>,
    pub status: String,
    pub selection_reason: Option<String>,
    pub metadata: serde_json::Value,
}

fn metadata_to_string(metadata: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(metadata).map_err(|e| format!("Serialize draft metadata: {}", e))
}

fn parse_json(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
}

pub fn create_draft_candidate(
    db: &Database,
    input: &DraftCandidateInput,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let review_notes = metadata_to_string(&input.review_notes)?;
    let metadata = metadata_to_string(&input.metadata)?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO draft_alternatives
            (id, project_id, chapter_plan_id, candidate_number, title, body_markdown, summary,
             word_count, prompt_hash, context_hash, model_profile_id, review_notes,
             estimated_cost_usd, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'candidate', ?14)",
        params![
            id,
            input.project_id,
            input.chapter_plan_id,
            input.candidate_number,
            input.title,
            input.body_markdown,
            input.summary,
            input.word_count,
            input.prompt_hash,
            input.context_hash,
            input.model_profile_id,
            review_notes,
            input.estimated_cost_usd,
            metadata
        ],
    )
    .map_err(|e| format!("Create draft candidate: {}", e))?;
    Ok(id)
}

pub fn list_draft_candidates(
    db: &Database,
    chapter_plan_id: &str,
) -> Result<Vec<DraftCandidate>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_plan_id, candidate_number, title, body_markdown,
                    summary, word_count, prompt_hash, context_hash, model_profile_id,
                    review_notes, estimated_cost_usd, status, selection_reason, metadata
             FROM draft_alternatives
             WHERE chapter_plan_id = ?1
             ORDER BY candidate_number ASC",
        )
        .map_err(|e| format!("Prepare draft candidates: {}", e))?;
    let candidates = stmt
        .query_map(params![chapter_plan_id], |row| {
            Ok(DraftCandidate {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_plan_id: row.get(2)?,
                candidate_number: row.get(3)?,
                title: row.get(4)?,
                body_markdown: row.get(5)?,
                summary: row.get(6)?,
                word_count: row.get(7)?,
                prompt_hash: row.get(8)?,
                context_hash: row.get(9)?,
                model_profile_id: row.get(10)?,
                review_notes: parse_json(row.get(11)?),
                estimated_cost_usd: row.get(12)?,
                status: row.get(13)?,
                selection_reason: row.get(14)?,
                metadata: parse_json(row.get(15)?),
            })
        })
        .map_err(|e| format!("Query draft candidates: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect draft candidates: {}", e))?;
    Ok(candidates)
}

pub fn select_draft_candidate(
    db: &Database,
    candidate_id: &str,
    selection_reason: &str,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin draft selection: {}", e))?;
    let (
        project_id,
        chapter_plan_id,
        title,
        body_markdown,
        summary,
        word_count,
        prompt_hash,
        context_hash,
        model_profile_id,
        review_notes_raw,
        estimated_cost_usd,
        candidate_metadata_raw,
    ): (
        String,
        String,
        String,
        String,
        Option<String>,
        i32,
        String,
        String,
        Option<String>,
        String,
        Option<f64>,
        String,
    ) = tx
        .query_row(
            "SELECT project_id, chapter_plan_id, title, body_markdown, summary, word_count,
                    prompt_hash, context_hash, model_profile_id, review_notes,
                    estimated_cost_usd, metadata
             FROM draft_alternatives WHERE id = ?1",
            params![candidate_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .map_err(|e| format!("Load selected draft candidate: {}", e))?;
    let sequence: i32 = tx
        .query_row(
            "SELECT sequence FROM chapter_plans WHERE id = ?1 AND project_id = ?2",
            params![chapter_plan_id, project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load draft candidate chapter plan: {}", e))?;
    tx.execute(
        "UPDATE draft_alternatives
         SET status = 'rejected', selection_reason = NULL, updated_at = datetime('now')
         WHERE project_id = ?1 AND chapter_plan_id = ?2 AND id <> ?3",
        params![project_id, chapter_plan_id, candidate_id],
    )
    .map_err(|e| format!("Reject other draft candidates: {}", e))?;
    tx.execute(
        "UPDATE draft_alternatives
         SET status = 'selected', selection_reason = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![selection_reason, candidate_id],
    )
    .map_err(|e| format!("Select draft candidate: {}", e))?;

    let chapter_id = match tx.query_row(
        "SELECT id FROM chapters WHERE project_id = ?1 AND chapter_plan_id = ?2 ORDER BY created_at DESC LIMIT 1",
        params![project_id, chapter_plan_id],
        |row| row.get::<_, String>(0),
    ) {
        Ok(existing_id) => existing_id,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let new_chapter_id = Database::new_uuid();
            tx.execute(
                "INSERT INTO chapters
                    (id, project_id, chapter_plan_id, sequence, title, final_version_id, status,
                     word_count, summary)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'draft', ?6, ?7)",
                params![
                    new_chapter_id,
                    project_id,
                    chapter_plan_id,
                    sequence,
                    title,
                    word_count,
                    summary,
                ],
            )
            .map_err(|e| format!("Create accepted draft chapter: {}", e))?;
            new_chapter_id
        }
        Err(e) => return Err(format!("Load existing accepted draft chapter: {}", e)),
    };
    let next_version_number: i32 = tx
        .query_row(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM chapter_versions WHERE chapter_id = ?1",
            params![chapter_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load next accepted draft version number: {}", e))?;
    let version_id = Database::new_uuid();
    let review_notes = serde_json::from_str::<serde_json::Value>(&review_notes_raw)
        .unwrap_or_else(|_| serde_json::json!({}));
    let candidate_metadata = serde_json::from_str::<serde_json::Value>(&candidate_metadata_raw)
        .unwrap_or_else(|_| serde_json::json!({}));
    let version_metadata = serde_json::json!({
        "draft_candidate_id": candidate_id,
        "selection_reason": selection_reason,
        "model_profile_id": model_profile_id,
        "review_notes": review_notes,
        "estimated_cost_usd": estimated_cost_usd,
        "candidate_metadata": candidate_metadata,
    })
    .to_string();
    tx.execute(
        "INSERT INTO chapter_versions
            (id, chapter_id, project_id, version_number, version_type, title, body_markdown,
             summary, word_count, prompt_hash, context_hash, created_by_agent, metadata)
         VALUES (?1, ?2, ?3, ?4, 'accepted_candidate', ?5, ?6, ?7, ?8, ?9, ?10,
                 'draft_candidate_selector', ?11)",
        params![
            version_id,
            chapter_id,
            project_id,
            next_version_number,
            title,
            body_markdown,
            summary,
            word_count,
            prompt_hash,
            context_hash,
            version_metadata,
        ],
    )
    .map_err(|e| format!("Create accepted draft version: {}", e))?;
    tx.execute(
        "UPDATE chapters
         SET title = ?1, final_version_id = ?2, status = 'draft', word_count = ?3,
             summary = ?4, updated_at = datetime('now')
         WHERE id = ?5",
        params![title, version_id, word_count, summary, chapter_id],
    )
    .map_err(|e| format!("Update accepted draft chapter: {}", e))?;
    tx.execute(
        "UPDATE chapter_plans SET status = 'in_progress', updated_at = datetime('now') WHERE id = ?1",
        params![chapter_plan_id],
    )
    .map_err(|e| format!("Update selected draft chapter plan: {}", e))?;
    tx.commit()
        .map_err(|e| format!("Commit draft selection: {}", e))?;
    Ok(())
}
