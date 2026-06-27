# Integrated Runtime Control Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the integrated Runtime control console, BGE-M3-safe RAG path, independent desktop pet, and consistency tests described in `docs/superpowers/specs/2026-06-28-integrated-runtime-control-design.md`.

**Architecture:** Keep the existing Tauri v2 + React 19 + Rust/rusqlite architecture. Add explicit embedding input kinds and RAG health in Rust, emit shared Runtime status to a separate pet webview, and reshape `RuntimePage.tsx` into workflow phases without removing existing advanced capabilities.

**Tech Stack:** Rust 2021, async-trait, rusqlite, Tauri v2, React 19, TypeScript, Vite, Node test runner.

---

## File Structure

- Modify `tauri-app/src-tauri/src/ai/client.rs`: add `EmbeddingInputKind` and `embed_with_kind`.
- Modify `tauri-app/src-tauri/src/ai/deepseek.rs`: centralize OpenAI-compatible embedding input preparation and BGE-M3 `passage:` / `query:` handling.
- Modify `tauri-app/src-tauri/src/ai/openai.rs` and `tauri-app/src-tauri/src/ai/openai_compat.rs`: delegate embedding kind to the shared DeepSeek/OpenAI-compatible implementation.
- Modify `tauri-app/src-tauri/src/ai/gemini.rs` and `tauri-app/src-tauri/src/ai/anthropic.rs`: satisfy the expanded trait without changing provider semantics.
- Modify `tauri-app/src-tauri/src/db/vector_store.rs`: add vector freshness metadata, RAG health structs, and richer retrieval traces.
- Modify `tauri-app/src-tauri/src/db/migrations.rs` and `tauri-app/src-tauri/migrations/001_init_sqlite.sql`: add vector freshness columns and backfill defaults.
- Modify `tauri-app/src-tauri/src/workflow/novel_bootstrap.rs`, `tauri-app/src-tauri/src/workflow/chapter_production.rs`, `tauri-app/src-tauri/src/commands/runtime.rs`, and `tauri-app/src-tauri/src/lib.rs`: use document/query embedding kinds at all RAG call sites.
- Modify `tauri-app/src-tauri/src/models/settings.rs` and `tauri-app/src-tauri/src/db/settings.rs`: persist pet position and visibility settings.
- Modify `tauri-app/src-tauri/tauri.conf.json` and `tauri-app/src-tauri/capabilities/default.json`: add the `pet` window and window permissions.
- Modify `tauri-app/src-tauri/src/lib.rs`: add pet commands and register them in `invoke_handler`.
- Modify `tauri-app/src/lib/tauriClient.ts`: add typed wrappers for RAG health and pet commands.
- Create `tauri-app/src/pages/PetWindow.tsx`: standalone pet window UI.
- Modify `tauri-app/src/App.tsx`: route `?window=pet` to `PetWindow`, emit pet status, and remove the shell-embedded `AppPet`.
- Modify `tauri-app/src/pages/RuntimePage.tsx`: replace the flat card grid with workflow phases.
- Modify `tauri-app/src/index.css`: add Runtime workflow layout and pet window styles.
- Modify tests:
  - `tauri-app/src-tauri/tests/rag_explainability_tests.rs`
  - `tauri-app/src-tauri/tests/provider_capability_tests.rs`
  - `tauri-app/src-tauri/tests/db_tests.rs`
  - `tauri-app/src/runtimePage.contract.test.mjs`
  - `tauri-app/src/fluentTokens.test.mjs`
  - `tauri-app/src/lib/tauriClient.contract.test.mjs`

## Tasks

### Task 1: Embedding Input Kinds And BGE-M3 Preparation

**Files:**
- Modify: `tauri-app/src-tauri/src/ai/client.rs`
- Modify: `tauri-app/src-tauri/src/ai/deepseek.rs`
- Modify: `tauri-app/src-tauri/src/ai/openai.rs`
- Modify: `tauri-app/src-tauri/src/ai/openai_compat.rs`
- Modify: `tauri-app/src-tauri/src/ai/gemini.rs`
- Modify: `tauri-app/src-tauri/src/ai/anthropic.rs`
- Test: `tauri-app/src-tauri/tests/rag_explainability_tests.rs`

- [ ] **Step 1: Add failing BGE-M3 preparation test**

Append this test to `tauri-app/src-tauri/tests/rag_explainability_tests.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml bge_m3_embedding_inputs_are_prepared_asymmetricly -- --nocapture
```

Expected: FAIL because `EmbeddingInputKind` and `prepare_embedding_inputs` are not defined.

- [ ] **Step 3: Add embedding kind API**

In `tauri-app/src-tauri/src/ai/client.rs`, add the enum above `ModelClient` and add the default method inside the trait:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingInputKind {
    Document,
    Query,
}
```

```rust
    async fn embed_with_kind(
        &self,
        texts: &[String],
        _kind: EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        self.embed(texts).await
    }
```

- [ ] **Step 4: Add BGE-M3 input preparation**

In `tauri-app/src-tauri/src/ai/deepseek.rs`, update imports and add these functions near `impl DeepSeekProvider`:

```rust
use crate::ai::client::{EmbeddingInputKind, ModelClient, ModelUsageReport};
```

```rust
fn is_bge_m3_model(model: &str) -> bool {
    let normalized = model.to_ascii_lowercase();
    normalized == "bge-m3" || normalized.ends_with("/bge-m3")
}

