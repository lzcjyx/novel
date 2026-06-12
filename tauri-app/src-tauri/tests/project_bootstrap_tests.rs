use async_trait::async_trait;
use serde_json::{json, Value};
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::db::{bible, chapters, knowledge_graph, projects, settings};
use tauri_app_lib::models::CreateProjectInput;
use tauri_app_lib::workflow::novel_bootstrap;
use tokio::sync::mpsc;

fn setup_db() -> tauri_app_lib::db::connection::Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("project_bootstrap.db");
    let db = tauri_app_lib::db::connection::Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    let mut app_settings = settings::get_settings(&db).unwrap();
    app_settings.data_dir = dir.path().join("novels").to_string_lossy().to_string();
    settings::save_settings(&db, &app_settings).unwrap();
    std::mem::forget(dir);
    db
}

fn test_input(name: &str) -> CreateProjectInput {
    CreateProjectInput {
        name: name.to_string(),
        description: Some("A bootstrap completeness regression novel".into()),
        genre: Some("fantasy".into()),
        sub_genre: Some("xianxia".into()),
        target_audience: Some("general".into()),
        tone: Some("serious".into()),
        style_profile_desc: Some("Fast-paced longform web novel prose".into()),
        target_total_words: Some(500000),
        daily_target_words: Some(3000),
    }
}

struct FailingBibleProvider;

#[async_trait]
impl ModelClient for FailingBibleProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Err("bible unavailable".into())
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(Vec::new())
    }
}

struct IncompleteBibleProvider;

#[async_trait]
impl ModelClient for IncompleteBibleProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({
            "world_overview": "一个过短且不完整的世界。",
            "power_system": {},
            "main_plot_threads": [],
            "characters": [
                {"name":"林初","role":"主角","personality":"坚韧","motivation":"活下去"}
            ],
            "locations": [],
            "organizations": [],
            "items": [],
            "style_guide": {"tone":"serious"},
            "canon_rules": [],
            "chapter_plans": []
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(Vec::new())
    }
}

struct CompleteBibleProvider;

