# Model Pricing Settings Plan

## Goal

Add configurable input/output token pricing defaults and apply them to chapter generation cost estimates.

## Tasks

- [x] Write a failing settings persistence test for configurable pricing rates.
- [x] Write a failing core pipeline assertion for usage-event cost snapshots.
- [x] Add optional pricing fields to `AppSettings`.
- [x] Persist, reload, and clear pricing fields in `db::settings`.
- [x] Pass configured pricing into chapter production usage recording.
- [x] Add Settings UI fields for input/output per-million-token rates.
- [x] Update token/cost specs and the main improvement plan.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests model_pricing_settings_are_persisted_and_clearable -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [x] `npm run build`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] Edge visual verification for Settings pricing fields.
- [x] `git diff --check`

## Follow-Up

- [ ] Add project-specific or per-model override tables if operators need multiple active model price profiles.
