use async_trait::async_trait;
use serde_json::{json, Value};
/// Autonomous benchmark: full pipeline test with mock AI provider.
/// Verifies: create novel → bible → plans → write chapter → reviews → export → delete.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

// ---- Mock AI Provider ----
struct MockProvider {
    canned_responses: Mutex<HashMap<String, Value>>,
    embed_vectors: Mutex<Vec<Vec<f32>>>,
}

impl MockProvider {
    fn new() -> Self {
        let mut cr = HashMap::new();
        // Bible generation response
        cr.insert("bible".into(), json!({
            "world_overview": "一个修仙世界，灵气复苏，宗门林立",
            "power_system": {"name":"修仙体系","description":"练气→筑基→金丹→元婴→化神","rules":"灵根决定上限","limitations":"突破需渡劫","progression":"九品灵根→一品天灵根"},
            "main_plot_threads": [
                {"name":"宗门复兴","description":"主角带领没落宗门重回巅峰","priority":1},
                {"name":"身世之谜","description":"主角身世隐藏惊天秘密","priority":2},
                {"name":"正邪之争","description":"正邪两道千年恩怨","priority":3}
            ],
            "characters": [
                {"name":"林风","role":"主角","personality":"坚韧果敢","motivation":"保护宗门","speech_style":"简洁有力","appearance":"剑眉星目","backstory":"孤儿，被宗门收养"},
                {"name":"云若曦","role":"女主角","personality":"清冷聪慧","motivation":"寻找真相","speech_style":"含蓄委婉","appearance":"倾国倾城","backstory":"神秘来历"},
                {"name":"赵无极","role":"反派","personality":"阴沉狠辣","motivation":"称霸天下","speech_style":"阴阳怪气","appearance":"鹰钩鼻","backstory":"魔教教主"},
                {"name":"张大山","role":"配角","personality":"豪爽耿直","motivation":"变强","speech_style":"粗犷直接","appearance":"虎背熊腰","backstory":"猎户出身"},
                {"name":"慕容雪","role":"配角","personality":"温柔善良","motivation":"守护家人","speech_style":"轻声细语","appearance":"白衣如雪","backstory":"慕容世家"},
                {"name":"黑风","role":"反派","personality":"残忍嗜血","motivation":"复仇","speech_style":"阴森","appearance":"黑袍遮面","backstory":"被主角父亲击败"}
            ],
            "locations": [
                {"name":"青云宗","type":"宗门","description":"坐落于青云山巅"},
                {"name":"魔渊","type":"禁地","description":"魔气弥漫的深渊"},
                {"name":"天剑城","type":"城市","description":"修仙者聚集的交易中心"},
                {"name":"龙脉秘境","type":"秘境","description":"隐藏龙脉传承的秘境"}
            ],
            "organizations": [
                {"name":"正道联盟","description":"正道各派联合组织","goals":"维护修仙界秩序"},
                {"name":"魔教","description":"邪道势力","goals":"颠覆正道统治"}
            ],
            "items": [
                {"name":"青云剑","description":"历代宗主传承之剑","abilities":"增幅剑气","limitations":"需金丹期以上"},
                {"name":"龙脉玉简","description":"记载龙脉秘密","abilities":"揭示秘境位置","limitations":"需特殊血脉"}
            ],
            "style_guide": {"narrative_perspective":"第三人称","tense":"过去时","tone":"热血","forbidden_phrases":["眼中闪过","嘴角上扬"],"preferred_techniques":["动作链","对话推进","感官细节"]},
            "canon_rules": [
                {"rule_type":"力量体系","rule_text":"灵根决定修炼上限","severity":"hard"},
                {"rule_type":"世界规则","rule_text":"灵气分布不均","severity":"hard"},
                {"rule_type":"时间规则","rule_text":"修炼无岁月","severity":"soft"},
                {"rule_type":"因果规则","rule_text":"杀孽过重会引天劫","severity":"hard"},
                {"rule_type":"空间规则","rule_text":"秘境独立于主世界","severity":"hard"}
            ],
            "chapter_plans": (0..10).map(|i| json!({
                "title": format!("第{}章", i+1),
                "outline": format!("第{}章的大纲描述", i+1),
                "target_word_count": 3500
            })).collect::<Vec<_>>()
        }));
        // Draft chapter response
        cr.insert("draft".into(), json!({
            "title": "青云初现",
            "body_markdown": "青云宗坐落在群山之巅，云雾缭绕之间，一座古朴的山门巍然矗立。林风站在山门前，望着那块刻着\'青云宗\'三个大字的石碑，心中涌起一股难以言喻的豪情。\n\n三年前，他还是一个流落街头的孤儿。如今，他已是青云宗的外门弟子。虽然只是最低级的弟子，但对他而言，这已是从地狱到天堂的跨越。\n\n林风握紧了拳头，掌心传来微微的刺痛——那是昨日练剑时留下的老茧。他知道，在这个弱肉强食的修仙世界，只有不断变强，才能守护自己所珍视的一切。",
            "summary": "林风回到青云宗，开始日常修炼，意外发现龙脉玉简的秘密",
            "word_count": 3200,
            "pov_character": "林风",
            "major_events": [{"event":"发现龙脉玉简","sequence":1}],
            "character_state_changes": [{"character":"林风","change":"获得龙脉玉简"}],
            "timeline_events": [{"label":"Day 1","event":"修炼开始"}],
            "foreshadowing_used": [],
            "foreshadowing_planted": [{"clue":"龙脉玉简发光","intended_payoff":"秘境即将开启"}],
            "new_canon_candidates": [],
            "continuity_notes": "初章，无连续性冲突",
            "used_context_ids": []
        }));
        // Review responses (score 92, no blocking)
        for agent in &[
            "continuity",
            "character",
            "plot_logic",
            "pacing",
            "style",
            "safety",
            "publication",
        ] {
            cr.insert(agent.to_string(), json!({
                "score": 92, "pass": true, "blocking_issues": [], "minor_issues": [], "recommendations": ["写得很好"]
            }));
        }
        Self {
            canned_responses: Mutex::new(cr),
            embed_vectors: Mutex::new(vec![vec![0.1_f32; 1536], vec![0.2_f32; 1536]]),
        }
    }
}

