# AI Novel Factory Improvement Plan

## Status

This plan tracks the multi-phase improvement goal for the local AI novel factory. It keeps implementation slices small enough to test, while aligning them with the larger roadmap: core writing loop, knowledge graph, RAG, Graph-RAG, quality evaluation, and performance operations.

## Phase 0: Audit And Baseline

- [x] Inspect repository structure, Tauri/Rust backend, React frontend, SQLite modules, workflows, prompts, and tests.
- [x] Identify high-value product gaps: RAG, graph UI, performance visibility, quality evaluation, and workflow reliability.
- [x] Preserve existing dirty worktree changes and avoid unrelated reverts.

Verification:

- `git status --short`
- `git diff --stat`
- targeted file and symbol searches with `rg`

## Phase 1: Core Writing Loop Foundation

- [x] Keep the chapter production workflow as the first reliability target.
- [x] Maintain deterministic tests around writing loop behavior.
- [x] Verify the core writing loop changes already present in the worktree.

Verification:

- `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- `npm run build`

## Phase 2: Knowledge Graph Workbench

Goal: make the declared Obsidian-like knowledge graph usable inside the desktop app.

Completed implementation:

- [x] Add `db::knowledge_graph` module export.
- [x] Add graph node and snapshot types.
- [x] Derive graph nodes from existing bible data.
- [x] Add edge creation validation and readback.
- [x] Add Tauri commands:
  - `get_knowledge_graph`
  - `create_knowledge_graph_edge`
  - `delete_knowledge_graph_edge`
- [x] Register graph commands in the Tauri handler.
- [x] Add React `Graph` navigation and `KnowledgeGraphPage`.
- [x] Add graph search, type filters, radial node canvas, inspector, edge form, and delete controls.
- [x] Add backend graph regression tests.

Verification completed:

- `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test knowledge_graph_tests -- --nocapture`
- `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- `npm run build`
- `git diff --check`
- Edge-based local visual check because Chrome is not installed on this machine.

Remaining follow-up:

- [x] Add AI-inferred edge creation after chapter/canon updates.
- [ ] Add persisted graph layout only if operators need stable manual placement.
- [ ] Consider a graph library only after the current graph reaches usability limits.

## Phase 3: RAG Foundation

Goal: make retrieval inspectable and testable before it is deeply wired into generation.

Tasks:

- [x] Audit current RAG, memory, prompt assembly, and chapter context code paths.
- [x] Write failing tests for deterministic retrieval from fixture sources.
- [x] Confirm vector records carry stable source IDs, source type, title, content, and timestamps.
- [x] Add retrieval API that returns scored chunks and source metadata.
- [x] Add context assembly API that records selected source IDs on chapter versions.
- [x] Add frontend context preview panel for a selected project/chapter.
- [x] Persist selected retrieval source keys, retrieval document IDs, retrieval trace, and graph context on generated chapter versions.
- [x] Verify retrieval tests, full Rust tests, frontend build, and whitespace checks.

Acceptance criteria:

- A test can seed sources and prove the top retrieved chunks are correct.
- Generated context can be traced back to concrete source IDs.
- The UI can show which sources are about to be used.

Remaining follow-up:

- [x] Add explicit content hashes to vector chunks.
- [x] Add storage-level vector chunk deduplication using content hashes.
- [x] Skip provider embedding calls in rebuild/bootstrap flows after filtering unchanged source hashes.

## Phase 4: Graph-RAG Integration

Goal: use canon relationships to improve retrieval.

Tasks:

- [x] Add plan-matched one-hop graph neighborhood lookup.
- [x] Include graph-neighborhood summaries in context assembly.
- [x] Bias retrieval toward directly connected characters, locations, organizations, items, and plot threads.
- [x] Add bounded two-hop graph expansion with strict summary token budget.
- [x] Add AI-inferred edge creation after canon updates with node-existence filtering.
- [x] Show graph-influenced context sources in the frontend.
- [x] Add tests proving connected graph nodes affect retrieval order.

Acceptance criteria:

- Generating around a canon entity pulls directly related sources.
- The UI can explain the relationship path that affected retrieval.

Remaining follow-up:

- [x] Add selected-node retrieval hints inside the Graph page.
- [x] Add direct graph neighborhood lookup by explicit node ID/type for Graph page workflows.

## Phase 5: Quality Evaluation

Goal: make chapter quality and consistency regressions visible.

Tasks:

- [x] Add deterministic canon consistency fixtures.
- [x] Integrate deterministic canon precheck into review aggregation.
- [x] Add chapter continuity checks for repeated previous ending hooks.
- [x] Add timeline and location continuity fixtures.
- [x] Expand timeline and location contradiction checks beyond explicitly locked fixture data.
- [x] Add style drift checks using existing style and review modules.
- [x] Expand style drift checks beyond explicit JSON style guide rules with explicit plain-text labels.
- [x] Separate benchmark output into latency, quality, token, and cost sections.
- [x] Include provider-reported usage and configured cost in benchmark output.
- [x] Add a quality summary view or job detail section in the frontend.
- [x] Show deterministic canon precheck issues in the Reviews UI.
- [x] Add project-level review score and agent quality aggregation tests.

Acceptance criteria:

- Broken canon references can be caught by tests.
- Benchmark output is comparable between runs.
- Review and repair outputs are persisted and inspectable.
- Project-level review quality is visible in the frontend.

## Phase 6: Performance And Operations

Goal: make generation slowdowns and failures diagnosable from the app.

Tasks:

- [x] Add phase duration fields or structured job events.
- [x] Record retry count and failure reason per phase.
- [x] Add job metadata support for token/cost usage summaries.
- [x] Record estimated draft and revision token usage in chapter production.
- [x] Show token/cost summary metrics in the Jobs page.
- [x] Capture provider-reported usage when `ModelClient` surfaces raw provider metadata.
- [x] Add configurable model pricing defaults and store usage event price snapshots.
- [x] Add a job timeline UI for queued, running, failed, repaired, and completed states.
- [x] Add slow-step tests or benchmark assertions where deterministic.

Acceptance criteria:

- Operators can inspect where a generation job spent time.
- Failed jobs show a useful reason without opening logs.
- Expensive phases are visible enough to guide prompt or model changes.

## Phase 7: Final Verification Loop

Run after each implementation slice:

- [x] Targeted tests for the changed backend behavior.
- [x] Full Rust test suite.
- [x] Frontend production build.
- [x] Browser or Edge visual verification for changed UI.
- [x] `git diff --check`.
- [x] Review new warnings and either fix them or document why they are unrelated.

Latest verification notes:

- Full Rust test suite passed after provider-reported usage support.
- Frontend production build passed.
- Edge visual verification passed for the changed Jobs UI during the slow-step diagnostics slice.
- Edge visual verification passed for the changed Reviews UI during the canon precheck display slice.
- `git diff --check` exits with code 0; remaining output is CRLF conversion warnings from Git on Windows.
