use tauri_app_lib::db::connection::Database;
use tauri_app_lib::{ai::client::ModelClient, workflow::director_mode};

fn setup_db(name: &str) -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(name);
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

struct FakeDirectorClient;

#[async_trait::async_trait]
impl ModelClient for FakeDirectorClient {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &serde_json::Value,
        _max_tokens: u32,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "candidates": [
                {
                    "title_options": ["雨站账本", "灵税旧票"],
                    "positioning": "悬疑仙侠长篇",
                    "target_reader": "喜欢强情节和硬设定的网文读者",
                    "core_hook": "旧票据会改写欠税者命运",
                    "series_promise": "查清灵税体系背后的旧朝祭司网络",
                    "first_30_chapter_promise": "主角从票据线索查到第一座地下灵脉",
                    "world_seed": {"factions": ["镇岳军"]},
                    "character_seed": {"protagonist": "沈砚"},
                    "volume_strategy": [{"volume": 1, "goal": "票据来源"}],
                    "golden_three_chapters": [
                        {"chapter": 1, "hook": "死人票据"},
                        {"chapter": 2, "hook": "账本追杀"},
                        {"chapter": 3, "hook": "地下灵脉"}
                    ],
                    "revision_note": "model candidate A"
                },
                {
                    "title_options": ["旧站无灯"],
                    "positioning": "民俗悬疑长篇",
                    "target_reader": "喜欢慢热调查的读者",
                    "core_hook": "每盏灯都对应一笔旧债",
                    "series_promise": "揭开旧站灯簿和失踪人口的关系",
                    "first_30_chapter_promise": "主角查到第一份灯簿名单",
                    "world_seed": {"locations": ["旧站"]},
                    "character_seed": {"protagonist": "许灯"},
                    "volume_strategy": [{"volume": 1, "goal": "灯簿名单"}],
                    "golden_three_chapters": [
                        {"chapter": 1, "hook": "灯灭人亡"},
                        {"chapter": 2, "hook": "名单出现"},
                        {"chapter": 3, "hook": "旧站回声"}
                    ],
                    "revision_note": "model candidate B"
                }
            ]
        }))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Err("FakeDirectorClient only supports JSON".to_string())
    }

    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Err("FakeDirectorClient does not embed".to_string())
    }
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "Director Fixture",
        Some("direction candidate fixture"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("quiet"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn director_candidate_round_trips_selected_checkpoint_state() {
    let db = setup_db("director-candidate-roundtrip.db");
    let project_id = insert_project(&db);

    let candidate_id = tauri_app_lib::db::director::upsert_direction_candidate(
        &db,
        &tauri_app_lib::db::director::DirectionCandidateInput {
            id: Some("dir-1".to_string()),
            project_id: Some(project_id.clone()),
            inspiration: "雨夜车站里，一张旧票据揭开灵税阴谋。".to_string(),
            title_options: vec!["雨站账本".to_string(), "旧票据".to_string()],
            positioning: "悬疑仙侠长篇".to_string(),
            target_reader: "喜欢强情节和硬设定的网文读者".to_string(),
            core_hook: "每张票据都改写一个人的命运".to_string(),
            series_promise: "查清灵税体系背后的旧朝祭司网络".to_string(),
            first_30_chapter_promise: "主角从票据线索查到第一座地下灵脉。".to_string(),
            world_seed: serde_json::json!({"factions": ["镇岳军", "旧朝祭司"]}),
            character_seed: serde_json::json!({"protagonist": "沈砚"}),
            volume_strategy: serde_json::json!([{"volume": 1, "goal": "查清票据来源"}]),
            golden_three_chapters: serde_json::json!([
                {"chapter": 1, "hook": "票据死人"},
                {"chapter": 2, "hook": "灵税账本"},
                {"chapter": 3, "hook": "旧站追杀"}
            ]),
            checkpoint_status: "draft".to_string(),
            revision_note: Some("first pass".to_string()),
            selected: false,
            metadata: serde_json::json!({"prompt_hash": "hash-1"}),
        },
    )
    .expect("candidate should persist");

    let loaded =
        tauri_app_lib::db::director::list_direction_candidates(&db, Some(project_id.as_str()))
            .expect("candidates should load");

    assert_eq!(candidate_id, "dir-1");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "dir-1");
    assert_eq!(loaded[0].project_id.as_deref(), Some(project_id.as_str()));
    assert_eq!(loaded[0].title_options[0], "雨站账本");
    assert_eq!(loaded[0].checkpoint_status, "draft");
    assert_eq!(loaded[0].revision_note.as_deref(), Some("first pass"));
    assert_eq!(loaded[0].metadata["prompt_hash"].as_str(), Some("hash-1"));
}

#[test]
fn selecting_direction_candidate_clears_sibling_selection() {
    let db = setup_db("director-candidate-selection.db");
    let project_id = insert_project(&db);

    for (id, selected) in [("dir-a", true), ("dir-b", false)] {
        tauri_app_lib::db::director::upsert_direction_candidate(
            &db,
            &tauri_app_lib::db::director::DirectionCandidateInput {
                id: Some(id.to_string()),
                project_id: Some(project_id.clone()),
                inspiration: "旧票据揭开灵税阴谋".to_string(),
                title_options: vec![format!("{} title", id)],
                positioning: "悬疑仙侠长篇".to_string(),
                target_reader: "网文读者".to_string(),
                core_hook: "票据改写命运".to_string(),
                series_promise: "查清旧朝祭司网络".to_string(),
                first_30_chapter_promise: "查到第一座地下灵脉".to_string(),
                world_seed: serde_json::json!({}),
                character_seed: serde_json::json!({}),
                volume_strategy: serde_json::json!([]),
                golden_three_chapters: serde_json::json!([]),
                checkpoint_status: "draft".to_string(),
                revision_note: None,
                selected,
                metadata: serde_json::json!({}),
            },
        )
        .unwrap();
    }

    tauri_app_lib::db::director::select_direction_candidate(
        &db,
        "dir-b",
        Some("operator picked B"),
    )
    .expect("selection should persist");

    let loaded =
        tauri_app_lib::db::director::list_direction_candidates(&db, Some(project_id.as_str()))
            .unwrap();
    let selected = loaded
        .iter()
        .filter(|candidate| candidate.selected)
        .map(|candidate| candidate.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(selected, vec!["dir-b"]);
    assert_eq!(
        loaded
            .iter()
            .find(|candidate| candidate.id == "dir-b")
            .unwrap()
            .revision_note
            .as_deref(),
        Some("operator picked B")
    );
}

#[tokio::test]
async fn director_workflow_generates_candidates_with_prompt_metadata() {
    let db = setup_db("director-workflow-generation.db");
    let project_id = insert_project(&db);

    let candidates = director_mode::generate_direction_candidates(
        &db,
        &FakeDirectorClient,
        director_mode::DirectionGenerationRequest {
            project_id: Some(project_id.clone()),
            inspiration: "雨夜车站里，一张旧票据揭开灵税阴谋。".to_string(),
            candidate_count: 2,
            model_profile_snapshot: serde_json::json!({"provider": "fake"}),
        },
    )
    .await
    .expect("director candidates should generate");

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].project_id.as_deref(),
        Some(project_id.as_str())
    );
    assert_eq!(candidates[0].title_options[0], "雨站账本");
    assert_eq!(
        candidates[0].metadata["input_inspiration"].as_str(),
        Some("雨夜车站里，一张旧票据揭开灵税阴谋。")
    );
    assert!(candidates[0].metadata["prompt_hash"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
    assert_eq!(
        candidates[0].metadata["model_profile_snapshot"]["provider"].as_str(),
        Some("fake")
    );
}

#[test]
fn director_bootstrap_handoff_uses_selected_candidate_without_creating_project() {
    let db = setup_db("director-bootstrap-handoff.db");
    let project_id = insert_project(&db);

    tauri_app_lib::db::director::upsert_direction_candidate(
        &db,
        &tauri_app_lib::db::director::DirectionCandidateInput {
            id: Some("dir-handoff".to_string()),
            project_id: Some(project_id.clone()),
            inspiration: "旧票据揭开灵税阴谋".to_string(),
            title_options: vec!["雨站账本".to_string()],
            positioning: "悬疑仙侠长篇".to_string(),
            target_reader: "网文读者".to_string(),
            core_hook: "票据改写命运".to_string(),
            series_promise: "查清旧朝祭司网络".to_string(),
            first_30_chapter_promise: "查到第一座地下灵脉".to_string(),
            world_seed: serde_json::json!({"factions": ["镇岳军"]}),
            character_seed: serde_json::json!({"protagonist": "沈砚"}),
            volume_strategy: serde_json::json!([{"volume": 1, "goal": "查清票据来源"}]),
            golden_three_chapters: serde_json::json!([{"chapter": 1, "hook": "票据死人"}]),
            checkpoint_status: "selected".to_string(),
            revision_note: None,
            selected: true,
            metadata: serde_json::json!({}),
        },
    )
    .unwrap();

    let handoff =
        director_mode::build_bootstrap_handoff(&db, "dir-handoff").expect("handoff should build");

    assert_eq!(handoff.candidate_id, "dir-handoff");
    assert_eq!(handoff.project_id.as_deref(), Some(project_id.as_str()));
    assert_eq!(handoff.suggested_title, "雨站账本");
    assert_eq!(handoff.world_seed["factions"][0].as_str(), Some("镇岳军"));
    assert_eq!(handoff.requires_human_review, true);
}
