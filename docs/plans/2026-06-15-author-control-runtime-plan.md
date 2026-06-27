# Author Control Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the missing author-control capabilities from `docs/analysis/2026-06-15-github-longform-agent-framework-comparison.md` without regressing the existing local chapter production runtime.

**Architecture:** Keep SQLite as authoritative state and expose every new surface through focused Rust modules, Tauri commands in `commands::runtime`, typed frontend calls in `src/lib/tauriClient.ts`, and page/feature modules outside `App.tsx`. Each new workflow must be deterministic under fake model tests and auditable through metadata, prompt/context hashes, and job events.

**Tech Stack:** Rust 2021, Tauri v2, rusqlite, serde/serde_json, React 19, TypeScript, Vite, existing node contract tests.

---

## File Structure

Create:

- `tauri-app/src-tauri/src/db/director.rs` - CRUD for direction candidates and selected direction state.
- `tauri-app/src-tauri/src/workflow/director_mode.rs` - inspiration-to-direction candidate generation, revision, selection, and bootstrap handoff payloads.
- `tauri-app/src-tauri/tests/director_mode_tests.rs` - deterministic fake-model tests for director candidates.
- `tauri-app/src-tauri/src/db/hard_facts.rs` - CRUD and status transitions for the hard fact ledger.
- `tauri-app/src-tauri/src/workflow/hard_fact_ledger.rs` - extraction payload parsing, relevance selection, context serialization, and contradiction precheck.
- `tauri-app/src-tauri/tests/hard_fact_ledger_tests.rs` - hard fact persistence, context selection, contradiction, and recovery tests.
- `tauri-app/src-tauri/src/db/style_assets.rs` - CRUD for style assets.
- `tauri-app/src-tauri/src/workflow/style_assets.rs` - style asset compilation into prompt/runtime payloads and anti-AI rule extraction.
- `tauri-app/src-tauri/tests/style_asset_tests.rs` - style asset compile and review/precheck integration tests.
- `tauri-app/src/pages/AuthorControlPage.tsx` - Director, Hard Facts, Style Assets, Memory Banks, and Run Artifacts UI entry point.
- `tauri-app/src/authorControlPage.contract.test.mjs` - frontend command and page-locality contract tests.

Modify:

- `tauri-app/src-tauri/migrations/001_init_sqlite.sql` - add director, hard fact, style asset, user recipe, feedback decision, and run artifact tables.
- `tauri-app/src-tauri/src/db/mod.rs` - register new DB modules.
- `tauri-app/src-tauri/src/workflow/mod.rs` - register new workflow modules.
- `tauri-app/src-tauri/src/workflow/writing_context.rs` - include hard facts, compiled style assets, manual pins, and unified source trace.
- `tauri-app/src-tauri/src/workflow/context_activation.rs` - use `entity_refs` as activation targets.
- `tauri-app/src-tauri/src/workflow/chapter_production.rs` - persist hard fact/style contribution metadata and task-owned row tracking.
- `tauri-app/src-tauri/src/workflow/canon_consistency.rs` - add hard fact contradiction and style asset anti-AI checks.
- `tauri-app/src-tauri/src/workflow/task_transaction.rs` - recover task-owned hard facts, style learning drafts, and run artifact records.
- `tauri-app/src-tauri/src/workflow/package_io.rs` - complete Bible and project package round-trip for all structured surfaces.
- `tauri-app/src-tauri/src/workflow/operator_recipes.rs` - support user recipes and richer step outputs.
- `tauri-app/src-tauri/src/extensions/host.rs` - convert declarative extension package kinds into real prompt/context/rubric/recipe/export contributions.
- `tauri-app/src-tauri/src/extensions/manifest.rs` - validate contribution payloads for the new package kinds.
- `tauri-app/src-tauri/src/commands/runtime.rs` - expose new commands for Director Mode, hard facts, style assets, memory banks, user recipes, feedback decisions, and artifacts.
- `tauri-app/src-tauri/src/lib.rs` - register new runtime commands.
- `tauri-app/src/lib/tauriClient.ts` - add typed command wrappers.
- `tauri-app/src/lib/tauriClient.contract.test.mjs` - lock new command names.
- `tauri-app/src/App.tsx` - add only navigation wiring to `AuthorControlPage`.
- `tauri-app/src/pages/RuntimePage.tsx` - link to Author Control where the existing runtime page is too low-level.

