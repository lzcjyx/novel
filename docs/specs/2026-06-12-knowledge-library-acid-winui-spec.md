# Knowledge Library, ACID Recovery, and WinUI 3 UI Spec

Date: 2026-06-12

## Goal

Finish the next reliability and UX slice without reducing the product target: learned Knowledge Library entries must be provably available to chapter generation, interrupted generation tasks must fail visibly and restore the project to its pre-run state, and the desktop UI must move from the current dark custom shell to a WinUI 3 / Windows 11 Fluent-style experience with comparable responsiveness.

## Current State

- `workflow::writing_context::build_writing_context` already loads top learning entries into `WritingContextPackage.learned_patterns`.
- `workflow::chapter_production::generate_next_chapter` already serializes the whole writing context into the draft prompt and marks selected learning entries used.
- Chapter version metadata currently records retrieval and graph context, but does not record which learning entries were selected for the generation run.
- `generation_jobs` can mark stale active jobs failed after restart, and tray Quit calls recovery with a zero-second timeout.
- The generation pipeline persists intermediate rows across many small SQLite operations. This avoids long database locks, but it does not yet restore the project to its pre-task state after app exit or crash.
- The UI still uses the earlier dark token system and several custom visual conventions, including emoji source markers in Learn.

## Requirements

### 1. Knowledge Library participates in chapter generation

Accepted behavior:

- The next chapter context preview continues showing selected Knowledge Library entries.
- The draft writer prompt explicitly instructs the model to apply relevant `writing_context.learned_patterns` and to include used entries in `used_context_ids` as `learning_entry:<id>`.
- Every generated draft chapter version stores Knowledge Library provenance in `chapter_versions.metadata`:
  - `selected_learning_entry_ids`: array of learning entry IDs provided to the draft writer.
  - `selected_learning_entries`: compact array with `id`, `category`, `pattern_name`, `source_type`, and `confidence`.
  - `learning_context_hash`: stable hash of the compact selected-learning payload.
- The generation job metadata stores the same selected IDs under `learning_context.selected_learning_entry_ids`.
- `learning_entries.usage_count` increments for entries selected into the generation context. If a later implementation parses exact `used_context_ids`, it may also store a stricter `used_learning_entry_ids`, but selected context use is the minimum contract for this slice.
- A regression test proves a learned pattern is present in the draft writer system prompt and in the saved chapter version metadata after generation.

### 2. Interrupted tasks are failed and rolled back

Accepted behavior:

- Starting a generation task writes a durable task snapshot before any chapter, version, review, blog, vector, or canon mutation happens.
- The task snapshot is stored in `generation_jobs.metadata.task_snapshot` and includes:
  - `project_id`, `chapter_plan_id`, `job_id`, `started_at`.
  - The plan's pre-run `status` and `metadata`.
  - Empty task-owned row lists for chapters, versions, reviews, review scores, blog posts, publication queue rows, vector documents, character states, timeline events, foreshadowing rows, and knowledge graph edges.
- Each task-created row that may outlive the current phase is either recorded in the task snapshot or tagged with `generation_job_id` in metadata before the phase can proceed.
- Recovery for active jobs performs one SQLite transaction per job:
  - Mark the job `failed` with the interruption reason.
  - Restore the chapter plan status and metadata to the snapshot values.
  - Delete task-owned rows created by the interrupted run.
  - Keep the failed `generation_jobs` row for audit and Jobs UI visibility.
  - Remove the advisory lock for the project.
- Recovery runs on app startup and on tray Quit.
- The window close button still hides the app to tray and does not fail the job. Only Quit Completely or process restart/crash recovery finalizes interrupted jobs as failed.
- A failed/interrupted task must not leave draft chapters, versions, reviews, graph edges, vector documents, or canon state that make the next generation task fail or consume wrong context.
- A regression test simulates a job with task-owned chapter/version/review/canon/vector rows, calls recovery, and proves only the failed job remains while the plan returns to its pre-run state.

### 3. ACID interpretation for long AI tasks

SQLite transactions must not be held across model calls. The ACID behavior for this product is therefore a durable Saga:

- Atomicity: a generation task has only two visible outcomes: completed/needs-human-review with artifacts, or failed with project data restored to the task's pre-run state.
- Consistency: foreign keys remain valid, chapter plans are not stranded in `in_progress`, and the next generation can run after recovery.
- Isolation: task-owned rows are tagged or recorded so recovery does not delete user-created rows unrelated to the generation task.
- Durability: the failed job record and recovery reason survive restart and are visible in Jobs.

### 4. Jobs UI shows recovery failures clearly

Accepted behavior:

- Jobs page renders recovered/interrupted failures as failed jobs with a readable error message.
- Failure detail includes the phase summary if present and the recovery reason.
- Dashboard running state becomes false after recovery and true while an active SQLite job exists.
- The Reset Stuck Job button uses the same recovery path rather than only marking a job failed.

### 5. WinUI 3 / Fluent-style UI refresh

Accepted behavior:

- Replace the current dark, gold-accent shell with a Windows 11 Fluent-inspired desktop shell:
  - Light theme first, with optional dark tokens preserved.
  - `Segoe UI Variable`, `Segoe UI`, system UI font stack.
  - Mica-like app background and neutral surfaces.
  - Windows accent blue `#0067c0`.
  - 4/8px spacing rhythm and component radii consistent with Fluent controls.
- App shell behaves like a WinUI 3 NavigationView:
  - Left navigation pane with compact icons plus text labels.
  - Top command/status bar for selected project, running state, and primary commands.
  - Main content scrolls independently; navigation remains stable.
- Controls use Fluent semantics:
  - Primary/secondary/danger buttons with visible hover, active, disabled, and focus states.
  - Segmented controls for generation mode and tabs.
  - InfoBar-style success/error/warning messages.
  - ProgressRing/ProgressBar-style running indicators.
  - List/card surfaces with subtle elevation and 1px stroke, not nested decorative cards.
- Remove emoji structural icons from the UI. Learn source markers use text badges or CSS markers that can later be replaced by a vector icon library.
- Performance constraints:
  - Avoid layout-shifting hover states.
  - Use transform/opacity for motion.
  - Respect `prefers-reduced-motion`.
  - Keep scroll containers bounded and avoid horizontal overflow at desktop widths and at the existing responsive breakpoint.

## Non-Goals

- Do not replace Tauri, React, or SQLite.
- Do not hold a database transaction across external model calls.
- Do not add a new UI framework or icon dependency in this slice.
- Do not implement external blog/social publishing.
- Do not rewrite all React pages into separate routed modules before the reliability fixes are complete.

## Verification

Required verification commands:

- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests knowledge_library_context_is_persisted_in_generation_metadata -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests interrupted_generation_recovery_rolls_back_task_owned_rows -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests reset_stuck_job_uses_recovery_and_restores_plan -- --nocapture`
- `node --test tauri-app/src/fluentTokens.test.mjs`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture`
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml`
- `npm run build` from `tauri-app`
- `git diff --check`

Manual/runtime verification:

- Start a chapter generation, inspect Dashboard context preview, and confirm learned entries are visible when the project has learning entries.
- Force an interrupted active job in a test database or dev runtime, restart/recover, and confirm Jobs shows failed while Chapters/Plans/Graph/Learn remain queryable.
- Inspect the UI at desktop width and the existing responsive breakpoint for no horizontal overflow, visible focus states, no emoji source icons, and stable controls.
