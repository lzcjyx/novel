use super::connection::Database;
use rusqlite::OptionalExtension;

pub fn run_migrations(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let sql = include_str!("../../migrations/001_init_sqlite.sql");
    conn.execute_batch(sql)
        .map_err(|e| format!("Migration failed: {}", e))?;

    migrate_generation_jobs_cancelled_status(&conn)?;
    migrate_chapter_versions_accepted_candidate_type(&conn)?;

    let has_content_hash = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(vector_document_metadata)")
            .map_err(|e| format!("Prepare vector schema check: {}", e))?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("Read vector schema: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect vector schema: {}", e))?;
        columns.iter().any(|column| column == "content_hash")
    };
    if !has_content_hash {
        conn.execute(
            "ALTER TABLE vector_document_metadata ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| format!("Add vector content hash column: {}", e))?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_vector_docs_content_hash ON vector_document_metadata(project_id, content_hash)",
        [],
    )
    .map_err(|e| format!("Create vector content hash index: {}", e))?;

    let missing_hashes = {
        let mut stmt = conn
            .prepare("SELECT id, content FROM vector_document_metadata WHERE content_hash = ''")
            .map_err(|e| format!("Prepare vector hash backfill: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Read vector hash backfill rows: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect vector hash backfill rows: {}", e))?;
        rows
    };
    for (id, content) in missing_hashes {
        let content_hash = crate::db::vector_store::compute_content_hash(&content);
        conn.execute(
            "UPDATE vector_document_metadata SET content_hash = ?1 WHERE id = ?2",
            rusqlite::params![content_hash, id],
        )
        .map_err(|e| format!("Backfill vector content hash: {}", e))?;
    }

    log::info!("Database migrations applied successfully.");
    Ok(())
}

fn migrate_generation_jobs_cancelled_status(conn: &rusqlite::Connection) -> Result<(), String> {
    let table_sql = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'generation_jobs'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("Read generation_jobs schema: {}", e))?;
    let Some(table_sql) = table_sql else {
        return Ok(());
    };
    if table_sql.contains("'cancelled'") {
        return Ok(());
    }

    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS generation_jobs_old_status_migration;
        ALTER TABLE generation_jobs RENAME TO generation_jobs_old_status_migration;
        CREATE TABLE generation_jobs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chapter_plan_id TEXT NOT NULL REFERENCES chapter_plans(id) ON DELETE CASCADE,
            job_date TEXT NOT NULL DEFAULT (date('now')),
            status TEXT NOT NULL DEFAULT 'started'
                CHECK (status IN ('started','draft_created','reviewing','revising','publishing','completed','failed','needs_human_review','skipped','cancelled')),
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            error_message TEXT,
            retry_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(project_id, chapter_plan_id, job_date)
        );
        INSERT INTO generation_jobs
            (id, project_id, chapter_plan_id, job_date, status, started_at, completed_at,
             error_message, retry_count, metadata, created_at, updated_at)
        SELECT id, project_id, chapter_plan_id, job_date, status, started_at, completed_at,
               error_message, retry_count, metadata, created_at, updated_at
        FROM generation_jobs_old_status_migration;
        DROP TABLE generation_jobs_old_status_migration;
        CREATE INDEX IF NOT EXISTS idx_generation_jobs_project_id ON generation_jobs(project_id);
        CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON generation_jobs(status);
        CREATE INDEX IF NOT EXISTS idx_generation_jobs_date ON generation_jobs(job_date);
        PRAGMA foreign_keys = ON;
        ",
    )
    .map_err(|e| format!("Migrate generation_jobs cancelled status: {}", e))?;

    Ok(())
}

fn migrate_chapter_versions_accepted_candidate_type(
    conn: &rusqlite::Connection,
) -> Result<(), String> {
    let table_sql = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'chapter_versions'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("Read chapter_versions schema: {}", e))?;
    let Some(table_sql) = table_sql else {
        return Ok(());
    };
    if table_sql.contains("'accepted_candidate'") {
        return Ok(());
    }

    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS chapter_versions_old_type_migration;
        ALTER TABLE chapter_versions RENAME TO chapter_versions_old_type_migration;
        CREATE TABLE chapter_versions (
            id TEXT PRIMARY KEY,
            chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            version_number INTEGER NOT NULL,
            version_type TEXT NOT NULL DEFAULT 'draft'
                CHECK (version_type IN ('draft','revised','final','accepted_candidate')),
            title TEXT,
            body_markdown TEXT,
            summary TEXT,
            word_count INTEGER,
            model_provider TEXT,
            model_name TEXT,
            prompt_hash TEXT,
            context_hash TEXT,
            created_by_agent TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO chapter_versions
            (id, chapter_id, project_id, version_number, version_type, title, body_markdown,
             summary, word_count, model_provider, model_name, prompt_hash, context_hash,
             created_by_agent, metadata, created_at, updated_at)
        SELECT id, chapter_id, project_id, version_number, version_type, title, body_markdown,
               summary, word_count, model_provider, model_name, prompt_hash, context_hash,
               created_by_agent, metadata, created_at, updated_at
        FROM chapter_versions_old_type_migration;
        DROP TABLE chapter_versions_old_type_migration;
        CREATE INDEX IF NOT EXISTS idx_chapter_versions_chapter_id ON chapter_versions(chapter_id);
        CREATE INDEX IF NOT EXISTS idx_chapter_versions_number ON chapter_versions(chapter_id, version_number);
        PRAGMA foreign_keys = ON;
        ",
    )
    .map_err(|e| format!("Migrate chapter_versions accepted_candidate type: {}", e))?;

    Ok(())
}
