# Vector Chunk Content Hash Plan

## Goal

Persist SHA-256 content hashes on RAG vector chunks.

## Tasks

- [x] Write a failing RAG explainability test for `content_hash`.
- [x] Add `content_hash` to `vector_document_metadata` initialization SQL.
- [x] Add an idempotent migration/backfill for existing vector records.
- [x] Add `content_hash` to `VectorDocument`.
- [x] Compute SHA-256 hashes in `insert_vector_document`.
- [x] Return hashes from `search_similar_documents`.
- [x] Add `source_content_hash_exists` for source-level re-embedding decisions.
- [x] Make same-source same-hash vector inserts idempotent.
- [x] Add vector index candidate filtering before provider embedding calls.
- [x] Use candidate filtering in bible bootstrap and manual vector rebuild flows.
- [x] Replace stale same-source vector rows when content changes.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests vector_documents_persist_content_hashes -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test graph_rag_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] `git diff --check`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests duplicate_vector_document_content_reuses_existing_doc -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests vector_index_candidates_skip_unchanged_content_hashes -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests bible_indexing_skips_embedding_for_unchanged_vector_hashes -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests -- --nocapture`

## Follow-Up

- [x] Add storage-level deduplication and source hash existence checks that use `content_hash`.
- [x] Skip provider embedding calls in rebuild/bootstrap flows after filtering unchanged source hashes.
