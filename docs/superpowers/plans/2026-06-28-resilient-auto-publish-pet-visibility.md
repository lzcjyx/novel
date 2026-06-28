# Resilient Auto Publish And Pet Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement portable scheduled writing/auto publishing with durable recovery and fix pet visibility when enabled.

**Architecture:** Extend the existing Tauri + Rust + SQLite app. Use `publication_queue` as a durable outbox, add a Firefly Git adapter behind portable target settings, expose scheduler controls in the shell, and make pet show operations clamp and force visibility.

**Tech Stack:** Rust 2021, rusqlite, std::process Git/pnpm integration, Tauri v2, React 19, TypeScript, Node test runner.

---

## File Structure

- Create `tauri-app/src-tauri/src/models/publication.rs`: target config, queue item, adapter result structs.
- Modify `tauri-app/src-tauri/src/models/mod.rs`: export publication models.
- Create `tauri-app/src-tauri/src/db/publication_queue.rs`: queue upsert, claim, recover, update, list helpers.
- Modify `tauri-app/src-tauri/src/db/mod.rs`: export publication queue module.
- Create `tauri-app/src-tauri/src/workflow/static_site_publish.rs`: Firefly/static-site markdown rendering and publish runner.
- Modify `tauri-app/src-tauri/src/workflow/mod.rs`: export static-site publishing module.
- Modify `tauri-app/src-tauri/src/models/settings.rs` and `tauri-app/src-tauri/src/db/settings.rs`: persist scheduler and publication target settings.
- Modify `tauri-app/src-tauri/src/lib.rs`: add commands, enqueue publish-ready chapters, recover interrupted queue items, improve pet visibility.
- Modify `tauri-app/src/lib/tauriClient.ts`: add typed publication/scheduler wrappers.
- Modify `tauri-app/src/App.tsx`: add navigation `定时写作` toggle and settings fields.
- Modify `tauri-app/src/fluentTokens.test.mjs` and `tauri-app/src/lib/tauriClient.contract.test.mjs`: UI/client contract tests.
- Create `tauri-app/src-tauri/tests/publication_queue_tests.rs`: backend publication tests.
- Modify `tauri-app/src-tauri/tests/db_tests.rs`: settings and pet visibility tests.

## Tasks

### Task 1: Publication Settings And UI Contracts

- [ ] Add failing Rust test proving scheduler and target settings round-trip.
- [ ] Add failing Node tests proving the shell contains `定时写作`, `publish_schedule_enabled`, `publication_target_path`, and client wrappers.
- [ ] Add settings fields with conservative defaults.
- [ ] Save/load settings.
- [ ] Add shell toggle and settings controls.
- [ ] Run targeted Rust and Node tests until green.

### Task 2: Durable Publication Queue

- [ ] Add failing tests for idempotent queue upsert, stale `publishing` recovery, and due-item selection.
- [ ] Implement `db::publication_queue`.
- [ ] Add queue models.
- [ ] Wire queue recovery on startup.
- [ ] Run targeted backend tests until green.

### Task 3: Firefly Static Site Adapter

- [ ] Add failing tests for Firefly frontmatter rendering, path sanitization, dirty repo blocking, and generated-file-only staging command planning.
- [ ] Implement Markdown/frontmatter rendering without secrets.
- [ ] Implement target validation against `.git`, `src/content.config.ts`, and posts directory.
- [ ] Implement command runner abstraction so tests can verify Git/pnpm commands without network.
- [ ] Run targeted backend tests until green.

### Task 4: Publish-Ready Integration

- [ ] Add failing test proving a `publish_ready` generation with scheduling enabled enqueues one queue row.
- [ ] Replace direct auto-publish draft creation with queue upsert plus existing local draft metadata.
- [ ] Add commands to list queue items, retry failed items, and process due items.
- [ ] Run targeted backend tests until green.

### Task 5: Pet Visibility Regression

- [ ] Add failing tests for clamping off-screen pet coordinates and show command forcing visible state.
- [ ] Refactor pet window preference logic into testable pure helpers.
- [ ] Clamp coordinates before setting position and before saving dragged position.
- [ ] Make explicit show call show, unminimize, and focus the pet window.
- [ ] Ensure the pet window renders a default visible state before receiving runtime events.
- [ ] Run targeted Rust and Node tests until green.

### Task 6: Full Verification

- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`.
- [ ] Run `node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs`.
- [ ] Run `npm run build` from `D:\novel\tauri-app`.
- [ ] Run `git diff --check`.
- [ ] Inspect `git status -sb` and ensure only aligned changes are present.

## Self-Review

- Spec coverage: settings, scheduler UI, queue outbox, Firefly adapter, privacy, recovery, and pet visibility are covered by tasks.
- Placeholder scan: no task contains open-ended placeholder text.
- Type consistency: publication settings, queue, adapter, shell toggle, and pet helper names are consistent with the intended implementation.
