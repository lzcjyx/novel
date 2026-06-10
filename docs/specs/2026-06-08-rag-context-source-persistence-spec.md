# RAG Context Source Persistence Spec

## Goal

Make every generated chapter version traceable to the retrieval sources and graph context that shaped its writing prompt.

## Scope

Persist the selected RAG context on chapter artifacts produced by the core writing loop.

Included:

- Chapter version metadata records selected retrieval source keys as `source_type:source_id`.
- Chapter version metadata records selected retrieval document IDs.
- Chapter version metadata embeds the retrieval trace used by writing context assembly.
- Chapter version metadata embeds the graph context used by writing context assembly.
- Regression coverage seeds a deterministic vector source and proves that generated chapter metadata contains the selected source.

Excluded:

- Provider-reported token usage.
- New frontend metadata viewer for historical chapter versions.

## Acceptance Criteria

- A core writing loop test seeds a vector document and proves the generated prompt includes its source title.
- The saved chapter version metadata includes `selected_retrieval_source_keys`.
- The saved chapter version metadata includes `retrieval_trace.sources`.
- Existing RAG explainability and Graph-RAG tests continue to pass.
- Full Rust tests, frontend build, and whitespace checks pass.
