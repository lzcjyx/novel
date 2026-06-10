# RAG Context Source Persistence Plan

## Goal

Persist the RAG source trace selected during chapter context assembly onto saved chapter versions.

## Tasks

- [x] Audit vector store, writing context assembly, prompt rendering, and chapter persistence paths.
- [x] Write a failing core writing loop test for persisted selected retrieval source IDs.
- [x] Add chapter version metadata update support.
- [x] Build context metadata from `WritingContextPackage`.
- [x] Persist selected retrieval source keys, retrieval document IDs, retrieval trace, and graph context after draft save.
- [x] Prove the generated prompt includes the seeded retrieval source title.
- [x] Prove saved chapter version metadata points back to the seeded retrieval source.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test rag_explainability_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test graph_rag_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] `git diff --check`

## Follow-Up

- [x] Add explicit content hashes to vector chunk records.
- [ ] Add a historical chapter version context viewer if operators need post-generation audit UI.
