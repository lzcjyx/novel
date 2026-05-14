use rusqlite::params;
use crate::db::connection::Database;

pub fn get_edges(db: &Database, project_id: &str) -> Result<Vec<KnowledgeGraphEdge>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_node_id, source_node_type, target_node_id, target_node_type,
                edge_type, description, auto_inferred, confidence, metadata, created_at, updated_at
         FROM knowledge_graph_edges WHERE project_id = ?1"
    ).map_err(|e| format!("Prepare: {}", e))?;

    stmt.query_map(params![project_id], |row| {
        Ok(KnowledgeGraphEdge {
            id: row.get(0)?, project_id: row.get(1)?,
            source_node_id: row.get(2)?, source_node_type: row.get(3)?,
            target_node_id: row.get(4)?, target_node_type: row.get(5)?,
            edge_type: row.get(6)?, description: row.get(7)?,
            auto_inferred: row.get::<_, i32>(8)? != 0,
            confidence: row.get(9).unwrap_or(1.0),
            metadata: row.get(10)?, created_at: row.get(11)?, updated_at: row.get(12)?,
        })
    }).map_err(|e| format!("Query: {}", e))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| format!("Collect: {}", e))
}

pub fn insert_edge(db: &Database, project_id: &str, source_id: &str, source_type: &str,
    target_id: &str, target_type: &str, edge_type: &str, description: Option<&str>,
    auto_inferred: bool, confidence: f64,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR IGNORE INTO knowledge_graph_edges (id, project_id, source_node_id, source_node_type, target_node_id, target_node_type, edge_type, description, auto_inferred, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, project_id, source_id, source_type, target_id, target_type, edge_type, description, auto_inferred as i32, confidence],
    ).map_err(|e| format!("Insert edge: {}", e))?;
    Ok(id)
}

pub fn delete_edge(db: &Database, edge_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute("DELETE FROM knowledge_graph_edges WHERE id = ?1", params![edge_id])
        .map_err(|e| format!("Delete edge: {}", e))?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeGraphEdge {
    pub id: String,
    pub project_id: String,
    pub source_node_id: String,
    pub source_node_type: String,
    pub target_node_id: String,
    pub target_node_type: String,
    pub edge_type: String,
    pub description: Option<String>,
    pub auto_inferred: bool,
    pub confidence: f64,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}
