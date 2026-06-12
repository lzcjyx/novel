# Core Writing Loop Design

## Goal

Improve the AI novel factory's daily writing loop so generated chapters stay coherent across chapters, use a less generic style, and give the user meaningful control before and after generation.

This is a focused first phase. It fixes the context, prompt, learning, status, and pipeline interaction defects that directly explain the current pain points:

- Context discontinuity between chapters.
- Generic web-novel prose despite style prompts.
- A low-agency UI that only offers broad "write now" and simple edit actions.

## Current Evidence

The project is a Tauri desktop app with a React frontend, Rust workflow backend, SQLite persistence, prompt templates, multiple model providers, and integration tests.

The current tests and build pass, but they do not cover the quality loop:

- `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml` passes the existing Rust tests.
- `npm run build` passes the frontend TypeScript/Vite build.
- Existing tests prove the happy path can run, but not that context is rich, prompts are fully rendered, learned style enters the draft, final canon uses the final text, or chapter plans finish correctly.

Key defects found in the current code:

- `weekly_planner.rs` loads `review_agents` as the planning prompt, so weekly planning is driven by a review prompt instead of a planner prompt.
- `draft_writer.md` contains `{{target_word_count}}`, but `chapter_production.rs` only renders `WRITING_BRIEF_JSON` and `CHARACTERS_JSON`.
- `bible_generation.md` and `canon_extractor.md` also keep template placeholders in the system prompt while real data is sent separately as user JSON.
- The draft writer receives previous chapter summaries, but not adjacent chapter body excerpts, prior hooks, character state deltas, or learned style patterns.
- `learning_entries` can be created through the Learn page, but they are not injected into the next `writing_brief`, not vectorized, and `reflect_on_chapter` is not called after production.
- The revision loop can produce a final revised chapter, but canon update still receives the original `draft` value.
- `chapter_plans` move from `planned` to `in_progress`, but are not marked `completed` at the end.
- `generation_jobs::create_generation_job` uses `INSERT OR IGNORE` and then returns the newly generated ID even when the insert was ignored.
- The frontend displays pipeline steps such as `retrieve_context`, `export`, and `update_canon`, but the backend does not emit all of those events.
- Legacy docs still describe Neon/n8n/Postgres operations even though README says this is now a local Tauri/SQLite app.

## Approaches Considered

### Approach A: Prompt-Only Patch

Tighten `draft_writer.md`, `style_reviewer.md`, and `revision_writer.md` without changing data flow.

Pros:

- Smallest code change.
- Low risk to existing workflow.

Cons:

- Does not fix missing context.
- Does not close the learning loop.
- Does not fix state machine drift.
- Likely produces only temporary style improvements.

### Approach B: Core Loop Repair

Fix prompt rendering, weekly planning, context assembly, learning injection, final canon update, plan/job status, and progress events. Add a small UI surface for generation controls and context visibility.

Pros:

- Directly targets all three pain points.
- Keeps the scope testable.
- Uses existing tables and workflow modules.
- Builds on the already-present Learn page instead of adding a new subsystem.

Cons:

- Touches several workflow files.
- Requires focused regression tests around previously untested quality behavior.

### Approach C: Full v2 Expansion

Build the larger v2 vision: knowledge graph UI, cost metrics, agent diagnostics, manual review approvals, local model discovery, and richer publishing.

Pros:

- Stronger long-term product direction.

Cons:

- Too broad for one safe implementation cycle.
- Delays fixes to the actual writing quality loop.
- Adds new surfaces before the core generation contract is reliable.

## Recommended Design

Use Approach B.

The first phase should make the current factory behave like a controlled writing system rather than a prompt runner. The backend becomes responsible for assembling a deliberate writing context package. The prompts consume only rendered, explicit data. The generated final text updates canon and learning state. The UI exposes the critical levers and evidence without turning into a large redesign.

## Architecture

Add a small set of focused workflow helpers:

- `workflow::writing_context` builds the complete `WritingContextPackage` for one chapter.
- `workflow::prompt_rendering` validates rendered prompts have no unresolved placeholders.
- `workflow::learning` exposes retrieval and usage helpers for learned style entries.
- Existing `chapter_production` orchestrates the same pipeline, but delegates context assembly and finalization work to helpers.

The chapter production flow becomes:

1. Select a planned chapter.
2. Create or reuse the real generation job ID.
3. Load canon, recent chapters, character states, learned patterns, and vector context.
4. Build `WritingContextPackage`.
5. Render and validate draft prompt.
6. Generate draft.
7. Review, revise, and re-review as today.
8. Persist the final selected version as the chapter state.
9. Mark the chapter plan completed when the final decision is terminal.
10. Export the final version.
11. Update canon from the final selected text, not the original draft.
12. Run self-reflection and store improvement notes.
13. Emit progress events for every UI timeline step.

## Writing Context Package

The draft writer should receive a structured package with these top-level fields:

- `project`: name, genre, target audience, tone, quality threshold.
- `chapter_plan`: sequence, title, outline, target word count, POV, required characters, required locations, plot goals, required foreshadowing.
- `continuity`: recent summaries, last two chapter body excerpts, previous ending hook, open questions, timeline events, character states.
- `canon`: characters, locations, organizations, items, magic systems, canon rules, active plot threads, unresolved foreshadowing, world lore.
- `retrieval`: vector documents with source type, source ID, title, content, similarity.
- `style`: project style guide, forbidden phrases, preferred techniques, positive examples, negative examples.
- `learned_patterns`: top learning entries grouped by category, including `pattern_name`, `pattern_description`, `example_text`, `application_notes`, confidence.
- `operator_controls`: user-selected generation mode, chapter intent, forbidden content, must-include beats, style emphasis.

