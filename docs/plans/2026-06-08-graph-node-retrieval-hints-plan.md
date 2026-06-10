# Graph Node Retrieval Hints Plan

## Goal

Add selected-node retrieval hints inside the Graph page and direct graph neighborhood lookup by explicit node id/type.

## Tasks

- [x] Write failing backend test for explicit graph node neighborhood lookup.
- [x] Add `KnowledgeGraphNeighborhood` and `KnowledgeGraphRetrievalHints`.
- [x] Add `get_node_neighborhood`.
- [x] Register `get_knowledge_graph_neighborhood` Tauri command.
- [x] Extend Graph page to load selected-node neighborhood data.
- [x] Render retrieval source key, connected source keys, and query terms in the inspector.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test knowledge_graph_tests -- --nocapture`
- [x] `npm run build`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] Edge visual verification for Graph UI.
- [x] `git diff --check`

## Follow-Up

- [x] Add multi-hop graph expansion with a strict token budget.
- [ ] Add persisted per-node retrieval result panels.