pub fn prepare_embedding_inputs(
    model: &str,
    kind: EmbeddingInputKind,
    texts: &[String],
) -> Vec<String> {
    if !is_bge_m3_model(model) {
        return texts.to_vec();
    }

    let prefix = match kind {
        EmbeddingInputKind::Document => "passage: ",
        EmbeddingInputKind::Query => "query: ",
    };
    texts
        .iter()
        .map(|text| format!("{}{}", prefix, text.trim()))
        .collect()
}
```

- [ ] **Step 5: Route provider embeddings through kind-aware preparation**

In `DeepSeekProvider`'s `ModelClient` impl, replace the current `embed` body with a helper method and kind-aware method:

```rust
    async fn embed_with_kind(
        &self,
        texts: &[String],
        kind: EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("Client: {}", e))?;

        let emb_url = if self.base_url.ends_with("/v1") || self.base_url.ends_with("/v1/") {
            format!("{}/embeddings", self.base_url.trim_end_matches('/'))
        } else {
            format!("{}/v1/embeddings", self.base_url.trim_end_matches('/'))
        };
        let prepared_inputs = prepare_embedding_inputs(&self.embedding_model, kind, texts);
        let resp = client
            .post(&emb_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({"model": self.embedding_model, "input": prepared_inputs}))
            .send()
            .await
            .map_err(|e| format!("HTTP: {}", e))?
            .json::<EmbeddingResponse>()
            .await
            .map_err(|e| format!("Parse: {}", e))?;

        Ok(resp.data.into_iter().map(|d| d.embedding).collect())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        self.embed_with_kind(texts, EmbeddingInputKind::Document).await
    }
```

- [ ] **Step 6: Delegate kind through OpenAI and OpenAI-compatible providers**

In `tauri-app/src-tauri/src/ai/openai.rs` and `tauri-app/src-tauri/src/ai/openai_compat.rs`, import `EmbeddingInputKind` and add this method to each `ModelClient` impl:

```rust
    async fn embed_with_kind(
        &self,
        texts: &[String],
        kind: EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        let deepseek = DeepSeekProvider {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            embedding_model: self.embedding_model.clone(),
            timeout_secs: self.timeout_secs,
        };
        deepseek.embed_with_kind(texts, kind).await
    }
```

Keep each existing `embed` method and make it call `embed_with_kind(texts, EmbeddingInputKind::Document)`.

- [ ] **Step 7: Run targeted test**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml bge_m3_embedding_inputs_are_prepared_asymmetricly -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add tauri-app/src-tauri/src/ai/client.rs tauri-app/src-tauri/src/ai/deepseek.rs tauri-app/src-tauri/src/ai/openai.rs tauri-app/src-tauri/src/ai/openai_compat.rs tauri-app/src-tauri/src/ai/gemini.rs tauri-app/src-tauri/src/ai/anthropic.rs tauri-app/src-tauri/tests/rag_explainability_tests.rs
git commit -m "feat: distinguish query and document embeddings"
```

### Task 2: Vector Freshness Metadata And RAG Health

**Files:**
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Modify: `tauri-app/src-tauri/src/db/migrations.rs`
- Modify: `tauri-app/src-tauri/src/db/vector_store.rs`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Test: `tauri-app/src-tauri/tests/rag_explainability_tests.rs`
- Test: `tauri-app/src-tauri/tests/db_tests.rs`
- Test: `tauri-app/src/lib/tauriClient.contract.test.mjs`

- [ ] **Step 1: Add failing vector metadata migration test**

Append to `tauri-app/src-tauri/tests/db_tests.rs`:

```rust
#[test]
fn vector_metadata_schema_tracks_embedding_freshness() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("vector-freshness.db");
    let db = tauri_app_lib::db::connection::Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();

    let columns = {
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("PRAGMA table_info(vector_document_metadata)")
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };

    for column in [
        "embedding_provider",
        "embedding_model",
        "embedding_kind",
        "embedding_dim",
        "indexed_at",
    ] {
        assert!(columns.contains(&column.to_string()), "missing {column}");
    }
}
```

- [ ] **Step 2: Add failing tauri client contract test**

Append to `tauri-app/src/lib/tauriClient.contract.test.mjs`:

```javascript
test("tauri client exposes RAG health command", () => {
  assert.match(source, /getRagHealth<T>\(projectId: string\)/);
  assert.match(source, /invoke<T>\("get_rag_health"/);
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml vector_metadata_schema_tracks_embedding_freshness -- --nocapture
node --test tauri-app/src/lib/tauriClient.contract.test.mjs
```

Expected: FAIL because freshness columns and client wrapper are missing.

- [ ] **Step 4: Add schema columns**

In `tauri-app/src-tauri/migrations/001_init_sqlite.sql`, extend `vector_document_metadata`:

```sql
    embedding_provider TEXT NOT NULL DEFAULT '',
    embedding_model TEXT NOT NULL DEFAULT '',
    embedding_kind TEXT NOT NULL DEFAULT 'document',
    embedding_dim INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT,
```

Place these after `embedding BLOB,`.

In `tauri-app/src-tauri/src/db/migrations.rs`, add a helper and call it after the content hash migration:

```rust
fn ensure_vector_column(
    conn: &rusqlite::Connection,
    existing_columns: &[String],
    column: &str,
    ddl: &str,
) -> Result<(), String> {
    if !existing_columns.iter().any(|existing| existing == column) {
        conn.execute(ddl, [])
            .map_err(|e| format!("Add vector column {column}: {e}"))?;
    }
    Ok(())
}
```

Then add:

```rust
    let vector_columns = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(vector_document_metadata)")
            .map_err(|e| format!("Prepare vector schema refresh: {}", e))?;
        stmt.query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("Read vector schema refresh: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect vector schema refresh: {}", e))?
    };
    ensure_vector_column(
        &conn,
        &vector_columns,
        "embedding_provider",
        "ALTER TABLE vector_document_metadata ADD COLUMN embedding_provider TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_vector_column(
        &conn,
        &vector_columns,
        "embedding_model",
        "ALTER TABLE vector_document_metadata ADD COLUMN embedding_model TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_vector_column(
        &conn,
        &vector_columns,
        "embedding_kind",
        "ALTER TABLE vector_document_metadata ADD COLUMN embedding_kind TEXT NOT NULL DEFAULT 'document'",
    )?;
    ensure_vector_column(
        &conn,
        &vector_columns,
        "embedding_dim",
        "ALTER TABLE vector_document_metadata ADD COLUMN embedding_dim INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_vector_column(
        &conn,
        &vector_columns,
        "indexed_at",
        "ALTER TABLE vector_document_metadata ADD COLUMN indexed_at TEXT",
    )?;
```

