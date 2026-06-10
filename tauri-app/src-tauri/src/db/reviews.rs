use crate::db::connection::Database;
use crate::models::{AgentQualityScore, AgentReview, ProjectQualitySummary, ReviewScores};
use rusqlite::params;
use std::collections::HashMap;

fn count_json_items(s: &str) -> i32 {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(items)) => items.len() as i32,
        _ => 0,
    }
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

pub fn save_agent_review(
    db: &Database,
    project_id: &str,
    chapter_id: &str,
    chapter_version_id: &str,
    agent_name: &str,
    score: i32,
    pass: bool,
    blocking_issues: &str,
    minor_issues: &str,
    recommendations: &str,
    raw_output: &str,
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
    db: &Database,
    project_id: &str,
    chapter_id: &str,
    chapter_version_id: &str,
    average_score: f64,
    final_score: f64,
    decision: &str,
    publish_allowed: bool,
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

    let reviews = stmt
        .query_map(params![chapter_id], |row| {
            Ok(AgentReview {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_id: row.get(2)?,
                chapter_version_id: row.get(3)?,
                agent_name: row.get(4)?,
                score: row.get(5)?,
                pass: row.get::<_, Option<i32>>(6)?.map(|v| v != 0),
                blocking_issues: row.get(7)?,
                minor_issues: row.get(8)?,
                recommendations: row.get(9)?,
                raw_output: row.get(10)?,
                metadata: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
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
        |row| {
            Ok(ReviewScores {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_id: row.get(2)?,
                chapter_version_id: row.get(3)?,
                average_score: row.get(4)?,
                final_score: row.get(5)?,
                decision: row.get(6)?,
                publish_allowed: row.get::<_, i32>(7)? != 0,
                blocking_issue_count: row.get(8)?,
                metadata: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    );
    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Get scores: {}", e)),
    }
}

pub fn get_project_quality_summary(
    db: &Database,
    project_id: &str,
) -> Result<ProjectQualitySummary, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;

    #[derive(Debug)]
    struct ScoreRow {
        average_score: Option<f64>,
        final_score: Option<f64>,
        decision: Option<String>,
        blocking_issue_count: i32,
    }

    let mut score_stmt = conn
        .prepare(
            "SELECT rs.average_score, rs.final_score, rs.decision, rs.blocking_issue_count
         FROM review_scores rs
         JOIN (
            SELECT chapter_id, MAX(rowid) AS latest_rowid
            FROM review_scores
            WHERE project_id = ?1
            GROUP BY chapter_id
         ) latest ON rs.rowid = latest.latest_rowid
         ORDER BY rs.created_at ASC, rs.rowid ASC",
        )
        .map_err(|e| format!("Prepare quality scores: {}", e))?;
    let score_rows = score_stmt
        .query_map(params![project_id], |row| {
            Ok(ScoreRow {
                average_score: row.get(0)?,
                final_score: row.get(1)?,
                decision: row.get(2)?,
                blocking_issue_count: row.get(3)?,
            })
        })
        .map_err(|e| format!("Query quality scores: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect quality scores: {}", e))?;

    let average_scores = score_rows
        .iter()
        .filter_map(|row| row.average_score)
        .collect::<Vec<_>>();
    let final_scores = score_rows
        .iter()
        .filter_map(|row| row.final_score)
        .collect::<Vec<_>>();
    let publish_ready_count = score_rows
        .iter()
        .filter(|row| row.decision.as_deref() == Some("publish_ready"))
        .count();
    let revise_count = score_rows
        .iter()
        .filter(|row| row.decision.as_deref() == Some("revise"))
        .count();
    let needs_human_review_count = score_rows
        .iter()
        .filter(|row| row.decision.as_deref() == Some("needs_human_review"))
        .count();
    let total_blocking_issues = score_rows
        .iter()
        .map(|row| row.blocking_issue_count)
        .sum::<i32>();
    let latest = score_rows.last();

    #[derive(Default)]
    struct AgentAccumulator {
        review_count: usize,
        scores: Vec<f64>,
        pass_count: usize,
        pass_observed_count: usize,
        blocking_issue_count: i32,
    }

    let mut agent_stmt = conn
        .prepare(
            "SELECT agent_name, score, pass, blocking_issues
         FROM agent_reviews
         WHERE project_id = ?1",
        )
        .map_err(|e| format!("Prepare agent quality: {}", e))?;
    let agent_rows = agent_stmt
        .query_map(params![project_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?
                    .unwrap_or_else(|| "unknown".into()),
                row.get::<_, Option<i32>>(1)?,
                row.get::<_, Option<i32>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Query agent quality: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect agent quality: {}", e))?;

    let mut by_agent: HashMap<String, AgentAccumulator> = HashMap::new();
    for (agent_name, score, pass, blocking_issues) in agent_rows {
        let entry = by_agent.entry(agent_name).or_default();
        entry.review_count += 1;
        if let Some(score) = score {
            entry.scores.push(score as f64);
        }
        if let Some(pass) = pass {
            entry.pass_observed_count += 1;
            if pass != 0 {
                entry.pass_count += 1;
            }
        }
        entry.blocking_issue_count += count_json_items(&blocking_issues);
    }

    let mut agent_scores = by_agent
        .into_iter()
        .map(|(agent_name, acc)| AgentQualityScore {
            agent_name,
            review_count: acc.review_count,
            average_score: average(&acc.scores),
            pass_rate: if acc.pass_observed_count == 0 {
                None
            } else {
                Some(acc.pass_count as f64 / acc.pass_observed_count as f64)
            },
            blocking_issue_count: acc.blocking_issue_count,
        })
        .collect::<Vec<_>>();
    agent_scores.sort_by(|a, b| a.agent_name.cmp(&b.agent_name));

    Ok(ProjectQualitySummary {
        project_id: project_id.into(),
        reviewed_chapter_count: score_rows.len(),
        publish_ready_count,
        revise_count,
        needs_human_review_count,
        average_score: average(&average_scores),
        average_final_score: average(&final_scores),
        total_blocking_issues,
        latest_decision: latest.and_then(|row| row.decision.clone()),
        latest_final_score: latest.and_then(|row| row.final_score),
        agent_scores,
    })
}