#[async_trait]
impl tauri_app_lib::ai::client::ModelClient for MockProvider {
    async fn generate_json(
        &self,
        system: &str,
        _user: &str,
        _schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        let map = self.canned_responses.lock().unwrap();
        if system.contains("canon_extractor") {
            Ok(json!({
                "chapter_summary": "章节 canon 已更新",
                "character_state_updates": [],
                "timeline_events": [],
                "new_lore": [],
                "foreshadowing_updates": [],
                "vector_documents": [],
                "human_review_required": []
            }))
        } else if system.contains("writing_context") || system.contains("顶尖中文网文职业写手")
        {
            Ok(map.get("draft").cloned().unwrap_or(json!({})))
        } else if system.contains("bible") || system.contains("世界观") {
            Ok(map.get("bible").cloned().unwrap_or(json!({})))
        } else if system.contains("draft") || system.contains("写手") {
            Ok(map.get("draft").cloned().unwrap_or(json!({})))
        } else if system.contains("continuity") {
            Ok(map.get("continuity").cloned().unwrap_or(json!({})))
        } else if system.contains("character") {
            Ok(map.get("character").cloned().unwrap_or(json!({})))
        } else if system.contains("plot_logic") {
            Ok(map.get("plot_logic").cloned().unwrap_or(json!({})))
        } else if system.contains("pacing") {
            Ok(map.get("pacing").cloned().unwrap_or(json!({})))
        } else if system.contains("style") {
            Ok(map.get("style").cloned().unwrap_or(json!({})))
        } else if system.contains("safety") {
            Ok(map.get("safety").cloned().unwrap_or(json!({})))
        } else if system.contains("publication") {
            Ok(map.get("publication").cloned().unwrap_or(json!({})))
        } else {
            Ok(map.get("draft").cloned().unwrap_or(json!({})))
        }
    }

    async fn generate_json_with_usage(
        &self,
        system: &str,
        user: &str,
        schema: &Value,
        max_tokens: u32,
    ) -> Result<(Value, Option<tauri_app_lib::ai::client::ModelUsageReport>), String> {
        let value = self.generate_json(system, user, schema, max_tokens).await?;
        let usage = if system.contains("writing_context")
            || system.contains("顶尖中文网文职业写手")
            || system.contains("draft")
            || system.contains("写手")
        {
            Some(tauri_app_lib::ai::client::ModelUsageReport {
                prompt_tokens: Some(2400),
                completion_tokens: Some(800),
                total_tokens: Some(3200),
            })
        } else {
            None
        };
        Ok((value, usage))
    }

