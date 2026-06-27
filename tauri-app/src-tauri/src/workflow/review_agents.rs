use crate::ai::client::ModelClient;
use crate::models::{AgentReview, Chapter, ChapterVersion, Project};
use crate::prompts;
use crate::workflow::canon_consistency::{self, CanonConsistencyIssue};
use std::collections::HashMap;

/// Full context for review agents — all canon data types included
#[derive(Debug, Clone)]
pub struct CanonContext {
    pub writing_brief_json: String,
    pub characters_json: String,
    pub character_states_json: String,
    pub previous_chapters_json: String,
    pub active_plot_threads_json: String,
    pub unresolved_foreshadowing_json: String,
    pub world_lore_json: String,
    pub locations_json: String,
    pub organizations_json: String,
    pub items_json: String,
    pub magic_systems_json: String,
    pub canon_rules_json: String,
    pub timeline_json: String,
    pub style_guide_json: String,
    pub extension_review_rubrics_json: String,
    pub blog_config_json: String,
    pub project_policy_json: String,
}

pub fn extension_review_rubrics_from_metadata(
    metadata: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut rubrics =
        crate::extensions::host::extension_contribution_payloads(metadata, "review_rubric_pack");
    rubrics.extend(crate::extensions::host::extension_contribution_payloads(
        metadata,
        "review_rubric",
    ));
    rubrics
}

pub async fn run_review_agents(
    provider: &dyn ModelClient,
    chapter: &Chapter,
    version: &ChapterVersion,
    canon: &CanonContext,
    _project: &Project,
) -> Result<Vec<AgentReview>, String> {
    let chapter_text = version.body_markdown.clone().unwrap_or_default();
    let chapter_title = version.title.clone().unwrap_or_default();

    // Helper: wrap review with 300s timeout so one stuck agent doesn't block others
    async fn timed_review(
        provider: &dyn ModelClient,
        name: &str,
        text: &str,
        title: &str,
        canon: &CanonContext,
    ) -> Result<AgentReview, String> {
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            run_single_review(provider, name, text, title, canon),
        )
        .await
        {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Err(e),
            Err(_elapsed) => Err(format!("{} timed out after 300s", name)),
        }
    }

    // Run all 7 reviews in parallel with timeout isolation
    let (a1, a2, a3, a4, a5, a6, a7) = tokio::join!(
        timed_review(
            provider,
            "continuity_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "character_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "plot_logic_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "pacing_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "style_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "safety_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
        timed_review(
            provider,
            "publication_reviewer",
            &chapter_text,
            &chapter_title,
            canon
        ),
    );

    let results = [a1, a2, a3, a4, a5, a6, a7];
    let agent_names = [
        "continuity_reviewer",
        "character_reviewer",
        "plot_logic_reviewer",
        "pacing_reviewer",
        "style_reviewer",
        "safety_reviewer",
        "publication_reviewer",
    ];
    let mut reviews = Vec::new();

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(review) => reviews.push(review),
            Err(e) => {
                reviews.push(AgentReview {
                    id: String::new(),
                    project_id: chapter.project_id.clone(),
                    chapter_id: chapter.id.clone(),
                    chapter_version_id: Some(version.id.clone()),
                    agent_name: agent_names[i].into(), // Preserve agent identity
                    score: Some(0),
                    pass: Some(false),
                    blocking_issues: format!("[{{\"issue\":\"Agent failed: {}\"}}]", e),
                    minor_issues: "[]".into(),
                    recommendations: "[]".into(),
                    raw_output: "{}".into(),
                    metadata: "{}".into(),
                    created_at: String::new(),
                    updated_at: String::new(),
                });
            }
        }
    }
    reviews.push(run_canon_consistency_precheck(
        &chapter.project_id,
        &chapter.id,
        chapter.sequence,
        version,
        &chapter_text,
        canon,
    ));
    Ok(reviews)
}

