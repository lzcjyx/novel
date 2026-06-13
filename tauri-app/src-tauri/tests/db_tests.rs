use rusqlite::Connection;
use tauri_app_lib::db::connection::Database;
use tempfile::tempdir;

// Helper: create an in-memory DB with migrations
fn setup_db() -> (Connection, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = Connection::open(&db_path).unwrap();

    // Run migration SQL (simplified for tests — just create the key tables)
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            genre TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            style_profile TEXT NOT NULL DEFAULT '{}',
            auto_publish INTEGER NOT NULL DEFAULT 0,
            quality_threshold INTEGER NOT NULL DEFAULT 85,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS chapters (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chapter_plan_id TEXT,
            sequence INTEGER NOT NULL,
            title TEXT,
            final_version_id TEXT,
            status TEXT NOT NULL DEFAULT 'draft',
            word_count INTEGER,
            summary TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS chapter_versions (
            id TEXT PRIMARY KEY,
            chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            version_number INTEGER NOT NULL,
            version_type TEXT NOT NULL DEFAULT 'draft',
            title TEXT,
            body_markdown TEXT,
            word_count INTEGER,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS chapter_plans (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            sequence INTEGER NOT NULL,
            title TEXT,
            outline TEXT,
            status TEXT NOT NULL DEFAULT 'planned',
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS agent_reviews (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
            agent_name TEXT,
            score INTEGER,
            pass INTEGER,
            blocking_issues TEXT NOT NULL DEFAULT '[]',
            minor_issues TEXT NOT NULL DEFAULT '[]',
            recommendations TEXT NOT NULL DEFAULT '[]',
            raw_output TEXT NOT NULL DEFAULT '{}',
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS review_scores (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
            average_score REAL,
            final_score REAL,
            decision TEXT,
            publish_allowed INTEGER NOT NULL DEFAULT 0,
            blocking_issue_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS generation_jobs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chapter_plan_id TEXT NOT NULL,
            job_date TEXT NOT NULL DEFAULT (date('now')),
            status TEXT NOT NULL DEFAULT 'started',
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            error_message TEXT,
            retry_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(project_id, chapter_plan_id, job_date)
        );
    ",
    )
    .unwrap();

    (conn, dir)
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn migrations_add_content_hash_to_existing_vector_table_before_index() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("old-vector-schema.db");
    let db = Database::open(&db_path).unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE vector_document_metadata (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_id TEXT,
                title TEXT,
                content TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                embedding BLOB,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            INSERT INTO vector_document_metadata
                (id, project_id, source_type, source_id, title, content, metadata)
            VALUES
                ('vec-old', 'project-old', 'chapter', 'chapter-old', 'Old vector', 'legacy vector content', '{}');
            ",
        )
        .unwrap();
    }

    tauri_app_lib::db::run_migrations(&db).unwrap();

    let (has_content_hash, persisted_hash, has_index): (bool, String, bool) = {
        let conn = db.conn.lock().unwrap();
        let has_content_hash = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(vector_document_metadata)")
                .unwrap();
            let columns = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            columns.iter().any(|column| column == "content_hash")
        };
        let persisted_hash: String = conn
            .query_row(
                "SELECT content_hash FROM vector_document_metadata WHERE id = 'vec-old'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let has_index = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_vector_docs_content_hash'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 1;
        (has_content_hash, persisted_hash, has_index)
    };

    assert!(has_content_hash);
    assert_eq!(
        persisted_hash,
        tauri_app_lib::db::vector_store::compute_content_hash("legacy vector content")
    );
    assert!(has_index);
}

#[test]
fn migrations_expand_generation_job_status_check_for_cancelled() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("old-generation-job-status.db");
    let db = Database::open(&db_path).unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE generation_jobs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                chapter_plan_id TEXT NOT NULL REFERENCES chapter_plans(id) ON DELETE CASCADE,
                job_date TEXT NOT NULL DEFAULT (date('now')),
                status TEXT NOT NULL DEFAULT 'started'
                    CHECK (status IN ('started','draft_created','reviewing','revising','publishing','completed','failed','needs_human_review','skipped')),
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(project_id, chapter_plan_id, job_date)
            );
            ",
        )
        .unwrap();
    }

    tauri_app_lib::db::run_migrations(&db).unwrap();

    let project_id = tauri_app_lib::db::projects::create_project(
        &db,
        "Cancelled Migration",
        None,
        Some("mystery"),
        None,
        Some("adult"),
        Some("quiet"),
        Some("quiet"),
        Some(100000),
        Some(1000),
    )
    .unwrap()
    .id;
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, status)
         VALUES ('plan-cancelled', ?1, 1, 'Cancelled', 'planned')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO generation_jobs (id, project_id, chapter_plan_id, status)
         VALUES ('job-cancelled', ?1, 'plan-cancelled', 'cancelled')",
        rusqlite::params![project_id],
    )
    .unwrap();
}

