# Chinese Usability and Pet Runtime Spec

Date: 2026-06-27

## Goal

Improve AI Novel Factory for Chinese-first desktop use by fixing project deletion on migrated databases, localizing visible English product copy, making Graph nodes readable without relying on initials, and adding a low-resource desktop pet that reflects app status and can be configured or disabled.

## Requirements

### 1. Project deletion migration repair

- Deleting a project must not fail with `Delete project: no such table: main.chapter_versions_old_type_migration`.
- Future migrations must not rewrite dependent foreign keys to the temporary table name used while rebuilding `chapter_versions`.
- Existing user databases already polluted by that migration must be repaired automatically during startup migration.
- Repair must preserve existing rows in dependent tables such as reviews, quality scores, hard facts, feedback decisions, and other tables that reference `chapter_versions`.
- The fix must be validated by a regression test that creates the broken schema state, runs migrations, and then deletes a project through `db::projects::delete_project`.

### 2. Chinese-first visible copy

- Top-level navigation, command bar status, dashboard controls, project creation/deletion prompts, Graph UI, Settings, Learn, Author Control, and Runtime page section headings should use Chinese labels.
- Product names and technical names that Chinese users expect to see in English remain unchanged, including `AI Novel Factory`, `DeepSeek`, `OpenAI`, `API Key`, `RAG`, `JSON`, model IDs, URLs, and command identifiers.
- Error prefixes should be understandable to Chinese users. User-facing UI should prefer `错误：...` over `Error: ...`.
- Existing tests that depend on visible English copy must be updated to assert the new Chinese copy.

### 3. Graph node readability

- Graph canvas nodes must show enough information to identify what they represent at a glance.
- Nodes must display a Chinese type badge and a short node label, not only uppercase initials.
- Long labels must be clamped visually so nodes stay stable and do not overlap excessively.
- The inspector, filters, counters, empty states, relationship editor, tooltips, and accessibility labels should use Chinese labels.
- Node type mapping must include known canon types: character, location, organization, item, plot thread, foreshadowing, canon rule, timeline event, world lore, magic system, and unknown fallback.

### 4. Low-resource desktop pet

- The app should show a small pet by default when opened.
- The pet must reflect software status:
  - no selected project: waiting for project selection
  - generation running or local loading: working
  - ready: idle
  - recent error message: attention
  - missing embedding provider: context-limited hint
- The pet must not require a new runtime dependency or high-frequency rendering loop.
- Settings must include a pet section with:
  - enable/disable switch
  - animation level: static, subtle, lively
  - compact mode toggle
- Settings must persist through the existing `AppSettings` and `system_settings` path.
- Reduced-motion users and static animation mode must not get continuous pet animation.

## Non-Goals

- Do not redesign the whole application shell.
- Do not introduce a 3D/canvas pet, WebGL, or new animation library.
- Do not translate prompt templates, schema field names, model provider names, or exported package keys.
- Do not downgrade the previous Author Control Runtime work.

## Acceptance Tests

- Rust migration/settings tests:
  - broken `chapter_versions_old_type_migration` foreign-key references are repaired and project delete succeeds.
  - pet settings round-trip through `save_settings` and `get_settings`.
- Frontend node tests:
  - Graph type labels and badges return Chinese names.
  - Graph display labels clamp long names.
- Frontend contract tests:
  - key navigation/Graph/Settings/Learn/Author Control/Runtime copy is Chinese.
  - Graph nodes render a type badge plus visible short label.
  - pet component and pet settings are present.
- Full verification:
  - `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`
  - `node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs`
  - `npm run build` from `D:\novel\tauri-app`
  - `git diff --check`
