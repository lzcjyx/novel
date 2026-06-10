use async_trait::async_trait;
use serde_json::json;
use serde_json::Value;
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::models::bible::{
    CanonRule, Character, CharacterState, Location, StyleGuide, TimelineEvent,
};
use tauri_app_lib::models::{BibleData, Chapter, ChapterVersion, Project};
use tauri_app_lib::workflow::canon_consistency::{
    detect_canon_consistency_issues, detect_review_precheck_issues_from_json,
};
use tauri_app_lib::workflow::review_agents::{self, CanonContext};
use tauri_app_lib::workflow::review_arbiter;

fn character(id: &str, name: &str, status: &str) -> Character {
    Character {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        name: name.to_string(),
        aliases: String::new(),
        role: None,
        personality: None,
        motivation: None,
        speech_style: None,
        appearance: None,
        backstory: None,
        relationship_map: "[]".to_string(),
        locked_fields: "[]".to_string(),
        status: status.to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn character_state(character_id: &str, physical_state: &str) -> CharacterState {
    CharacterState {
        id: format!("state-{character_id}"),
        project_id: "project-1".to_string(),
        character_id: character_id.to_string(),
        after_chapter_id: None,
        physical_state: Some(physical_state.to_string()),
        emotional_state: None,
        knowledge_state: None,
        relationship_state: "{}".to_string(),
        location_id: None,
        inventory: "[]".to_string(),
        open_conflicts: "[]".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn locked_character_location(character_id: &str, location_id: &str) -> CharacterState {
    CharacterState {
        id: format!("state-{character_id}-location"),
        project_id: "project-1".to_string(),
        character_id: character_id.to_string(),
        after_chapter_id: None,
        physical_state: Some("健康".to_string()),
        emotional_state: None,
        knowledge_state: None,
        relationship_state: "{}".to_string(),
        location_id: Some(location_id.to_string()),
        inventory: "[]".to_string(),
        open_conflicts: "[]".to_string(),
        metadata: json!({ "locked_location": true }).to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn location(id: &str, name: &str) -> Location {
    Location {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        name: name.to_string(),
        r#type: Some("place".to_string()),
        description: None,
        rules: None,
        connected_locations: "[]".to_string(),
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn locked_timeline_event(id: &str, sequence: i32, summary: &str) -> TimelineEvent {
    TimelineEvent {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        chapter_id: None,
        event_time_label: None,
        sequence: Some(sequence),
        event_summary: Some(summary.to_string()),
        involved_characters: "[]".to_string(),
        involved_locations: "[]".to_string(),
        consequences: String::new(),
        status: "planned".to_string(),
        metadata: json!({ "locked_sequence": true }).to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn planned_timeline_event(id: &str, sequence: i32, summary: &str) -> TimelineEvent {
    let mut event = locked_timeline_event(id, sequence, summary);
    event.metadata = "{}".to_string();
    event
}

fn explicit_style_guide() -> StyleGuide {
    StyleGuide {
        id: "style-1".to_string(),
        project_id: "project-1".to_string(),
        name: Some("克制悬疑".to_string()),
        style_text: Some(
            json!({
                "forbidden_phrases": ["眼中闪过"],
                "required_phrases": ["杯口转向墙角"],
                "enforce_required_phrases": true,
                "style_precheck_severity": "blocking"
            })
            .to_string(),
        ),
        positive_examples: "[]".to_string(),
        negative_examples: "[]".to_string(),
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn plain_text_style_guide() -> StyleGuide {
    StyleGuide {
        id: "style-plain".to_string(),
        project_id: "project-1".to_string(),
        name: Some("Plain text style".to_string()),
        style_text: Some(
            "Forbidden phrases:\n- 眼中闪过\nRequired phrases:\n- 杯口转向墙角\nEnforce required phrases: true\nStyle precheck severity: blocking"
                .to_string(),
        ),
        positive_examples: "[]".to_string(),
        negative_examples: "[]".to_string(),
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn forbidden_rule(term: &str) -> CanonRule {
    CanonRule {
        id: "rule-1".to_string(),
        project_id: "project-1".to_string(),
        rule_type: Some("hard_forbidden_term".to_string()),
        rule_text: Some("黑火术已经被世界规则禁止".to_string()),
        severity: "hard".to_string(),
        locked: true,
        status: "active".to_string(),
        metadata: json!({ "forbidden_terms": [term] }).to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn precheck_flags_unlocked_latest_location_and_planned_future_timeline() {
    let characters = vec![character("char-main", "林风", "active")];
    let mut latest_state = locked_character_location("char-main", "loc-cell");
    latest_state.metadata = "{}".to_string();
    let states = vec![latest_state];
    let locations = vec![
        location("loc-cell", "北塔囚室"),
        location("loc-market", "南城集市"),
    ];
    let future_event = "林风在南城集市公开揭露城主罪证";
    let timeline = vec![planned_timeline_event("event-future", 5, future_event)];
    let chapter = format!("林风出现在南城集市，众人围住他。{future_event}，城主府随即震动。");

    let issues = detect_review_precheck_issues_from_json(
        &chapter,
        "{}",
        &serde_json::to_string(&characters).unwrap(),
        &serde_json::to_string(&states).unwrap(),
        "[]",
        &serde_json::to_string(&locations).unwrap(),
        &serde_json::to_string(&timeline).unwrap(),
        "[]",
        3,
    );

    let location_issue = issues
        .iter()
        .find(|issue| issue.rule_type == "location_continuity_conflict")
        .expect("latest known location conflict should be flagged");
    let timeline_issue = issues
        .iter()
        .find(|issue| issue.rule_type == "future_timeline_event")
        .expect("planned future timeline event should be flagged");

    assert_eq!(location_issue.severity, "warning");
    assert_eq!(timeline_issue.severity, "blocking");
}

#[test]
fn precheck_flags_plain_text_style_rules() {
    let style_guides = vec![plain_text_style_guide()];
    let issues = detect_review_precheck_issues_from_json(
        "林风眼中闪过怒意，直接说出了所有恐惧。",
        "{}",
        "[]",
        "[]",
        "[]",
        "[]",
        "[]",
        &serde_json::to_string(&style_guides).unwrap(),
        4,
    );
    let issue_types = issues
        .iter()
        .map(|issue| (issue.rule_type.as_str(), issue.severity.as_str()))
        .collect::<Vec<_>>();

    assert!(issue_types.contains(&("style_forbidden_phrase", "blocking")));
    assert!(issue_types.contains(&("style_required_phrase_missing", "blocking")));
}

struct PassingReviewProvider;

#[async_trait]
impl ModelClient for PassingReviewProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!({}))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(json!({
            "score": 92,
            "pass": true,
            "blocking_issues": [],
            "minor_issues": [],
            "recommendations": []
        })
        .to_string())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

#[test]
fn deterministic_canon_check_flags_forbidden_terms_and_dead_character_actions() {
    let mut bible = BibleData::empty();
    bible
        .characters
        .push(character("char-dead", "玄霜", "active"));
    bible.canon_rules.push(forbidden_rule("黑火术"));

    let states = vec![character_state("char-dead", "死亡")];
    let chapter = "玄霜推门走入大厅，抬手释放黑火术，众人立刻退后。";

    let issues = detect_canon_consistency_issues(chapter, &bible, &states);

    assert_eq!(issues.len(), 2);
    assert!(issues
        .iter()
        .any(|issue| issue.rule_type == "forbidden_term"));
    assert!(issues
        .iter()
        .any(|issue| issue.rule_type == "dead_character_appearance"));
    assert!(issues.iter().all(|issue| issue.severity == "blocking"));
}

#[test]
fn deterministic_canon_check_ignores_clean_chapters() {
    let mut bible = BibleData::empty();
    bible
        .characters
        .push(character("char-live", "林风", "active"));
    bible.canon_rules.push(forbidden_rule("黑火术"));

    let states = vec![character_state("char-live", "健康")];
    let chapter = "林风推开山门，只用普通剑招逼退敌人。";

    let issues = detect_canon_consistency_issues(chapter, &bible, &states);

    assert!(issues.is_empty());
}

#[tokio::test]
async fn review_pipeline_includes_deterministic_canon_precheck() {
    let provider = PassingReviewProvider;
    let mut bible = BibleData::empty();
    bible
        .characters
        .push(character("char-dead", "玄霜", "active"));
    bible.canon_rules.push(forbidden_rule("黑火术"));
    let states = vec![character_state("char-dead", "死亡")];

    let chapter = Chapter {
        id: "chapter-1".to_string(),
        project_id: "project-1".to_string(),
        chapter_plan_id: None,
        sequence: 1,
        title: Some("违规章".to_string()),
        final_version_id: Some("version-1".to_string()),
        status: "draft".to_string(),
        word_count: Some(32),
        summary: None,
        published_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let version = ChapterVersion {
        id: "version-1".to_string(),
        chapter_id: chapter.id.clone(),
        project_id: chapter.project_id.clone(),
        version_number: 1,
        version_type: "draft".to_string(),
        title: chapter.title.clone(),
        body_markdown: Some("玄霜推门走入大厅，抬手释放黑火术，众人立刻退后。".to_string()),
        summary: None,
        word_count: Some(32),
        model_provider: None,
        model_name: None,
        prompt_hash: None,
        context_hash: None,
        created_by_agent: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let project = Project {
        id: chapter.project_id.clone(),
        name: "Test Project".to_string(),
        genre: None,
        target_audience: None,
        style_profile: "{}".to_string(),
        total_target_words: None,
        daily_target_words: None,
        auto_publish: false,
        quality_threshold: 85,
        blog_provider: None,
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let canon = CanonContext {
        writing_brief_json: "{}".to_string(),
        characters_json: serde_json::to_string(&bible.characters).unwrap(),
        character_states_json: serde_json::to_string(&states).unwrap(),
        previous_chapters_json: "[]".to_string(),
        active_plot_threads_json: "[]".to_string(),
        unresolved_foreshadowing_json: "[]".to_string(),
        world_lore_json: "[]".to_string(),
        locations_json: "[]".to_string(),
        organizations_json: "[]".to_string(),
        items_json: "[]".to_string(),
        magic_systems_json: "[]".to_string(),
        canon_rules_json: serde_json::to_string(&bible.canon_rules).unwrap(),
        timeline_json: "[]".to_string(),
        style_guide_json: "[]".to_string(),
        blog_config_json: "{}".to_string(),
        project_policy_json: "{}".to_string(),
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .unwrap();
    let precheck = reviews
        .iter()
        .find(|review| review.agent_name == "canon_consistency_precheck")
        .expect("deterministic canon precheck review should be included");
    let blocking_issues = serde_json::from_str::<Value>(&precheck.blocking_issues).unwrap();
    let aggregation = review_arbiter::aggregate_reviews(&reviews, 85, 1, 0);

    assert_eq!(precheck.pass, Some(false));
    assert_eq!(blocking_issues.as_array().unwrap().len(), 2);
    assert_eq!(aggregation.blocking_issue_count, 2);
    assert_eq!(aggregation.decision, "revise");
}

#[tokio::test]
async fn review_pipeline_flags_repeated_previous_ending_hook() {
    let provider = PassingReviewProvider;
    let repeated_hook = "潮湿墙面上缓缓浮现出林风的旧名，门后传来三声轻扣，像是有人隔着黑暗确认他的身世秘密。林风屏住呼吸，终于意识到追查多年的答案就在门后。";
    let chapter = Chapter {
        id: "chapter-2".to_string(),
        project_id: "project-1".to_string(),
        chapter_plan_id: None,
        sequence: 2,
        title: Some("重复章".to_string()),
        final_version_id: Some("version-2".to_string()),
        status: "draft".to_string(),
        word_count: Some(80),
        summary: None,
        published_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let version = ChapterVersion {
        id: "version-2".to_string(),
        chapter_id: chapter.id.clone(),
        project_id: chapter.project_id.clone(),
        version_number: 1,
        version_type: "draft".to_string(),
        title: chapter.title.clone(),
        body_markdown: Some(format!("{repeated_hook}随后他才开始新的行动。")),
        summary: None,
        word_count: Some(80),
        model_provider: None,
        model_name: None,
        prompt_hash: None,
        context_hash: None,
        created_by_agent: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let project = Project {
        id: chapter.project_id.clone(),
        name: "Test Project".to_string(),
        genre: None,
        target_audience: None,
        style_profile: "{}".to_string(),
        total_target_words: None,
        daily_target_words: None,
        auto_publish: false,
        quality_threshold: 85,
        blog_provider: None,
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let canon = CanonContext {
        writing_brief_json: json!({
            "continuity": {
                "previous_ending_hook": repeated_hook
            }
        })
        .to_string(),
        characters_json: "[]".to_string(),
        character_states_json: "[]".to_string(),
        previous_chapters_json: "[]".to_string(),
        active_plot_threads_json: "[]".to_string(),
        unresolved_foreshadowing_json: "[]".to_string(),
        world_lore_json: "[]".to_string(),
        locations_json: "[]".to_string(),
        organizations_json: "[]".to_string(),
        items_json: "[]".to_string(),
        magic_systems_json: "[]".to_string(),
        canon_rules_json: "[]".to_string(),
        timeline_json: "[]".to_string(),
        style_guide_json: "[]".to_string(),
        blog_config_json: "{}".to_string(),
        project_policy_json: "{}".to_string(),
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .unwrap();
    let precheck = reviews
        .iter()
        .find(|review| review.agent_name == "canon_consistency_precheck")
        .unwrap();
    let blocking_issues = serde_json::from_str::<Value>(&precheck.blocking_issues).unwrap();
    let first_issue = &blocking_issues.as_array().unwrap()[0];
    let aggregation = review_arbiter::aggregate_reviews(&reviews, 85, 1, 0);

    assert_eq!(precheck.pass, Some(false));
    assert_eq!(first_issue["rule_type"], "repeated_previous_ending");
    assert_eq!(aggregation.blocking_issue_count, 1);
    assert_eq!(aggregation.decision, "revise");
}

#[tokio::test]
async fn review_pipeline_flags_timeline_and_location_continuity() {
    let provider = PassingReviewProvider;
    let characters = vec![character("char-main", "林风", "active")];
    let states = vec![locked_character_location("char-main", "loc-cell")];
    let locations = vec![
        location("loc-cell", "北塔囚室"),
        location("loc-market", "南城集市"),
    ];
    let future_event = "林风在南城集市公开揭露城主罪证";
    let timeline = vec![locked_timeline_event("event-future", 5, future_event)];
    let chapter = Chapter {
        id: "chapter-3".to_string(),
        project_id: "project-1".to_string(),
        chapter_plan_id: None,
        sequence: 3,
        title: Some("提前揭露".to_string()),
        final_version_id: Some("version-3".to_string()),
        status: "draft".to_string(),
        word_count: Some(64),
        summary: None,
        published_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let version = ChapterVersion {
        id: "version-3".to_string(),
        chapter_id: chapter.id.clone(),
        project_id: chapter.project_id.clone(),
        version_number: 1,
        version_type: "draft".to_string(),
        title: chapter.title.clone(),
        body_markdown: Some(format!(
            "林风出现在南城集市，众人围住他。{future_event}，城主府随即震动。"
        )),
        summary: None,
        word_count: Some(64),
        model_provider: None,
        model_name: None,
        prompt_hash: None,
        context_hash: None,
        created_by_agent: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let project = Project {
        id: chapter.project_id.clone(),
        name: "Test Project".to_string(),
        genre: None,
        target_audience: None,
        style_profile: "{}".to_string(),
        total_target_words: None,
        daily_target_words: None,
        auto_publish: false,
        quality_threshold: 85,
        blog_provider: None,
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let canon = CanonContext {
        writing_brief_json: "{}".to_string(),
        characters_json: serde_json::to_string(&characters).unwrap(),
        character_states_json: serde_json::to_string(&states).unwrap(),
        previous_chapters_json: "[]".to_string(),
        active_plot_threads_json: "[]".to_string(),
        unresolved_foreshadowing_json: "[]".to_string(),
        world_lore_json: "[]".to_string(),
        locations_json: serde_json::to_string(&locations).unwrap(),
        organizations_json: "[]".to_string(),
        items_json: "[]".to_string(),
        magic_systems_json: "[]".to_string(),
        canon_rules_json: "[]".to_string(),
        timeline_json: serde_json::to_string(&timeline).unwrap(),
        style_guide_json: "[]".to_string(),
        blog_config_json: "{}".to_string(),
        project_policy_json: "{}".to_string(),
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .unwrap();
    let precheck = reviews
        .iter()
        .find(|review| review.agent_name == "canon_consistency_precheck")
        .unwrap();
    let blocking_issues = serde_json::from_str::<Value>(&precheck.blocking_issues).unwrap();
    let issue_types = blocking_issues
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["rule_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    let aggregation = review_arbiter::aggregate_reviews(&reviews, 85, 1, 0);

    assert_eq!(precheck.pass, Some(false));
    assert!(issue_types.contains(&"location_continuity_conflict"));
    assert!(issue_types.contains(&"future_timeline_event"));
    assert_eq!(aggregation.blocking_issue_count, 2);
    assert_eq!(aggregation.decision, "revise");
}

#[tokio::test]
async fn review_pipeline_flags_explicit_style_drift() {
    let provider = PassingReviewProvider;
    let style_guides = vec![explicit_style_guide()];
    let chapter = Chapter {
        id: "chapter-4".to_string(),
        project_id: "project-1".to_string(),
        chapter_plan_id: None,
        sequence: 4,
        title: Some("文风漂移".to_string()),
        final_version_id: Some("version-4".to_string()),
        status: "draft".to_string(),
        word_count: Some(48),
        summary: None,
        published_at: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let version = ChapterVersion {
        id: "version-4".to_string(),
        chapter_id: chapter.id.clone(),
        project_id: chapter.project_id.clone(),
        version_number: 1,
        version_type: "draft".to_string(),
        title: chapter.title.clone(),
        body_markdown: Some("林风眼中闪过怒意，直接说出了所有恐惧。".to_string()),
        summary: None,
        word_count: Some(48),
        model_provider: None,
        model_name: None,
        prompt_hash: None,
        context_hash: None,
        created_by_agent: None,
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let project = Project {
        id: chapter.project_id.clone(),
        name: "Test Project".to_string(),
        genre: None,
        target_audience: None,
        style_profile: "{}".to_string(),
        total_target_words: None,
        daily_target_words: None,
        auto_publish: false,
        quality_threshold: 85,
        blog_provider: None,
        status: "active".to_string(),
        metadata: "{}".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let canon = CanonContext {
        writing_brief_json: "{}".to_string(),
        characters_json: "[]".to_string(),
        character_states_json: "[]".to_string(),
        previous_chapters_json: "[]".to_string(),
        active_plot_threads_json: "[]".to_string(),
        unresolved_foreshadowing_json: "[]".to_string(),
        world_lore_json: "[]".to_string(),
        locations_json: "[]".to_string(),
        organizations_json: "[]".to_string(),
        items_json: "[]".to_string(),
        magic_systems_json: "[]".to_string(),
        canon_rules_json: "[]".to_string(),
        timeline_json: "[]".to_string(),
        style_guide_json: serde_json::to_string(&style_guides).unwrap(),
        blog_config_json: "{}".to_string(),
        project_policy_json: "{}".to_string(),
    };

    let reviews = review_agents::run_review_agents(&provider, &chapter, &version, &canon, &project)
        .await
        .unwrap();
    let precheck = reviews
        .iter()
        .find(|review| review.agent_name == "canon_consistency_precheck")
        .unwrap();
    let blocking_issues = serde_json::from_str::<Value>(&precheck.blocking_issues).unwrap();
    let issue_types = blocking_issues
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["rule_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    let aggregation = review_arbiter::aggregate_reviews(&reviews, 85, 1, 0);

    assert_eq!(precheck.pass, Some(false));
    assert!(issue_types.contains(&"style_forbidden_phrase"));
    assert!(issue_types.contains(&"style_required_phrase_missing"));
    assert_eq!(aggregation.blocking_issue_count, 2);
    assert_eq!(aggregation.decision, "revise");
}
