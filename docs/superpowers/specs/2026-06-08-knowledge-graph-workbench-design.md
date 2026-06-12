# Knowledge Graph Workbench Design

## Goal

Turn the declared Obsidian-style knowledge graph into a usable workbench. The first slice should expose graph data through typed backend commands, infer a useful node snapshot from the existing novel bible, and add a dense graph page where users can inspect canon relationships without leaving the desktop app.

## Current Evidence

The project already has the core storage but the product loop is incomplete:

- `knowledge_graph_edges` exists in `tauri-app/src-tauri/migrations/001_init_sqlite.sql`.
- `tauri-app/src-tauri/src/db/knowledge_graph.rs` has edge CRUD helpers.
- `tauri-app/src-tauri/src/db/mod.rs` does not export `knowledge_graph`.
- `tauri-app/src-tauri/src/lib.rs` does not register graph Tauri commands.
- `tauri-app/src/App.tsx` has no graph navigation entry or page.
- `README.md` advertises an interactive Obsidian-style graph, so this is a visible product gap.

The frontend is a React/Tauri single file app with a dark PlayStation-style operational UI. The graph workbench should fit that style: dense, scan-friendly, and work-oriented rather than decorative.

## Recommended Scope

Build a pragmatic graph workbench without adding third-party graph dependencies in this slice. A full force-directed canvas can come later. This slice should produce an inspectable relationship map using CSS-positioned nodes and explicit edge rows, which is enough to make the graph data visible, searchable, filterable, and editable.

## Data Model

Add graph view types in Rust:

- `KnowledgeGraphNode`
  - `id`
  - `node_type`
  - `label`
  - `subtitle`
  - `description`
  - `status`
  - `degree`
- `KnowledgeGraphEdge`
  - Existing db edge type.
- `KnowledgeGraphSnapshot`
  - `nodes`
  - `edges`
  - `orphan_count`

Nodes are derived from existing Bible tables, not stored in a new table:

- characters
- locations
- organizations
- items
- world_lore
- magic_systems
- plot_threads
- foreshadowing
- timeline_events
- canon_rules

Edges remain stored in `knowledge_graph_edges`. This keeps the stored interface narrow and avoids duplicating canon data.

## Backend Commands

Add these Tauri commands:

- `get_knowledge_graph(project_id) -> KnowledgeGraphSnapshot`
- `create_knowledge_graph_edge(project_id, source_id, source_type, target_id, target_type, edge_type, description) -> KnowledgeGraphEdge`
- `delete_knowledge_graph_edge(edge_id) -> ()`

Validation rules:

- Empty project, source, target, or edge type returns an error.
- Self-edges with the same source ID and target ID return an error.
- `create_knowledge_graph_edge` returns the created edge by reading it back from SQLite.

## Frontend UX

Add a `Graph` navigation item and `KnowledgeGraphPage`.

The page should include:

- Compact stat strip: nodes, edges, orphan nodes, selected type.
- Search input.
- Type filter buttons.
- A graph canvas using stable, non-overlapping radial coordinates by node type.
- Clickable nodes with type color and degree indicator.
- Side inspector for the selected node, listing description and connected edges.
- Manual edge creator with source node, target node, edge type, and description.
- Edge list with delete controls.

The page should load from `get_knowledge_graph` whenever the selected project changes or an edge is added/deleted.

## Visual Direction

Use the existing dark operational style. The memorable element is a "canon radar" graph: compact nodes arranged in rings by type, with a clear inspector for relationship work. Avoid a marketing hero or decorative background. The UI should feel like an editor tool.

## Testing

Add backend tests for:

- Snapshot generation includes Bible nodes.
- Degree counts reflect stored edges.
- Edge insertion returns the inserted edge and rejects invalid/self edges.
- Edge deletion removes it from the snapshot.

Frontend verification is via `npm run build`. The first slice does not need browser automation unless layout breaks in build or obvious runtime issues appear.

## Out Of Scope

- AI relationship extraction after each chapter.
- Persisted graph node layout.
- Cytoscape.js or other force-directed graph libraries.
- Full graph diffing/history.
- Graph nodes for every chapter version.
- RAG evaluation and pipeline metrics.

## Acceptance Criteria

- Graph commands are registered and compile.
- `db::knowledge_graph` is exported.
- The Graph page is reachable from the sidebar.
- Users can view Bible nodes and stored graph edges.
- Users can create and delete manual edges.
- Selected nodes show connected relationship details.
- Rust tests for graph backend pass.
- Frontend production build passes.
- Existing Rust tests continue to pass.