#[async_trait]
impl ModelClient for CompleteBibleProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({
            "world_overview": "澜岳洲灵潮复苏，各城邦、宗门和古老商会围绕灵脉归属形成新的秩序。旧朝遗族暗中寻找被封存的天门钥印，边境妖潮则逼迫凡人与修士重新结盟。故事从没落宗门青岚院开始，少年沈砚被卷入灵脉异动，并逐步发现宗门衰败、旧朝秘约和妖潮源头互相牵连。",
            "power_system": {
                "name": "灵契九阶",
                "description": "修士以本命灵契沟通天地灵脉，借契印施展术法。",
                "rules": "契印越完整，能承载的灵压越高；越阶使用术法会损伤神魂。",
                "limitations": "每次突破都必须偿还契约代价，代价会影响寿命、记忆或情感。",
                "progression": "启契、凝纹、筑印、照脉、归元、登阙、问天、开门、合道"
            },
            "main_plot_threads": [
                {"name":"青岚复院","description":"沈砚寻找青岚院衰败真相并重建宗门根基。","priority":1},
                {"name":"旧朝钥印","description":"旧朝遗族争夺天门钥印，牵出百年前的背叛。","priority":2},
                {"name":"妖潮源流","description":"边境妖潮并非自然灾害，而是封印松动的征兆。","priority":3}
            ],
            "characters": [
                {"name":"沈砚","role":"主角","personality":"谨慎坚韧","motivation":"守住青岚院","speech_style":"克制直接","appearance":"青衣瘦削","backstory":"被青岚院收养的孤儿"},
                {"name":"陆青禾","role":"女主角","personality":"冷静敏锐","motivation":"追查家族失踪案","speech_style":"简短锋利","appearance":"白衣佩刀","backstory":"边城陆氏幸存者"},
                {"name":"闻照夜","role":"反派","personality":"温和残忍","motivation":"打开天门","speech_style":"含笑试探","appearance":"玄袍金瞳","backstory":"旧朝祭司后裔"},
                {"name":"裴万钧","role":"反派","personality":"霸道多疑","motivation":"夺取灵脉税权","speech_style":"压迫感强","appearance":"魁梧披甲","backstory":"镇岳军统领"},
                {"name":"苏照霜","role":"配角","personality":"聪明贪财","motivation":"摆脱商会控制","speech_style":"轻快圆滑","appearance":"红衣短发","backstory":"云舟商会账房"},
                {"name":"钟离岫","role":"配角","personality":"沉默守信","motivation":"偿还师门旧债","speech_style":"言简意赅","appearance":"灰衣剑修","backstory":"流亡剑宗弟子"}
            ],
            "locations": [
                {"name":"青岚院","type":"宗门","description":"位于断云岭的没落小宗门，地下残留旧灵脉。"},
                {"name":"望潮城","type":"边城","description":"抵御妖潮的一线城池，也是各方物资集散地。"},
                {"name":"黑砚谷","type":"禁地","description":"百年前旧朝祭坛所在，灵雾会吞噬记忆。"},
                {"name":"云舟商会","type":"势力驻地","description":"移动云舟组成的商业城寨，掌握灵材运输。"}
            ],
            "organizations": [
                {"name":"青岚院","description":"曾经守护断云岭的宗门，如今只剩空壳。","goals":"恢复护山大阵并查清衰败真相。"},
                {"name":"镇岳军","description":"望潮城军政势力，掌控边境军械和灵税。","goals":"垄断灵脉收益并压制宗门复兴。"}
            ],
            "items": [
                {"name":"裂纹契印","description":"沈砚体内残缺的本命契印。","abilities":"感应旧灵脉。","limitations":"过度使用会丢失记忆。"},
                {"name":"霜骨刀","description":"陆青禾家传短刀。","abilities":"斩断妖气。","limitations":"需要血脉温养。"}
            ],
            "style_guide": {
                "narrative_perspective":"第三人称",
                "tense":"过去时",
                "tone":"紧张克制",
                "forbidden_phrases":["嘴角上扬","眼中闪过"],
                "preferred_techniques":["动作推进","细节伏笔","短句压迫"]
            },
            "canon_rules": [
                {"rule_type":"力量体系","rule_text":"契印不能无代价越阶承载灵压。","severity":"hard"},
                {"rule_type":"地理规则","rule_text":"断云岭地下旧灵脉只在夜潮时显形。","severity":"hard"},
                {"rule_type":"政治规则","rule_text":"镇岳军对边境灵税有实际控制权。","severity":"hard"},
                {"rule_type":"记忆规则","rule_text":"黑砚谷灵雾会吞噬进入者最强烈的一段记忆。","severity":"hard"},
                {"rule_type":"妖潮规则","rule_text":"妖潮会优先攻击灵脉破损处。","severity":"hard"}
            ],
            "chapter_plans": [
                {"title":"断云夜潮","outline":"沈砚在青岚院后山发现夜潮异动，裂纹契印首次苏醒。镇岳军税吏趁乱逼债，青岚院面临撤院。","target_word_count":3500},
                {"title":"白衣入城","outline":"陆青禾追查失踪线索来到断云岭，与沈砚因同一枚旧朝残符交手。两人确认残符指向望潮城。","target_word_count":3500},
                {"title":"税令如山","outline":"裴万钧派人封锁青岚院灵田，逼沈砚交出后山地契。沈砚用契印感应到地契下藏着护山阵眼。","target_word_count":3500},
                {"title":"云舟账房","outline":"苏照霜带着商会账册登门讨债，却暗示镇岳军账目有假。沈砚答应护送她进望潮城换取线索。","target_word_count":3500},
                {"title":"妖火试阵","outline":"小股妖潮袭击山门，沈砚冒险启动残阵救下弟子。代价是他遗忘了师父临终前的一句话。","target_word_count":3500},
                {"title":"望潮暗市","outline":"众人进入望潮城暗市寻找残符来源，遇见流亡剑修钟离岫。闻照夜第一次派人试探沈砚的契印。","target_word_count":3500},
                {"title":"旧祭司名","outline":"陆青禾查到家族失踪案与旧朝祭司有关。沈砚在黑市拍品中看到青岚院失落阵图的一角。","target_word_count":3500},
                {"title":"军府夜宴","outline":"裴万钧设宴拉拢各宗，要求青岚院并入镇岳军供奉体系。沈砚拒绝后被迫接受三日后的阵斗。","target_word_count":3500},
                {"title":"黑砚来信","outline":"闻照夜送来一封只写给沈砚的信，指出裂纹契印属于旧朝天门钥印。陆青禾怀疑他与家族失踪有关。","target_word_count":3500},
                {"title":"阵斗开局","outline":"阵斗开始，沈砚以残阵抵抗镇岳军术师，却发现对方术法专克青岚传承。第一层危机展开但没有解决。","target_word_count":3500}
            ]
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1_f32; 1536]).collect())
    }
}

