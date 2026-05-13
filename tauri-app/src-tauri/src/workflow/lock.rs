use rusqlite::params;
use crate::db::connection::Database;

#[derive(Debug, Clone, Copy)]
pub enum LockType { ChapterGeneration = 1, WeeklyPlanning = 2 }

/// RAII guard: releases lock on drop. Holds raw ptr to db (safe: db outlives every lock).
pub struct GenerationLock { db: *const Database, project_id: String, lock_id: i64 }

// SAFETY: Database lives in Tauri AppState for app lifetime. Lock never outlives it.
unsafe impl Send for GenerationLock {}

impl GenerationLock {
    pub fn acquire(db: &Database, project_id: &str, lock_type: LockType) -> Result<Self, String> {
        let id = lock_type as i64;
        if try_acquire_impl(db, id, project_id) {
            Ok(Self { db: db as *const Database, project_id: project_id.into(), lock_id: id })
        } else {
            Err(format!("Lock {:?} busy for project {}", lock_type, &project_id[..8]))
        }
    }
}

impl Drop for GenerationLock {
    fn drop(&mut self) { release_impl(unsafe { &*self.db }, self.lock_id, &self.project_id); }
}

/// Try to acquire advisory lock. Returns true if acquired.
fn try_acquire_impl(db: &Database, lock_id: i64, holder: &str) -> bool {
    let conn = match db.conn.lock() { Ok(c) => c, Err(_) => return false };
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    match conn.execute(
        "INSERT OR IGNORE INTO advisory_locks (lock_id, acquired_at, holder) VALUES (?1, ?2, ?3)",
        params![lock_id, now, holder],
    ) { Ok(rows) => rows > 0, Err(_) => false }
}

/// Release advisory lock.
fn release_impl(db: &Database, lock_id: i64, holder: &str) -> bool {
    let conn = match db.conn.lock() { Ok(c) => c, Err(_) => return false };
    conn.execute("DELETE FROM advisory_locks WHERE lock_id = ?1 AND holder = ?2", params![lock_id, holder]).is_ok()
}

/// Cleanup locks older than timeout_secs.
pub fn cleanup_stale_locks(db: &Database, timeout_secs: i64) {
    if let Ok(conn) = db.conn.lock() {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(timeout_secs);
        let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let _ = conn.execute("DELETE FROM advisory_locks WHERE acquired_at < ?1", params![cutoff_str]);
    }
}
