use rusqlite::params;
use crate::db::connection::Database;
use crate::models::VectorDocument;

/// Serialize f32 slice to BLOB bytes for SQLite storage (little-endian)
fn f32_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize BLOB bytes back to f32 vector
fn blob_to_f32(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .filter_map(|c| <[u8; 4]>::try_from(c).ok().map(f32::from_le_bytes))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let na: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if na < 1e-9 || nb < 1e-9 { 0.0 } else { dot / (na * nb) }
}

/// Insert a vector document with its embedding stored as BLOB
pub fn insert_vector_document(
    db: &Database,
    project_id: &str,
    source_type: &str,
    source_id: Option<&str>,
    title: &str,
    content: &str,
    metadata: &str,
    embedding: &[f32],
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let blob = f32_to_blob(embedding);

    conn.execute(
        "INSERT INTO vector_document_metadata (id, project_id, source_type, source_id, title, content, metadata, embedding)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, project_id, source_type, source_id.unwrap_or(""), title, content, metadata, blob],
    ).map_err(|e| format!("Insert vector doc: {}", e))?;

    Ok(id)
}

/// Search similar documents using real cosine similarity against stored embeddings.
/// All documents are loaded and compared in-memory — fine for ≤10,000 docs on desktop.
pub fn search_similar_documents(
    db: &Database,
    project_id: &str,
    query_embedding: &[f32],
    top_k: usize,
) -> Result<Vec<VectorDocument>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_type, source_id, title, content, metadata, embedding, created_at, updated_at
         FROM vector_document_metadata WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare: {}", e))?;

    #[derive(Debug)]
    struct RawDoc {
        id: String, project_id: String, source_type: String,
        source_id: Option<String>, title: Option<String>,
        content: String, metadata: String,
        embedding_blob: Option<Vec<u8>>,
        created_at: Option<String>, updated_at: Option<String>,
    }

    let docs: Vec<RawDoc> = stmt.query_map(params![project_id], |row| {
        Ok(RawDoc {
            id: row.get(0)?, project_id: row.get(1)?, source_type: row.get(2)?,
            source_id: row.get(3)?, title: row.get(4)?,
            content: row.get(5)?, metadata: row.get(6)?,
            embedding_blob: row.get(7)?,
            created_at: row.get(8)?, updated_at: row.get(9)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| format!("Collect: {}", e))?;

    let mut scored: Vec<VectorDocument> = docs.into_iter().map(|d| {
        let sim = d.embedding_blob.as_deref().map(|blob| {
            let emb = blob_to_f32(blob);
            cosine_similarity(query_embedding, &emb)
        }).unwrap_or(0.0);
        VectorDocument {
            id: d.id, project_id: d.project_id, source_type: d.source_type,
            source_id: d.source_id, title: d.title,
            content: d.content, metadata: d.metadata,
            created_at: d.created_at.unwrap_or_default(), updated_at: d.updated_at.unwrap_or_default(),
            embedding: None, similarity: Some(sim),
        }
    }).collect();

    scored.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    Ok(scored)
}

pub fn delete_vector_documents_by_source(db: &Database, source_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute("DELETE FROM vector_document_metadata WHERE source_id = ?1", params![source_id])
        .map_err(|e| format!("Delete vector docs: {}", e))?;
    Ok(())
}
