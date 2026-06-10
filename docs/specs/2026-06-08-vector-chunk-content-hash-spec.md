# Vector Chunk Content Hash Spec

## Goal

Make RAG vector chunks auditable and deduplicatable by persisting a stable content hash for each stored chunk.

## Scope

Add explicit `content_hash` support to `vector_document_metadata`.

Included:

- New and migrated vector records have a SHA-256 hash of `content`.
- Hashes are persisted in the SQLite vector metadata table.
- Hashes are returned on `VectorDocument` search results.
- Existing databases are migrated with an idempotent column add and backfill.
- `source_content_hash_exists` can tell callers whether a source already has an identical content hash.
- Re-inserting the same project/source type/source id/content hash returns the existing document id and does not create a duplicate vector row.
- Bootstrap and manual rebuild flows filter vector candidates by hash before calling the embedding provider.
- Re-inserting the same project/source type/source id with changed content replaces stale vector rows for that source.

Deferred:

- Multi-chunk source replacement policies beyond the current single-record source fixtures.

## Acceptance Criteria

- A test inserts a vector document and verifies the persisted `content_hash`.
- A test verifies retrieved `VectorDocument` values carry the same hash.
- A test verifies duplicate same-source content reuses the existing vector document.
- A test verifies unchanged vector candidates are skipped before embedding.
- A test verifies bible indexing does not call the provider again for unchanged content and does call it after content changes.
- RAG explainability and Graph-RAG tests continue to pass.
- Full Rust tests, frontend build, and whitespace checks pass.
