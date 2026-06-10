# Generation Job Observability Spec

## Goal

Make generation performance and failure states inspectable from the app. Operators should be able to see which phase a job reached, how long the pipeline has taken, where it failed, and whether retries were involved without opening logs.

## Current Gap

`generation_jobs` already has a `metadata` column and the runtime emits transient `PipelineEvent` values to the UI while a job is running. Those events were not persisted, so the Jobs page could only show a coarse status, timestamps, and an error string.

## Scope

Use the existing `generation_jobs.metadata` JSON column. Do not add a migration for this slice.

Persist:

- `phase_events`: ordered phase events with `step`, `status`, `detail`, `progress_pct`, `elapsed_ms`, `duration_ms`, and `timestamp`.
- `phase_summary`: compact fields for `phase_count`, `last_step`, `last_status`, `last_detail`, `failure_reason`, `completed_at`, `total_elapsed_ms`, `slowest_step`, `slowest_duration_ms`, `slow_step_count`, and `slow_steps`.

Expose:

- Timeline rows in the Jobs page.
- Phase count, elapsed time, retry count, and last step.
- Slowest phase and per-phase duration.
- Failure reason when a job fails.

## Phase Names

Reuse existing pipeline names:

- `acquire_lock`
- `load_canon`
- `retrieve_context`
- `generate_draft`
- `aggregate_reviews`
- `revise`
- `export`
- `update_canon`
- `complete`

## Acceptance Criteria

- Job phase events are written to `generation_jobs.metadata`.
- Phase events distinguish cumulative elapsed time from per-phase duration.
- Slow phases are summarized deterministically from persisted phase durations.
- Terminal status updates preserve a failure/completion summary.
- Real chapter production records phase events alongside existing UI events.
- Jobs UI renders phase timeline, slowest phase, and failure summary.
- Backend observability tests pass.
- Full Rust tests, frontend build, whitespace check, and Edge visual verification pass.
