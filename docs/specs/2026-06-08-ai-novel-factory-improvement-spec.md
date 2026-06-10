# AI Novel Factory Improvement Spec

## Goal

Make the project behave like a durable AI novel production factory, not only a chapter generator. The system should keep long-form canon stable, expose relationships visually, retrieve relevant context before writing, measure output quality, and keep generation cost and latency visible.

## Current Audit Summary

The repository is a Tauri desktop app with a React frontend, Rust commands, and SQLite persistence. The strongest existing foundation is local-first project data, novel bible entities, generation jobs, workflow modules, prompt files, and benchmark tests.

The main product gaps are:

- Core factory loop: chapter planning, drafting, review, repair, canon update, and learning need to remain a tested pipeline.
- Knowledge graph: storage existed, but it needed backend commands and a reachable editor UI.
- RAG: retrieval should be treated as a first-class pipeline with inspectable sources, not only hidden prompt assembly.
- UI: operators need dense workbench pages for graph, writing status, context sources, and quality signals.
- Performance: generation jobs need clearer duration, token, retry, and queue visibility.
- Reliability: workflows need targeted regression tests and smoke checks around failure states.

## Product Capabilities

### 1. Core Writing Loop

The writing loop should support:

- Weekly or batch planning.
- Chapter context assembly.
- Draft generation.
- Multi-agent review.
- Repair pass.
- Canon update.
- Learning/memory update.
- Persisted generation job status.

Acceptance criteria:

- The loop can be tested without a real model provider by using deterministic fixtures.
- Failed steps leave an inspectable job record.
- Chapter text, review notes, repair notes, and canon updates are persisted.

### 2. Knowledge Graph Workbench

The graph should expose canon entities and relationship edges in an Obsidian-like workbench.

Required first slice:

- Derive nodes from bible tables.
- Store only edges in `knowledge_graph_edges`.
- Add Tauri commands for snapshot, edge creation, and edge deletion.
- Add a `Graph` page with node filtering, search, inspector, and manual edge editing.

Out of scope for the first slice:

- AI-inferred relationship extraction.
- Persisted node layout.
- Third-party force-directed graph rendering.

Acceptance criteria:

- Graph commands compile and are registered.
- Users can reach the graph page from the sidebar.
- Users can view nodes, inspect relationships, create manual edges, and delete edges.
- Rust graph tests, full Rust tests, frontend build, and diff whitespace checks pass.

### 3. RAG Pipeline

RAG should be explicit enough for an operator to debug why a chapter used a source.

Required capabilities:

- Source ingestion for project notes, bible content, previous chapters, review feedback, and generated memory.
- Chunking with stable IDs, source metadata, stable content hashes, and storage-level duplicate suppression.
- Query-time retrieval with scoring.
- Context assembly with traceable source attribution.
- A frontend context panel showing selected sources before generation.

Acceptance criteria:

- Retrieval can be tested deterministically with fixture sources.
- Generated context includes source IDs and scores.
- The chapter workflow records selected context sources in generated chapter version metadata.

### 4. Graph-RAG Integration

The graph and RAG should reinforce each other.

Required capabilities:

- Selected graph node can show related retrieved chunks.
- Graph edges can bias retrieval for character, location, organization, item, and plot-thread context.
- Generation context can include a compact graph neighborhood summary.

Acceptance criteria:

- Given a selected character or location, the retriever prioritizes directly connected canon items.
- The UI can explain which graph relationships influenced the context.

### 5. Quality And Evaluation

The system needs repeatable checks for long-form writing quality.

Required capabilities:

- Canon consistency checks.
- Chapter continuity checks.
- Style drift checks.
- Review/repair regression tests.
- Benchmark summaries that separate latency, quality, and token/cost metrics.

Acceptance criteria:

- A deterministic test suite catches broken workflow contracts.
- Benchmark output is human-readable and comparable between runs.

### 6. Performance And Operations

Operators need to see where generation time and failures occur.

Required capabilities:

- Job timeline with phase durations.
- Retry and failure reason visibility.
- Token and estimated cost tracking when provider metadata and configured pricing are available.
- Slow-query or slow-step diagnostics.

Acceptance criteria:

- A generation job can be inspected from queued to completed/failed.
- Expensive or slow phases are visible without reading logs.

## Priorities

1. Keep the core writing loop stable with tests.
2. Make the knowledge graph usable.
3. Build RAG as a transparent pipeline.
4. Add Graph-RAG context explainability.
5. Improve performance and evaluation dashboards.
6. Expand visual polish only after the operator workflows are reliable.

## Risks

- Adding complex UI before backend contracts are stable will create brittle workflows.
- Hidden prompt context makes RAG bugs hard to diagnose.
- AI review and repair loops can become expensive without phase-level job metrics.
- A graph visualization without editing and inspection is decorative rather than operational.
