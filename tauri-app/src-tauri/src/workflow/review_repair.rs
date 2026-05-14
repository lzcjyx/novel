use std::collections::HashMap;
use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{chapters, reviews};
use crate::models::*;
use crate::prompts;

pub async fn retry_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    chapter_id: &str,
) -> Result<RevisionResult, String> {
    let chapter = chapters::get_chapter(db, chapter_id)?;
    let version = chapters::get_latest_version(db, chapter_id)?
        .ok_or_else(|| "No version found".to_string())?;
    let agent_reviews = reviews::get_agent_reviews(db, chapter_id)?;
    let settings = crate::db::settings::get_settings(db)?;

    let chapter_text = version.body_markdown.unwrap_or_default();
    let revision_template = prompts::load_prompt("revision_writer")?;

    let mut vars = HashMap::new();
    vars.insert("CHAPTER_JSON".to_string(), chapter_text.clone());
    vars.insert("REVIEW_REPORTS_JSON".to_string(), serde_json::to_string(&agent_reviews).unwrap_or_default());

    let rendered = prompts::render_prompt(&revision_template, &vars.iter().map(|(k,v)| (k.as_str(), v.clone())).collect());
    let (sys, user) = if let Some(pos) = rendered.find("\n\n") {
        (rendered[..pos].to_string(), rendered[pos..].trim_start_matches('\n').to_string())
    } else { (rendered.clone(), rendered) };

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "body_markdown": {"type": "string"},
            "summary": {"type": "string"},
            "word_count": {"type": "integer"},
            "fixed_issues": {"type": "array"},
            "unfixed_issues": {"type": "array"},
            "enhancements_made": {"type": "array"},
            "change_log": {"type": "string"},
            "final_notes": {"type": "string"}
        }
    });

    let revised = provider.generate_json(&sys, &user, &schema, 32768).await?;

    let new_title = revised["title"].as_str().unwrap_or("Revised").to_string();
    let new_body = revised["body_markdown"].as_str().unwrap_or("").to_string();
    let new_wc = revised["word_count"].as_i64().unwrap_or(new_body.len() as i64) as i32;
    let new_version_number = version.version_number + 1;

    let new_version_id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count, model_provider, model_name, created_by_agent)
         VALUES (?1, ?2, ?3, ?4, 'revised', ?5, ?6, ?7, ?8, ?9, 'revision_writer')",
        rusqlite::params![new_version_id, chapter_id, chapter.project_id, new_version_number,
            new_title, new_body, new_wc, settings.provider, settings.model],
    ).map_err(|e| format!("Insert version: {}", e))?;
    drop(conn);

    chapters::update_chapter_after_revision(
        db, chapter_id, &chapter.project_id, &new_version_id,
        &new_title, &new_body, new_wc, "",
        "revised", 0.0, "revised",
    )?;

    Ok(RevisionResult {
        ok: true,
        message: format!("Chapter revised: v{}", new_version_number),
        chapter_id: Some(chapter_id.to_string()),
        version_number: Some(new_version_number),
        new_score: None,
        decision: Some("revised".into()),
    })
}