- [ ] **Step 5: Add metadata structs and insert path**

In `tauri-app/src-tauri/src/db/vector_store.rs`, add:

```rust
#[derive(Debug, Clone)]
pub struct VectorEmbeddingMetadata {
    pub provider: String,
    pub model: String,
    pub kind: crate::ai::client::EmbeddingInputKind,
    pub dim: i32,
}

impl VectorEmbeddingMetadata {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        kind: crate::ai::client::EmbeddingInputKind,
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
            crate::ai::client::EmbeddingInputKind::Document => "document",
            crate::ai::client::EmbeddingInputKind::Query => "query",
        }
    }
}
```

Add `insert_vector_document_with_embedding_metadata`:

```rust
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
             WHERE project_id = ?1 AND source_type = ?2 AND source_id = ?3
               AND content_hash = ?4 AND embedding_provider = ?5
               AND embedding_model = ?6 AND embedding_kind = ?7
               AND embedding_dim = ?8
             ORDER BY created_at ASC
             LIMIT 1",
            params![
                project_id,
                source_type,
                source_id,
                content_hash,
                embedding_metadata.provider,
                embedding_metadata.model,
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
            embedding_metadata.provider,
            embedding_metadata.model,
            embedding_metadata.kind_key(),
            embedding_metadata.dim,
        ],
    )
    .map_err(|e| format!("Insert vector doc: {}", e))?;

    Ok(id)
}
```

Keep the existing `insert_vector_document` and make it call the new function with provider `"legacy"`, model `"unknown"`, kind `Document`, and the embedding length.

- [ ] **Step 6: Add RAG health structs and command wrapper**

In `vector_store.rs`, add:

```rust
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

pub fn get_rag_health(
    db: &Database,
    project_id: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dim: i32,
) -> Result<RagHealth, String> {
    if embedding_provider.trim().is_empty() || embedding_provider == "none" {
        return Ok(RagHealth {
            state: "disabled".into(),
            message: "RAG 向量检索未启用。".into(),
            document_count: 0,
            stale_count: 0,
            embedding_provider: embedding_provider.into(),
            embedding_model: embedding_model.into(),
            embedding_dim,
            last_indexed_at: None,
        });
    }

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let (document_count, stale_count, last_indexed_at): (i64, i64, Option<String>) = conn
        .query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN embedding_provider <> ?2 OR embedding_model <> ?3 OR embedding_dim <> ?4 THEN 1 ELSE 0 END),
                    MAX(indexed_at)
             FROM vector_document_metadata
             WHERE project_id = ?1",
            params![project_id, embedding_provider, embedding_model, embedding_dim],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0), row.get(2)?)),
        )
        .map_err(|e| format!("Read RAG health: {}", e))?;

    let (state, message) = if document_count == 0 {
        ("empty", "RAG 已配置，但当前项目还没有向量索引。")
    } else if stale_count > 0 {
        ("stale", "RAG 索引与当前 Embedding 配置不一致，需要重建。")
    } else {
        ("usable", "RAG 向量检索可用。")
    };

    Ok(RagHealth {
        state: state.into(),
        message: message.into(),
        document_count,
        stale_count,
        embedding_provider: embedding_provider.into(),
        embedding_model: embedding_model.into(),
        embedding_dim,
        last_indexed_at,
    })
}
```

In `tauri-app/src-tauri/src/commands/runtime.rs`, add:

```rust
#[tauri::command]
pub async fn get_rag_health(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<db::vector_store::RagHealth, String> {
    let settings = db::settings::get_settings(&state.db)?;
    db::vector_store::get_rag_health(
        &state.db,
        &project_id,
        &settings.embedding_provider,
        &settings.embedding_model,
        settings.embedding_dim,
    )
}
```

Register `commands::runtime::get_rag_health` in `tauri-app/src-tauri/src/lib.rs`.

In `tauri-app/src/lib/tauriClient.ts`, add:

```ts
  getRagHealth<T>(projectId: string): Promise<T> {
    return invoke<T>("get_rag_health", { projectId });
  },
```

- [ ] **Step 7: Run targeted tests**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml vector_metadata_schema_tracks_embedding_freshness -- --nocapture
node --test tauri-app/src/lib/tauriClient.contract.test.mjs
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add tauri-app/src-tauri/migrations/001_init_sqlite.sql tauri-app/src-tauri/src/db/migrations.rs tauri-app/src-tauri/src/db/vector_store.rs tauri-app/src-tauri/src/commands/runtime.rs tauri-app/src-tauri/src/lib.rs tauri-app/src/lib/tauriClient.ts tauri-app/src-tauri/tests/rag_explainability_tests.rs tauri-app/src-tauri/tests/db_tests.rs tauri-app/src/lib/tauriClient.contract.test.mjs
git commit -m "feat: report rag health and vector freshness"
```

### Task 3: Move Production RAG Call Sites To Document And Query Embeddings

**Files:**
- Modify: `tauri-app/src-tauri/src/workflow/novel_bootstrap.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src-tauri/tests/rag_explainability_tests.rs`

- [ ] **Step 1: Add failing fake-provider kind test**

Update `CountingEmbeddingProvider` in `rag_explainability_tests.rs`:

```rust
#[derive(Default)]
struct CountingEmbeddingProvider {
    batches: Mutex<Vec<Vec<String>>>,
    kinds: Mutex<Vec<tauri_app_lib::ai::client::EmbeddingInputKind>>,
}
```

Add this method to its `ModelClient` impl:

```rust
    async fn embed_with_kind(
        &self,
        texts: &[String],
        kind: tauri_app_lib::ai::client::EmbeddingInputKind,
    ) -> Result<Vec<Vec<f32>>, String> {
        self.kinds.lock().unwrap().push(kind);
        self.embed(texts).await
    }
