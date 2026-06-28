use std::path::PathBuf;

use tauri_app_lib::db::connection::Database;
use tauri_app_lib::models::{PublicationQueueInput, StaticSitePost};
use tauri_app_lib::workflow::static_site_publish::{
    plan_firefly_git_steps, render_firefly_markdown, sanitize_post_slug, write_firefly_post,
};

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("publication-queue.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "自动发布测试",
        Some("测试项目"),
        Some("悬疑"),
        None,
        Some("成人"),
        Some("冷静"),
        Some("中文连载"),
        Some(200000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn publication_target_settings_round_trip() {
    let db = setup_db();
    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();

    settings.publish_schedule_enabled = true;
    settings.publication_target_provider = "firefly_git".to_string();
    settings.publication_target_path = r"D:\Learning\Code\Git\website\Firefly".to_string();
    settings.publication_posts_dir = "src/content/posts".to_string();
    settings.publication_remote_name = "origin".to_string();
    settings.publication_branch = Some("master".to_string());
    settings.publication_build_command = "pnpm build".to_string();
    settings.publication_commit_template = "publish: add {title}".to_string();
    settings.publication_push_enabled = true;
    settings.publication_dry_run = false;
    settings.publication_validate_build = true;
    tauri_app_lib::db::settings::save_settings(&db, &settings).unwrap();

    let loaded = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    assert!(loaded.publish_schedule_enabled);
    assert_eq!(loaded.publication_target_provider, "firefly_git");
    assert_eq!(
        loaded.publication_target_path,
        r"D:\Learning\Code\Git\website\Firefly"
    );
    assert_eq!(loaded.publication_posts_dir, "src/content/posts");
    assert_eq!(loaded.publication_remote_name, "origin");
    assert_eq!(loaded.publication_branch.as_deref(), Some("master"));
    assert_eq!(loaded.publication_build_command, "pnpm build");
    assert!(loaded.publication_push_enabled);
    assert!(loaded.publication_validate_build);
}

#[test]
fn publication_queue_upsert_is_idempotent_and_recovers_interrupted_publish() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status)
         VALUES ('chapter-pub', ?1, 1, '雨夜旧案', 'final')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let input = PublicationQueueInput {
        project_id: project_id.clone(),
        chapter_id: "chapter-pub".to_string(),
        chapter_version_id: None,
        provider: "firefly_git".to_string(),
        scheduled_at: Some("2026-06-28 09:00:00".to_string()),
        metadata: serde_json::json!({"slug": "rain-night-case"}),
    };

    let first = tauri_app_lib::db::publication_queue::upsert_pending_publication(&db, &input)
        .expect("first queue row should insert");
    let second = tauri_app_lib::db::publication_queue::upsert_pending_publication(&db, &input)
        .expect("second queue row should update same item");
    assert_eq!(first, second);

    tauri_app_lib::db::publication_queue::claim_publication(&db, &first)
        .expect("pending row should be claimable");
    let recovered = tauri_app_lib::db::publication_queue::recover_interrupted_publications(
        &db,
        "app restarted while publishing",
    )
    .expect("recovery should succeed");

    assert_eq!(recovered, 1);
    let due =
        tauri_app_lib::db::publication_queue::list_due_publications(&db, "2026-06-28 09:01:00")
            .expect("due queue should load");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, first);
    assert_eq!(due[0].status, "pending");
    assert!(due[0]
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("app restarted"));
}

#[test]
fn firefly_markdown_renderer_uses_valid_frontmatter_and_redacts_internal_metadata() {
    let post = StaticSitePost {
        title: "雨夜旧案: \"钥匙\"".to_string(),
        slug: "雨夜 旧案!".to_string(),
        published: "2026-06-28".to_string(),
        description: "钥匙与旧站台: 引出第一条线索。".to_string(),
        tags: vec!["悬疑".to_string(), "连载: 第一卷".to_string()],
        category: Some("小说连载".to_string()),
        lang: Some("zh-CN".to_string()),
        body_markdown: "正文内容\n\n<!-- api_key: should-not-leak -->".to_string(),
    };

    let markdown = render_firefly_markdown(&post);

    assert!(markdown.contains("title: \"雨夜旧案: \\\"钥匙\\\"\""));
    assert!(markdown.contains("published: \"2026-06-28\""));
    assert!(markdown.contains("description: \"钥匙与旧站台: 引出第一条线索。\""));
    assert!(markdown.contains("tags: [\"悬疑\", \"连载: 第一卷\"]"));
    assert!(markdown.contains("category: \"小说连载\""));
    assert!(markdown.contains("draft: false"));
    assert!(markdown.contains("lang: \"zh-CN\""));
    assert!(!markdown.contains("api_key"));
    assert_eq!(sanitize_post_slug(&post.slug), "yu-ye-jiu-an");
}