#[tokio::test]
async fn bootstrap_fails_and_cleans_project_when_bible_generation_fails() {
    let db = setup_db();
    let provider = FailingBibleProvider;
    let (log_tx, _log_rx) = mpsc::channel::<String>(100);

    let result =
        novel_bootstrap::bootstrap_novel(&db, &provider, &test_input("Failing Bootstrap"), &log_tx)
            .await;

    let err = result.expect_err("bootstrap should fail when bible generation fails");
    assert!(
        err.contains("Bible generation failed"),
        "unexpected error: {err}"
    );
    let projects = projects::list_projects(&db).unwrap();
    assert!(
        projects.is_empty(),
        "partial project should be cleaned up after failed bootstrap"
    );
}

#[tokio::test]
async fn bootstrap_rejects_incomplete_bible_and_cleans_project() {
    let db = setup_db();
    let provider = IncompleteBibleProvider;
    let (log_tx, _log_rx) = mpsc::channel::<String>(100);

    let result = novel_bootstrap::bootstrap_novel(
        &db,
        &provider,
        &test_input("Incomplete Bootstrap"),
        &log_tx,
    )
    .await;

    let err = result.expect_err("bootstrap should fail when bible is incomplete");
    assert!(
        err.contains("Bootstrap validation failed"),
        "unexpected error: {err}"
    );
    let projects = projects::list_projects(&db).unwrap();
    assert!(
        projects.is_empty(),
        "partial project should be cleaned up after incomplete bootstrap"
    );
}

#[tokio::test]
async fn bootstrap_success_persists_required_bible_plans_and_graph_nodes() {
    let db = setup_db();
    let provider = CompleteBibleProvider;
    let (log_tx, _log_rx) = mpsc::channel::<String>(100);

    let project = novel_bootstrap::bootstrap_novel(
        &db,
        &provider,
        &test_input("Complete Bootstrap"),
        &log_tx,
    )
    .await
    .expect("complete bootstrap should succeed");

    let bible_data = bible::get_bible(&db, &project.id).unwrap();
    assert_eq!(bible_data.characters.len(), 6);
    assert_eq!(bible_data.locations.len(), 4);
    assert_eq!(bible_data.organizations.len(), 2);
    assert!(!bible_data.world_lore.is_empty());
    assert_eq!(bible_data.magic_systems.len(), 1);
    assert_eq!(bible_data.canon_rules.len(), 5);
    assert_eq!(bible_data.plot_threads.len(), 3);

    let plans = chapters::get_chapter_plans(&db, &project.id).unwrap();
    assert_eq!(plans.len(), 10);
    assert!(plans.iter().all(|plan| plan.status == "planned"));
    assert!(plans
        .iter()
        .all(|plan| plan.target_word_count == Some(3500)));

    let graph = knowledge_graph::get_snapshot(&db, &project.id).unwrap();
    assert!(
        !graph.nodes.is_empty(),
        "graph snapshot should expose bible-derived nodes"
    );
}