## Execution Order

1. Data contracts and migration.
2. Director Mode MVP.
3. Hard Fact Ledger MVP.
4. Style Asset MVP.
5. Writing context integration.
6. Package round-trip completion.
7. Runtime and Author Control UI.
8. User recipe authoring and feedback decision loop.
9. Extension contribution adapters.
10. Run artifacts and human-readable audit export.
11. Long task recovery summaries.
12. Full verification and completion audit.

## Task 1: Data Contracts and Migration

**Files:**

- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Modify: `tauri-app/src-tauri/src/db/mod.rs`
- Create: `tauri-app/src-tauri/src/db/director.rs`
- Create: `tauri-app/src-tauri/src/db/hard_facts.rs`
- Create: `tauri-app/src-tauri/src/db/style_assets.rs`
- Test: `tauri-app/src-tauri/tests/director_mode_tests.rs`
- Test: `tauri-app/src-tauri/tests/hard_fact_ledger_tests.rs`
- Test: `tauri-app/src-tauri/tests/style_asset_tests.rs`

- [ ] **Step 1: Write failing migration/CRUD tests**

Add tests that create an in-memory database with existing migrations and assert these rows round-trip:

```rust
#[test]
fn director_candidate_round_trips_selected_checkpoint_state() {
    let db = setup_db("director-candidate-roundtrip.db");
    let candidate_id = tauri_app_lib::db::director::upsert_direction_candidate(
        &db,
        &tauri_app_lib::db::director::DirectionCandidateInput {
            id: Some("dir-1".to_string()),
            project_id: Some("project-1".to_string()),
            inspiration: "雨夜车站里，一张旧票据揭开灵税阴谋。".to_string(),
            title_options: vec!["雨站账本".to_string(), "旧票据".to_string()],
            positioning: "悬疑仙侠长篇".to_string(),
            target_reader: "喜欢强情节和硬设定的网文读者".to_string(),
            core_hook: "每张票据都改写一个人的命运".to_string(),
            series_promise: "查清灵税体系背后的旧朝祭司网络".to_string(),
            first_30_chapter_promise: "主角从票据线索查到第一座地下灵脉。".to_string(),
            world_seed: serde_json::json!({"factions": ["镇岳军", "旧朝祭司"]}),
            character_seed: serde_json::json!({"protagonist": "沈砚"}),
            volume_strategy: serde_json::json!([{"volume": 1, "goal": "查清票据来源"}]),
            golden_three_chapters: serde_json::json!([
                {"chapter": 1, "hook": "票据死人"},
                {"chapter": 2, "hook": "灵税账本"},
                {"chapter": 3, "hook": "旧站追杀"}
            ]),
            checkpoint_status: "draft".to_string(),
            revision_note: Some("first pass".to_string()),
            selected: false,
            metadata: serde_json::json!({"prompt_hash": "hash-1"}),
        },
    ).expect("candidate should persist");

    let loaded = tauri_app_lib::db::director::list_direction_candidates(&db, Some("project-1"))
        .expect("candidates should load");
    assert_eq!(candidate_id, "dir-1");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].title_options[0], "雨站账本");
    assert_eq!(loaded[0].checkpoint_status, "draft");
}
```

```rust
#[test]
fn hard_fact_round_trips_status_and_source_quote() {
    let db = setup_db("hard-fact-roundtrip.db");
    let fact_id = tauri_app_lib::db::hard_facts::upsert_hard_fact(
        &db,
        &tauri_app_lib::db::hard_facts::HardFactInput {
            id: Some("fact-1".to_string()),
            project_id: "project-1".to_string(),
            chapter_id: Some("chapter-1".to_string()),
            chapter_version_id: Some("version-1".to_string()),
            fact_type: "amount".to_string(),
            subject: "灵税票据".to_string(),
            predicate: "records_amount".to_string(),
            object: "三百枚灵石".to_string(),
            value_text: "灵税票据金额为三百枚灵石".to_string(),
            certainty: 0.97,
            source_quote: Some("票面写着三百枚灵石。".to_string()),
            scope: "project".to_string(),
            status: "active".to_string(),
            metadata: serde_json::json!({"source": "chapter_final"}),
        },
    ).expect("hard fact should persist");

    let facts = tauri_app_lib::db::hard_facts::list_hard_facts(&db, "project-1", true)
        .expect("hard facts should load");
    assert_eq!(fact_id, "fact-1");
    assert_eq!(facts[0].object, "三百枚灵石");
    assert_eq!(facts[0].status, "active");
}
```

