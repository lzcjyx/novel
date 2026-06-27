use crate::ai::client::EmbeddingInputKind;
use crate::db::connection::Database;
use crate::models::VectorDocument;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSource {
    pub rank: usize,
    pub document_id: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub excerpt: String,
    pub similarity: Option<f64>,
    pub relevance_label: String,
    #[serde(default)]
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub source_count: usize,
    pub best_similarity: Option<f64>,
    pub sources: Vec<RetrievalSource>,
}

#[derive(Debug, Clone)]
pub struct VectorIndexCandidate {
    pub source_id: String,
    pub source_type: String,
    pub title: String,
    pub content: String,
    pub metadata: String,
}

impl VectorIndexCandidate {
    pub fn new(
        source_id: impl Into<String>,
        source_type: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        metadata: impl Into<String>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            source_type: source_type.into(),
            title: title.into(),
            content: content.into(),
            metadata: metadata.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VectorEmbeddingMetadata {
    pub provider: String,
    pub model: String,
    pub kind: EmbeddingInputKind,
    pub dim: i32,
}

impl VectorEmbeddingMetadata {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        kind: EmbeddingInputKind,
        embedding: &[f32],
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            kind,
            dim: embedding.len() as i32,
        }
    }

    pub fn kind_key(&self) -> &'static str {
        match self.kind {
            EmbeddingInputKind::Document => "document",
            EmbeddingInputKind::Query => "query",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagHealth {
    pub state: String,
    pub message: String,
    pub document_count: i64,
    pub stale_count: i64,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dim: i32,
    pub last_indexed_at: Option<String>,
}

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
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let na: f64 = a
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    let nb: f64 = b
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    if na < 1e-9 || nb < 1e-9 {
        0.0
    } else {
        dot / (na * nb)
    }
}

fn content_excerpt(content: &str, max_chars: usize) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut excerpt = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        excerpt.push_str("...");
    }
    excerpt
}

fn relevance_label(similarity: Option<f64>) -> String {
    let score = similarity.unwrap_or(0.0);
    if score >= 0.82 {
        "high"
    } else if score >= 0.55 {
        "medium"
    } else if score > 0.0 {
        "low"
    } else {
        "none"
    }
    .to_string()
}

pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn source_content_hash_exists(
    db: &Database,
    project_id: &str,
    source_type: &str,
    source_id: Option<&str>,
    content: &str,
) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let source_id = source_id.unwrap_or("");
    let content_hash = compute_content_hash(content);
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM vector_document_metadata
             WHERE project_id = ?1 AND source_type = ?2 AND source_id = ?3 AND content_hash = ?4",
            params![project_id, source_type, source_id, content_hash],
            |row| row.get(0),
        )
        .map_err(|e| format!("Check vector content hash: {}", e))?;
    Ok(count > 0)
}

pub fn filter_vector_index_candidates(
    db: &Database,
    project_id: &str,
    candidates: Vec<VectorIndexCandidate>,
) -> Result<Vec<VectorIndexCandidate>, String> {
    let mut pending = Vec::new();
    for candidate in candidates {
        if !source_content_hash_exists(
            db,
            project_id,
            &candidate.source_type,
            Some(&candidate.source_id),
            &candidate.content,
        )? {
            pending.push(candidate);
        }
    }
    Ok(pending)
}

pub fn build_retrieval_trace(docs: &[VectorDocument]) -> RetrievalTrace {
    let sources = docs
        .iter()
        .enumerate()
        .map(|(idx, doc)| RetrievalSource {
            rank: idx + 1,
            document_id: doc.id.clone(),
            source_type: doc.source_type.clone(),
            source_id: doc.source_id.clone(),
            title: doc.title.clone(),
            excerpt: content_excerpt(&doc.content, 180),
            similarity: doc.similarity,
            relevance_label: relevance_label(doc.similarity),
            metadata: doc.metadata.clone(),
        })
        .collect::<Vec<_>>();

    RetrievalTrace {
        source_count: sources.len(),
        best_similarity: sources.first().and_then(|source| source.similarity),
        sources,
    }
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
    let embedding_metadata =
        VectorEmbeddingMetadata::new("legacy", "unknown", EmbeddingInputKind::Document, embedding);
    insert_vector_document_with_embedding_metadata(
        db,
        project_id,
        source_type,
        source_id,
        title,
        content,
        metadata,
        embedding,
        &embedding_metadata,
    )
}

