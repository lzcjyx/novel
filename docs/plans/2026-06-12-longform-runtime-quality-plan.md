# Longform Runtime Quality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix reviewer runtime safety, status consistency, longform bootstrap pacing, and markdown storage lifecycle without weakening review quality.

**Architecture:** Keep changes inside existing Rust modules and prompt files. Add small pure helpers for reviewer parsing, status resolution, and project paper directory metadata so behavior is testable without launching Tauri.

**Tech Stack:** Rust, Tauri commands, rusqlite, serde_json, existing prompt markdown files, existing Vitest/Vite frontend build.

---

## File Map

- Modify `tauri-app/src-tauri/src/workflow/review_agents.rs`: safe preview, robust JSON extraction, score validation.
- Modify `tauri-app/src-tauri/prompts/style_reviewer.md`: explicit 0-100 rubric and pass consistency.
- Modify `tauri-app/src-tauri/prompts/bible_generation.md`: longform first-10-chapters pacing rules.
- Modify `tauri-app/src-tauri/src/lib.rs`: status combines memory and SQLite; delete uses persisted project paper directory; rebuild RAG uses embedding provider only.
- Modify `tauri-app/src-tauri/src/db/projects.rs`: project metadata helpers for paper directory.
- Modify `tauri-app/src-tauri/src/workflow/novel_bootstrap.rs`: persist bootstrap paper directory.
- Modify `tauri-app/src-tauri/src/export/markdown.rs`: export to persisted project paper directory.
- Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`: make markdown export a required artifact for successful generation.
- Modify tests in `tauri-app/src-tauri/tests/core_writing_loop_tests.rs` and `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`.

## Tasks

### Task 1: Reviewer Parser Regression Tests

- [ ] Add tests that call review agents with Chinese output longer than 300 bytes and score below 20.
- [ ] Verify the test currently fails by panicking at `review_agents.rs`.
- [ ] Add a test where provider output wraps valid JSON in prose/fences and assert the score is preserved as 95, not defaulted to 0.
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests reviewer -- --nocapture`

### Task 2: Reviewer Parser Fix

- [ ] Add `preview_chars(input, max_chars)` using `chars().take(max_chars).collect()`.
- [ ] Add `extract_review_json(raw)` that tries raw JSON, cleaned fences, then first `{` through last `}`.
- [ ] Replace byte-slice logging with safe preview.
- [ ] Return parse failures as normal agent failure reviews through the existing `timed_review` error path.
- [ ] Run the Task 1 test command again and require pass.

### Task 3: Review Quality Prompt Guardrails

- [ ] Update `style_reviewer.md` with a 0-100 rubric:
  - 90-100 publish-ready style;
  - 75-89 publishable with minor edits;
  - 60-74 needs revision;
  - 40-59 major rewrite;
  - 0-39 unusable or unreadable.
- [ ] State `pass=true` only for score >= 75 and no blocking issue.
- [ ] Add a prompt test asserting rubric and pass threshold text exists.
- [ ] Run the prompt test.

### Task 4: Status Consistency

- [ ] Add a pure helper in `lib.rs`, `project_is_running(db, memory_running, project_id)`.
- [ ] Test that a SQLite job in `reviewing` makes the helper return true even when memory is false.
- [ ] Update `get_status` to call the helper for selected or active projects.
- [ ] Run generation job observability tests.

### Task 5: Longform Bootstrap Pacing

- [ ] Update `bible_generation.md` so `chapter_plans` means the first 10 immediate chapter plans only.
- [ ] Explicitly forbid resolving the central conflict, final villain, core mystery, endgame romance, or final power endpoint in those 10 plans.
- [ ] Add prompt tests asserting the longform opening-arc constraints are present.
- [ ] Run the prompt tests.

### Task 6: Persist Project Paper Directory

- [ ] Add metadata helpers in `db/projects.rs`:
  - `set_project_paper_dir(db, project_id, paper_dir)`;
  - `get_project_paper_dir(db, project_id, fallback_data_dir)`;
  - `project_paper_dirs_for_cleanup(db, project_id, fallback_data_dir)`.
- [ ] Tests must confirm fallback behavior for old projects and persisted behavior after settings changes.
- [ ] Update `novel_bootstrap.rs` to persist the created directory.
- [ ] Run affected tests.

### Task 7: Markdown Export and Deletion Lifecycle

- [ ] Update `export_chapter_markdown` and `export_novel_markdown` to use `get_project_paper_dir`.
- [ ] Update generation pipeline to fail the job if export fails.
- [ ] Update `delete_project` to compute cleanup directories before SQLite deletion and remove persisted plus fallback directories.
- [ ] Add tests for export path and cleanup path selection.
- [ ] Run core loop and db/project tests.

### Task 8: Final Verification

- [ ] Run targeted Rust tests:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [ ] Run generation observability tests:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture`
- [ ] Run all Rust tests:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml`
- [ ] Run frontend build:
  `npm run build` from `tauri-app`.
- [ ] Run whitespace check:
  `git diff --check`.