```rust
#[test]
fn style_asset_round_trips_features_and_anti_ai_rules() {
    let db = setup_db("style-asset-roundtrip.db");
    let asset_id = tauri_app_lib::db::style_assets::upsert_style_asset(
        &db,
        &tauri_app_lib::db::style_assets::StyleAssetInput {
            id: Some("style-asset-1".to_string()),
            project_id: "project-1".to_string(),
            name: "克制悬疑动作".to_string(),
            asset_type: "prose_rule".to_string(),
            scope_type: "project".to_string(),
            scope_id: None,
            features: serde_json::json!({"cadence": "short action chains"}),
            positive_examples: vec!["他把杯口转向墙角。".to_string()],
            negative_examples: vec!["他心中充满了复杂情绪。".to_string()],
            anti_ai_rules: serde_json::json!({"forbidden_phrases": ["眼中闪过"]}),
            enabled: true,
            priority: 20,
            metadata: serde_json::json!({"source": "manual"}),
        },
    ).expect("style asset should persist");

    let assets = tauri_app_lib::db::style_assets::list_style_assets(&db, "project-1", true)
        .expect("style assets should load");
    assert_eq!(asset_id, "style-asset-1");
    assert_eq!(assets[0].positive_examples[0], "他把杯口转向墙角。");
    assert!(assets[0].enabled);
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test director_mode_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test hard_fact_ledger_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test style_asset_tests -- --nocapture
```

Expected: fail because the new modules and functions do not exist.

- [ ] **Step 3: Add schema and CRUD modules**

Add SQLite tables with `created_at` and `updated_at` fields and indexes on `project_id`, `status`, and `enabled` where applicable. Implement inputs and row structs with explicit JSON serialization helpers copied from the style of `db::context_rules` and `db::prompt_presets`.

- [ ] **Step 4: Register DB modules**

Add:

```rust
pub mod director;
pub mod hard_facts;
pub mod style_assets;
```

to `tauri-app/src-tauri/src/db/mod.rs`.

- [ ] **Step 5: Run tests and verify GREEN**

Run the same three commands from Step 2.

Expected: all three test files pass.

## Task 2: Director Mode MVP

**Files:**

- Create: `tauri-app/src-tauri/src/workflow/director_mode.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Modify: `tauri-app/src/lib/tauriClient.contract.test.mjs`
- Test: `tauri-app/src-tauri/tests/director_mode_tests.rs`

- [ ] **Step 1: Write failing generation and selection tests**

Add a fake `ModelClient` that returns JSON with exactly two direction candidates. Assert `generate_direction_candidates` stores both, records prompt hash metadata, and `select_direction_candidate` marks one selected while clearing selection on siblings for the same project/inspiration group.

- [ ] **Step 2: Run director tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test director_mode_tests -- --nocapture
```

Expected: fail because workflow functions are missing.

- [ ] **Step 3: Implement workflow functions**

Expose:

```rust
pub async fn generate_direction_candidates(
    db: &Database,
    provider: &dyn ModelClient,
    request: DirectionGenerationRequest,
) -> Result<Vec<DirectionCandidate>, String>

pub fn select_direction_candidate(
    db: &Database,
    candidate_id: &str,
    revision_note: Option<&str>,
) -> Result<DirectionCandidate, String>

pub fn build_bootstrap_handoff(
    db: &Database,
    candidate_id: &str,
) -> Result<DirectorBootstrapHandoff, String>
```

The workflow must parse strict JSON, validate 2 to 3 candidates, and never auto-create a project without explicit bootstrap action.

- [ ] **Step 4: Add Tauri commands and typed client wrappers**

Add command wrappers:

- `generate_direction_candidates`
- `list_direction_candidates`
- `select_direction_candidate`
- `get_director_bootstrap_handoff`

Add matching methods to `tauriClient.ts` and expected command names to `tauriClient.contract.test.mjs`.

