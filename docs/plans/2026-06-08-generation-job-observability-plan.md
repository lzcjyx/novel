# Generation Job Observability Plan

## Goal

Persist pipeline phase events into generation job metadata and show them in the Jobs page.

## Tasks

- [x] Write failing backend tests for persisted phase events and terminal failure summaries.
- [x] Add `record_job_phase_event` in `db::generation_jobs`.
- [x] Store ordered `phase_events` and derived `phase_summary` in `generation_jobs.metadata`.
- [x] Update terminal `update_job_status` calls to preserve failure/completion summary.
- [x] Make stuck-job reset preserve failure summary through the same terminal-status path.
- [x] Record actual chapter production events in metadata alongside transient `PipelineEvent` emission.
- [x] Extend Jobs UI to parse metadata and render timeline, elapsed time, retry count, last step, and failure reason.
- [x] Add per-phase `duration_ms` while preserving cumulative `elapsed_ms`.
- [x] Summarize slowest step and slow steps in `phase_summary`.
- [x] Show slowest step in the Jobs UI.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture`
- [x] Full Rust test suite.
- [x] Frontend production build.
- [x] Edge visual verification for Jobs UI.
- [x] `git diff --check`.
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests slow_phase_diagnostics_are_summarized -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] Edge visual verification for Jobs UI.
- [x] `git diff --check`.

## Follow-Up

- [x] Add token/cost metadata summaries and Jobs page metrics for estimated model usage.
- [x] Capture provider-reported token usage when model providers expose usage metadata through `ModelClient`.
- [x] Add slow-step or latency-threshold metadata output once enough phase data exists.
- [ ] Add slow-step benchmark report sections once benchmark fixtures produce representative phase durations.
