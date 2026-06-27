use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::db::connection::Database;
use tauri_app_lib::models::ChapterPlan;
use tauri_app_lib::workflow::writing_context::OperatorControls;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("rag-explainability.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "RAG Test",
        Some("deterministic retrieval"),
        Some("mystery"),
        None,
        Some("adult"),
        Some("restrained"),
        Some("cold"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn retrieval_trace_ranks_and_explains_sources() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let station_id = tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-2"),
        "Rain station clue",
        "Lin Bai returns to the old station and finds the missing watch hidden under the platform sign.",
        r#"{"chapter":2}"#,
        &[1.0, 0.0],
    )
    .unwrap();
    tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "style_guide",
        Some("style-a"),
        "Quiet prose",
        "Use short sentences and keep emotional reactions understated.",
        "{}",
        &[0.0, 1.0],
    )
    .unwrap();

    let docs =
        tauri_app_lib::db::vector_store::search_similar_documents(&db, &project_id, &[1.0, 0.0], 2)
            .unwrap();
    let trace = tauri_app_lib::db::vector_store::build_retrieval_trace(&docs);

    assert_eq!(trace.source_count, 2);
    assert_eq!(trace.sources[0].rank, 1);
    assert_eq!(trace.sources[0].document_id, station_id);
    assert_eq!(trace.sources[0].source_type, "chapter");
    assert_eq!(trace.sources[0].source_id.as_deref(), Some("chapter-2"));
    assert_eq!(trace.sources[0].relevance_label, "high");
    assert!(trace.sources[0].similarity.unwrap() > 0.99);
    assert!(trace.sources[0].excerpt.contains("old station"));
}

#[test]
fn vector_documents_persist_content_hashes() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let content = "Lin Bai pins the red umbrella receipt under the station clock.";
    let expected_hash = {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    };
    let doc_id = tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-hash"),
        "Hash fixture",
        content,
        "{}",
        &[1.0, 0.0],
    )
    .unwrap();

    let persisted_hash: String = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT content_hash FROM vector_document_metadata WHERE id = ?1",
            rusqlite::params![doc_id],
            |row| row.get(0),
        )
        .unwrap()
    };
    let docs =
        tauri_app_lib::db::vector_store::search_similar_documents(&db, &project_id, &[1.0, 0.0], 1)
            .unwrap();

    assert_eq!(persisted_hash, expected_hash);
    assert_eq!(docs[0].content_hash, expected_hash);
}

#[test]
fn duplicate_vector_document_content_reuses_existing_doc() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let content = "The station ledger repeats the same umbrella clue.";

    assert!(
        !tauri_app_lib::db::vector_store::source_content_hash_exists(
            &db,
            &project_id,
            "chapter",
            Some("chapter-dedupe"),
            content,
        )
        .unwrap()
    );

    let first_id = tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-dedupe"),
        "Dedupe fixture",
        content,
        r#"{"version":1}"#,
        &[1.0, 0.0],
    )
    .unwrap();
    let second_id = tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-dedupe"),
        "Dedupe fixture duplicate",
        content,
        r#"{"version":2}"#,
        &[0.0, 1.0],
    )
    .unwrap();

    let count: i64 = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM vector_document_metadata WHERE project_id = ?1 AND source_type = 'chapter' AND source_id = 'chapter-dedupe'",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap()
    };

    assert_eq!(second_id, first_id);
    assert_eq!(count, 1);
    assert!(tauri_app_lib::db::vector_store::source_content_hash_exists(
        &db,
        &project_id,
        "chapter",
        Some("chapter-dedupe"),
        content,
    )
    .unwrap());
}

#[test]
fn vector_index_candidates_skip_unchanged_content_hashes() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let unchanged_content = "The station ledger keeps the same umbrella clue.";
    tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-unchanged"),
        "Unchanged fixture",
        unchanged_content,
        "{}",
        &[1.0, 0.0],
    )
    .unwrap();
    tauri_app_lib::db::vector_store::insert_vector_document(
        &db,
        &project_id,
        "chapter",
        Some("chapter-changed"),
        "Changed fixture old",
        "The station ledger still points to the old platform clue.",
        "{}",
        &[0.5, 0.5],
    )
    .unwrap();

    let candidates = vec![
        tauri_app_lib::db::vector_store::VectorIndexCandidate::new(
            "chapter-unchanged",
            "chapter",
            "Unchanged fixture",
            unchanged_content,
            "{}",
        ),
        tauri_app_lib::db::vector_store::VectorIndexCandidate::new(
            "chapter-changed",
            "chapter",
            "Changed fixture",
            "The station ledger now points to a fresh rooftop clue.",
            "{}",
        ),
        tauri_app_lib::db::vector_store::VectorIndexCandidate::new(
            "chapter-new",
            "chapter",
            "New fixture",
            "A new witness marks the platform map.",
            "{}",
        ),
    ];

    let pending = tauri_app_lib::db::vector_store::filter_vector_index_candidates(
        &db,
        &project_id,
        candidates,
    )
    .unwrap();
    let pending_sources = pending
        .iter()
        .map(|candidate| candidate.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(pending_sources, vec!["chapter-changed", "chapter-new"]);
}