    async fn generate_text(
        &self,
        _system: &str,
        _user: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        // Review agents use generate_text, not generate_json. Return valid JSON.
        Ok(r#"{"score":92,"pass":true,"blocking_issues":[],"minor_issues":[],"recommendations":["很好"]}"#.into())
    }
    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(self.embed_vectors.lock().unwrap().clone())
    }
}

// ---- Tests ----
use tauri_app_lib::db::connection::Database;
use tauri_app_lib::db::{bible, chapters, generation_jobs, projects, reviews};
use tauri_app_lib::models::*;
use tauri_app_lib::workflow::*;
use tokio::sync::mpsc;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("bench.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    // Don't drop dir — keep it alive for the test
    std::mem::forget(dir);
    db
}

#[derive(Default)]
struct BenchmarkReport {
    latency: Vec<(String, String)>,
    quality: Vec<(String, String)>,
    token: Vec<(String, String)>,
    cost: Vec<(String, String)>,
}

impl BenchmarkReport {
    fn add_latency(&mut self, label: impl Into<String>, elapsed_ms: u128) {
        self.latency.push((label.into(), format!("{elapsed_ms}ms")));
    }

    fn add_quality(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.quality.push((label.into(), value.into()));
    }

    fn add_token(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.token.push((label.into(), value.into()));
    }

    fn add_cost(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.cost.push((label.into(), value.into()));
    }

    fn render_section(output: &mut String, title: &str, rows: &[(String, String)]) {
        output.push_str(&format!("## {title}\n"));
        if rows.is_empty() {
            output.push_str("- n/a\n");
        } else {
            for (label, value) in rows {
                output.push_str(&format!("- {label}: {value}\n"));
            }
        }
        output.push('\n');
    }

    fn render(&self) -> String {
        let mut output = "=== Benchmark Report ===\n\n".to_string();
        Self::render_section(&mut output, "Latency", &self.latency);
        Self::render_section(&mut output, "Quality", &self.quality);
        Self::render_section(&mut output, "Token", &self.token);
        Self::render_section(&mut output, "Cost", &self.cost);
        output
    }
}

#[test]
fn benchmark_report_has_required_sections() {
    let mut report = BenchmarkReport::default();
    report.add_latency("bootstrap", 42);
    report.add_quality("average_score", "92.0");
    report.add_token("total_tokens", "12000");
    report.add_cost("estimated_cost_usd", "n/a");

    let rendered = report.render();
    let latency_pos = rendered.find("## Latency").unwrap();
    let quality_pos = rendered.find("## Quality").unwrap();
    let token_pos = rendered.find("## Token").unwrap();
    let cost_pos = rendered.find("## Cost").unwrap();

    assert!(latency_pos < quality_pos);
    assert!(quality_pos < token_pos);
    assert!(token_pos < cost_pos);
    assert!(rendered.contains("bootstrap: 42ms"));
    assert!(rendered.contains("average_score: 92.0"));
    assert!(rendered.contains("total_tokens: 12000"));
    assert!(rendered.contains("estimated_cost_usd: n/a"));
}

