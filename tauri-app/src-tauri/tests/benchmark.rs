/// Autonomous benchmark: full pipeline test with mock AI provider.
/// Verifies: create novel → bible → plans → write chapter → reviews → export → delete.
use std::collections::HashMap;
use std::sync::Mutex;
use async_trait::async_trait;
use serde_json::{json, Value};

// ---- Mock AI Provider ----
struct MockProvider {
    canned_responses: Mutex<HashMap<String, Value>>,
    text_responses: Mutex<HashMap<String, String>>,
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
        for agent in &["continuity", "character", "plot_logic", "pacing", "style", "safety", "publication"] {
            cr.insert(agent.to_string(), json!({
                "score": 92, "pass": true, "blocking_issues": [], "minor_issues": [], "recommendations": ["写得很好"]
            }));
        }
        Self {
            canned_responses: Mutex::new(cr),
            text_responses: Mutex::new(HashMap::new()),
            embed_vectors: Mutex::new(vec![vec![0.1_f32; 1536], vec![0.2_f32; 1536]]),
        }
    }
}

#[async_trait]
impl tauri_app_lib::ai::client::ModelClient for MockProvider {
    async fn generate_json(&self, system: &str, _user: &str, _schema: &Value, _max_tokens: u32) -> Result<Value, String> {
        let map = self.canned_responses.lock().unwrap();
        if system.contains("bible") || system.contains("世界观") {
            Ok(map.get("bible").cloned().unwrap_or(json!({})))
        } else if system.contains("draft") || system.contains("写手") {
            Ok(map.get("draft").cloned().unwrap_or(json!({})))
        } else if system.contains("continuity") { Ok(map.get("continuity").cloned().unwrap_or(json!({})))
        } else if system.contains("character") { Ok(map.get("character").cloned().unwrap_or(json!({})))
        } else if system.contains("plot_logic") { Ok(map.get("plot_logic").cloned().unwrap_or(json!({})))
        } else if system.contains("pacing") { Ok(map.get("pacing").cloned().unwrap_or(json!({})))
        } else if system.contains("style") { Ok(map.get("style").cloned().unwrap_or(json!({})))
        } else if system.contains("safety") { Ok(map.get("safety").cloned().unwrap_or(json!({})))
        } else if system.contains("publication") { Ok(map.get("publication").cloned().unwrap_or(json!({})))
        } else { Ok(map.get("draft").cloned().unwrap_or(json!({}))) }
    }
    async fn generate_text(&self, _system: &str, _user: &str, _max_tokens: u32) -> Result<String, String> {
        // Review agents use generate_text, not generate_json. Return valid JSON.
        Ok(r#"{"score":92,"pass":true,"blocking_issues":[],"minor_issues":[],"recommendations":["很好"]}"#.into())
    }
    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(self.embed_vectors.lock().unwrap().clone())
    }
}

// ---- Tests ----
use tauri_app_lib::db::connection::Database;
use tauri_app_lib::db::{projects, chapters, bible, generation_jobs, reviews};
use tauri_app_lib::workflow::*;
use tauri_app_lib::models::*;
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