#[test]
fn bge_m3_embedding_inputs_are_prepared_asymmetricly() {
    let docs = vec!["旧车站的怀表线索".to_string()];
    let queries = vec!["本章需要找回怀表".to_string()];

    let document_inputs = tauri_app_lib::ai::deepseek::prepare_embedding_inputs(
        "BAAI/bge-m3",
        tauri_app_lib::ai::client::EmbeddingInputKind::Document,
        &docs,
    );
    let query_inputs = tauri_app_lib::ai::deepseek::prepare_embedding_inputs(
        "BAAI/bge-m3",
        tauri_app_lib::ai::client::EmbeddingInputKind::Query,
        &queries,
    );
    let ordinary_inputs = tauri_app_lib::ai::deepseek::prepare_embedding_inputs(
        "text-embedding-3-small",
        tauri_app_lib::ai::client::EmbeddingInputKind::Query,
        &queries,
    );

    assert_eq!(document_inputs, vec!["passage: 旧车站的怀表线索"]);
    assert_eq!(query_inputs, vec!["query: 本章需要找回怀表"]);
    assert_eq!(ordinary_inputs, queries);
}

#[derive(Default)]
struct CountingEmbeddingProvider {
    batches: Mutex<Vec<Vec<String>>>,
}

#[async_trait]
impl ModelClient for CountingEmbeddingProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(serde_json::json!({}))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        self.batches.lock().unwrap().push(texts.to_vec());
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[tokio::test]
async fn bible_indexing_skips_embedding_for_unchanged_vector_hashes() {
    let db = setup_db();
    let project_id = insert_project(&db);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO characters
             (id, project_id, name, personality, motivation, speech_style, appearance, backstory)
             VALUES ('char-index', ?1, 'Lin Bai', 'restrained', 'solve the old case', 'short lines', 'old coat', 'station case survivor')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    let provider = CountingEmbeddingProvider::default();
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(10);
    tauri_app_lib::workflow::novel_bootstrap::embed_and_index_bible(
        &db,
        &provider,
        &project_id,
        &log_tx,
    )
    .await;
    assert_eq!(provider.batches.lock().unwrap().len(), 1);
    assert_eq!(provider.batches.lock().unwrap()[0].len(), 1);

    tauri_app_lib::workflow::novel_bootstrap::embed_and_index_bible(
        &db,
        &provider,
        &project_id,
        &log_tx,
    )
    .await;
    assert_eq!(
        provider.batches.lock().unwrap().len(),
        1,
        "unchanged bible content should not trigger another embedding call"
    );

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE characters SET motivation = 'find the rooftop witness' WHERE id = 'char-index'",
            [],
        )
        .unwrap();
    }
    tauri_app_lib::workflow::novel_bootstrap::embed_and_index_bible(
        &db,
        &provider,
        &project_id,
        &log_tx,
    )
    .await;

    let batches = provider.batches.lock().unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[1].len(), 1);
    assert!(batches[1][0].contains("find the rooftop witness"));
    drop(batches);

    let (stored_count, stored_content): (i64, String) = {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*), MAX(content)
             FROM vector_document_metadata
             WHERE project_id = ?1 AND source_type = 'character' AND source_id = 'char-index'",
            rusqlite::params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    };
    assert_eq!(stored_count, 1);
    assert!(stored_content.contains("find the rooftop witness"));
}

#[test]
fn retrieval_trace_handles_empty_results() {
    let trace = tauri_app_lib::db::vector_store::build_retrieval_trace(&[]);

    assert_eq!(trace.source_count, 0);
    assert!(trace.sources.is_empty());
    assert!(trace.best_similarity.is_none());
}

#[test]
fn retrieval_query_includes_plan_and_operator_controls() {
    let plan = ChapterPlan {
        id: "plan-a".into(),
        project_id: "project-a".into(),
        volume_id: None,
        sequence: 3,
        title: Some("Old Station Return".into()),
        outline: Some("The detective confronts the hidden witness.".into()),
        pov_character_id: Some("lin-bai".into()),
        target_word_count: Some(3000),
        required_characters: "Lin Bai, witness".into(),
        required_locations: "old station".into(),
        plot_goals: "reveal the missing watch".into(),
        required_foreshadowing: "red umbrella".into(),
        status: "planned".into(),
        metadata: "{}".into(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let controls = OperatorControls {
        generation_mode: Some("controlled".into()),
        chapter_intent: Some("make the clue feel costly".into()),
        must_include_beats: Some("platform sign".into()),
        forbidden_moves: Some("no confession".into()),
        style_emphasis: Some("quiet dread".into()),
    };

    let query =
        tauri_app_lib::workflow::writing_context::build_retrieval_query(&plan, Some(&controls));

    assert!(query.contains("Old Station Return"));
    assert!(query.contains("old station"));
    assert!(query.contains("missing watch"));
    assert!(query.contains("platform sign"));
    assert!(query.contains("quiet dread"));
    assert!(!query.contains("  "));
}