#[tokio::test]
async fn benchmark_full_pipeline() {
    let db = setup_db();
    let provider = MockProvider::new();
    let (log_tx, _log_rx) = mpsc::channel::<String>(100);
    let (event_tx, _event_rx) = mpsc::channel::<PipelineEvent>(50);
    let suite_started = Instant::now();
    let mut report = BenchmarkReport::default();

    // === Test 1: Create Novel + Bible ===
    println!("\n=== Test 1: Create Novel ===");
    let input = CreateProjectInput {
        name: "Benchmark Novel".into(),
        description: Some("A test novel".into()),
        genre: Some("fantasy".into()),
        sub_genre: None,
        target_audience: Some("general".into()),
        tone: Some("热血".into()),
        style_profile_desc: None,
        target_total_words: Some(500000),
        daily_target_words: Some(3000),
    };
    let started = Instant::now();
    let project = novel_bootstrap::bootstrap_novel(&db, &provider, &input, &log_tx)
        .await
        .unwrap();
    report.add_latency("bootstrap_novel", started.elapsed().as_millis());
    assert!(!project.id.is_empty(), "Project should have ID");
    println!("  Project {} created", &project.id[..8]);

    let bible_data = bible::get_bible(&db, &project.id).unwrap();
    assert!(
        bible_data.characters.len() >= 6,
        "Should have >=6 characters, got {}",
        bible_data.characters.len()
    );
    assert!(bible_data.locations.len() >= 4, "Should have >=4 locations");
    assert!(
        bible_data.plot_threads.len() >= 3,
        "Should have >=3 plot threads"
    );
    println!(
        "  Bible: {} chars, {} locations, {} lore, {} threads, {} rules",
        bible_data.characters.len(),
        bible_data.locations.len(),
        bible_data.world_lore.len(),
        bible_data.plot_threads.len(),
        bible_data.canon_rules.len()
    );

    // === Test 2: Chapter Plans ===
    println!("\n=== Test 2: Chapter Plans ===");
    let started = Instant::now();
    let plans = chapters::get_chapter_plans(&db, &project.id).unwrap();
    report.add_latency("load_chapter_plans", started.elapsed().as_millis());
    assert_eq!(
        plans.len(),
        10,
        "Should have 10 chapter plans, got {}",
        plans.len()
    );
    assert_eq!(plans[0].status, "planned");
    println!("  {} chapter plans created", plans.len());

    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    settings.input_cost_per_million = Some(2.0);
    settings.output_cost_per_million = Some(8.0);
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    // === Test 3: Generate Chapter ===
    println!("\n=== Test 3: Generate Chapter ===");
    let started = Instant::now();
    let result = chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project.id,
        false,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .unwrap();
    report.add_latency("generate_next_chapter", started.elapsed().as_millis());
    assert!(
        result.ok,
        "Chapter generation should succeed: {}",
        result.message
    );
    assert!(result.chapter_id.is_some(), "Should have chapter_id");
    println!(
        "  Chapter: {} ({} words, score {:.0})",
        result.chapter_title.unwrap_or_default(),
        result.word_count.unwrap_or(0),
        result.final_score.unwrap_or(0.0)
    );

    let chapter_list = chapters::get_chapters(&db, &project.id).unwrap();
    assert_eq!(chapter_list.len(), 1, "Should have 1 chapter");
    let chap = &chapter_list[0];
    assert!(
        chap.word_count.unwrap_or(0) > 100,
        "Chapter should have content"
    );

    let versions = chapters::get_chapter_versions(&db, &chap.id).unwrap();
    assert!(!versions.is_empty(), "Should have at least one version");
    println!("  Versions: {}", versions.len());

    // === Test 4: Reviews ===
    println!("\n=== Test 4: Reviews ===");
    let agent_reviews = reviews::get_agent_reviews(&db, &chap.id).unwrap();
    assert!(
        !agent_reviews.is_empty(),
        "Should have agent reviews, got {}",
        agent_reviews.len()
    );
    let avg_score: f64 = agent_reviews
        .iter()
        .filter_map(|r| r.score)
        .map(|s| s as f64)
        .sum::<f64>()
        / agent_reviews.len() as f64;
    assert!(
        avg_score > 50.0,
        "Average review score should be > 50, got {:.1}",
        avg_score
    );
    println!(
        "  {} reviews, avg score {:.1}",
        agent_reviews.len(),
        avg_score
    );
    report.add_quality("agent_review_count", agent_reviews.len().to_string());
    report.add_quality("average_score", format!("{avg_score:.1}"));

    let scores = reviews::get_review_scores(&db, &chap.id).unwrap();
    assert!(scores.is_some(), "Should have review scores");
    let decision = scores.and_then(|s| s.decision).unwrap_or_default();
    println!("  Decision: {:?}", decision);
    report.add_quality("decision", decision);

    // === Test 5: Job Status ===
    println!("\n=== Test 5: Job Status ===");
    let jobs = generation_jobs::get_generation_jobs(&db, &project.id).unwrap();
    assert!(!jobs.is_empty(), "Should have generation jobs");
    let status = &jobs[0].status;
    assert!(
        status == "completed" || status == "needs_human_review",
        "Job status should be completed/needs_human_review, got {}",
        status
    );
    println!("  Job status: {}", status);
    let metadata = serde_json::from_str::<Value>(&jobs[0].metadata).unwrap_or_else(|_| json!({}));
    let usage = metadata
        .get("usage_summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    report.add_token(
        "model_calls",
        usage
            .get("call_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
    );
    report.add_token(
        "provider_reported_model_calls",
        usage
            .get("provider_reported_call_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
    );
    report.add_token(
        "estimated_model_calls",
        usage
            .get("estimated_call_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
    );
    report.add_token(
        "prompt_tokens",
        usage
            .get("prompt_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .to_string(),
    );
    report.add_token(
        "completion_tokens",
        usage
            .get("completion_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .to_string(),
    );
    report.add_token(
        "total_tokens",
        usage
            .get("total_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .to_string(),
    );
    let estimated_cost = usage
        .get("estimated_cost_usd")
        .and_then(Value::as_f64)
        .map(|cost| format!("{cost:.4}"))
        .unwrap_or_else(|| "n/a".to_string());
    report.add_cost("estimated_cost_usd", estimated_cost);

    // === Test 6: Idempotency ===
    println!("\n=== Test 6: Idempotency ===");
    let today_chapters = generation_jobs::get_today_chapter_count(&db, &project.id).unwrap();
    assert!(today_chapters > 0, "Should have chapters today");
    // Attempt duplicate generation (should be blocked)
    let result2 = chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project.id,
        false,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .unwrap();
    assert!(
        !result2.ok,
        "Duplicate generation should be blocked without force=true"
    );
    println!("  Duplicate blocked: {}", result2.message);

    // === Test 7: Cleanup ===
    println!("\n=== Test 7: Delete Novel ===");
    projects::delete_project(&db, &project.id).unwrap();
    let remaining = projects::list_projects(&db).unwrap();
    assert!(
        remaining.is_empty(),
        "No projects should remain after deletion"
    );
    println!("  Deleted. Remaining projects: {}", remaining.len());

    report.add_latency("total_suite", suite_started.elapsed().as_millis());
    let rendered_report = report.render();
    assert!(rendered_report.contains("provider_reported_model_calls: 1"));
    assert!(!rendered_report.contains("estimated_cost_usd: n/a"));
    println!("\n{}", rendered_report);
    println!("\n=== ALL TESTS PASSED ===");
}

// ---- Web Learn HTML parsing tests ----
#[test]
fn test_html_parsing_extracts_content() {
    let html = r#"<!DOCTYPE html><html><head><script>var x=1;</script></head><body>
    <nav>Home About Contact</nav>
    <p>这是一段测试小说内容。秋风萧瑟，落叶纷飞，林风独自走在古道上。</p>
    <p>他心中思绪万千，回想起这些年的经历，不禁感慨万千。从默默无闻到名震天下，这条路走得太过艰难。</p>
    <div class="sidebar">Subscribe to newsletter</div>
    <footer>Copyright 2024 All rights reserved. This site uses cookies.</footer>
    </body></html>"#;

    let raw = html
        .replace("<br>", "\n")
        .replace("<p>", "\n")
        .replace("</p>", "\n");
    let raw = regex::Regex::new(r"<[^>]*>").unwrap().replace_all(&raw, "");
    let raw = raw.replace("&nbsp;", " ").replace("&amp;", "&");

    let lines: Vec<&str> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            l.len() > 40
                && !l.starts_with("function")
                && !l.starts_with("var ")
                && !l.to_lowercase().contains("cookie")
                && !l.to_lowercase().contains("subscribe")
        })
        .collect();
    let content = lines.join("\n");

    assert!(content.contains("秋风萧瑟"), "Should extract novel content");
    assert!(
        content.contains("名震天下"),
        "Should extract all paragraphs"
    );
    assert!(
        !content.contains("Contact"),
        "Should filter short nav lines"
    );
    assert!(!content.contains("cookie"), "Should filter cookie notices");
    println!(
        "  HTML parsing test passed — extracted {} chars",
        content.len()
    );
}

#[test]
fn test_html_parsing_handles_empty() {
    let html = "<html><head></head><body></body></html>";
    let raw = regex::Regex::new(r"<[^>]*>").unwrap().replace_all(html, "");
    let lines: Vec<&str> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.len() > 40)
        .collect();
    assert!(
        lines.is_empty(),
        "Empty page should produce no content lines"
    );
    println!("  Empty HTML test passed");
}

#[test]
fn test_html_tag_stripping() {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    let result = re.replace_all("<p>Hello</p>", "");
    assert!(result.contains("Hello"));
    println!("  HTML tag stripping works");
}
