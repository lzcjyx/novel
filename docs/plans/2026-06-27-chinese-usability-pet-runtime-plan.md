# Chinese Usability and Pet Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix project deletion on migrated databases, make the desktop app Chinese-first, make Graph nodes self-explanatory, and add a low-resource configurable status pet.

**Architecture:** Keep the current Tauri + Rust + SQLite + React architecture. Repair database schema in `db::migrations`, persist pet preferences through `AppSettings`, keep Graph label helpers in the existing graph layout module, and implement the pet as a lightweight React/CSS component inside the existing shell.

**Tech Stack:** Rust 2021, rusqlite, serde, Tauri v2 commands already present, React 19, TypeScript, Vite, node test runner.

---

## File Structure

Modify:

- `tauri-app/src-tauri/src/db/migrations.rs` - prevent future temporary-table foreign key pollution and repair existing polluted schemas.
- `tauri-app/src-tauri/src/models/settings.rs` - add pet settings to `AppSettings`.
- `tauri-app/src-tauri/src/db/settings.rs` - load and save pet settings.
- `tauri-app/src-tauri/tests/db_tests.rs` - migration regression and pet settings persistence tests.
- `tauri-app/src/graphLayout.js` - exported Chinese Graph display helpers.
- `tauri-app/src/graphLayout.d.ts` - helper type declarations.
- `tauri-app/src/graphLayout.test.mjs` - Graph helper tests.
- `tauri-app/src/App.tsx` - Chinese copy, Graph node rendering, pet component, and pet settings controls.
- `tauri-app/src/pages/AuthorControlPage.tsx` - Chinese-visible Author Control copy.
- `tauri-app/src/pages/RuntimePage.tsx` - Chinese-visible Runtime copy.
- `tauri-app/src/index.css` - Graph node label layout and pet styling.
- `tauri-app/src/fluentTokens.test.mjs` - update shell/Graph/pet contract assertions.
- `tauri-app/src/authorControlPage.contract.test.mjs` - update Author Control labels.
- `tauri-app/src/runtimePage.contract.test.mjs` - update Runtime page labels.
- `tauri-app/src/lib/tauriClient.contract.test.mjs` - only if settings command wrappers become necessary; otherwise leave unchanged.

## Tasks

### Task 1: Deletion regression test

- [ ] Add a Rust test that creates a database whose `agent_reviews` foreign key references `chapter_versions_old_type_migration`.
- [ ] Insert a project, chapter, chapter version, and review row.
- [ ] Drop the temporary table so the schema matches the user failure.
- [ ] Run `tauri_app_lib::db::run_migrations(&db)`.
- [ ] Assert the repaired `agent_reviews` table SQL references `chapter_versions(id)`.
- [ ] Call `tauri_app_lib::db::projects::delete_project(&db, project_id)` and assert it succeeds.
- [ ] Run the targeted test and confirm it fails before production changes.

### Task 2: Migration repair

- [ ] Update `migrate_chapter_versions_accepted_candidate_type` so future table rebuilds set `PRAGMA legacy_alter_table = ON` only around the temporary rename, then restore it.
- [ ] Add `repair_chapter_version_temp_foreign_keys` called from `run_migrations` after chapter version migration.
- [ ] Detect tables whose `sqlite_master.sql` contains `REFERENCES "chapter_versions_old_type_migration"` or `REFERENCES chapter_versions_old_type_migration`.
- [ ] Rebuild each polluted table with the same columns, rows, and indexes but corrected table SQL.
- [ ] Keep repair scoped to known app tables that may reference chapter versions.
- [ ] Run the targeted Rust test and then the full db test file.

### Task 3: Pet settings tests and persistence

- [ ] Add a Rust settings test that writes `pet_enabled = false`, `pet_animation_level = "static"`, and `pet_compact_mode = true`, saves settings, reloads settings, and asserts values round-trip.
- [ ] Add fields to `AppSettings` with defaults: enabled, subtle animation, non-compact.
- [ ] Load and save these fields through `system_settings`.
- [ ] Run the targeted settings test until green.

### Task 4: Graph helper tests and implementation

- [ ] Add tests for `graphTypeLabel`, `graphNodeBadge`, and `graphNodeDisplayLabel`.
- [ ] Implement helpers in `graphLayout.js` and declarations in `graphLayout.d.ts`.
- [ ] Update `App.tsx` Graph node rendering to show:
  - Chinese type badge
  - clamped visible node label
  - degree pill
- [ ] Update CSS so node dimensions are stable and readable.
- [ ] Run node Graph tests until green.

### Task 5: Chinese copy contracts and localization

- [ ] Update source contract tests to assert Chinese labels for navigation, Graph, Settings, Learn, Author Control, and Runtime.
- [ ] Translate high-visibility user copy in `App.tsx`, `AuthorControlPage.tsx`, and `RuntimePage.tsx`.
- [ ] Preserve technical names and JSON/provider terms.
- [ ] Run node contract tests until green.

### Task 6: Pet UI implementation

- [ ] Add `AppPet` in `App.tsx` using existing `settings`, `status`, `loading`, `msg`, and `selected` state.
- [ ] Render nothing when `settings.pet_enabled` is false.
- [ ] Render state classes for idle, working, attention, context-limited, and waiting.
- [ ] Add Settings controls for enable switch, animation level, and compact mode.
- [ ] Add CSS for lightweight pet shape, status line, compact mode, animation levels, and reduced motion.
- [ ] Run node contract tests and build.

### Task 7: Full verification

- [ ] Run `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`.
- [ ] Run `node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs`.
- [ ] Run `npm run build` in `D:\novel\tauri-app`.
- [ ] Run `git diff --check`.
- [ ] Fix any failures without reducing scope or changing the goal.
