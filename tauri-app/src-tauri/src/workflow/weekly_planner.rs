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

fn story_phase(progress_percent: f64) -> &'static str {
    if progress_percent < 12.0 {
        "opening"
    } else if progress_percent < 35.0 {
        "early_development"
    } else if progress_percent < 65.0 {
        "middle_build"
    } else if progress_percent < 85.0 {
        "late_build"
    } else {
        "endgame"
    }
}

fn pacing_directive(phase: &str) -> String {
    if phase == "endgame" {
        "The story is in endgame; plans may resolve long-running threads only when supported by recent chapters and canon."
            .to_string()
    } else {
        format!(
            "Use longform pacing for phase {phase}: plan only the next local movement, escalate pressure gradually, preserve endgame mysteries, and must not resolve final villain/core mystery/final romance/final power endpoint."
        )
    }
}

pub fn build_weekly_planner_context(
    db: &Database,
    project_id: &str,
) -> Result<serde_json::Value, String> {
    let project = projects::get_project(db, project_id)?;
    let existing_chapters = chapters::get_chapters(db, project_id)?;
    let canon = bible::get_bible(db, project_id)?;
    let plans = chapters::get_chapter_plans(db, project_id)?;
    let planned_count = plans
        .iter()
        .filter(|p| p.status == "planned" || p.status == "in_progress")
        .count();
    let total_target_words = project.total_target_words.unwrap_or(500_000).max(1);
    let daily_target_words = project.daily_target_words.unwrap_or(3_000).max(1);
    let completed_words = existing_chapters
        .iter()
        .filter_map(|chapter| chapter.word_count)
        .map(|count| count.max(0) as i64)
        .sum::<i64>();
    let estimated_total_chapters =
        ((total_target_words as f64) / (daily_target_words as f64)).ceil() as i64;
    let progress_percent =
        (completed_words as f64 / total_target_words as f64 * 100.0).clamp(0.0, 100.0);
    let phase = story_phase(progress_percent);
    let next_sequence = existing_chapters
        .iter()
        .map(|chapter| chapter.sequence)
        .max()
        .unwrap_or(0)
        + 1;
    let next_plan_sequence = plans
        .iter()
        .map(|plan| plan.sequence)
        .max()
        .unwrap_or(next_sequence - 1)
        + 1;

    let mut recent_chapter_summaries = existing_chapters
        .iter()
        .rev()
        .take(6)
        .map(|chapter| {
            json!({
                "sequence": chapter.sequence,
                "title": chapter.title,
                "summary": chapter.summary,
                "word_count": chapter.word_count,
                "status": chapter.status,
            })
        })
        .collect::<Vec<_>>();
    recent_chapter_summaries.reverse();

    Ok(json!({
        "project_name": project.name,
        "genre": project.genre,
        "total_target_words": total_target_words,
        "daily_target_words": daily_target_words,
        "completed_words": completed_words,
        "estimated_total_chapters": estimated_total_chapters,
        "story_progress_percent": (progress_percent * 10.0).round() / 10.0,
        "story_phase": phase,
        "chapters_written": existing_chapters.len(),
        "next_sequence": next_sequence,
        "next_plan_sequence": next_plan_sequence,
        "existing_plans_count": planned_count,
        "recent_chapter_summaries": recent_chapter_summaries,
        "existing_plans": plans.iter().map(|p| json!({
            "sequence": p.sequence,
            "title": p.title,
            "outline": p.outline,
            "status": p.status,
            "plot_goals": p.plot_goals,
            "required_foreshadowing": p.required_foreshadowing,
        })).collect::<Vec<_>>(),
        "active_plot_threads": canon.plot_threads.iter().filter(|pt| pt.arc_status == "active" || pt.arc_status == "open").map(|pt| json!({
            "name": pt.name, "description": pt.description, "priority": pt.priority, "arc_status": pt.arc_status
        })).collect::<Vec<_>>(),
        "unresolved_foreshadowing": canon.foreshadowing.iter().filter(|f| f.status == "open").map(|f| json!({
            "clue": f.clue_text, "intended_payoff": f.intended_payoff, "importance": f.importance
        })).collect::<Vec<_>>(),
        "characters": canon.characters.iter().map(|c| json!({
            "id": c.id, "name": c.name, "role": c.role, "personality": c.personality
        })).collect::<Vec<_>>(),
        "pacing_directive": pacing_directive(phase),
    }))
}

pub async fn run_weekly_arc_planner(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    log_tx: &mpsc::Sender<String>,
) -> Result<WeeklyPlanResult, String> {
    log(log_tx, "=== Weekly Arc Planner ===");

    let plans = chapters::get_chapter_plans(db, project_id)?;
    let context = build_weekly_planner_context(db, project_id)?;

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
            let required_characters = ch
                .get("required_characters")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let required_locations = ch
                .get("required_locations")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let plot_goals = ch
                .get("plot_goals")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let required_foreshadowing = ch
                .get("required_foreshadowing")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let metadata = json!({
                "pov_character": ch.get("pov_character").and_then(|v| v.as_str()),
                "ending_hook": ch.get("ending_hook").and_then(|v| v.as_str()),
                "story_phase": context.get("story_phase").and_then(|v| v.as_str()),
                "story_progress_percent": context.get("story_progress_percent").and_then(|v| v.as_f64()),
                "planner_pacing": context.get("pacing_directive").and_then(|v| v.as_str()),
            })
            .to_string();

            let _ = conn.execute(
                "INSERT OR IGNORE INTO chapter_plans
                 (id, project_id, sequence, title, outline, target_word_count, required_characters, required_locations, plot_goals, required_foreshadowing, status, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'planned', ?11)",
                rusqlite::params![
                    id,
                    project_id,
                    seq,
                    title,
                    outline,
                    wc,
                    required_characters,
                    required_locations,
                    plot_goals,
                    required_foreshadowing,
                    metadata
                ],
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
                required_characters,
                required_locations,
                plot_goals,
                required_foreshadowing,
                status: "planned".into(),
                metadata,
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
