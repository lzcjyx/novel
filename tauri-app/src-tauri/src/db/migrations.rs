use super::connection::Database;

pub fn run_migrations(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let sql = include_str!("../../migrations/001_init_sqlite.sql");
    conn.execute_batch(sql)
        .map_err(|e| format!("Migration failed: {}", e))?;

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
