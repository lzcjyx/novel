use rusqlite::params;
use crate::db::connection::Database;
use crate::models::{AgentReview, ReviewScores};

pub fn save_agent_review(
    db: &Database, project_id: &str, chapter_id: &str, chapter_version_id: &str,
    agent_name: &str, score: i32, pass: bool, blocking_issues: &str,
    minor_issues: &str, recommendations: &str, raw_output: &str,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO agent_reviews (id, project_id, chapter_id, chapter_version_id, agent_name, score, pass, blocking_issues, minor_issues, recommendations, raw_output)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, project_id, chapter_id, chapter_version_id, agent_name, score, pass as i32,
            blocking_issues, minor_issues, recommendations, raw_output],
    ).map_err(|e| format!("Insert review: {}", e))?;
    Ok(id)
}

pub fn save_review_scores(
    db: &Database, project_id: &str, chapter_id: &str, chapter_version_id: &str,
    average_score: f64, final_score: f64, decision: &str, publish_allowed: bool,
    blocking_issue_count: i32,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO review_scores (id, project_id, chapter_id, chapter_version_id, average_score, final_score, decision, publish_allowed, blocking_issue_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, project_id, chapter_id, chapter_version_id, average_score, final_score,
            decision, publish_allowed as i32, blocking_issue_count],
    ).map_err(|e| format!("Insert scores: {}", e))?;
    Ok(id)
}

pub fn get_agent_reviews(db: &Database, chapter_id: &str) -> Result<Vec<AgentReview>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, chapter_id, chapter_version_id, agent_name, score, pass,
                blocking_issues, minor_issues, recommendations, raw_output, metadata, created_at, updated_at
         FROM agent_reviews WHERE chapter_id = ?1 ORDER BY agent_name"
    ).map_err(|e| format!("Prepare: {}", e))?;

    let reviews = stmt.query_map(params![chapter_id], |row| {
        Ok(AgentReview {
            id: row.get(0)?, project_id: row.get(1)?, chapter_id: row.get(2)?,
            chapter_version_id: row.get(3)?, agent_name: row.get(4)?, score: row.get(5)?,
            pass: row.get::<_, Option<i32>>(6)?.map(|v| v != 0),
            blocking_issues: row.get(7)?, minor_issues: row.get(8)?,
            recommendations: row.get(9)?, raw_output: row.get(10)?,
            metadata: row.get(11)?, created_at: row.get(12)?, updated_at: row.get(13)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect: {}", e))?;

    Ok(reviews)
}

pub fn get_review_scores(db: &Database, chapter_id: &str) -> Result<Option<ReviewScores>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let result = conn.query_row(
        "SELECT id, project_id, chapter_id, chapter_version_id, average_score, final_score,
                decision, publish_allowed, blocking_issue_count, metadata, created_at, updated_at
         FROM review_scores WHERE chapter_id = ?1 ORDER BY created_at DESC LIMIT 1",
        params![chapter_id],
        |row| Ok(ReviewScores {
            id: row.get(0)?, project_id: row.get(1)?, chapter_id: row.get(2)?,
            chapter_version_id: row.get(3)?, average_score: row.get(4)?, final_score: row.get(5)?,
            decision: row.get(6)?, publish_allowed: row.get::<_, i32>(7)? != 0,
            blocking_issue_count: row.get(8)?, metadata: row.get(9)?,
            created_at: row.get(10)?, updated_at: row.get(11)?,
        }),
    );
    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Get scores: {}", e)),
    }
}
