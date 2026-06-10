# Token And Cost Observability Plan

## Goal

Record model usage in generation job metadata and show token/cost metrics in the Jobs page.

## Tasks

- [x] Write a failing backend test for persisted `model_usage_events` and derived `usage_summary`.
- [x] Add `record_job_model_usage` and `estimate_tokens` in `db::generation_jobs`.
- [x] Summarize prompt, completion, total tokens, call count, and optional estimated cost.
- [x] Record estimated draft model usage in chapter production.
- [x] Record estimated revision model usage in chapter production.
- [x] Extend Jobs UI metadata types and metric row for token/cost values.
- [x] Extend `ModelClient` with provider-reported usage carriers.
- [x] Record provider-reported draft usage when available.
- [x] Mark each usage event with `usage_source`.
- [x] Summarize provider-reported and estimated call counts.
- [x] Add configurable input/output per-million-token rates to Settings.
- [x] Persist, reload, and clear configured model pricing rates.
- [x] Apply configured pricing rates to chapter production usage events.
- [x] Store pricing-rate snapshots on each usage event.

## Verification

- [x] Targeted backend token/cost metadata test.
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] Edge visual verification for Jobs UI.
- [x] `git diff --check`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] `git diff --check`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests model_pricing_settings_are_persisted_and_clearable -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture`
- [x] Edge visual verification for Settings pricing fields.

## Follow-Up

- [x] Surface provider-reported usage from `ModelClient` responses.
- [x] Add configurable model pricing defaults for production cost estimates.
- [ ] Add project-specific or per-model override tables if operators need multiple price profiles.
- [x] Add benchmark sections for latency, quality, token, and cost comparisons.