fn run_canon_consistency_precheck(
    project_id: &str,
    chapter_id: &str,
    chapter_sequence: i32,
    version: &ChapterVersion,
    chapter_text: &str,
    canon: &CanonContext,
) -> AgentReview {
    let issues = canon_consistency::detect_review_precheck_issues_from_json(
        chapter_text,
        &canon.writing_brief_json,
        &canon.characters_json,
        &canon.character_states_json,
        &canon.canon_rules_json,
        &canon.locations_json,
        &canon.timeline_json,
        &canon.style_guide_json,
        chapter_sequence,
    );
    let blocking = issues
        .iter()
        .filter(|issue| issue.severity == "blocking")
        .cloned()
        .collect::<Vec<_>>();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity != "blocking")
        .cloned()
        .collect::<Vec<_>>();
    let pass = blocking.is_empty();
    let score = if pass { 100 } else { 0 };

    AgentReview {
        id: String::new(),
        project_id: project_id.to_string(),
        chapter_id: chapter_id.to_string(),
        chapter_version_id: Some(version.id.clone()),
        agent_name: "canon_consistency_precheck".to_string(),
        score: Some(score),
        pass: Some(pass),
        blocking_issues: serialize_precheck_issues(&blocking),
        minor_issues: serialize_precheck_issues(&warnings),
        recommendations: if pass {
            "[]".to_string()
        } else {
            serde_json::json!(["Resolve deterministic canon violations before publication."])
                .to_string()
        },
        raw_output: serde_json::json!({ "issues": issues }).to_string(),
        metadata: serde_json::json!({ "source": "deterministic_canon_consistency" }).to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn serialize_precheck_issues(issues: &[CanonConsistencyIssue]) -> String {
    serde_json::Value::Array(
        issues
            .iter()
            .map(|issue| {
                serde_json::json!({
                    "issue": issue.message,
                    "rule_type": issue.rule_type,
                    "severity": issue.severity,
                    "evidence": issue.evidence,
                })
            })
            .collect(),
    )
    .to_string()
}

fn preview_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn extract_review_json(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Ok(value);
    }

    let cleaned = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if cleaned != trimmed {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(cleaned) {
            return Ok(value);
        }
    }

    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start <= end {
            let candidate = &trimmed[start..=end];
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) {
                return Ok(value);
            }
        }
    }

    Err(format!(
        "Could not parse reviewer JSON. Raw preview: {}",
        preview_chars(trimmed, 300)
    ))
}

fn extract_score(output: &serde_json::Value) -> Result<i32, String> {
    let score = output
        .get("score")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| "Reviewer JSON missing numeric score".to_string())?;
    if !score.is_finite() || !(0.0..=100.0).contains(&score) {
        return Err(format!("Reviewer score out of 0-100 range: {}", score));
    }
    Ok(score.round() as i32)
}

fn json_field_or_empty(output: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = output.get(*key) {
            if !value.is_null() {
                return value.to_string();
            }
        }
    }
    "[]".to_string()
}

fn review_metadata(agent_name: &str, output: &serde_json::Value) -> String {
    if agent_name != "publication_reviewer" {
        return "{}".to_string();
    }

    let blog_metadata = output
        .get("blog_metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    serde_json::json!({
        "blog_metadata": blog_metadata,
        "publication_interface": {
            "provider_kind": "local_draft",
            "target": "blog",
            "external_publish_ready": false
        }
    })
    .to_string()
}

