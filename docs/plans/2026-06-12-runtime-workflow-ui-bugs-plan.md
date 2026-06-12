# Runtime Workflow UI Bugs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the reported Learn, Graph, Publication, Dashboard, and human-review-save bugs with regression coverage.

**Architecture:** Keep backend fixes in the existing workflow/db modules and keep the Graph UI inside the current React page. Add one small frontend graph layout helper so drag math and live offsets are testable without a browser framework.

**Tech Stack:** Rust/Tauri, rusqlite, React 19, Vite, Node built-in test runner.

---

## Files

- Modify: `tauri-app/src-tauri/src/workflow/learning.rs`
- Modify: `tauri-app/src-tauri/src/db/projects.rs`
- Modify: `tauri-app/src-tauri/src/db/blog_posts.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src-tauri/src/workflow/review_agents.rs`
- Modify: `tauri-app/src-tauri/prompts/publication_reviewer.md`
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`
- Create: `tauri-app/src/graphLayout.js`
- Create: `tauri-app/src/graphLayout.test.mjs`
- Modify tests: `tauri-app/src-tauri/tests/learning_intake_tests.rs`
- Modify tests: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

## Tasks

### Task 1: Learn extraction normalization

- [ ] Add a failing test in `learning_intake_tests.rs` where `generate_json` returns `type`, `name`, and `description`; assert the result has a normalized category, non-Unknown name, and non-empty description.
- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests learning_extraction_accepts_alias_fields -- --nocapture` and confirm it fails for the current `Unknown` behavior.
- [ ] Add alias helpers in `workflow::learning` and skip unusable items.
- [ ] Re-run the targeted learning test and the whole `learning_intake_tests` file.

### Task 2: Plans Left semantics

- [ ] Add a failing test in `core_writing_loop_tests.rs` that creates one `in_progress` plan with a chapter and asserts `projects::get_project_stats(...).plans_left == 0`.
- [ ] Run the targeted test and confirm it fails because stats count `in_progress`.
- [ ] Change `get_project_stats` to count only `status = 'planned'`.
- [ ] Re-run the targeted stats test and the existing human-review generation test.

### Task 3: Human edit save lock safety

- [ ] Extract the edit-save database work into a public helper in `lib.rs` or a db module so it can be tested without Tauri `State`.
- [ ] Add a failing test that saves an edit and immediately reads latest version, plans, jobs, bible, graph snapshot, learning entries, and writing context from the same database.
- [ ] Run the targeted test and confirm the current helper deadlocks or fails before the lock ordering fix.
- [ ] Load chapter/latest version before acquiring the insert/update lock, then perform the write in one scoped lock.
- [ ] Re-run the targeted test.

### Task 4: Publication scoring and metadata seam

- [ ] Add a test that publication reviewer output with `blog_metadata` is preserved on the `AgentReview` returned by `run_review_agents`.
- [ ] Add a test that `publish_blog_draft` or its helper creates a `blog_posts` row whose metadata contains `publication_metadata`.
- [ ] Update the publication reviewer prompt with 0-100 score bands.
- [ ] Add a blog post insert helper that accepts metadata and use latest publication reviewer metadata when creating a local blog draft.
- [ ] Re-run targeted core writing loop tests.

### Task 5: Graph live layout and drag

- [ ] Add `graphLayout.js` with deterministic base positioning, bounded live offsets, and pointer-to-percent coordinate conversion.
- [ ] Add `graphLayout.test.mjs` using `node:test` to verify positions are bounded, flow offsets move a node, and drag coordinates clamp to the canvas.
- [ ] Run `node --test tauri-app/src/graphLayout.test.mjs` and confirm it fails before the helper exists.
- [ ] Import the helper from `App.tsx`, replace fixed render-only positions with stateful positions, add pointer handlers, and preserve edge coordinates from live positions.
- [ ] Update CSS for draggable graph nodes.
- [ ] Run the node graph test and `npm run build`.

### Task 6: Full verification

- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture`.
- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture`.
- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`.
- [ ] Run `node --test tauri-app/src/graphLayout.test.mjs`.
- [ ] Run `npm run build` in `tauri-app`.
- [ ] Run `git diff --check`.