#[test]
fn firefly_publish_redacts_secrets_paths_and_commit_messages() {
    let post = StaticSitePost {
        title: "发布 sk-123456789012345678901234".to_string(),
        slug: "privacy-check".to_string(),
        published: "2026-06-28".to_string(),
        description: "本地路径 D:\\novel\\private.db 与 Bearer abcdefghijklmnopqrstuvwxyz"
            .to_string(),
        tags: vec!["token=abcdefghijklmnopqrstuvwxyz".to_string()],
        category: Some("C:\\Users\\secret\\drafts".to_string()),
        lang: Some("zh-CN".to_string()),
        body_markdown: "正文可见\napi_key=abcdefghijklmnopqrstuvwxyz\n路径 D:\\novel\\private.db\nBearer abcdefghijklmnopqrstuvwxyz".to_string(),
    };

    let markdown = render_firefly_markdown(&post);
    assert!(!markdown.contains("sk-123456789012345678901234"));
    assert!(!markdown.contains("D:\\novel"));
    assert!(!markdown.contains("C:\\Users"));
    assert!(!markdown.contains("abcdefghijklmnopqrstuvwxyz"));
    assert!(markdown.contains("***REDACTED***"));
    assert!(markdown.contains("[LOCAL_PATH_REDACTED]"));

    let redacted_commit =
        tauri_app_lib::workflow::static_site_publish::redact_publish_commit_message(
            "publish: add sk-123456789012345678901234 from D:\\novel\\private.db",
        );
    assert!(!redacted_commit.contains("sk-123456789012345678901234"));
    assert!(!redacted_commit.contains("D:\\novel"));
}

#[test]
fn firefly_git_plan_stages_only_generated_post_and_keeps_push_optional() {
    let repo = PathBuf::from(r"D:\Learning\Code\Git\website\Firefly");
    let steps = plan_firefly_git_steps(
        &repo,
        "src/content/posts/rain-night-case.md",
        "pnpm build",
        "publish: add 雨夜旧案",
        "origin",
        Some("master"),
        false,
    );

    assert_eq!(steps[0], vec!["pnpm", "build"]);
    assert_eq!(steps[1], vec!["git", "status", "--porcelain"]);
    assert_eq!(
        steps[2],
        vec!["git", "add", "--", "src/content/posts/rain-night-case.md"]
    );
    assert_eq!(
        steps[3],
        vec!["git", "commit", "-m", "publish: add 雨夜旧案"]
    );
    assert!(
        steps
            .iter()
            .all(|step| !step.contains(&"--all".to_string())),
        "publisher must not stage unrelated repository changes"
    );
    assert!(
        steps.iter().all(|step| !step.contains(&"push".to_string())),
        "push must be optional"
    );
}

#[test]
fn firefly_writer_creates_post_under_configured_posts_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src").join("content.config.ts"),
        "defineCollection({});",
    )
    .unwrap();
    let post = StaticSitePost {
        title: "雨夜旧案".to_string(),
        slug: "rain-night-case".to_string(),
        published: "2026-06-28".to_string(),
        description: "钥匙与旧站台引出第一条线索。".to_string(),
        tags: vec!["悬疑".to_string()],
        category: Some("小说连载".to_string()),
        lang: Some("zh-CN".to_string()),
        body_markdown: "正文内容".to_string(),
    };

    let relative = write_firefly_post(dir.path(), "src/content/posts", &post).unwrap();
    let full_path = dir.path().join(&relative);
    let content = std::fs::read_to_string(full_path).unwrap();

    assert_eq!(
        relative,
        PathBuf::from("src/content/posts/rain-night-case.md")
    );
    assert!(content.contains("title: \"雨夜旧案\""));
    assert!(content.contains("正文内容"));
}

#[test]
fn publish_ready_chapter_enqueues_static_site_publication_when_schedule_is_enabled() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let mut settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();
    settings.publish_schedule_enabled = true;
    settings.publication_target_provider = "firefly_git".to_string();
    settings.publication_posts_dir = "src/content/posts".to_string();
    settings.publication_remote_name = "origin".to_string();
    settings.publication_branch = Some("master".to_string());
    settings.publication_push_enabled = false;
    settings.publication_validate_build = true;

    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status)
         VALUES ('chapter-scheduled', ?1, 1, '雨夜旧案', 'final')",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let first = tauri_app_lib::workflow::chapter_production::enqueue_publication_if_enabled(
        &db,
        &settings,
        &project,
        "chapter-scheduled",
        None,
        "雨夜旧案",
    )
    .unwrap()
    .expect("schedule should enqueue");
    let second = tauri_app_lib::workflow::chapter_production::enqueue_publication_if_enabled(
        &db,
        &settings,
        &project,
        "chapter-scheduled",
        None,
        "雨夜旧案",
    )
    .unwrap()
    .expect("second enqueue should update same row");

    assert_eq!(first, second);
    let due =
        tauri_app_lib::db::publication_queue::list_due_publications(&db, "2999-01-01 00:00:00")
            .unwrap();
    assert_eq!(due.len(), 1);
    let metadata: serde_json::Value = serde_json::from_str(&due[0].metadata).unwrap();
    assert_eq!(metadata["target"]["provider"].as_str(), Some("firefly_git"));
    assert_eq!(
        metadata["target"]["posts_dir"].as_str(),
        Some("src/content/posts")
    );
    assert_eq!(metadata["target"]["push_enabled"].as_bool(), Some(false));
}