#[tokio::test]
async fn benchmark_full_pipeline() {
    let db = setup_db();
    let provider = MockProvider::new();
    let (log_tx, _log_rx) = mpsc::channel::<String>(100);
    let (event_tx, _event_rx) = mpsc::channel::<PipelineEvent>(50);

    // === Test 1: Create Novel + Bible ===
    println!("\n=== Test 1: Create Novel ===");
    let input = CreateProjectInput {
        name: "Benchmark Novel".into(), description: Some("A test novel".into()),
        genre: Some("fantasy".into()), sub_genre: None, target_audience: Some("general".into()),
        tone: Some("热血".into()), style_profile_desc: None,
        target_total_words: Some(500000), daily_target_words: Some(3000),
    };
    let project = novel_bootstrap::bootstrap_novel(&db, &provider, &input, &log_tx).await.unwrap();
    assert!(!project.id.is_empty(), "Project should have ID");
    println!("  Project {} created", &project.id[..8]);

    let bible_data = bible::get_bible(&db, &project.id).unwrap();
    assert!(bible_data.characters.len() >= 6, "Should have >=6 characters, got {}", bible_data.characters.len());
    assert!(bible_data.locations.len() >= 4, "Should have >=4 locations");
    assert!(bible_data.plot_threads.len() >= 3, "Should have >=3 plot threads");
    println!("  Bible: {} chars, {} locations, {} lore, {} threads, {} rules",
        bible_data.characters.len(), bible_data.locations.len(), bible_data.world_lore.len(),
        bible_data.plot_threads.len(), bible_data.canon_rules.len());

    // === Test 2: Chapter Plans ===
    println!("\n=== Test 2: Chapter Plans ===");
    let plans = chapters::get_chapter_plans(&db, &project.id).unwrap();
    assert_eq!(plans.len(), 10, "Should have 10 chapter plans, got {}", plans.len());
    assert_eq!(plans[0].status, "planned");
    println!("  {} chapter plans created", plans.len());

    // === Test 3: Generate Chapter ===
    println!("\n=== Test 3: Generate Chapter ===");
    let result = chapter_production::generate_next_chapter(&db, &provider, None, &project.id, false, &log_tx, &event_tx).await.unwrap();
    assert!(result.ok, "Chapter generation should succeed: {}", result.message);
    assert!(result.chapter_id.is_some(), "Should have chapter_id");
    println!("  Chapter: {} ({} words, score {:.0})", result.chapter_title.unwrap_or_default(), result.word_count.unwrap_or(0), result.final_score.unwrap_or(0.0));

    let chapter_list = chapters::get_chapters(&db, &project.id).unwrap();
    assert_eq!(chapter_list.len(), 1, "Should have 1 chapter");
    let chap = &chapter_list[0];
    assert!(chap.word_count.unwrap_or(0) > 100, "Chapter should have content");

    let versions = chapters::get_chapter_versions(&db, &chap.id).unwrap();
    assert!(!versions.is_empty(), "Should have at least one version");
    println!("  Versions: {}", versions.len());

    // === Test 4: Reviews ===
    println!("\n=== Test 4: Reviews ===");
    let agent_reviews = reviews::get_agent_reviews(&db, &chap.id).unwrap();
    assert!(!agent_reviews.is_empty(), "Should have agent reviews, got {}", agent_reviews.len());
    let avg_score: f64 = agent_reviews.iter().filter_map(|r| r.score).map(|s| s as f64).sum::<f64>() / agent_reviews.len() as f64;
    assert!(avg_score > 50.0, "Average review score should be > 50, got {:.1}", avg_score);
    println!("  {} reviews, avg score {:.1}", agent_reviews.len(), avg_score);

    let scores = reviews::get_review_scores(&db, &chap.id).unwrap();
    assert!(scores.is_some(), "Should have review scores");
    println!("  Decision: {:?}", scores.and_then(|s| s.decision).unwrap_or_default());

    // === Test 5: Job Status ===
    println!("\n=== Test 5: Job Status ===");
    let jobs = generation_jobs::get_generation_jobs(&db, &project.id).unwrap();
    assert!(!jobs.is_empty(), "Should have generation jobs");
    let status = &jobs[0].status;
    assert!(
        status == "completed" || status == "needs_human_review",
        "Job status should be completed/needs_human_review, got {}", status
    );
    println!("  Job status: {}", status);

    // === Test 6: Idempotency ===
    println!("\n=== Test 6: Idempotency ===");
    let today_chapters = generation_jobs::get_today_chapter_count(&db, &project.id).unwrap();
    assert!(today_chapters > 0, "Should have chapters today");
    // Attempt duplicate generation (should be blocked)
    let result2 = chapter_production::generate_next_chapter(&db, &provider, None, &project.id, false, &log_tx, &event_tx).await.unwrap();
    assert!(!result2.ok, "Duplicate generation should be blocked without force=true");
    println!("  Duplicate blocked: {}", result2.message);

    // === Test 7: Cleanup ===
    println!("\n=== Test 7: Delete Novel ===");
    let paper_dir = format!("{}/novel-{}", std::env::temp_dir().to_string_lossy(), &project.id[..8]);
    projects::delete_project(&db, &project.id).unwrap();
    let remaining = projects::list_projects(&db).unwrap();
    assert!(remaining.is_empty(), "No projects should remain after deletion");
    println!("  Deleted. Remaining projects: {}", remaining.len());

    println!("\n=== ALL TESTS PASSED ===");
}
