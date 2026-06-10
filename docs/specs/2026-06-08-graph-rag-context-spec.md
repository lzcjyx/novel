# Graph-RAG Context Spec

## Goal

Use the knowledge graph to improve retrieval context for chapter generation. The graph should not only be visual; its relationships should influence which canon and memory sources are surfaced before writing.

## Scope

This slice adds a graph-neighborhood context package and uses it to rerank already retrieved vector documents.

Included:

- Match chapter plans to graph seed nodes.
- Extract bounded graph neighbors from `knowledge_graph_edges` up to two hops from matched seeds.
- Add graph context to `WritingContextPackage`.
- Boost retrieval documents whose `source_type/source_id` match graph seeds or neighbors.
- Show graph context in the frontend Context Preview.
- Show selected-node retrieval hints inside the Graph page.
- Accept AI-inferred `knowledge_graph_edges` from the canon extractor after chapter updates when both endpoints exist in the current graph snapshot.
- De-duplicate identical graph edges by project, endpoints, node types, and edge type.

Excluded:

- Full graph path search beyond the bounded two-hop context window.
- Persisted per-node retrieval result panels inside the Graph page.
- New embedding or vector indexes.

## Matching Rules

Plan fields map to graph node types:

- `pov_character_id` and `required_characters` match `character`.
- `required_locations` matches `location`.
- `plot_goals` matches `plot_thread`.
- `required_foreshadowing` matches `foreshadowing`.

Matched nodes become graph seeds. Directly connected nodes and their immediate neighbors become graph context neighbors, capped by a strict neighbor count and summary token budget.

## Retrieval Bias

After vector search returns scored documents, documents whose `(source_type, source_id)` match graph seed or neighbor keys receive a bounded score boost. The reranked list is then used for `retrieval` and `retrieval_trace`.

## Acceptance Criteria

- Tests prove graph context matches plan entities and summarizes connected neighbors.
- Tests prove two-hop graph context stays within the configured token budget.
- Tests prove connected graph sources can outrank unrelated higher-similarity sources after Graph-RAG reranking.
- `WritingContextPackage` serializes `graph_context`.
- Context Preview shows graph relationship explanations.
- Graph page shows selected-node retrieval hints and direct neighbor source keys.
- Canon updates persist valid AI-inferred graph edges as `auto_inferred=true` and skip missing-node hallucinations.
- Full Rust tests, frontend build, whitespace check, and Edge visual verification pass.
