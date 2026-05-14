use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create data directory: {}", e))?;
        }

        let conn = Connection::open(path)
            .map_err(|e| format!("Cannot open database: {}", e))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;"
        ).map_err(|e| format!("Cannot set pragmas: {}", e))?;

        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn new_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
