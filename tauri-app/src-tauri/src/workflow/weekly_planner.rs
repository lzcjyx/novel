use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{bible, chapters, projects};
use crate::models::*;
use crate::prompts;
use serde_json::json;
use tokio::sync::mpsc;

fn log(log_tx: &mpsc::Sender<String>, msg: &str) {
    let _ = log_tx.try_send(format!(
        "[{}] {}",
        chrono::Local::now().format("%H:%M:%S"),
        msg
    ));
}

pub async fn run_weekly_arc_planner(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    log_tx: &mpsc::Sender<String>,
) -> Result<WeeklyPlanResult, String> {
    log(log_tx, "=== Weekly Arc Planner ===");

    let project = projects::get_project(db, project_id)?;
    let existing_chapters = chapters::get_chapters(db, project_id)?;
    let canon = bible::get_bible(db, project_id)?;
    let plans = chapters::get_chapter_plans(db, project_id)?;
    let planned_count = plans
        .iter()
        .filter(|p| p.status == "planned" || p.status == "in_progress")
        .count();

    let context = serde_json::json!({
        "project_name": project.name,
        "genre": project.genre,
        "total_target_words": project.total_target_words,
        "chapters_written": existing_chapters.len(),
        "existing_plans_count": planned_count,
        "existing_plans": plans.iter().map(|p| json!({
            "sequence": p.sequence, "title": p.title, "status": p.status
        })).collect::<Vec<_>>(),
        "active_plot_threads": canon.plot_threads.iter().filter(|pt| pt.arc_status == "active").map(|pt| json!({
            "name": pt.name, "description": pt.description, "priority": pt.priority
        })).collect::<Vec<_>>(),
        "unresolved_foreshadowing": canon.foreshadowing.iter().filter(|f| f.status == "open").map(|f| json!({
            "clue": f.clue_text, "intended_payoff": f.intended_payoff, "importance": f.importance
        })).collect::<Vec<_>>(),
        "characters": canon.characters.iter().map(|c| json!({
            "name": c.name, "role": c.role, "personality": c.personality
        })).collect::<Vec<_>>(),
    });

    let template = prompts::load_prompt("weekly_planner")?;
    let context_json = serde_json::to_string_pretty(&context).unwrap_or_default();
    let vars = std::collections::HashMap::from([("WEEKLY_PLANNER_CONTEXT_JSON", context_json)]);
    let system = crate::workflow::prompt_rendering::render_prompt_strict(
        "weekly_planner",
        &template,
        &vars,
    )?;
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "weekly_plan": {
                "type": "object",
                "properties": {
                    "chapters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "sequence": {"type": "integer"},
                                "title": {"type": "string"},
                                "outline": {"type": "string"},
                                "target_word_count": {"type": "integer"},
                                "pov_character": {"type": "string"},
                                "plot_goals": {"type": "array"},
                                "required_foreshadowing": {"type": "array"}
                            }
                        }
                    },
                    "plot_thread_updates": {"type": "array"},
                    "foreshadowing_updates": {"type": "array"},
                    "weekly_summary": {"type": "string"}
                }
            }
        }
    });

    log(log_tx, "Calling AI for arc plan...");
    let plan = provider
        .generate_json(
            &system,
            "请基于 system prompt 中的上下文生成下一组章节计划，只输出 JSON。",
            &schema,
            16384,
        )
        .await?;

    // Parse and insert chapter plans
    let chapters_arr = find_chapters(&plan);
    let mut created = 0;
    let mut new_plans = Vec::new();

    if let Some(arr) = chapters_arr {
        let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
        let max_seq = plans.iter().map(|p| p.sequence).max().unwrap_or(0);

        for (i, ch) in arr.iter().enumerate() {
            let id = Database::new_uuid();
            let seq = ch
                .get("sequence")
                .and_then(|v| v.as_i64())
                .unwrap_or((max_seq + i as i32 + 1) as i64) as i32;
            let title = ch
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let outline = ch
                .get("outline")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let wc = ch
                .get("target_word_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(3000) as i32;

            let _ = conn.execute(
                "INSERT OR IGNORE INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'planned')",
                rusqlite::params![id, project_id, seq, title, outline, wc],
            );
            new_plans.push(ChapterPlan {
                id,
                project_id: project_id.to_string(),
                volume_id: None,
                sequence: seq,
                title,
                outline,
                pov_character_id: None,
                target_word_count: Some(wc),
                required_characters: "[]".into(),
                required_locations: "[]".into(),
                plot_goals: ch
                    .get("plot_goals")
                    .map(|v| v.to_string())
                    .unwrap_or("[]".into()),
                required_foreshadowing: ch
                    .get("required_foreshadowing")
                    .map(|v| v.to_string())
                    .unwrap_or("[]".into()),
                status: "planned".into(),
                metadata: "{}".into(),
                created_at: String::new(),
                updated_at: String::new(),
            });
            created += 1;
        }
        drop(conn);
    }

    log(log_tx, &format!("Created {} new chapter plans", created));

    Ok(WeeklyPlanResult {
        ok: true,
        message: format!("Created {} chapter plans", created),
        plans_created: created,
        plans: new_plans,
    })
}

fn find_chapters(plan: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    // Try multiple possible paths
    plan["weekly_plan"]["chapters"]
        .as_array()
        .or_else(|| plan["chapters"].as_array())
        .or_else(|| plan["weekly_plan"]["chapter_plans"].as_array())
        .or_else(|| plan["new_chapter_plans"].as_array())
        .or_else(|| plan["chapter_plans"].as_array())
}