- [ ] **Step 5: Verify director mode**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test director_mode_tests -- --nocapture
node --test tauri-app/src/lib/tauriClient.contract.test.mjs
```

Expected: director tests pass and command contract includes the four new commands.

## Task 3: Hard Fact Ledger MVP

**Files:**

- Create: `tauri-app/src-tauri/src/workflow/hard_fact_ledger.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Modify: `tauri-app/src-tauri/src/workflow/writing_context.rs`
- Modify: `tauri-app/src-tauri/src/workflow/canon_consistency.rs`
- Modify: `tauri-app/src-tauri/src/workflow/task_transaction.rs`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Test: `tauri-app/src-tauri/tests/hard_fact_ledger_tests.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`
- Test: `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`

- [ ] **Step 1: Write failing context and contradiction tests**

Add tests proving:

- `select_relevant_hard_facts` returns active facts whose subject/object appears in the chapter plan title, summary, required characters, required locations, or operator notes.
- `build_writing_context` includes `hard_facts` and source trace entries.
- `detect_hard_fact_contradictions` flags a chapter that changes "三百枚灵石" to "五百枚灵石" for the same subject/predicate.
- interrupted job recovery removes task-owned hard facts.

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test hard_fact_ledger_tests -- --nocapture
```

Expected: fail because workflow integration is missing.

- [ ] **Step 3: Implement hard fact selection and context payload**

Add:

```rust
pub fn select_relevant_hard_facts(
    db: &Database,
    project_id: &str,
    plan: &ChapterPlan,
    operator_controls: Option<&OperatorControls>,
    limit: usize,
) -> Result<Vec<HardFact>, String>
```

Serialize selected facts into `WritingContextPackage.hard_facts` and add trace records with `source_type = "hard_fact"` and `source_key = "hard_fact:<id>"`.

- [ ] **Step 4: Implement contradiction precheck**

Add deterministic comparison for same `subject + predicate` where the active fact `object` or normalized `value_text` differs from explicit numeric/amount/location/ownership text in the candidate chapter. Return a blocking `CanonIssue` with `rule_type = "hard_fact_conflict"`.

- [ ] **Step 5: Add recovery tracking**

Extend task snapshot owned-row lists and recovery delete/restore logic to include hard fact IDs created during the task.

- [ ] **Step 6: Verify hard fact ledger**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test hard_fact_ledger_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture
```

Expected: all pass with no recovery regressions.

## Task 4: Style Asset MVP

**Files:**

- Create: `tauri-app/src-tauri/src/workflow/style_assets.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Modify: `tauri-app/src-tauri/src/workflow/writing_context.rs`
- Modify: `tauri-app/src-tauri/src/workflow/canon_consistency.rs`
- Modify: `tauri-app/src-tauri/src/workflow/review_agents.rs`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Test: `tauri-app/src-tauri/tests/style_asset_tests.rs`
- Test: `tauri-app/src-tauri/tests/canon_consistency_tests.rs`

- [ ] **Step 1: Write failing style compile and anti-AI tests**

Assert:

- active style assets are ordered by priority then name.
- compiled payload includes features, positive examples, negative examples, and anti-AI rules.
- `build_writing_context` includes `style_assets` inside the style section.
- canon consistency precheck flags a forbidden phrase from a style asset.

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test style_asset_tests -- --nocapture
```

Expected: fail because style asset workflow integration is missing.

- [ ] **Step 3: Implement style asset compiler**

Add:

```rust
pub fn compile_style_assets(
    db: &Database,
    project_id: &str,
    scope: StyleAssetScope,
) -> Result<CompiledStyleAssetPayload, String>
```

The payload must be deterministic and include `asset_ids`, `prompt_instructions`, `positive_examples`, `negative_examples`, and `anti_ai_rules`.

- [ ] **Step 4: Integrate into writing context and precheck**

Add compiled style asset data to `WritingContextPackage.style` and let `canon_consistency` read forbidden and required style patterns from both existing style guides and new style assets.

- [ ] **Step 5: Verify style assets**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test style_asset_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test canon_consistency_tests -- --nocapture
```

Expected: all pass and existing style guide behavior remains unchanged.

## Task 5: Context Activation Entity Refs and Manual Pins

**Files:**

- Modify: `tauri-app/src-tauri/src/workflow/context_activation.rs`
- Modify: `tauri-app/src-tauri/src/workflow/writing_context.rs`
- Modify: `tauri-app/src-tauri/tests/context_activation_tests.rs`
- Modify: `tauri-app/src-tauri/tests/rag_explainability_tests.rs`

- [ ] **Step 1: Write failing tests for entity refs and pins**

Assert a rule with `entity_refs = ["character:lin-bai"]` activates when a chapter plan requires the matching character source key, even if keywords do not match. Assert operator controls can pin `hard_fact:<id>` and `style_asset:<id>` source keys into the context trace.

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test context_activation_tests -- --nocapture
```