pub fn insert_vector_document_with_embedding_metadata(
    db: &Database,
    project_id: &str,
    source_type: &str,
    source_id: Option<&str>,
    title: &str,
    content: &str,
    metadata: &str,
    embedding: &[f32],
    embedding_metadata: &VectorEmbeddingMetadata,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let source_id = source_id.unwrap_or("");
    let content_hash = compute_content_hash(content);
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id
             FROM vector_document_metadata
             WHERE project_id = ?1 AND source_type = ?2 AND source_id = ?3 AND content_hash = ?4
               AND embedding_provider = ?5 AND embedding_model = ?6
               AND embedding_kind = ?7 AND embedding_dim = ?8
             ORDER BY created_at ASC
             LIMIT 1",
            params![
                project_id,
                source_type,
                source_id,
                content_hash,
                embedding_metadata.provider.as_str(),
                embedding_metadata.model.as_str(),
                embedding_metadata.kind_key(),
                embedding_metadata.dim,
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Find existing vector doc: {}", e))?;

    if let Some(existing_id) = existing_id {
        return Ok(existing_id);
    }

    if !source_id.is_empty() {
        conn.execute(
            "DELETE FROM vector_document_metadata
             WHERE project_id = ?1 AND source_type = ?2 AND source_id = ?3",
            params![project_id, source_type, source_id],
        )
        .map_err(|e| format!("Delete stale vector docs: {}", e))?;
    }

    let blob = f32_to_blob(embedding);

    conn.execute(
        "INSERT INTO vector_document_metadata
         (id, project_id, source_type, source_id, title, content, content_hash, metadata,
          embedding, embedding_provider, embedding_model, embedding_kind, embedding_dim, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))",
        params![
            id,
            project_id,
            source_type,
            source_id,
            title,
            content,
            content_hash,
            metadata,
            blob,
            embedding_metadata.provider.as_str(),
            embedding_metadata.model.as_str(),
            embedding_metadata.kind_key(),
            embedding_metadata.dim,
        ],
    )
    .map_err(|e| format!("Insert vector doc: {}", e))?;

    Ok(id)
}

pub fn get_rag_health(
    db: &Database,
    project_id: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dim: i32,
) -> Result<RagHealth, String> {
    let provider = embedding_provider.trim();
    let model = embedding_model.trim();
    if provider.is_empty() || provider == "none" {
        return Ok(RagHealth {
            state: "disabled".into(),
            message: "RAG 向量检索未启用。".into(),
            document_count: 0,
            stale_count: 0,
            embedding_provider: provider.into(),
            embedding_model: model.into(),
            embedding_dim,
            last_indexed_at: None,
        });
    }

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (document_count, stale_count, last_indexed_at): (i64, Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT COUNT(*),
                    SUM(CASE
                        WHEN embedding_provider <> ?2
                          OR embedding_model <> ?3
                          OR embedding_kind <> 'document'
                          OR embedding_dim <> ?4
                        THEN 1 ELSE 0 END),
                    MAX(indexed_at)
             FROM vector_document_metadata
             WHERE project_id = ?1",
            params![project_id, provider, model, embedding_dim],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("Read RAG health: {}", e))?;
    let stale_count = stale_count.unwrap_or(0);

    let (state, message) = if document_count == 0 {
        (
            "empty",
            format!("RAG 已配置为 {provider}/{model}，但当前项目还没有向量索引。"),
        )
    } else if stale_count > 0 {
        (
            "stale",
            format!(
                "RAG 索引需要重建：{stale_count}/{document_count} 条向量与当前 embedding 配置不一致。"
            ),
        )
    } else {
        (
            "usable",
            format!("RAG 可用：{document_count} 条向量已匹配当前 embedding 配置。"),
        )
    };

    Ok(RagHealth {
        state: state.into(),
        message,
        document_count,
        stale_count,
        embedding_provider: provider.into(),
        embedding_model: model.into(),
        embedding_dim,
        last_indexed_at,
    })
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
        "SELECT id, project_id, source_type, source_id, title, content, content_hash, metadata, embedding, created_at, updated_at
         FROM vector_document_metadata WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare: {}", e))?;

    #[derive(Debug)]
    struct RawDoc {
        id: String,
        project_id: String,
        source_type: String,
        source_id: Option<String>,
        title: Option<String>,
        content: String,
        content_hash: String,
        metadata: String,
        embedding_blob: Option<Vec<u8>>,
        created_at: Option<String>,
        updated_at: Option<String>,
    }

    let docs: Vec<RawDoc> = stmt
        .query_map(params![project_id], |row| {
            Ok(RawDoc {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_type: row.get(2)?,
                source_id: row.get(3)?,
                title: row.get(4)?,
                content: row.get(5)?,
                content_hash: row.get(6)?,
                metadata: row.get(7)?,
                embedding_blob: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect: {}", e))?;

    let mut scored: Vec<VectorDocument> = docs
        .into_iter()
        .map(|d| {
            let sim = d
                .embedding_blob
                .as_deref()
                .map(|blob| {
                    let emb = blob_to_f32(blob);
                    cosine_similarity(query_embedding, &emb)
                })
                .unwrap_or(0.0);
            VectorDocument {
                id: d.id,
                project_id: d.project_id,
                source_type: d.source_type,
                source_id: d.source_id,
                title: d.title,
                content: d.content,
                content_hash: d.content_hash,
                metadata: d.metadata,
                created_at: d.created_at.unwrap_or_default(),
                updated_at: d.updated_at.unwrap_or_default(),
                embedding: None,
                similarity: Some(sim),
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    Ok(scored)
}

pub fn delete_vector_documents_by_source(db: &Database, source_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "DELETE FROM vector_document_metadata WHERE source_id = ?1",
        params![source_id],
    )
    .map_err(|e| format!("Delete vector docs: {}", e))?;
    Ok(())
}