#[test]
fn test_project_crud() {
    let (conn, _dir) = setup_db();
    let id = uuid();
    let name = "Test Novel";

    // INSERT
    conn.execute(
        "INSERT INTO projects (id, name, genre, status) VALUES (?1, ?2, 'fantasy', 'active')",
        rusqlite::params![id, name],
    )
    .unwrap();

    // SELECT
    let result: String = conn
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(result, name);

    // DELETE
    conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id])
        .unwrap();
}

#[test]
fn test_chapter_and_version_creation() {
    let (conn, _dir) = setup_db();
    let project_id = uuid();
    let chapter_id = uuid();
    let version_id = uuid();
    let plan_id = uuid();

    conn.execute(
        "INSERT INTO projects (id, name) VALUES (?1, 'Test')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title) VALUES (?1, ?2, 1, 'Chapter 1')",
        rusqlite::params![plan_id, project_id],
    ).unwrap();

    // Insert chapter first (FK: chapter_versions.chapter_id -> chapters.id)
    conn.execute(
        "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status) VALUES (?1, ?2, ?3, 1, 'Ch.1', 'draft')",
        rusqlite::params![chapter_id, project_id, plan_id],
    ).unwrap();

    // Then insert version
    conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count) VALUES (?1, ?2, ?3, 1, 'draft', 'Ch.1', 'Test content', 12)",
        rusqlite::params![version_id, chapter_id, project_id],
    ).unwrap();

    // Verify FK constraint: inserting a version referencing a non-existent chapter should fail
    let bad_result = conn.execute(
        "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, body_markdown, word_count) VALUES (?1, 'nonexistent', ?2, 1, 'draft', 'Bad', 'Bad', 0)",
        rusqlite::params![uuid(), project_id],
    );
    assert!(bad_result.is_err(), "Should fail FK constraint");

    // Update final_version_id
    conn.execute(
        "UPDATE chapters SET final_version_id = ?1 WHERE id = ?2",
        rusqlite::params![version_id, chapter_id],
    )
    .unwrap();
}

#[test]
fn test_generation_job_idempotency() {
    let (conn, _dir) = setup_db();
    let project_id = uuid();
    let plan_id = uuid();

    conn.execute(
        "INSERT INTO projects (id, name) VALUES (?1, 'Test')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence) VALUES (?1, ?2, 1)",
        rusqlite::params![plan_id, project_id],
    )
    .unwrap();

    let job_id = uuid();
    let date = "2026-05-09";

    // First insert succeeds
    conn.execute(
        "INSERT INTO generation_jobs (id, project_id, chapter_plan_id, job_date) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![job_id, project_id, plan_id, date],
    ).unwrap();

    // Second insert with same project_id + plan_id + date should fail (UNIQUE constraint)
    let dup_result = conn.execute(
        "INSERT INTO generation_jobs (id, project_id, chapter_plan_id, job_date) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![uuid(), project_id, plan_id, date],
    );
    assert!(
        dup_result.is_err(),
        "Duplicate generation_job should be rejected"
    );
}

#[test]
fn test_agent_review_crud() {
    let (conn, _dir) = setup_db();
    let project_id = uuid();
    let chapter_id = uuid();

    conn.execute(
        "INSERT INTO projects (id, name) VALUES (?1, 'Test')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title) VALUES (?1, ?2, 1, 'Ch.1')",
        rusqlite::params![chapter_id, project_id],
    )
    .unwrap();

    let review_id = uuid();
    conn.execute(
        "INSERT INTO agent_reviews (id, project_id, chapter_id, agent_name, score, pass, blocking_issues, minor_issues) VALUES (?1, ?2, ?3, 'continuity_reviewer', 85, 1, '[]', '[]')",
        rusqlite::params![review_id, project_id, chapter_id],
    ).unwrap();

    let (name, score, pass): (String, i32, i32) = conn
        .query_row(
            "SELECT agent_name, score, pass FROM agent_reviews WHERE id = ?1",
            rusqlite::params![review_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(name, "continuity_reviewer");
    assert_eq!(score, 85);
    assert_eq!(pass, 1);
}
