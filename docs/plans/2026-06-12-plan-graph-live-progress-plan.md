# Plan Graph Live Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve longform plan generation, chapter-driven knowledge graph edges, flowing Graph UI, and live text progress feedback.

**Architecture:** Add small testable backend helpers for weekly pacing context and graph edge resolution. Keep UI changes inside existing React/CSS files, using existing Tauri events and graph APIs. No new runtime dependencies.

**Tech Stack:** Rust, Tauri, SQLite/rusqlite, React 19, TypeScript, CSS animations with `prefers-reduced-motion`.

---

## File Map

- Modify `tauri-app/src-tauri/src/workflow/weekly_planner.rs`: build explicit longform pacing context and persist richer plan metadata.
- Modify `tauri-app/src-tauri/prompts/weekly_planner.md`: enforce phase-aware local movement pacing.
- Modify `tauri-app/src-tauri/src/workflow/canon_updater.rs`: resolve graph edges by ID or unique label and add deterministic chapter graph edges.
- Modify `tauri-app/src-tauri/prompts/canon_extractor.md`: allow ID or exact label fields for edges.
- Modify `tauri-app/src/App.tsx`: live progress feed, graph auto-refresh, selected-edge emphasis, graph UI structure.
- Modify `tauri-app/src/index.css`: flowing graph edge animation, graph node motion, live feed layout, reduced-motion rules.
- Modify `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`: weekly planner context/prompt tests.
- Modify `tauri-app/src-tauri/tests/knowledge_graph_tests.rs`: graph edge name resolution and deterministic edges tests.

## Tasks

### Task 1: Weekly Planner Pacing Context Tests

- [ ] Add a Rust test that creates a 500,000 word project, inserts completed chapters with summaries, calls a pure weekly planner context helper, and asserts `story_progress_percent`, `estimated_total_chapters`, `story_phase`, `next_sequence`, and recent summaries are present.
- [ ] Add a prompt test asserting `weekly_planner.md` contains `story_phase`, `story_progress_percent`, `next local movement`, and `endgame`.
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests weekly_planner -- --nocapture`
- [ ] Expected before implementation: missing helper or missing context assertions fail.

### Task 2: Weekly Planner Implementation

- [ ] Add `WeeklyPlannerContext` and `build_weekly_planner_context(db, project_id)`.
- [ ] Compute estimated total chapters as `ceil(total_target_words / daily_target_words)` with safe defaults.
- [ ] Compute story phase from progress:
  - `< 0.12 opening`;
  - `< 0.35 early_development`;
  - `< 0.65 middle_build`;
  - `< 0.85 late_build`;
  - otherwise `endgame`.
- [ ] Include last 6 completed chapter summaries and all currently planned/in-progress plan summaries.
- [ ] Use this helper in `run_weekly_arc_planner`.
- [ ] Store plan metadata with `pov_character`, `ending_hook`, `story_phase`, and `planner_pacing`.
- [ ] Run Task 1 command until green.

### Task 3: Graph Edge Resolution Tests

- [ ] Add a test provider that returns one `knowledge_graph_edges` entry using labels instead of IDs:
  `source_label: "林白"`, `source_node_type: "character"`, `target_label: "旧车站"`, `target_node_type: "location"`.
- [ ] Assert `update_canon_after_chapter` creates a `character -> location` edge using the correct IDs.
- [ ] Add a second test where extracted timeline event involves `林白` and `旧车站`; assert deterministic edges connect `林白 -> timeline_event` and `timeline_event -> 旧车站`.
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test knowledge_graph_tests -- --nocapture`
- [ ] Expected before implementation: label edge and deterministic edge assertions fail.

### Task 4: Graph Edge Backend Implementation

- [ ] Add a node index in `canon_updater.rs` from graph snapshot with keys:
  - `type:id`;
  - `type:normalized_label` when unique.
- [ ] Extend AI edge persistence to read `source_node_id` or `source_label`, and `target_node_id` or `target_label`.
- [ ] Add deterministic edge creation after timeline/foreshadowing inserts using extracted event arrays and newly inserted timeline IDs.
- [ ] Treat invalid or ambiguous nodes as skipped warnings.
- [ ] Run Task 3 command until green.

### Task 5: Live Progress Feed UI Tests

- [ ] Add TypeScript-safe structures in `App.tsx` for `LiveProgressEntry`.
- [ ] Use build verification as the UI regression gate because this project has no frontend test runner.
- [ ] Ensure generated feed lines contain phase label, status, percentage, and detail.
- [ ] Run `npm run build` before and after implementation.

### Task 6: Live Progress Feed UI Implementation

- [ ] In `Dashboard`, add `progressFeed` state and a `liveProgressRef`.
- [ ] On `pipeline-step`, append a line for non-preview events and auto-scroll the feed.
- [ ] Preserve the last feed when loading becomes false.
- [ ] Render a `Live Progress Feed` panel next to the preview in `live-workbench`.
- [ ] Add `.live-progress-feed`, `.live-feed-line`, and semantic status classes in CSS.

### Task 7: Flowing Graph UI Implementation

- [ ] In `KnowledgeGraphPage`, refresh graph on `pipeline-step` events with `step === "update_canon"` or `step === "complete"`.
- [ ] Render edge groups with base and animated flow paths.
- [ ] Add selected-edge class when edge touches selected node.
- [ ] Add graph stats for selected node degree.
- [ ] Add accessible labels to graph nodes and SVG paths.
- [ ] Add CSS for `stroke-dasharray`, `graphFlow`, subtle node float, edge labels, and reduced-motion.
- [ ] Keep mobile layout single-column and graph canvas stable.

### Task 8: Final Verification

- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test knowledge_graph_tests -- --nocapture`
- [ ] Run all Rust tests:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml`
- [ ] Run frontend build:
  `npm run build` from `tauri-app`.
- [ ] Run whitespace check:
  `git diff --check`.