Expected: fail on entity-ref activation and manual pin behavior.

- [ ] **Step 3: Implement entity-ref activation**

Build an activation target set from plan required characters, locations, graph source keys, and operator controls. Treat an entity ref hit as `activation_reason = "entity_ref"`.

- [ ] **Step 4: Implement manual pins in context package**

Extend `OperatorControls` with stable fields:

```rust
pub pinned_source_keys: Vec<String>,
pub unpinned_source_keys: Vec<String>,
```

Pinned records enter trace with `source_reason = "manual_pin"` and unpinned records are excluded unless required by a hard fact conflict check.

- [ ] **Step 5: Verify context integration**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test context_activation_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test rag_explainability_tests -- --nocapture
```

Expected: all pass and existing graph/vector traces remain intact.

## Task 6: Complete Package Round Trip

**Files:**

- Modify: `tauri-app/src-tauri/src/workflow/package_io.rs`
- Modify: `tauri-app/src-tauri/tests/import_export_tests.rs`

- [ ] **Step 1: Write failing full Bible round-trip test**

Create a source project with rows in characters, locations, organizations, items, magic systems, world lore, timeline events, plot threads, foreshadowing, canon rules, style guides, style assets, and hard facts. Export a Bible package, import into a clean target project, and assert each table has the same stable content after ID remapping.

- [ ] **Step 2: Run import/export tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test import_export_tests -- --nocapture
```

Expected: fail because current `insert_bible_rows` imports only part of the Bible surfaces.

- [ ] **Step 3: Implement complete `insert_bible_rows`**

Follow existing row insertion style in `package_io.rs`. Preserve target `project_id`, stable package metadata, and rollback on any invalid row.

- [ ] **Step 4: Include new modules in project package**

Add style assets, hard facts, user recipes, feedback decisions, run artifact records, and extension contributions to project package validation and import/export.

- [ ] **Step 5: Verify round trip**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test import_export_tests -- --nocapture
```

Expected: all package tests pass, including invalid package rollback.

## Task 7: Author Control UI

**Files:**

- Create: `tauri-app/src/pages/AuthorControlPage.tsx`
- Create: `tauri-app/src/authorControlPage.contract.test.mjs`
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/lib/tauriClient.ts`
- Modify: `tauri-app/src/lib/tauriClient.contract.test.mjs`

- [ ] **Step 1: Write failing frontend contract tests**

Assert:

- `App.tsx` imports `AuthorControlPage`.
- nav labels include `authorControl: "Author Control"`.
- `AuthorControlPage.tsx` contains Director, Hard Facts, Style Assets, and Memory Banks headings.
- `tauriClient.ts` exposes director, hard fact, style asset, memory bank, and artifact commands.

- [ ] **Step 2: Run node tests and verify RED**

Run:

```powershell
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
```

Expected: fail until page and client methods exist.

- [ ] **Step 3: Implement page module and navigation wiring**

Keep `App.tsx` change limited to import, nav label/icon, and page switch. Put forms, lists, and command calls in `AuthorControlPage.tsx`.

- [ ] **Step 4: Verify frontend contracts**

Run:

```powershell
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
npm run build
```

Expected: contract tests and production build pass.

## Task 8: User Recipes and Feedback Decision Loop

**Files:**

- Modify: `tauri-app/src-tauri/src/workflow/operator_recipes.rs`
- Modify: `tauri-app/src-tauri/src/db/*`
- Modify: `tauri-app/src-tauri/src/commands/runtime.rs`
- Modify: `tauri-app/src-tauri/tests/operator_recipes_tests.rs`
- Create: `tauri-app/src-tauri/tests/feedback_decision_tests.rs`

- [ ] **Step 1: Write failing user recipe tests**

Assert user recipes persist, validate against the existing action whitelist, expose parameter schema, and export/import through project package.

- [ ] **Step 2: Write failing feedback decision tests**

Assert reader feedback can create a revision candidate with status `pending`, approval promotes it to a chapter version, rejection leaves it searchable but excluded from default context.

- [ ] **Step 3: Implement user recipe persistence and validation**

Use the existing built-in recipe action execution path. Store custom recipes in SQLite and reject unknown actions before any job row is created.

