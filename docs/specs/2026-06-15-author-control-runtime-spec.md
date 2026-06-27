# Author Control Runtime Spec

Date: 2026-06-15

Source analysis: `docs/analysis/2026-06-15-github-longform-agent-framework-comparison.md`

## Goal

Turn AI Novel Factory from a working local production pipeline into an author-controlled long-form runtime. The project must keep its current Tauri + Rust + SQLite architecture and existing chapter production workflow, while adding the missing author-facing control surfaces identified in the GitHub long-form agent framework comparison: staged director mode, hard fact ledger, style assets, memory/audit surfaces, complete interop, effective declarative extensions, and inspectable run artifacts.

## Current State

The SillyTavern-inspired runtime foundation is already partly implemented:

- Prompt Runtime exists in `workflow::prompt_runtime` and `db::prompt_presets`.
- Context Activation exists in `workflow::context_activation` and `db::context_rules`.
- Model Profiles exist in `ai::provider_capabilities` and `db::model_profiles`.
- Operator Recipes exist in `workflow::operator_recipes`.
- Draft Alternatives exist in `db::draft_alternatives`.
- Project and Bible packages exist in `workflow::package_io`.
- Lorebook import exists in `workflow::lorebook_import`.
- Declarative Extension Host exists in `extensions::host` and `extensions::manifest`.
- Runtime UI has been split into `src/pages/RuntimePage.tsx`, with command calls centralized in `src/lib/tauriClient.ts`.

The new work must not regress those modules. It must deepen them and fill the gaps that remain against the comparison document.

## Non-Goals

- Do not replace Tauri, React, Rust, or SQLite.
- Do not introduce Docker, a required web server, Qdrant, Chroma, Prisma, or an external workflow engine.
- Do not turn the product into a chat-first roleplay frontend.
- Do not make the app fully autonomous without author checkpoints.
- Do not execute arbitrary extension code. Extensions remain declarative and permissioned.
- Do not hold SQLite transactions across model calls.

## Requirements

### 1. Preserve the Existing Runtime Foundation

Accepted behavior:

- Existing prompt runtime tests continue to prove deterministic unit ordering, phase filtering, unresolved variable errors, preview payloads, and prompt package round-trip.
- Existing context activation tests continue to prove keyword activation, secondary keyword filtering, sticky rules, cooldown, token clipping, and trace persistence.
- Existing provider capability tests continue to prove named profile persistence, workflow validation, and fallback behavior.
- Existing operator recipe tests continue to prove context-only runs, candidate generation, cancellation, job events, and metadata.
- Existing extension host tests continue to prove manifest validation, disabled behavior, permission denial, hook ordering, and trace output.
- Existing import/export tests continue to prove project package validation and rollback.

Verification:

- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test prompt_runtime_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test context_activation_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test provider_capability_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test operator_recipes_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test extension_host_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test import_export_tests -- --nocapture`

### 2. Director Mode

Accepted behavior:

- An author can start from one inspiration string and request 2 to 3 book direction candidates.
- Each direction candidate stores:
  - `id`
  - `project_id` if attached to an existing project, otherwise `null`
  - `inspiration`
  - `title_options`
  - `positioning`
  - `target_reader`
  - `core_hook`
  - `series_promise`
  - `first_30_chapter_promise`
  - `world_seed`
  - `character_seed`
  - `volume_strategy`
  - `golden_three_chapters`
  - `checkpoint_status`
  - `revision_note`
  - `selected`
  - `metadata`
- A selected direction can be promoted into the existing bootstrap path without bypassing human review.
- Direction candidate metadata records the prompt hash, model profile snapshot, generation time, and input inspiration.
- Direction candidate generation is deterministic in tests by using fake model output.
- Re-running or revising one candidate does not overwrite other candidates.

### 3. Hard Fact Ledger

Accepted behavior:

- Finalized or accepted chapter versions can produce hard facts separate from broad Canon.
- A hard fact stores:
  - `id`
  - `project_id`
  - `chapter_id`
  - `chapter_version_id`
  - `fact_type`
  - `subject`
  - `predicate`
  - `object`
  - `value_text`
  - `certainty`
  - `source_quote`
  - `scope`
  - `status`
  - `metadata`
- The writing context builder includes relevant active hard facts for the next chapter.
- The draft prompt preview exposes hard facts under a distinct `hard_facts` context section.
- Review precheck can flag direct contradictions against active hard facts before model review.
- Hard facts can be superseded or disputed without deleting their audit trail.
- Interrupted generation recovery must remove task-owned hard facts or restore their prior status.

### 4. Style Asset Module

Accepted behavior:

- Style assets are first-class data, not only free text inside style guides or learning entries.
- A style asset stores:
  - `id`
  - `project_id`
  - `name`
  - `asset_type`
  - `scope_type`
  - `scope_id`
  - `features`
  - `positive_examples`
  - `negative_examples`
  - `anti_ai_rules`
  - `enabled`
  - `priority`
  - `metadata`
- Style assets can be compiled into a deterministic prompt/runtime payload.
- Writing context includes the compiled active style asset payload.
- The style reviewer and canon consistency precheck can use anti-AI rules and required/forbidden patterns from style assets.
- Style asset contribution is traceable in chapter version metadata.
- Learning intake may create draft style assets, but the author must approve enabling them.

### 5. Context Rules and Memory Banks

Accepted behavior:

- Context Activation must use `entity_refs` as real activation targets, not only persisted metadata.
- Operator controls can pin and unpin source keys for the current generation run.
- Context preview explains rule hits, entity hits, graph paths, vector scores, manual pins, learning entries, hard facts, and style assets in one source trace.
- Memory Banks are author-facing views over existing durable sources:
  - Canon
  - Hard Facts
  - Character State
  - Timeline
  - Learning Entries
  - Style Assets
- Editing a memory bank entry uses the same backend command and validation path as the canonical module behind it.

### 6. Prompt Workbench Deepening

Accepted behavior:

- Prompt presets support versioned snapshots.
- Prompt units support parameters, temporary override values, and few-shot examples.
- A prompt preset can be cloned from a built-in or imported package.
- Prompt Workbench can dry-run an assembled prompt for a selected workflow and chapter plan without calling a model.
- Prompt A/B draft trials store separate prompt hashes and candidate metadata.
- Existing prompt unit ordering and trace behavior remains deterministic.

### 7. Operator Recipe Authoring

Accepted behavior:

- Built-in recipes remain declarative.
- User-authored recipes are stored in SQLite and use the same action whitelist as built-in recipes.
- A recipe can define a simple parameter schema for author input.
- Recipe runs show step input, output, status, error, duration, and generated artifact references.
- User recipes can be exported and imported.
- A failed step leaves a readable failure reason and does not partially promote chapter artifacts.

### 8. Reader Feedback Decision Loop

Accepted behavior:

- Reader feedback can create revision candidates without directly modifying chapter content or Canon.
- An author can approve, reject, or defer a feedback-driven revision candidate.
- Approved feedback revisions record the decision and update chapter version metadata.
- Rejected or deferred feedback remains searchable but does not pollute generation context unless explicitly pinned.

### 9. Complete Package Round Trip

Accepted behavior:

- Novel Bible package import/export round-trips all structured Bible surfaces:
  - characters
  - locations
  - organizations
  - items
  - magic or power systems
  - world lore
  - timeline events
  - plot threads
  - foreshadowing
  - canon rules
  - style guides
  - style assets
  - hard facts
- Project package import/export includes the above plus runtime surfaces:
  - context rules
  - prompt presets
  - model profiles
  - operator recipes
  - draft alternatives
  - extension packages
  - reader feedback decisions
- Invalid package import performs no partial writes.
- Import metadata preserves original IDs, source package format, imported timestamp, and remapped IDs.

### 10. Extension Adapters with Real Workflow Impact

Accepted behavior:

- Enabled declarative extensions can contribute concrete package kinds:
  - prompt packs
  - context rule packs
  - review rubric packs
  - recipe packs
  - export templates
- Extension contributions must be explicit, visible in Runtime UI, and disabled by default.
- Extension output changes workflow input only through existing module interfaces.
- Hook trace records which extension contribution changed which prompt/context/rubric/recipe/export input.
- Permission denial prevents the contribution and leaves a readable trace.

### 11. Human-Readable Audit Export and Run Artifacts

Accepted behavior:

- Each generation job can optionally export inspectable artifacts to a local run directory:
  - `status.json`
  - `prompt/system.md`
  - `prompt/user.md`
  - `context/package.json`
  - `context/trace.json`
  - `output/draft.md`
  - `reviews/*.json`
  - `usage.json`
  - `events.jsonl`
- Audit export can produce a Claude-Book-style sidecar:
  - `bible/`
  - `state/chapter-NN/`
  - `timeline/history.md`
  - `memory/hard-facts.md`
  - `memory/style-assets.md`
- Run artifacts never replace SQLite as the authoritative state.
- Artifact export failures do not mark the generation itself successful; they must be surfaced in job metadata.

### 12. Long Task Robustness

Accepted behavior:

- Long generation jobs can save recovery summaries with prompt hash, context hash, latest phase, latest artifact IDs, and resumable operator instruction.
- Recovery mode can show where the job stopped and whether it can be retried, repaired, or only audited.
- Context compression summaries are stored separately from canonical project memory and require author approval before becoming generation context.

### 13. UI Locality

Accepted behavior:

- New author control surfaces live in page or feature modules, not directly inside `App.tsx`.
- Tauri command calls continue to go through `src/lib/tauriClient.ts`.
- Existing page behavior and navigation labels remain stable.
- Frontend contract tests cover new command names and page module boundaries.

## Verification Gates

Focused verification for each slice must include its direct Rust tests and any changed frontend contract tests.

Full verification before claiming the goal complete:

- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`
- `node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs`
- `npm run build` from `tauri-app`
- `git diff --check`

Manual verification before a release-quality claim:

- Create or select a project.
- Generate or import Director Mode candidates, promote one, and confirm project bootstrap data remains editable.
- Generate a context preview and confirm prompt runtime, context rules, hard facts, style assets, graph context, vector retrieval, learning entries, and manual pins are explained.
- Generate draft alternatives and select one without overwriting rejected candidates.
- Export and re-import a project package in a clean database and confirm structured Bible data round-trips.
- Enable a declarative extension pack and confirm its contribution appears in trace; disable it and confirm output no longer changes.
