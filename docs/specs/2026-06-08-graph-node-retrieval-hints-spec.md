# Graph Node Retrieval Hints Spec

## Goal

Make selected graph nodes actionable for RAG workflows. Operators should be able to click a node and see the exact source key and directly connected source keys that can bias retrieval.

## Scope

Add explicit one-hop neighborhood lookup by `project_id`, `node_id`, and `node_type`.

Included:

- Backend `KnowledgeGraphNeighborhood` with center node, neighboring nodes, connecting edges, and retrieval hints.
- Retrieval hints include `source_key`, `connected_source_keys`, and human-readable `query_terms`.
- Tauri command for Graph page workflows.
- Graph inspector panel showing retrieval hints for the selected node.

Excluded:

- Multi-hop expansion.
- Persisted per-node vector retrieval results.
- AI-inferred edge creation.

## Acceptance Criteria

- Tests prove explicit node neighborhood lookup returns only directly connected neighbors and source keys.
- Graph page calls the explicit neighborhood command when a node is selected.
- Graph inspector shows selected-node retrieval hints without layout overflow.
- Full Rust tests, frontend build, whitespace check, and Edge visual verification pass.
