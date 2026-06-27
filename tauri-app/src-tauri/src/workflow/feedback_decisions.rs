use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRevisionCandidateInput {
    pub id: Option<String>,
    pub project_id: String,
    pub feedback_id: String,
    pub chapter_id: String,
    pub title: String,
    pub body_markdown: String,
    pub summary: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRevisionDecision {
    pub id: String,
    pub project_id: String,
    pub feedback_id: String,
    pub chapter_id: String,
    pub title: String,
    pub body_markdown: String,
    pub summary: Option<String>,
    pub status: String,
    pub decision_note: Option<String>,
    pub resulting_chapter_version_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FeedbackDecisionAction {
    Approve,
    Reject,
    Defer,
}

pub fn create_feedback_revision_candidate(
    db: &Database,
    input: &FeedbackRevisionCandidateInput,
) -> Result<String, String> {
    if input.project_id.trim().is_empty()
        || input.feedback_id.trim().is_empty()
        || input.chapter_id.trim().is_empty()
    {
        return Err("project_id, feedback_id, and chapter_id are required".to_string());
    }
    if input.title.trim().is_empty() || input.body_markdown.trim().is_empty() {
        return Err("feedback revision title and body are required".to_string());
    }
    let id = input.id.clone().unwrap_or_else(Database::new_uuid);
    let metadata = serde_json::to_string(&input.metadata)
        .map_err(|e| format!("Serialize feedback decision metadata: {}", e))?;
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO feedback_revision_decisions
            (id, project_id, feedback_id, chapter_id, title, body_markdown, summary, status, metadata, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            feedback_id = excluded.feedback_id,
            chapter_id = excluded.chapter_id,
            title = excluded.title,
            body_markdown = excluded.body_markdown,
            summary = excluded.summary,
            status = 'pending',
            decision_note = NULL,
            resulting_chapter_version_id = NULL,
            metadata = excluded.metadata,
            updated_at = datetime('now')",
        params![
            id,
            input.project_id.trim(),
            input.feedback_id.trim(),
            input.chapter_id.trim(),
            input.title.trim(),
            input.body_markdown,
            input.summary,
            metadata,
        ],
    )
    .map_err(|e| format!("Create feedback revision candidate: {}", e))?;
    Ok(id)
}

pub fn list_feedback_decisions(
    db: &Database,
    project_id: &str,
) -> Result<Vec<FeedbackRevisionDecision>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, feedback_id, chapter_id, title, body_markdown, summary,
                    status, decision_note, resulting_chapter_version_id, metadata
             FROM feedback_revision_decisions
             WHERE project_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| format!("Prepare feedback decisions: {}", e))?;
    let decisions = stmt
        .query_map(params![project_id], row_to_decision)
        .map_err(|e| format!("Query feedback decisions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect feedback decisions: {}", e))?;
    Ok(decisions)
}

pub fn decide_feedback_revision(
    db: &Database,
    decision_id: &str,
    action: FeedbackDecisionAction,
    decision_note: Option<&str>,
) -> Result<FeedbackRevisionDecision, String> {
    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin feedback decision: {}", e))?;
    let decision = load_decision_tx(&tx, decision_id)?;
    if decision.status != "pending" {
        return Err(format!(
            "feedback decision '{}' is already {}",
            decision.id, decision.status
        ));
    }

    let (status, resulting_version_id) = match action {
        FeedbackDecisionAction::Approve => {
            let version_id = Database::new_uuid();
            let version_number: i32 = tx
                .query_row(
                    "SELECT COALESCE(MAX(version_number), 0) + 1 FROM chapter_versions WHERE chapter_id = ?1",
                    params![decision.chapter_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Next feedback revision version: {}", e))?;
            let word_count = decision.body_markdown.chars().count() as i32;
            let metadata = json!({
                "feedback_decision": {
                    "decision_id": decision.id,
                    "feedback_id": decision.feedback_id,
                    "decision_note": decision_note,
                }
            })
            .to_string();
            tx.execute(
                "INSERT INTO chapter_versions
                    (id, chapter_id, project_id, version_number, version_type, title,
                     body_markdown, summary, word_count, created_by_agent, metadata)
                 VALUES (?1, ?2, ?3, ?4, 'accepted_candidate', ?5, ?6, ?7, ?8, 'feedback_decision', ?9)",
                params![
                    version_id,
                    decision.chapter_id,
                    decision.project_id,
                    version_number,
                    decision.title,
                    decision.body_markdown,
                    decision.summary,
                    word_count,
                    metadata,
                ],
            )
            .map_err(|e| format!("Create approved feedback chapter version: {}", e))?;
            tx.execute(
                "UPDATE chapters
                 SET final_version_id = ?1, title = ?2, status = 'revised',
                     word_count = ?3, summary = COALESCE(?4, summary), updated_at = datetime('now')
                 WHERE id = ?5",
                params![
                    version_id,
                    decision.title,
                    word_count,
                    decision.summary,
                    decision.chapter_id,
                ],
            )
            .map_err(|e| format!("Promote approved feedback revision: {}", e))?;
            ("approved", Some(version_id))
        }
        FeedbackDecisionAction::Reject => ("rejected", None),
        FeedbackDecisionAction::Defer => ("deferred", None),
    };

    tx.execute(
        "UPDATE feedback_revision_decisions
         SET status = ?1, decision_note = ?2, resulting_chapter_version_id = ?3, updated_at = datetime('now')
         WHERE id = ?4",
        params![status, decision_note, resulting_version_id, decision_id],
    )
    .map_err(|e| format!("Update feedback decision: {}", e))?;
    let updated = load_decision_tx(&tx, decision_id)?;
    tx.commit()
        .map_err(|e| format!("Commit feedback decision: {}", e))?;
    Ok(updated)
}

fn load_decision_tx(
    tx: &rusqlite::Transaction<'_>,
    decision_id: &str,
) -> Result<FeedbackRevisionDecision, String> {
    tx.query_row(
        "SELECT id, project_id, feedback_id, chapter_id, title, body_markdown, summary,
                status, decision_note, resulting_chapter_version_id, metadata
         FROM feedback_revision_decisions
         WHERE id = ?1",
        params![decision_id],
        row_to_decision,
    )
    .map_err(|e| format!("Load feedback decision '{}': {}", decision_id, e))
}

fn row_to_decision(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeedbackRevisionDecision> {
    let metadata_raw: String = row.get(10)?;
    Ok(FeedbackRevisionDecision {
        id: row.get(0)?,
        project_id: row.get(1)?,
        feedback_id: row.get(2)?,
        chapter_id: row.get(3)?,
        title: row.get(4)?,
        body_markdown: row.get(5)?,
        summary: row.get(6)?,
        status: row.get(7)?,
        decision_note: row.get(8)?,
        resulting_chapter_version_id: row.get(9)?,
        metadata: serde_json::from_str(&metadata_raw).unwrap_or_else(|_| json!({})),
    })
}