async fn run_single_review(
    provider: &dyn ModelClient,
    agent_name: &str,
    chapter_text: &str,
    chapter_title: &str,
    canon: &CanonContext,
) -> Result<AgentReview, String> {
    // Load the review agent prompt and replace ALL placeholders
    let template = prompts::load_prompt(agent_name).unwrap_or_else(|_| {
        format!(
            "You are {}. Review the chapter and output valid JSON.",
            agent_name
        )
    });

    let mut vars: HashMap<String, String> = HashMap::new();
    // Core placeholders
    vars.insert("CHAPTER_JSON".into(), chapter_text.to_string());
    vars.insert("CHAPTER_TITLE".into(), chapter_title.to_string());

    // Canon data — each agent expects a specific set of these
    vars.insert("CANON_JSON".into(), canon.canon_rules_json.clone());
    vars.insert(
        "RECENT_SUMMARIES_JSON".into(),
        canon.previous_chapters_json.clone(),
    );
    vars.insert(
        "WRITING_BRIEF_JSON".into(),
        canon.writing_brief_json.clone(),
    );
    vars.insert(
        "PLOT_THREADS_JSON".into(),
        canon.active_plot_threads_json.clone(),
    );
    vars.insert(
        "FORESHADOWING_JSON".into(),
        canon.unresolved_foreshadowing_json.clone(),
    );
    vars.insert("CHARACTERS_JSON".into(), canon.characters_json.clone());
    vars.insert(
        "CHARACTER_STATES_JSON".into(),
        canon.character_states_json.clone(),
    );
    vars.insert("STYLE_GUIDE_JSON".into(), canon.style_guide_json.clone());
    vars.insert(
        "EXTENSION_REVIEW_RUBRICS_JSON".into(),
        canon.extension_review_rubrics_json.clone(),
    );
    vars.insert("BLOG_CONFIG_JSON".into(), canon.blog_config_json.clone());
    vars.insert(
        "PROJECT_POLICY_JSON".into(),
        canon.project_policy_json.clone(),
    );
    // Previously missing — locations, items, magic, timeline (critical for coherence)
    vars.insert("LOCATIONS_JSON".into(), canon.locations_json.clone());
    vars.insert(
        "ORGANIZATIONS_JSON".into(),
        canon.organizations_json.clone(),
    );
    vars.insert("ITEMS_JSON".into(), canon.items_json.clone());
    vars.insert(
        "MAGIC_SYSTEMS_JSON".into(),
        canon.magic_systems_json.clone(),
    );
    vars.insert("TIMELINE_JSON".into(), canon.timeline_json.clone());

    let render_vars: HashMap<&str, String> =
        vars.iter().map(|(k, v)| (k.as_ref(), v.clone())).collect();
    let system_prompt = crate::workflow::prompt_rendering::render_prompt_strict(
        agent_name,
        &template,
        &render_vars,
    )?;

    // User prompt is the chapter content
    let user_prompt = format!("请评审以下章节内容，严格按照上述 JSON schema 输出评审结果（只输出 JSON，不要其他文字）。\n\n章节内容：\n{}", chapter_text);

    // Use generate_text to avoid json_object mode issues
    let raw = provider
        .generate_text(&system_prompt, &user_prompt, 16384)
        .await?;

    // Extract JSON from the response
    let output = extract_review_json(&raw)?;

    let score = extract_score(&output)?;
    let pass = output["pass"].as_bool().unwrap_or(false);
    let blocking = json_field_or_empty(&output, &["blocking_issues"]);
    let minor = json_field_or_empty(&output, &["minor_issues"]);
    let recommendations =
        json_field_or_empty(&output, &["recommendations", "global_recommendations"]);

    if score < 20 {
        eprintln!(
            "[WARN] {} returned score={}, raw first 300 chars: {}",
            agent_name,
            score,
            preview_chars(&raw, 300)
        );
    }

    Ok(AgentReview {
        id: String::new(),
        project_id: String::new(),
        chapter_id: String::new(),
        chapter_version_id: None,
        agent_name: agent_name.to_string(),
        score: Some(score),
        pass: Some(pass),
        blocking_issues: blocking,
        minor_issues: minor,
        recommendations,
        raw_output: output.to_string(),
        metadata: review_metadata(agent_name, &output),
        created_at: String::new(),
        updated_at: String::new(),
    })
}