The first implementation should support operator controls with safe defaults even before the UI exposes every field.

## Prompt Rendering

Prompt rendering must become explicit and testable:

- Every prompt used in a workflow must be rendered through one helper.
- If `{{...}}` remains after rendering, the workflow returns an error before calling the model.
- `draft_writer.md` should use one placeholder: `{{WRITING_CONTEXT_JSON}}`.
- `bible_generation.md` should render `{{PROJECT_INPUT_JSON}}` into the prompt instead of leaving the placeholder in system text.
- `canon_extractor.md` should render `PROJECT_ID`, `CHAPTER_ID`, `CHAPTER_TEXT`, and `EXISTING_CANON_JSON`.
- A prompt template test must catch unresolved placeholders.

This avoids silently sending template artifacts to the model.

## Weekly Planner

Add a dedicated `weekly_planner.md` prompt and register it in `prompts::PromptRegistry`.

The weekly planner prompt should:

- Continue from existing chapters and open plans.
- Respect active plot threads, unresolved foreshadowing, character states, and project style.
- Produce chapter plans with sequence, title, outline, target word count, POV, plot goals, required characters, required locations, and required foreshadowing.
- Avoid generic "new beginning" or filler plans.

`weekly_planner.rs` should load `weekly_planner`, not `review_agents`.

## Learning Loop

The existing Learn page should become part of generation:

- `learn_from_text` stores entries as today.
- After storing learning entries, optionally insert them into vector metadata when an embedding provider is available. If embeddings are unavailable, they still enter `learned_patterns` by recency and confidence.
- `chapter_production` loads the top learned patterns before draft generation.
- After finalization, run `reflect_on_chapter` using final chapter text, review scores, and current learning entries.
- Store reflection entries as `source_type = 'self_reflection'`.
- Increment `usage_count` and set `last_used_at` for learning entries included in a writing context.

This makes style evolution cumulative instead of a separate library page.

## Canon Finalization

Canon update must operate on the final selected chapter content:

- If the draft passes, use the draft JSON.
- If a revision becomes the final version, construct a final chapter JSON from the revised output.
- Use the final body, final summary, final title, final events, and final notes where available.
- If revision output lacks canon fields, preserve draft canon fields only when still compatible with the final body; otherwise pass an explicit `revision_final_notes` field to the canon extractor.

The first implementation can keep a conservative rule: body/title/summary always come from the final version, and structured event arrays come from the final model output when present, otherwise from the draft.

## State Machine

Fix status transitions:

- `create_generation_job` must return the inserted job ID or the existing conflicting job ID.
- A completed terminal generation must mark the associated `chapter_plans.status` as `completed`.
- A human-review terminal generation should leave the plan as `in_progress`, because it still needs user action.
- Failed generation should keep the plan as `planned` if no chapter was saved, or `in_progress` if a chapter requires review.
- `plans_left` should count only `planned` and human-actionable `in_progress` plans.

Add tests for the real job ID and plan status transitions.

## User Interaction

Keep this phase minimal and useful:

- Add a Dashboard "Chapter Control" panel for the selected project.
- Controls:
  - generation mode: balanced, continuity-first, style-first, experimental.
  - chapter intent text.
  - must-include beats textarea.
  - forbidden moves textarea.
  - style emphasis textarea.
- Show a compact context preview before generation:
  - next chapter plan.
  - last chapter hook or ending excerpt.
  - number of learned patterns available.
  - RAG status.
- Send controls to backend as optional `operator_controls`.

The backend should also work with defaults when controls are omitted, so tray actions and tests remain stable.

## Pipeline Events

Backend must emit events for all frontend timeline steps:

- `acquire_lock`
- `load_canon`
- `retrieve_context`
- `generate_draft`
- `aggregate_reviews`
- `revise`
- `export`
- `update_canon`
- `complete`

Each event should include `status`, `detail`, `progress_pct`, and timestamp. Errors should emit a failed event where possible before returning.

## Documentation

Refresh docs that conflict with the desktop implementation:

- Add a concise local operations note for Tauri/SQLite.
- Mark old Neon/n8n docs as legacy instead of deleting them.
- Add troubleshooting entries for unresolved prompt placeholders, missing embedding provider, job/plan status drift, and stale running locks.

## Testing

Add or update tests to prove:

- Prompt rendering rejects unresolved placeholders.
- Weekly planner loads the dedicated prompt.
- Writing context includes recent body excerpts, character states, style guides, and learned patterns.
- Learning entries included in context get usage metadata updated.
- Revision finalization updates canon with final body/title/summary.
- Completed generation marks the chapter plan completed.
- Conflicting job creation returns the real existing job ID.
- Pipeline emits the expected step names.
- Frontend build still passes.

The final verification commands for this phase are:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml
npm run build
git diff --check
```

## Out Of Scope

The following are intentionally deferred:

- Knowledge graph UI.
- Cost and token metrics.
- Agent diagnostics dashboards.
- Full manual review approval workflow.
- Local model discovery.
- Blog publishing upgrades.
- Large React component decomposition unless needed for the control panel.

## Acceptance Criteria

- The writing workflow no longer sends unresolved prompt placeholders to providers.
- Weekly planning uses a dedicated planner prompt.
- Draft generation receives a richer context package, including recent chapter excerpts and learned style patterns.
- Learned patterns influence future chapters and self-reflection creates new improvement notes.
- Canon update uses the final selected chapter version.
- Job and plan statuses match the terminal outcome.
- Dashboard gives the user meaningful pre-generation controls and a truthful pipeline timeline.
- Existing tests and new regression tests pass.
- Frontend production build passes.
- Documentation no longer presents the old Neon/n8n workflow as the active operational path.
