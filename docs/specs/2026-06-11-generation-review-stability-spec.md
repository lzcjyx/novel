# Generation Review Stability Spec

## Goal

Fix three reported AI Novel Factory stability regressions without reducing the writing workflow: Reviews must stay usable during generation, RAG-off mode must not create artificial continuity failures, and generation jobs must reach a terminal state after revise/post-processing instead of later reappearing as failed.

## Findings

- The Reviews page treats nullable backend score fields as if they are always numbers. Rust `Option<f64>` values serialize to `null`, and the React page calls `.toFixed()` when a field is `null` but not `undefined`. During an active chapter run, quality summary rows can exist before any numeric score exists, causing a React render exception and a black WebView.
- The generation workflow receives `emb_provider: None` when embedding is not configured, but then falls back to the main LLM provider for `embed()`. This makes RAG-off mode attempt embedding calls with the wrong provider/model and hides the difference between "RAG disabled" and "RAG configured but returned no documents".
- The continuity reviewer prompt does not explicitly say that missing RAG/vector documents are infrastructure context, not a story continuity defect. Review output can therefore penalize missing retrieval evidence instead of evaluating available canon and recent chapters.
- The job pipeline marks progress near revise/export/update-canon before running non-critical post-processing. If post-processing returns an error after a revised chapter is saved, the Tauri command marks the latest job as failed even though the chapter artifact exists.
- The Jobs page only fetches once on mount, so an open Jobs view can keep showing stale progress during a run unless the user navigates away or restarts.

## Scope

### Reviews UI Resilience

- Treat `null`, `undefined`, and `NaN` scores as `n/a`.
- Never call numeric formatting methods on backend optional fields without a finite-number guard.
- Keep the Reviews page renderable while no chapter is selected, while a chapter is mid-review, and while quality summaries contain no numeric scores.

### RAG-Off Semantics

- When no embedding provider is configured, skip vector retrieval explicitly.
- Emit and persist a retrieval phase detail that says RAG is disabled instead of reporting an embedding failure.
- Keep writing context usable from structured canon, graph context, recent summaries, recent body excerpts, and learning entries.
- When embedding is later configured and tested, the existing rebuild/index and preview flows should immediately use the configured embedding provider.

### Continuity Review Semantics

- Update the continuity reviewer prompt so RAG 0 docs, missing vector search, or embedding disabled state is never itself a blocking story issue.
- The reviewer should judge only evidence present in canon, recent chapters, character states, timeline, and the current chapter.

### Job Finalization

- After a valid draft/revision is saved and final review decision is known, optional post-processing failures must be recorded as phase details but must not flip the job to `failed`.
- Canon update and self-reflection remain useful, but they are non-critical after chapter content and review state are persisted.
- Jobs should finish as `completed` or `needs_human_review` when the core writing artifact is valid.
- The Jobs view should refresh while selected so users can see progress move past revise without restarting.

## Non-Goals

- Do not implement provider-wide token streaming.
- Do not force the user to configure embeddings before writing.
- Do not lower review thresholds or ignore real canon/blocking issues.
- Do not rewrite the React app into multiple pages in this slice.

## Acceptance Criteria

- Opening Reviews during chapter generation does not blank the app when summary score fields are `null`.
- RAG disabled mode does not call `embed()` on the main LLM provider.
- Context retrieval phase reports disabled/skipped state when embedding provider is `none`.
- Continuity prompt explicitly excludes RAG availability from blocking story continuity defects.
- A chapter generation run with no embedding provider can complete using structured context.
- Post-review post-processing errors do not cause a saved/revised chapter job to become `failed`.
- Jobs page auto-refreshes while visible.
- Targeted Rust tests pass.
- Frontend build passes.
- Full Rust test suite and whitespace check pass before completion.
