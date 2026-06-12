use crate::db::connection::Database;
use crate::models::BlogPost;
use rusqlite::params;

pub fn create_blog_post(
    db: &Database,
    project_id: &str,
    chapter_id: &str,
    provider: &str,
    title: &str,
    slug: &str,
    url: Option<&str>,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR IGNORE INTO blog_posts (id, project_id, chapter_id, provider, title, slug, url, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft')",
        params![id, project_id, chapter_id, provider, title, slug, url],
    ).map_err(|e| format!("Create blog post: {}", e))?;
    Ok(id)
}

pub fn get_blog_posts(db: &Database, project_id: &str) -> Result<Vec<BlogPost>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_id, provider, external_post_id, title, slug, url,
                status, published_at, metadata, created_at, updated_at
         FROM blog_posts WHERE project_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Prepare: {}", e))?;

    let posts = stmt
        .query_map(params![project_id], |row| {
            Ok(BlogPost {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_id: row.get(2)?,
                provider: row.get(3)?,
                external_post_id: row.get(4)?,
                title: row.get(5)?,
                slug: row.get(6)?,
                url: row.get(7)?,
                status: row.get(8)?,
                published_at: row.get(9)?,
                metadata: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect: {}", e))?;

    Ok(posts)
}
