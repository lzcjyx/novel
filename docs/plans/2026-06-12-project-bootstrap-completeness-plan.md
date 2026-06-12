# Project Bootstrap Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make project creation atomic for required bootstrap artifacts: project, Bible, initial chapter plans, and derived graph nodes.

**Architecture:** Keep the fix inside the Rust bootstrap workflow and add focused integration tests. Introduce small validation and cleanup helpers in `workflow/novel_bootstrap.rs` so Tauri command behavior changes through the existing `bootstrap_novel` call.

**Tech Stack:** Rust, Tokio tests, rusqlite, serde_json, existing `ModelClient` trait, existing SQLite migrations.

---

## File Map

- Create `tauri-app/src-tauri/tests/project_bootstrap_tests.rs`: regression tests for atomic failure, incomplete Bible rejection, and success artifact validation.
- Modify `tauri-app/src-tauri/src/workflow/novel_bootstrap.rs`: fail on model generation errors, propagate insert errors, validate required artifacts, and cleanup partial projects on required failure.
- Modify `docs/specs/2026-06-12-project-bootstrap-completeness-spec.md`: captured behavior contract.
- Modify `docs/plans/2026-06-12-project-bootstrap-completeness-plan.md`: this task list.

## Tasks

### Task 1: Add Failing Bootstrap Regression Tests

- [ ] Create `tauri-app/src-tauri/tests/project_bootstrap_tests.rs` with a temporary SQLite database helper that opens a temp DB and runs migrations.
- [ ] Add a `FailingBibleProvider` implementing `ModelClient` where `generate_json` returns `Err("bible unavailable")`, `generate_text` returns `Ok(String::new())`, and `embed` returns an empty vector list.
- [ ] Add test `bootstrap_fails_and_cleans_project_when_bible_generation_fails`:
  - Build a `CreateProjectInput` with a name, genre, target words, and daily words.
  - Call `novel_bootstrap::bootstrap_novel`.
  - Assert the result is `Err`.
  - Assert the error contains `Bible generation failed`.
  - Assert `db::projects::list_projects(&db)` is empty.
- [ ] Add an `IncompleteBibleProvider` where `generate_json` returns a JSON object with one character, no locations, no organizations, no canon rules, no plot threads, and no chapter plans.
- [ ] Add test `bootstrap_rejects_incomplete_bible_and_cleans_project`:
  - Call `bootstrap_novel`.
  - Assert the result is `Err`.
  - Assert the error contains `Bootstrap validation failed`.
  - Assert `projects::list_projects(&db)` is empty.
- [ ] Add a `CompleteBibleProvider` where `generate_json` returns 6 characters, 4 locations, 2 organizations, 2 items, style guide, 5 canon rules, 3 plot threads, one power system, world overview, and 10 chapter plans.
- [ ] Add test `bootstrap_success_persists_required_bible_plans_and_graph_nodes`:
  - Call `bootstrap_novel`.
  - Assert `bible::get_bible` returns the required counts.
  - Assert `chapters::get_chapter_plans` returns 10 planned rows.
  - Assert `knowledge_graph::get_snapshot` returns non-empty nodes.
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test project_bootstrap_tests -- --nocapture`
- [ ] Expected before implementation: at least the failing-provider test fails because the old code returns `Ok(project)`.

### Task 2: Make Required Bootstrap Failures Fatal

- [ ] Modify `bootstrap_novel` so Bible model errors return `Err("Bible generation failed: ...")` after deleting the just-created project.
- [ ] Add `cleanup_partial_project(db, project_id, reason) -> String` in `novel_bootstrap.rs`:
  - Call `projects::delete_project(db, project_id)`.
  - Return `reason` if cleanup succeeds.
  - Return `format!("{reason}; cleanup failed: {cleanup_error}")` if cleanup fails.
- [ ] Wrap required post-project work in a helper path so errors from Bible insertion and validation call cleanup before returning.
- [ ] Keep paper directory creation and metadata persistence required, because existing successful bootstrap already depends on a project storage directory.

### Task 3: Propagate Bible Insert Errors

- [ ] Replace ignored `let _ = conn.execute(...)` calls for style guides, characters, locations, organizations, items, world lore, power systems, canon rules, plot threads, and chapter plans with `conn.execute(...).map_err(|e| format!("Insert <entity>: {}", e))?`.
- [ ] Keep `INSERT OR IGNORE` semantics where already used, but do not ignore database execution errors.
- [ ] Insert `target_word_count` from each chapter plan when present.
- [ ] Run the targeted test command again and fix compile or behavior failures.

### Task 4: Validate Required Artifacts Before Success

- [ ] Add `validate_bootstrap_artifacts(db, project_id) -> Result<(), String>` in `novel_bootstrap.rs`.
- [ ] Load Bible data with `db::bible::get_bible`.
- [ ] Load chapter plans with `db::chapters::get_chapter_plans`.
- [ ] Load graph snapshot with `db::knowledge_graph::get_snapshot`.
- [ ] Check the required counts from the spec exactly, including exactly 10 chapter plans.
- [ ] Return a single error string listing every missing or insufficient artifact, prefixed with `Bootstrap validation failed`.
- [ ] Call validation after `insert_bible_records` and before vector indexing.
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test project_bootstrap_tests -- --nocapture`
- [ ] Expected after implementation: all project bootstrap tests pass.

### Task 5: Regression and Full Verification

- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test benchmark benchmark_full_pipeline -- --nocapture`
- [ ] Run:
  `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml`
- [ ] Run from `tauri-app`:
  `npm run build`
- [ ] Run:
  `git diff --check`
- [ ] Review `git diff --stat` and confirm only project bootstrap docs, tests, and workflow code changed.
