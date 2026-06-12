# Generation Review Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix Reviews black-screen rendering, RAG-off continuity degradation, and revise/post-processing job finalization failures.

**Architecture:** Keep the existing Tauri + React + SQLite workflow. Add small guard helpers at the existing UI and workflow seams: nullable number formatting in React, explicit retrieval status in Rust, prompt guidance for continuity review, and non-critical post-processing handling after chapter persistence.

**Tech Stack:** Rust 2021, Tauri 2, Tokio, rusqlite, React, TypeScript, Vite.

---

## File Structure

- Modify `tauri-app/src/App.tsx`: nullable score formatting and Jobs auto-refresh.
- Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`: skip vector retrieval when embedding provider is absent; make post-review post-processing non-critical.
- Modify `tauri-app/src-tauri/prompts/continuity_reviewer.md`: RAG-off continuity instruction.
- Modify `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`: regression tests for no implicit embedding fallback and post-processing-safe finalization where reachable.
- Modify `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`: retrieval disabled phase metadata test.

## Tasks

### Task 1: Reviews Nullable Score Crash

- [ ] Add `isFiniteNumber`, `formatScore`, and `scoreTone` helpers in `App.tsx`.
- [ ] Replace Reviews `.toFixed()` calls on nullable backend fields with the helpers.
- [ ] Build with `npm run build` to prove TypeScript accepts the nullable guards.

### Task 2: Explicit RAG-Off Retrieval

- [ ] Add a regression test proving `generate_next_chapter(..., emb_provider: None, ...)` does not call the main provider's `embed()`.
- [ ] Change `chapter_production.rs` so retrieval is attempted only when `emb_provider` is `Some`.
- [ ] Emit and persist `retrieve_context` detail as `RAG disabled; using structured context` when embedding is not configured.
- [ ] Run the targeted core writing loop test.

### Task 3: Continuity Prompt Semantics

- [ ] Add a prompt regression assertion that the continuity reviewer prompt says missing RAG/vector search is not a blocking continuity issue.
- [ ] Update `tauri-app/src-tauri/prompts/continuity_reviewer.md` with that instruction.
- [ ] Run the targeted prompt test.

### Task 4: Job Finalization After Revise

- [ ] Wrap canon update after export in a non-critical branch that logs and records `update_canon` as `skipped` or `failed_noncritical` instead of returning `Err`.
- [ ] Keep self-reflection best-effort as it already is.
- [ ] Ensure the terminal `complete` event and `update_job_status` still run after non-critical post-processing problems.
- [ ] Run core writing loop tests and generation job observability tests.

### Task 5: Jobs Page Live Refresh

- [ ] Replace one-shot Jobs fetch with `loadJobs()` plus a timer while the page is mounted.
- [ ] Refresh Jobs after `app-resume`, focus, and visibility restore if the page is open.
- [ ] Build with `npm run build`.

### Task 6: Verification

- [ ] Run `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture`.
- [ ] Run `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture`.
- [ ] Run `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml`.
- [ ] Run `npm run build` in `tauri-app`.
- [ ] Run `git diff --check`.
- [ ] Review diff for unrelated churn and leftover debug output.

