# Graph-RAG Context Plan

## Goal

Wire the knowledge graph into writing context assembly and make the influence visible in Context Preview.

## Tasks

- [x] Write failing tests for plan-to-graph neighborhood matching.
- [x] Write failing tests for graph-biased retrieval reranking.
- [x] Add `GraphContext`, `GraphContextNode`, and `GraphContextNeighbor`.
- [x] Add `build_graph_context`.
- [x] Add `rerank_retrieval_with_graph_context`.
- [x] Add `graph_context` to `WritingContextPackage`.
- [x] Show graph link count, summary, and relationship chips in Context Preview.
- [x] Add selected-node retrieval hints inside the Graph page.
- [x] Add direct graph neighborhood lookup by explicit node ID/type for Graph page workflows.
- [x] Add two-hop graph expansion with strict neighbor and summary token budgets.
- [x] Add AI-inferred edge creation after canon updates with node-existence filtering.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test graph_rag_tests -- --nocapture`
- [x] `npm run build`
- [x] Full Rust test suite.
- [x] Edge visual verification.
- [x] `git diff --check`.
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test graph_rag_tests graph_context_expands_two_hops_with_token_budget -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test knowledge_graph_tests canon_update_persists_valid_ai_inferred_graph_edges -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test knowledge_graph_tests -- --nocapture`

## Follow-Up

- [x] Add multi-hop graph expansion with a strict token budget.
- [x] Add AI-inferred edge creation after canon updates.
