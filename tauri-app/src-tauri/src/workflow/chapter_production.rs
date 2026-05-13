use std::collections::HashMap;
use sha2::{Sha256, Digest};
use tokio::sync::mpsc;
use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::db::{projects, chapters, reviews, generation_jobs, bible, blog_posts};
use crate::models::*;
use crate::prompts;
use crate::workflow::{lock, review_agents, review_arbiter, canon_updater};
use crate::export::markdown;

fn log(log_tx: &mpsc::Sender<String>, msg: &str) {
    let _ = log_tx.try_send(format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg));
}

fn emit(event_tx: &mpsc::Sender<PipelineEvent>, step: &str, status: &str, detail: Option<&str>, progress_pct: f64) {
    let _ = event_tx.try_send(PipelineEvent {
        step: step.into(), status: status.into(),
        elapsed_ms: None, detail: detail.map(|s| s.into()), progress_pct, timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
    });
}

pub async fn generate_next_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    emb_provider: Option<&dyn ModelClient>,
    project_id: &str,
    force: bool,
    log_tx: &mpsc::Sender<String>,
    event_tx: &mpsc::Sender<PipelineEvent>,
) -> Result<GenerationResult, String> {
    // 1. Check if already has a chapter today (unless force)
    let today_count = generation_jobs::get_today_chapter_count(db, project_id)?;
    if today_count > 0 && !force {
        return Ok(GenerationResult {
            ok: false, message: "Already generated a chapter today. Use force=true to override.".into(),
            chapter_id: None, chapter_title: None, sequence: None, word_count: None,
            final_score: None, decision: None, filename: None,
        });
    }

    // 2. Check if a job is already running
    if generation_jobs::is_job_running(db, project_id)? {
        return Ok(GenerationResult {
            ok: false, message: "A generation job is already running for this project.".into(),
            chapter_id: None, chapter_title: None, sequence: None, word_count: None,
            final_score: None, decision: None, filename: None,
        });
    }

    // 3. Acquire advisory lock (RAII guard auto-releases on drop)
    let _lock_guard = match lock::GenerationLock::acquire(db, project_id, lock::LockType::ChapterGeneration) {
        Ok(g) => g,
        Err(_) => return Ok(GenerationResult {
            ok: false, message: "Could not acquire generation lock. Another job may be running.".into(),
            chapter_id: None, chapter_title: None, sequence: None, word_count: None,
            final_score: None, decision: None, filename: None,
        }),
    };
    emit(event_tx, "acquire_lock", "done", None, 3.0);

    log(log_tx, "=== Starting Daily Chapter Production ===");

    // 4. Load project config
    let project = projects::get_project(db, project_id)?;
    let settings = crate::db::settings::get_settings(db)?;
    log(log_tx, &format!("Project: {} | Quality threshold: {}", project.name, project.quality_threshold));

    // 5. Select next chapter plan
    let plan = match chapters::get_next_chapter_plan(db, project_id)? {
        Some(p) => p,
        None => {
            log(log_tx, "No chapter plan found. Run Weekly Arc Planner first.");
            return Ok(GenerationResult {
                ok: false,
                message: "No chapter plans available. Please run 'Generate Weekly Plan' first.".into(),
                chapter_id: None, chapter_title: None, sequence: None, word_count: None,
                final_score: None, decision: None, filename: None,
            });
        }
    };
    log(log_tx, &format!("Chapter plan: Ch.{} — {}", plan.sequence, plan.title.as_deref().unwrap_or("Untitled")));

    // 6. Create generation job (idempotent)
    let job_id = generation_jobs::create_generation_job(db, project_id, &plan.id)?;
    log(log_tx, &format!("Job created: {}", &job_id[..8]));

    // 7. Load structured canon
    let canon_data = bible::get_bible(db, project_id)?;
    log(log_tx, &format!("Loaded canon: {} chars, {} lore, {} threads, {} foreshadowing",
        canon_data.characters.len(), canon_data.world_lore.len(),
        canon_data.plot_threads.len(), canon_data.foreshadowing.len()));
    emit(event_tx, "load_canon", "done", Some(&format!("{} chars, {} lore", canon_data.characters.len(), canon_data.world_lore.len())), 10.0);

    // 8. Build retrieval query + retrieve vector context
    let retrieval_query = format!("{} {} {} {}",
        plan.title.as_deref().unwrap_or(""),
        plan.outline.as_deref().unwrap_or(""),
        plan.required_characters,
        plan.required_locations
    );
    log(log_tx, "Retrieving vector context...");

    let embed_client = emb_provider.unwrap_or(provider);
    let vector_context = if !retrieval_query.trim().is_empty() {
        match embed_client.embed(&[retrieval_query]).await {
            Ok(embeddings) if !embeddings.is_empty() => {
                match crate::db::vector_store::search_similar_documents(db, project_id, &embeddings[0], 12) {
                    Ok(docs) => {
                        log(log_tx, &format!("Found {} relevant documents", docs.len()));
                        docs.iter().map(|d| format!("[{}] {}", d.source_type, d.content))
                            .collect::<Vec<_>>().join("\n\n")
                    }
                    Err(e) => { log(log_tx, &format!("Vector search fallback: {}", e)); String::new() }
                }
            }
            _ => {
                log(log_tx, "Embedding failed or empty, continuing without vector context");
                String::new()
            }
        }
    } else {
        String::new()
    };

    // 9. Build writing brief
    let prev_chapters = chapters::get_chapters(db, project_id)?;
    let prev_context = prev_chapters.iter()
        .map(|c| format!("Ch.{}: {}", c.sequence, c.summary.as_deref().unwrap_or("")))
        .collect::<Vec<_>>().join("\n");

    let writing_brief = serde_json::json!({
        "project_name": project.name,
        "genre": project.genre,
        "chapter_sequence": plan.sequence,
        "chapter_title": plan.title,
        "outline": plan.outline,
        "target_word_count": plan.target_word_count.unwrap_or(settings.daily_target_words),
        "pov_character": plan.pov_character_id,
        "required_characters": plan.required_characters,
        "required_locations": plan.required_locations,
        "plot_goals": plan.plot_goals,
        "required_foreshadowing": plan.required_foreshadowing,
        "characters": serde_json::to_string(&canon_data.characters).unwrap_or_default(),
        "previous_chapters": prev_context,
        "active_plot_threads": serde_json::to_string(&canon_data.plot_threads).unwrap_or_default(),
        "unresolved_foreshadowing": serde_json::to_string(&canon_data.foreshadowing).unwrap_or_default(),
        "world_lore": serde_json::to_string(&canon_data.world_lore).unwrap_or_default(),
        "timeline": serde_json::to_string(&canon_data.timeline_events).unwrap_or_default(),
        "style_guide": serde_json::to_string(&canon_data.style_guides).unwrap_or_default(),
        "retrieval_context": vector_context,
        "quality_threshold": project.quality_threshold,
        "auto_publish": project.auto_publish,
    });

    let prompt_hash = {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(&writing_brief).unwrap_or_default());
        hex::encode(hasher.finalize())
    };

    // 10. Render draft writer prompt
    let draft_template = prompts::load_prompt("draft_writer")?;
    let mut vars = HashMap::new();
    vars.insert("WRITING_BRIEF_JSON".to_string(), serde_json::to_string_pretty(&writing_brief).unwrap_or_default());
    vars.insert("CHARACTERS_JSON".to_string(), serde_json::to_string(&canon_data.characters).unwrap_or_default());
    let rendered = prompts::render_prompt(&draft_template, &vars.iter().map(|(k,v)| (k.as_str(), v.clone())).collect());

    // Split into system + user (first section before blank line = system, rest = user)
    let (system_prompt, user_prompt) = if let Some(pos) = rendered.find("\n\n") {
        let (sys, user) = rendered.split_at(pos);
        (sys.to_string(), user.trim_start_matches('\n').to_string())
    } else {
        (rendered.clone(), rendered)
    };

    log(log_tx, "Calling draft writer...");

    // 11. Call draft writer
    let json_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "body_markdown": {"type": "string"},
            "summary": {"type": "string"},
            "word_count": {"type": "integer"},
            "pov_character": {"type": "string"},
            "major_events": {"type": "array"},
            "character_state_changes": {"type": "array"},
            "timeline_events": {"type": "array"},
            "foreshadowing_used": {"type": "array"},
            "foreshadowing_planted": {"type": "array"},
            "new_canon_candidates": {"type": "array"},
            "continuity_notes": {"type": "string"},
            "used_context_ids": {"type": "array"}
        }
    });

    let draft = match provider.generate_json(&system_prompt, &user_prompt, &json_schema, 32768).await {
        Ok(d) => {
            let wc = d["word_count"].as_i64().unwrap_or(0);
            emit(event_tx, "generate_draft", "done", Some(&format!("{} words", wc)), 35.0);
            log(log_tx, "Draft generated successfully");
            d
        }
        Err(e) => {
            generation_jobs::update_job_status(db, &job_id, "failed", Some(&e))?;
            log(log_tx, &format!("Draft generation failed: {}", e));
            return Err(format!("Draft generation failed: {}", e));
        }
    };

    // 12. Validate & save draft version
    let title = draft["title"].as_str().unwrap_or("Untitled").to_string();
    let body = draft["body_markdown"].as_str().unwrap_or("").to_string();
    let summary = draft["summary"].as_str().unwrap_or("").to_string();

    // Reject empty body — AI returned invalid content
    if body.len() < 50 {
        let err_msg = format!(
            "AI returned insufficient content. body_markdown length: {} chars. Raw keys: {:?}",
            body.len(),
            draft.as_object().map(|o| o.keys().collect::<Vec<_>>()).unwrap_or_default()
        );
        log(log_tx, &err_msg);
        generation_jobs::update_job_status(db, &job_id, "failed", Some(&err_msg))?;
        return Err(err_msg);
    }

    let word_count = draft["word_count"].as_i64().unwrap_or(body.len() as i64) as i32;

    let chapter_title = title.to_string();
    let (chapter_id, version_id) = chapters::save_draft_version(
        db, project_id, &plan.id, plan.sequence,
        &title, &body, word_count, &summary,
        &settings.provider, &settings.model, &prompt_hash, "",
    )?;

    log(log_tx, &format!("Draft saved: {} ({} words)", &chapter_id[..8], word_count));

    generation_jobs::update_job_status(db, &job_id, "reviewing", None)?;

    // 13. Run review agents
    let chapter = chapters::get_chapter(db, &chapter_id)?;
    let version = chapters::get_latest_version(db, &chapter_id)?.ok_or("Version not found")?;

    // Build previous chapter body context (last 2 chapters, truncated to 3000 chars each)
    let prev_bodies: Vec<String> = prev_chapters.iter().rev().take(2).filter_map(|c| {
        chapters::get_latest_version(db, &c.id).ok().flatten()
            .and_then(|v| v.body_markdown)
            .map(|body| {
                let truncated: String = body.chars().take(3000).collect();
                format!("Ch.{} — {}:\n{}", c.sequence, c.title.as_deref().unwrap_or(""), truncated)
            })
    }).collect();
    let prev_body_context = prev_bodies.join("\n\n---\n\n");

    // Load character states for continuity checking
    let character_states = crate::db::bible::get_character_states(db, project_id).unwrap_or_default();

    let canon_ctx = review_agents::CanonContext {
        writing_brief_json: serde_json::to_string(&writing_brief).unwrap_or_default(),
        characters_json: serde_json::to_string(&canon_data.characters).unwrap_or_default(),
        character_states_json: serde_json::to_string(&character_states).unwrap_or_default(),
        previous_chapters_json: if !prev_body_context.is_empty() { prev_body_context } else { prev_context },
        active_plot_threads_json: serde_json::to_string(&canon_data.plot_threads).unwrap_or_default(),
        unresolved_foreshadowing_json: serde_json::to_string(&canon_data.foreshadowing).unwrap_or_default(),
        world_lore_json: serde_json::to_string(&canon_data.world_lore).unwrap_or_default(),
        locations_json: serde_json::to_string(&canon_data.locations).unwrap_or_default(),
        organizations_json: serde_json::to_string(&canon_data.organizations).unwrap_or_default(),
        items_json: serde_json::to_string(&canon_data.items).unwrap_or_default(),
        magic_systems_json: serde_json::to_string(&canon_data.magic_systems).unwrap_or_default(),
        canon_rules_json: serde_json::to_string(&canon_data.canon_rules).unwrap_or_default(),
        timeline_json: serde_json::to_string(&canon_data.timeline_events).unwrap_or_default(),
        style_guide_json: serde_json::to_string(&canon_data.style_guides).unwrap_or_default(),
        blog_config_json: serde_json::json!({
            "provider": project.blog_provider.as_deref().unwrap_or("local"),
            "status": "draft",
            "auto_publish": project.auto_publish,
        }).to_string(),
        project_policy_json: serde_json::json!({
            "auto_publish": project.auto_publish,
            "quality_threshold": project.quality_threshold,
        }).to_string(),
    };

    log(log_tx, "Running 7 parallel review agents...");
    let agent_reviews = review_agents::run_review_agents(provider, &chapter, &version, &canon_ctx, &project).await?;

    // Log each review score
    for review in &agent_reviews {
        log(log_tx, &format!("  {} score={} pass={}", review.agent_name, review.score.unwrap_or(0), review.pass.unwrap_or(false)));
    }

    // Save review records with retry
    for review in &agent_reviews {
        let mut saved = false;
        for attempt in 0..2 {
            match reviews::save_agent_review(
                db, project_id, &chapter_id, &version_id,
                &review.agent_name, review.score.unwrap_or(0), review.pass.unwrap_or(false),
                &review.blocking_issues, &review.minor_issues, &review.recommendations, &review.raw_output,
            ) {
                Ok(_) => { saved = true; break; }
                Err(e) if attempt == 0 => { log(log_tx, &format!("Save review {} failed, retrying: {}", review.agent_name, e)); }
                Err(e) => { log(log_tx, &format!("Save review {} failed after retry: {}", review.agent_name, e)); }
            }
        }
        if !saved {
            log(log_tx, &format!("CRITICAL: Could not save review for {}", review.agent_name));
        }
    }

    // 14. Aggregate reviews
    let aggregation = review_arbiter::aggregate_reviews(
        &agent_reviews, project.quality_threshold, settings.max_revise_count, 0,
    );
    log(log_tx, &format!("Reviews: avg={:.1} final={:.1} decision={} blocking={}",
        aggregation.average_score, aggregation.final_score, aggregation.decision, aggregation.blocking_issue_count));
    emit(event_tx, "aggregate_reviews", "done", Some(&format!("avg={:.1} decision={}", aggregation.average_score, aggregation.decision)), 65.0);

    let _ = reviews::save_review_scores(
        db, project_id, &chapter_id, &version_id,
        aggregation.average_score, aggregation.final_score, &aggregation.decision,
        aggregation.publish_allowed, aggregation.blocking_issue_count,
    );

    // 15. Revise loop: keep revising until score >= threshold or retries exhausted
    let mut revise_count = 0;
    let mut current_draft = draft.clone();
    let mut current_version_id = version_id.clone();
    let mut current_aggregation = aggregation;
    let mut current_reviews = agent_reviews.clone();

    let (final_decision, final_score, _final_version_id) = loop {
        if current_aggregation.decision == "needs_human_review" || current_aggregation.decision == "publish_ready" {
            break (current_aggregation.decision.clone(), current_aggregation.final_score, current_version_id.clone());
        }
        if current_aggregation.decision != "revise" {
            break (current_aggregation.decision.clone(), current_aggregation.final_score, current_version_id.clone());
        }
        if revise_count >= settings.max_revise_count {
            log(log_tx, &format!("Max revisions ({}) reached. Marking for human review.", settings.max_revise_count));
            generation_jobs::update_job_status(db, &job_id, "needs_human_review", None)?;
            break ("needs_human_review".into(), current_aggregation.final_score, current_version_id.clone());
        }

        revise_count += 1;
        log(log_tx, &format!("Revision {}/{} needed (score {:.0} < threshold {}). Calling revision writer...",
            revise_count, settings.max_revise_count, current_aggregation.final_score, project.quality_threshold));

        let revision_template = prompts::load_prompt("revision_writer")?;
        let rev_input = serde_json::json!({
            "chapter": current_draft,
            "reviews": current_reviews.iter().map(|r| serde_json::json!({
                "agent": r.agent_name, "score": r.score, "pass": r.pass,
                "blocking_issues": r.blocking_issues, "minor_issues": r.minor_issues,
                "recommendations": r.recommendations,
            })).collect::<Vec<_>>(),
            "average_score": current_aggregation.average_score,
            "blocking_issue_count": current_aggregation.blocking_issue_count,
        });
        let mut rev_vars = HashMap::new();
        rev_vars.insert("REVISION_INPUT_JSON".to_string(), serde_json::to_string_pretty(&rev_input).unwrap_or_default());
        let rev_rendered = prompts::render_prompt(&revision_template, &rev_vars.iter().map(|(k,v)| (k.as_str(), v.clone())).collect());
        let (rev_sys, rev_user) = if let Some(pos) = rev_rendered.find("\n\n") {
            let (s, u) = rev_rendered.split_at(pos);
            (s.to_string(), u.trim_start_matches('\n').to_string())
        } else { (rev_rendered.clone(), rev_rendered) };

        match provider.generate_json(&rev_sys, &rev_user, &json_schema, 32768).await {
            Ok(revised) => {
                let rev_body = revised["body_markdown"].as_str().unwrap_or("").to_string();
                let rev_wc = revised["word_count"].as_i64().unwrap_or(rev_body.len() as i64) as i32;
                if rev_body.len() < 100 {
                    log(log_tx, "Revision too short, keeping previous version");
                    break (current_aggregation.decision.clone(), current_aggregation.final_score, current_version_id.clone());
                }
                let rev_version_id = Database::new_uuid();
                {
                    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
                    conn.execute(
                        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count, model_provider, model_name, created_by_agent)
                         VALUES (?1, ?2, ?3, ?4, 'revised', ?5, ?6, ?7, ?8, ?9, 'revision_writer')",
                        rusqlite::params![rev_version_id, chapter_id, project_id, 1 + revise_count, title, rev_body, rev_wc, settings.provider, settings.model],
                    ).map_err(|e| format!("Insert revision: {}", e))?;
                }
                let _ = chapters::update_chapter_after_revision(
                    db, &chapter_id, project_id, &rev_version_id, &title, &rev_body, rev_wc, &summary,
                    "revised", current_aggregation.final_score, &current_aggregation.decision,
                );
                log(log_tx, &format!("Revision {} saved ({} words)", revise_count, rev_wc));
                current_draft = revised;
                current_version_id = rev_version_id;

                // Re-run reviews after revision
                log(log_tx, &format!("Re-running reviews after revision {}...", revise_count));
                emit(event_tx, "revise", "done", Some(&format!("revision {} complete", revise_count)), 70.0 + (revise_count as f64 * 10.0));
                let chapter = chapters::get_chapter(db, &chapter_id)?;
                let version = chapters::get_latest_version(db, &chapter_id)?.ok_or("Version not found")?;
                current_reviews = review_agents::run_review_agents(provider, &chapter, &version, &canon_ctx, &project).await?;
                for review in &current_reviews {
                    log(log_tx, &format!("  {} score={} pass={}", review.agent_name, review.score.unwrap_or(0), review.pass.unwrap_or(false)));
                    let _ = reviews::save_agent_review(db, project_id, &chapter_id, &current_version_id,
                        &review.agent_name, review.score.unwrap_or(0), review.pass.unwrap_or(false),
                        &review.blocking_issues, &review.minor_issues, &review.recommendations, &review.raw_output,
                    );
                }
                current_aggregation = review_arbiter::aggregate_reviews(&current_reviews, project.quality_threshold, settings.max_revise_count, revise_count);
                log(log_tx, &format!("Post-revision reviews: avg={:.1} decision={}", current_aggregation.average_score, current_aggregation.decision));
            }
            Err(e) => {
                log(log_tx, &format!("Revision {} failed: {}", revise_count, e));
                break (current_aggregation.decision.clone(), current_aggregation.final_score, current_version_id.clone());
            }
        }
    };

    // Update chapter status based on final decision
    if final_decision == "needs_human_review" {
        let _ = chapters::update_chapter_after_revision(
            db, &chapter_id, project_id, &current_version_id, &title,
            current_draft["body_markdown"].as_str().unwrap_or(&body), word_count, &summary,
            "needs_human_review", final_score, &final_decision,
        );
        generation_jobs::update_job_status(db, &job_id, "needs_human_review", None)?;
    } else {
        let _ = chapters::update_chapter_after_revision(
            db, &chapter_id, project_id, &current_version_id, &title,
            current_draft["body_markdown"].as_str().unwrap_or(&body), word_count, &summary,
            if final_decision == "publish_ready" { "final" } else { "revised" }, final_score, &final_decision,
        );
    }
    let body = current_draft["body_markdown"].as_str().unwrap_or("").to_string();
    let word_count = current_draft["word_count"].as_i64().unwrap_or(body.len() as i64) as i32;

    // 16. Generate blog metadata if auto_publish
    let filename = if project.auto_publish && final_decision == "publish_ready" {
        log(log_tx, "Generating blog metadata...");
        // Write chapter to .md file
        let result = markdown::export_chapter_markdown(db, &chapter_id, &settings.data_dir);
        match result {
            Ok(path) => {
                log(log_tx, &format!("Chapter exported: {}", path));
                let _ = blog_posts::create_blog_post(db, project_id, &chapter_id, &project.blog_provider.unwrap_or_else(|| "local".into()), &title, "", None);
                Some(path)
            }
            Err(e) => {
                log(log_tx, &format!("Export failed: {}", e));
                None
            }
        }
    } else {
        // Just write to file
        let result = markdown::export_chapter_markdown(db, &chapter_id, &settings.data_dir);
        match result {
            Ok(path) => {
                log(log_tx, &format!("Chapter exported: {}", path));
                Some(path)
            }
            Err(e) => {
                log(log_tx, &format!("Export failed: {}", e));
                None
            }
        }
    };

    // 17. Update canon
    log(log_tx, "Updating canon...");
    canon_updater::update_canon_after_chapter(db, provider, project_id, &chapter_id, &draft).await?;

    // 18. Update job status
    generation_jobs::update_job_status(db, &job_id, "completed", None)?;
    emit(event_tx, "complete", "done", Some(&format!("{} words, score {:.0}", word_count, final_score)), 100.0);
    log(log_tx, "=== Chapter production complete ===");

    Ok(GenerationResult {
        ok: true,
        message: format!("Chapter {} generated: {} (score: {:.0})", plan.sequence, title, final_score),
        chapter_id: Some(chapter_id),
        chapter_title: Some(chapter_title),
        sequence: Some(plan.sequence),
        word_count: Some(word_count),
        final_score: Some(final_score),
        decision: Some(final_decision),
        filename,
    })
}