```

Append this assertion to `bible_indexing_skips_embedding_for_unchanged_vector_hashes` after the first indexing call:

```rust
    assert_eq!(
        provider.kinds.lock().unwrap()[0],
        tauri_app_lib::ai::client::EmbeddingInputKind::Document
    );
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml bible_indexing_skips_embedding_for_unchanged_vector_hashes -- --nocapture
```

Expected: FAIL because indexing still calls `embed` rather than `embed_with_kind(Document)`.

- [ ] **Step 3: Update document indexing call sites**

In `novel_bootstrap.rs`, replace:

```rust
    match provider.embed(&text_contents).await {
```

with:

```rust
    match provider
        .embed_with_kind(
            &text_contents,
            crate::ai::client::EmbeddingInputKind::Document,
        )
        .await
    {
```

When inserting vectors, call `insert_vector_document_with_embedding_metadata`:

```rust
                    let embedding_metadata =
                        crate::db::vector_store::VectorEmbeddingMetadata::new(
                            "workflow",
                            "configured",
                            crate::ai::client::EmbeddingInputKind::Document,
                            &embeddings[i],
                        );
                    let _ = crate::db::vector_store::insert_vector_document_with_embedding_metadata(
                        db,
                        project_id,
                        &candidate.source_type,
                        Some(&candidate.source_id),
                        &candidate.title,
                        &candidate.content,
                        &candidate.metadata,
                        &embeddings[i],
                        &embedding_metadata,
                    );
```

In `lib.rs` `rebuild_vector_index`, replace `embed.embed(&contents)` with:

```rust
    let embeddings = embed
        .embed_with_kind(&contents, crate::ai::client::EmbeddingInputKind::Document)
        .await
        .map_err(|e| format!("Embed: {}", e))?;
```

- [ ] **Step 4: Update query retrieval call sites**

In `commands/runtime.rs`, replace:

```rust
            if let Ok(embeddings) = embed_client.embed(&[retrieval_query]).await {
```

with:

```rust
            if let Ok(embeddings) = embed_client
                .embed_with_kind(
                    &[retrieval_query],
                    ai::client::EmbeddingInputKind::Query,
                )
                .await
            {
```

In `chapter_production.rs`, replace:

```rust
        match embed_client.embed(&[retrieval_query.clone()]).await {
```

with:

```rust
        match embed_client
            .embed_with_kind(
                &[retrieval_query.clone()],
                crate::ai::client::EmbeddingInputKind::Query,
            )
            .await
        {
```

- [ ] **Step 5: Run targeted tests**

Run:

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml bible_indexing_skips_embedding_for_unchanged_vector_hashes -- --nocapture
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml retrieval_query_includes_plan_and_operator_controls -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add tauri-app/src-tauri/src/workflow/novel_bootstrap.rs tauri-app/src-tauri/src/workflow/chapter_production.rs tauri-app/src-tauri/src/commands/runtime.rs tauri-app/src-tauri/src/lib.rs tauri-app/src-tauri/tests/rag_explainability_tests.rs
git commit -m "feat: use asymmetric embeddings in rag workflows"
```

### Task 4: Independent Desktop Pet Window

**Files:**
- Modify: `tauri-app/src-tauri/tauri.conf.json`
- Modify: `tauri-app/src-tauri/capabilities/default.json`
- Modify: `tauri-app/src-tauri/src/models/settings.rs`
- Modify: `tauri-app/src-tauri/src/db/settings.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Create: `tauri-app/src/pages/PetWindow.tsx`
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`
- Test: `tauri-app/src/fluentTokens.test.mjs`
- Test: `tauri-app/src/lib/tauriClient.contract.test.mjs`
- Test: `tauri-app/src-tauri/tests/db_tests.rs`

- [ ] **Step 1: Add failing frontend pet window tests**

In `tauri-app/src/fluentTokens.test.mjs`, add:

```javascript
test("desktop pet runs in an independent Tauri window", () => {
  const config = JSON.parse(tauriConfig);
  const petWindow = config.app.windows.find((window) => window.label === "pet");
  assert.ok(petWindow, "missing pet window");
  assert.equal(petWindow.decorations, false);
  assert.equal(petWindow.transparent, true);
  assert.equal(petWindow.alwaysOnTop, true);
  assert.equal(petWindow.skipTaskbar, true);
  assert.match(app, /PetWindow/);
  assert.match(app, /window=pet/);
  assert.match(app, /emitTo\("pet",\s*"pet-status"/);
  assert.doesNotMatch(app, /<AppPet /);
});
```

In `tauri-app/src/lib/tauriClient.contract.test.mjs`, add:

```javascript
test("tauri client exposes pet window commands", () => {
  assert.match(source, /showPetWindow\(\): Promise<void>/);
  assert.match(source, /hidePetWindow\(\): Promise<void>/);
  assert.match(source, /showMainWindow\(\): Promise<void>/);
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
node --test tauri-app/src/fluentTokens.test.mjs tauri-app/src/lib/tauriClient.contract.test.mjs
```

Expected: FAIL because the pet window, route, events, and wrappers do not exist.

- [ ] **Step 3: Add pet settings persistence**

In `AppSettings`, add:

```rust
    pub pet_window_visible: bool,
    pub pet_position_x: Option<i32>,
    pub pet_position_y: Option<i32>,
```

In `Default`, add:

```rust
            pet_window_visible: true,
            pet_position_x: None,
            pet_position_y: None,
```

In `db/settings.rs`, load and save keys:

```rust
            "pet_window_visible" => {
                settings.pet_window_visible = v != "false";
            }
            "pet_position_x" => {
                settings.pet_position_x = parse_optional_i32(v);
            }
            "pet_position_y" => {
                settings.pet_position_y = parse_optional_i32(v);
            }
```

Add helper:

```rust
fn parse_optional_i32(value: &str) -> Option<i32> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "null" {
        None
    } else {
        trimmed.parse::<i32>().ok()
    }
}
```

Save:

```rust
    save_setting_optional(db, "pet_position_x", settings.pet_position_x.map(|value| value.to_string()).as_deref())?;
    save_setting_optional(db, "pet_position_y", settings.pet_position_y.map(|value| value.to_string()).as_deref())?;
    save_setting(
        db,
        "pet_window_visible",
        if settings.pet_window_visible { "true" } else { "false" },
    )?;
```

- [ ] **Step 4: Add pet Tauri window config**

In `tauri-app/src-tauri/tauri.conf.json`, add a second window:

```json
{
  "label": "pet",
  "title": "AI Novel Factory Pet",
  "url": "index.html?window=pet",
  "width": 220,
  "height": 132,
  "minWidth": 120,
  "minHeight": 96,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": true
}
```

In `capabilities/default.json`, set:

```json
"windows": ["main", "pet"]
```

and add permissions:

```json
"core:window:allow-show",
"core:window:allow-set-focus",
"core:window:allow-set-position",
"core:window:allow-outer-position"
```

- [ ] **Step 5: Add pet commands**

In `lib.rs`, add:

```rust
#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| format!("Show main window: {}", e))?;
        window.unminimize().map_err(|e| format!("Unminimize main window: {}", e))?;
        window.set_focus().map_err(|e| format!("Focus main window: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn show_pet_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pet") {
        window.show().map_err(|e| format!("Show pet window: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn hide_pet_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pet") {
        window.hide().map_err(|e| format!("Hide pet window: {}", e))?;
    }
    Ok(())
}
```

If the existing private helper is named `show_main_window`, rename it to `show_main_window_impl` and have the command call that helper. Register the three commands in `invoke_handler`.

In `tauriClient.ts`, add:

```ts
  showMainWindow(): Promise<void> {
    return invoke<void>("show_main_window");
  },

  showPetWindow(): Promise<void> {
    return invoke<void>("show_pet_window");
  },

  hidePetWindow(): Promise<void> {
    return invoke<void>("hide_pet_window");
  },
```

- [ ] **Step 6: Create PetWindow frontend**

Create `tauri-app/src/pages/PetWindow.tsx`:

```tsx
import { useEffect, useMemo, useState, type PointerEvent as ReactPointerEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauriClient } from "../lib/tauriClient";

interface PetStatusPayload {
  selected: string;
  projectName?: string;
  loading: boolean;
  running: boolean;
  message: string;
  ragState?: string;
  animationLevel: string;
  compact: boolean;
}

const initialStatus: PetStatusPayload = {
  selected: "",
  loading: false,
  running: false,
  message: "",
  ragState: "unknown",
  animationLevel: "subtle",
  compact: false,
};

export function PetWindow() {
  const [status, setStatus] = useState(initialStatus);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<PetStatusPayload>("pet-status", (event) => setStatus(event.payload))
      .then((handler) => { unlisten = handler; });
    return () => { if (unlisten) unlisten(); };
  }, []);

  const state = useMemo(() => {
    if (!status.selected) return "waiting";
    if (status.message.includes("错误") || status.message.toLowerCase().includes("error")) return "attention";
    if (status.loading || status.running) return "working";
    if (status.ragState && status.ragState !== "usable") return "context";
    return "idle";
  }, [status]);

  const copy = {
    waiting: ["等待项目", "选一个项目开始写作"],
    attention: ["需要查看", status.message || "刚刚有错误或提示"],
    working: ["正在工作", "生成流程运行中"],
    context: ["上下文受限", `RAG 状态：${status.ragState || "unknown"}`],
    idle: ["待命", status.projectName || "可以继续推进章节"],
  }[state] as [string, string];

  const handleDrag = async (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    await getCurrentWindow().startDragging();
  };

  return (
    <main
      className={`pet-window pet-window-${state} pet-window-${status.animationLevel || "subtle"} ${status.compact ? "pet-window-compact" : ""}`}
      onPointerDown={handleDrag}
      onDoubleClick={() => tauriClient.showMainWindow()}
    >
      <button className="pet-face" type="button" onClick={() => setExpanded((value) => !value)}>
        <span className="pet-eye pet-eye-left" />
        <span className="pet-eye pet-eye-right" />
        <span className="pet-mouth" />
      </button>
      {expanded && (
        <section className="pet-bubble">
          <strong>{copy[0]}</strong>
          <span>{copy[1]}</span>
          <span>{status.projectName || "未选择项目"}</span>
        </section>
      )}
    </main>
  );
}
```

- [ ] **Step 7: Route App to PetWindow and emit status**

In `App.tsx`, import:

```tsx
import { emitTo } from "@tauri-apps/api/event";
import { PetWindow } from "./pages/PetWindow";
```

At the start of `App()`:

```tsx
  const isPetWindow = new URLSearchParams(window.location.search).get("window") === "pet";
  if (isPetWindow) return <PetWindow />;
```

After `selectedProject` is computed:

```tsx
  useEffect(() => {
    emitTo("pet", "pet-status", {
      selected,
      projectName: selectedProject?.name,
      loading,
      running: Boolean(status?.is_running),
      message: msg,
      ragState: settings?.embedding_provider && settings.embedding_provider !== "none" ? "usable" : "disabled",
      animationLevel: settings?.pet_animation_level || "subtle",
      compact: Boolean(settings?.pet_compact_mode),
    });
  }, [selected, selectedProject?.name, loading, status?.is_running, msg, settings?.embedding_provider, settings?.pet_animation_level, settings?.pet_compact_mode]);
```

Remove the shell-level `<AppPet ... />` render and remove the `AppPet` function.

- [ ] **Step 8: Add CSS**

In `index.css`, add:

```css
.pet-window {
  width: 100vw;
  height: 100vh;
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  padding: 10px;
  color: var(--text-primary);
  background: transparent;
  user-select: none;
}

.pet-face {
  width: 64px;
  height: 64px;
  position: relative;
  border: 1px solid var(--control-stroke-strong);
  border-radius: 18px 18px 16px 16px;
  background: rgba(255, 255, 255, 0.94);
  box-shadow: 0 10px 26px rgba(0, 0, 0, 0.18);
  cursor: grab;
}

.pet-face:active {
  cursor: grabbing;
}

.pet-eye {
  position: absolute;
  top: 24px;
  width: 6px;
  height: 10px;
  border-radius: 999px;
  background: var(--text-primary);
}

.pet-eye-left { left: 21px; }
.pet-eye-right { right: 21px; }

.pet-mouth {
  position: absolute;
  left: 26px;
  bottom: 17px;
  width: 12px;
  height: 6px;
  border-bottom: 2px solid var(--text-secondary);
  border-radius: 0 0 999px 999px;
}

.pet-bubble {
  min-width: 116px;
  max-width: 132px;
  padding: 8px 10px;
  border: 1px solid var(--control-stroke);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.96);
  box-shadow: 0 10px 26px rgba(0, 0, 0, 0.14);
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.pet-bubble strong {
  font-size: 12px;
  line-height: 1.2;
}

.pet-bubble span {
  font-size: 10px;
  line-height: 1.35;
  color: var(--text-secondary);
}

.pet-window-subtle.pet-window-working .pet-face,
.pet-window-lively .pet-face {
  animation: petBreathe 2.8s ease-in-out infinite;
}

.pet-window-static .pet-face,
.pet-window-static .pet-eye {
  animation: none !important;
}
```

- [ ] **Step 9: Run targeted tests**

Run:

```bash
node --test tauri-app/src/fluentTokens.test.mjs tauri-app/src/lib/tauriClient.contract.test.mjs
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml settings_round_trip_pet_preferences -- --nocapture
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```bash
git add tauri-app/src-tauri/tauri.conf.json tauri-app/src-tauri/capabilities/default.json tauri-app/src-tauri/src/models/settings.rs tauri-app/src-tauri/src/db/settings.rs tauri-app/src-tauri/src/lib.rs tauri-app/src/lib/tauriClient.ts tauri-app/src/pages/PetWindow.tsx tauri-app/src/App.tsx tauri-app/src/index.css tauri-app/src/fluentTokens.test.mjs tauri-app/src/lib/tauriClient.contract.test.mjs tauri-app/src-tauri/tests/db_tests.rs
git commit -m "feat: move pet into independent desktop window"
```

### Task 5: Runtime Workflow Control Console

**Files:**
- Modify: `tauri-app/src/pages/RuntimePage.tsx`
- Modify: `tauri-app/src/index.css`
- Modify: `tauri-app/src/runtimePage.contract.test.mjs`

- [ ] **Step 1: Add failing Runtime workflow contract test**

Append to `runtimePage.contract.test.mjs`:

```javascript
test("runtime page is organized as a five phase workflow console", () => {
  for (const phase of ["准备", "上下文", "生成", "审阅", "交付"]) {
    assert.ok(runtimeSource.includes(phase), `missing workflow phase ${phase}`);
  }
  assert.match(runtimeSource, /runtimePhases/);
  assert.match(runtimeSource, /activePhase/);
  assert.match(runtimeSource, /className="runtime-console"/);
  assert.match(runtimeSource, /className="runtime-taskbar"/);
  assert.match(runtimeSource, /getRagHealth/);
  assert.match(runtimeSource, /workflowBindings/);
  assert.doesNotMatch(runtimeSource, /className="status-grid" style=\{\{ gridTemplateColumns: "repeat\(auto-fit, minmax\(320px, 1fr\)\)"/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
node --test tauri-app/src/runtimePage.contract.test.mjs
```

Expected: FAIL because Runtime still uses the old flat `status-grid`.

- [ ] **Step 3: Add Runtime phase state and RAG health loading**

In `RuntimePage.tsx`, add interfaces:

```tsx
interface RagHealth { state: string; message: string; document_count: number; stale_count: number; embedding_provider: string; embedding_model: string; embedding_dim: number; last_indexed_at?: string | null; }
type RuntimePhaseId = "prepare" | "context" | "generate" | "review" | "deliver";
```

Add state:

```tsx
  const [activePhase, setActivePhase] = useState<RuntimePhaseId>("prepare");
  const [ragHealth, setRagHealth] = useState<RagHealth | null>(null);
```

Add phase metadata:

```tsx
  const runtimePhases: { id: RuntimePhaseId; label: string; detail: string }[] = [
    { id: "prepare", label: "准备", detail: "章节、模型、RAG" },
    { id: "context", label: "上下文", detail: "规则、Lorebook、Trace" },
    { id: "generate", label: "生成", detail: "配方、提示词、候选" },
    { id: "review", label: "审阅", detail: "质量、反馈、修订" },
    { id: "deliver", label: "交付", detail: "包、扩展、导出" },
  ];
```

Add loader:

```tsx
  const loadRagHealth = useCallback(async () => {
    if (!selected) { setRagHealth(null); return; }
    try {
      setRagHealth(await tauriClient.getRagHealth<RagHealth>(selected));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, [selected]);
```

Call it in the existing `useEffect` that loads plans and context rules:

```tsx
  useEffect(() => { loadPlans(); loadContextRules(); loadRagHealth(); }, [loadPlans, loadContextRules, loadRagHealth]);
```

- [ ] **Step 4: Add workflow binding helper**

Add:

```tsx
  const workflowBindings = [
    { id: "draft", label: "Draft", value: settings?.draft_model_profile_id },
    { id: "review", label: "Review", value: settings?.review_model_profile_id },
    { id: "repair", label: "Repair", value: settings?.repair_model_profile_id },
    { id: "embedding", label: "Embedding", value: settings?.embedding_model_profile_id },
    { id: "summarization", label: "Summarization", value: settings?.summarization_model_profile_id },
  ];
```

- [ ] **Step 5: Extract phase render functions**

Before the `return` in `RuntimePage.tsx`, create these render functions and move the exact existing card bodies into them. The function bodies are not new UI; they are the existing JSX blocks relocated from the old flat grid:

- `renderContextPhase`: the existing "上下文规则" card, including manual context rule creation, Lorebook import, and context rule list.
- `renderGeneratePhase`: the existing "操作配方", "草稿候选", and "提示词预设" cards.
- `renderDeliverPhase`: the existing "项目包", "小说圣经包", and "扩展 Manifest" cards.

Keep handler names, state names, labels, and button disabled logic unchanged while moving those JSX blocks.

- [ ] **Step 6: Replace top-level Runtime JSX**

Replace the current returned fragment body with:

```tsx
    <section className="runtime-console">
      <header className="runtime-taskbar">
        <div>
          <h2 className="page-title">运行台</h2>
          <p className="text-meta">围绕当前章节计划检查上下文、模型、生成、审阅和交付。</p>
        </div>
        <div className="runtime-plan-picker">
          <label>章节计划</label>
          <select className="select" value={selectedPlanId} onChange={e => setSelectedPlanId(e.target.value)} disabled={!selected || plans.length === 0}>
            <option value="">选择计划</option>
            {plans.map(plan => <option key={plan.id} value={plan.id}>{plan.sequence}. {plan.title || "Untitled"} ({plan.status})</option>)}
          </select>
        </div>
        <div className={`runtime-rag-chip runtime-rag-${ragHealth?.state || "unknown"}`}>
          <strong>{ragHealth?.state || "unknown"}</strong>
          <span>{ragHealth?.message || "RAG 状态等待读取"}</span>
        </div>
      </header>

      {runtimeMsg && <div className={`msg-banner ${runtimeMsg.startsWith("错误") ? "msg-error" : "msg-success"}`}>{runtimeMsg}</div>}

      <nav className="runtime-phase-tabs" aria-label="运行台流程">
        {runtimePhases.map(phase => (
          <button
            key={phase.id}
            className={`runtime-phase-tab ${activePhase === phase.id ? "active" : ""}`}
            type="button"
            onClick={() => setActivePhase(phase.id)}
          >
            <strong>{phase.label}</strong>
            <span>{phase.detail}</span>
          </button>
        ))}
      </nav>

      {selectedPlan && <p className="runtime-plan-outline">{selectedPlan.outline || "暂无大纲"}</p>}

      {activePhase === "prepare" && (
        <section className="runtime-phase-panel">
          <div className="runtime-panel-main">
            <h3 className="section-title">模型路由</h3>
            <div className="runtime-binding-table">
              {workflowBindings.map(binding => (
                <div key={binding.id} className="runtime-binding-row">
                  <strong>{binding.label}</strong>
                  <span>{binding.value || "未绑定，使用全局默认"}</span>
                  <button className="btn btn-secondary btn-sm" onClick={() => { setProfileWorkflow(binding.id); setActivePhase("prepare"); }}>编辑</button>
                </div>
              ))}
            </div>
          </div>
          <aside className="runtime-panel-side">
            <h3 className="section-title">RAG 健康</h3>
            <p className="text-body">{ragHealth?.message || "未读取 RAG 状态。"}</p>
            <button className="btn btn-secondary btn-sm" onClick={loadRagHealth} disabled={!selected}>刷新 RAG 状态</button>
          </aside>
        </section>
      )}

      {activePhase === "context" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderContextPhase()}
        </section>
      )}

      {activePhase === "generate" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderGeneratePhase()}
        </section>
      )}

      {activePhase === "review" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          <div className="card">
            <h3 className="section-title">审阅与修订</h3>
            <p className="text-meta">质量摘要、反馈决策和修订候选会在这里归位。</p>
          </div>
        </section>
      )}

      {activePhase === "deliver" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderDeliverPhase()}
        </section>
      )}
    </section>
```

- [ ] **Step 7: Add Runtime CSS**

In `index.css`, add:

```css
.runtime-console {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.runtime-taskbar {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) minmax(240px, 360px) minmax(220px, 320px);
  gap: var(--space-md);
  align-items: end;
  padding-bottom: var(--space-md);
  border-bottom: 1px solid var(--control-stroke);
}

.runtime-plan-picker label {
  display: block;
  margin-bottom: 4px;
  color: var(--on-dark-mute);
  font-size: var(--text-caption-sm);
}

.runtime-rag-chip {
  min-height: 58px;
  padding: 9px 11px;
  border: 1px solid var(--control-stroke);
  border-radius: var(--radius-md);
  background: var(--surface-solid);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.runtime-rag-chip strong {
  color: var(--text-primary);
  font-size: 12px;
  text-transform: uppercase;
}

.runtime-rag-chip span {
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1.35;
}

.runtime-rag-usable { border-color: var(--success); }
.runtime-rag-stale,
.runtime-rag-empty,
.runtime-rag-disabled,
.runtime-rag-unknown { border-color: var(--warning); }

.runtime-phase-tabs {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--space-sm);
}

.runtime-phase-tab {
  min-height: 58px;
  padding: 8px 10px;
  border: 1px solid var(--control-stroke);
  border-radius: var(--radius-sm);
  background: var(--surface-solid);
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
}

.runtime-phase-tab.active {
  border-color: var(--accent);
  background: var(--accent-subtle);
}

.runtime-phase-tab strong,
.runtime-phase-tab span {
  display: block;
}

.runtime-phase-tab span {
  margin-top: 2px;
  color: var(--text-secondary);
  font-size: 11px;
}

.runtime-plan-outline {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--control-stroke);
  border-radius: var(--radius-sm);
  background: var(--surface-subtle);
  color: var(--on-dark-body);
  line-height: 1.55;
}

.runtime-phase-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(260px, 360px);
  gap: var(--space-md);
  align-items: start;
}

.runtime-phase-panel-stack {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--space-md);
}

.runtime-binding-table {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.runtime-binding-row {
  display: grid;
  grid-template-columns: 120px minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--control-stroke);
}
```

- [ ] **Step 8: Run targeted test**

Run:

```bash
node --test tauri-app/src/runtimePage.contract.test.mjs
```

Expected: PASS.

- [ ] **Step 9: Commit**

Run:

```bash
git add tauri-app/src/pages/RuntimePage.tsx tauri-app/src/index.css tauri-app/src/runtimePage.contract.test.mjs
git commit -m "feat: reshape runtime as workflow console"
```

### Task 6: Feature Consistency Contracts

**Files:**
- Modify: `tauri-app/src/runtimePage.contract.test.mjs`
- Modify: `tauri-app/src/fluentTokens.test.mjs`
- Modify: `tauri-app/src/pages/RuntimePage.tsx`
- Modify: `tauri-app/src/App.tsx`

- [ ] **Step 1: Add duplicate-entry regression tests**

Append to `runtimePage.contract.test.mjs`:

```javascript
test("runtime feature entry points are grouped by workflow responsibility", () => {
  const expectedGroups = [
    "模型路由",
    "RAG 健康",
    "手动上下文规则",
    "SillyTavern Lorebook JSON",
    "操作配方",
    "提示词预设",
    "草稿候选",
    "项目包",
    "小说圣经包",
    "扩展 Manifest",
  ];
  for (const group of expectedGroups) {
    assert.ok(runtimeSource.includes(group), `missing grouped capability ${group}`);
  }
  assert.equal((runtimeSource.match(/扩展 Manifest/g) || []).length, 1);
  assert.equal((runtimeSource.match(/项目包/g) || []).length, 1);
});
```

Append to `fluentTokens.test.mjs`:

```javascript
test("main shell no longer renders the pet inside the main window", () => {
  assert.doesNotMatch(app, /function AppPet/);
  assert.doesNotMatch(app, /className=\{`app-pet/);
  assert.match(app, /PetWindow/);
});
```

- [ ] **Step 2: Run tests to verify failures**

Run:

```bash
node --test tauri-app/src/runtimePage.contract.test.mjs tauri-app/src/fluentTokens.test.mjs
```

Expected: FAIL until the previous Runtime and pet tasks have fully removed duplicate shell entries.

- [ ] **Step 3: Clean final UI placement**

In `RuntimePage.tsx`:

- Keep context rules and Lorebook only in the `上下文` phase.
- Keep operator recipes, prompt presets, prompt units, and draft candidates only in the `生成` phase.
- Keep package IO and extensions only in the `交付` phase.
- Keep workflow model binding and RAG health only in the `准备` phase.

In `App.tsx`:

- Keep pet settings in `SettingsPage`.
- Do not render pet status in the main shell.

- [ ] **Step 4: Run consistency tests**

Run:

```bash
node --test tauri-app/src/runtimePage.contract.test.mjs tauri-app/src/fluentTokens.test.mjs
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add tauri-app/src/runtimePage.contract.test.mjs tauri-app/src/fluentTokens.test.mjs tauri-app/src/pages/RuntimePage.tsx tauri-app/src/App.tsx
git commit -m "test: lock runtime capability boundaries"
```

### Task 7: Full Verification

**Files:**
- No new files.
- Verify: full backend, frontend, build, and diff hygiene.

- [ ] **Step 1: Run backend tests**

Run:

```bash
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture
```

Expected: PASS. Any failing test must be fixed without weakening the RAG, pet, Runtime, or consistency requirements.

- [ ] **Step 2: Run frontend contract tests**

Run:

```bash
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
```

Expected: PASS.

- [ ] **Step 3: Run production build**

Run from `D:\novel\tauri-app`:

```bash
npm run build
```

Expected: PASS with TypeScript and Vite build success.

- [ ] **Step 4: Run diff hygiene**

Run:

```bash
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 5: Inspect final changed files**

Run:

```bash
git status --short
git diff --stat
```

Expected: only files involved in this plan plus pre-existing unrelated workspace changes are present. Do not stage unrelated pre-existing changes.

- [ ] **Step 6: Commit verification fixes**

If Step 1-4 required fixes, commit only files touched for this plan:

```bash
git add tauri-app/src-tauri/src tauri-app/src-tauri/tests tauri-app/src tauri-app/src-tauri/tauri.conf.json tauri-app/src-tauri/capabilities/default.json tauri-app/src-tauri/migrations/001_init_sqlite.sql
git commit -m "fix: verify integrated runtime control"
```

If there are no fixes after the previous task commits, skip this commit.

## Self-Review

- Spec coverage: Tasks 1-3 cover BGE-M3 asymmetric embedding, RAG health, vector freshness, trace readiness, and document/query workflows. Task 4 covers independent desktop pet behavior, drag support, status assistant behavior, and reduced-motion-compatible CSS. Task 5 covers Runtime workflow UI. Task 6 covers feature consistency and duplicated entry point prevention. Task 7 covers full verification.
- Placeholder scan: The plan contains no unresolved marker text, no unfilled code blocks, and no open-ended implementation steps.
- Type consistency: `EmbeddingInputKind`, `embed_with_kind`, `VectorEmbeddingMetadata`, `RagHealth`, `get_rag_health`, `getRagHealth`, `PetWindow`, `showPetWindow`, `hidePetWindow`, and `showMainWindow` are named consistently across tasks.