- [ ] **Step 4: Implement feedback decisions**

Add a workflow module that creates revision candidates and requires explicit approve/reject/defer commands before chapter or memory updates happen.

- [ ] **Step 5: Verify recipes and feedback**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test operator_recipes_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test feedback_decision_tests -- --nocapture
```

Expected: all pass.

## Task 9: Extension Contribution Adapters

**Files:**

- Modify: `tauri-app/src-tauri/src/extensions/manifest.rs`
- Modify: `tauri-app/src-tauri/src/extensions/host.rs`
- Modify: `tauri-app/src-tauri/src/workflow/prompt_runtime.rs`
- Modify: `tauri-app/src-tauri/src/workflow/context_activation.rs`
- Modify: `tauri-app/src-tauri/src/workflow/operator_recipes.rs`
- Modify: `tauri-app/src-tauri/tests/extension_host_tests.rs`

- [ ] **Step 1: Write failing extension contribution tests**

Assert enabled extension prompt packs alter assembled prompt input through `prompt_runtime`, context rule packs create visible activation rules, recipe packs add available recipes, and disabled extensions have no effect.

- [ ] **Step 2: Run extension tests and verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test extension_host_tests -- --nocapture
```

Expected: fail because contribution adapters are not wired into workflow inputs.

- [ ] **Step 3: Implement adapters**

Map package kinds to existing module inputs:

- `prompt_pack` -> prompt preset/unit contribution.
- `context_rule_pack` -> context rule contribution.
- `review_rubric_pack` -> reviewer rubric overlay.
- `recipe_pack` -> operator recipe contribution.
- `export_template` -> audit/export target contribution.

- [ ] **Step 4: Verify extension impact**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test extension_host_tests -- --nocapture
```

Expected: enabled contributions alter only their declared inputs and write trace entries.

## Task 10: Run Artifacts and Audit Export

**Files:**

- Create: `tauri-app/src-tauri/src/workflow/run_artifacts.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/src/export/mod.rs`
- Create: `tauri-app/src-tauri/src/export/audit.rs`
- Create: `tauri-app/src-tauri/tests/run_artifacts_tests.rs`

- [ ] **Step 1: Write failing artifact tests**

Assert a job with prompt/context/output/review metadata can write an artifact directory containing `status.json`, prompt markdown, context JSON, output markdown, review JSON, usage JSON, and events JSONL.

- [ ] **Step 2: Implement run artifact writer**

Write files under the existing project data directory. Store artifact paths in job metadata. On write failure, record an artifact error in metadata.

- [ ] **Step 3: Implement audit export**

Produce `bible/`, `state/chapter-NN/`, `timeline/history.md`, `memory/hard-facts.md`, and `memory/style-assets.md` from SQLite state without making files authoritative.

- [ ] **Step 4: Verify artifacts**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test run_artifacts_tests -- --nocapture
```

Expected: all artifact files are created in a temp directory and metadata records success.

## Task 11: Long Task Recovery Summaries

**Files:**

- Modify: `tauri-app/src-tauri/src/workflow/task_transaction.rs`
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`
- Modify: `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`

- [ ] **Step 1: Write failing recovery summary tests**

Assert an interrupted job metadata contains `recovery_summary` with `latest_phase`, `prompt_hash`, `context_hash`, `latest_artifact_ids`, and `operator_recovery_options`.

- [ ] **Step 2: Implement summary recording**

Update job metadata at phase boundaries and during recovery. Keep summary separate from Canon and learning memory.

- [ ] **Step 3: Verify recovery summaries**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture
```

Expected: interrupted jobs remain failed and auditable with recovery summaries.

## Task 12: Full Verification

**Files:**

- All changed files.

- [ ] **Step 1: Run Rust tests**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture
```

Expected: all Rust tests pass.

- [ ] **Step 2: Run frontend contract tests**

Run:

```powershell
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
```

Expected: all frontend contract tests pass.

- [ ] **Step 3: Run frontend build**

Run from `tauri-app`:

```powershell
npm run build
```

Expected: TypeScript and Vite build exit 0.

- [ ] **Step 4: Run diff hygiene**

Run:

```powershell
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 5: Completion audit**

Read `docs/specs/2026-06-15-author-control-runtime-spec.md` and verify each accepted behavior against current files and command output. Keep the goal active until each requirement has direct evidence.
