use super::connection::Database;

pub fn run_migrations(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

    let sql = include_str!("../../migrations/001_init_sqlite.sql");
    conn.execute_batch(sql)
        .map_err(|e| format!("Migration failed: {}", e))?;

    log::info!("Database migrations applied successfully.");
    Ok(())
}
