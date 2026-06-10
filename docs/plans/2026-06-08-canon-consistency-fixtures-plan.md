# Canon Consistency Fixtures Plan

## Goal

Create deterministic canon consistency fixtures and a small checker integrated into review prechecks.

## Tasks

- [x] Write failing fixture tests for forbidden canon terms and dead-character appearances.
- [x] Add `workflow::canon_consistency`.
- [x] Detect `metadata.forbidden_terms` in active canon rules.
- [x] Detect dead character appearances from character status or latest physical state.
- [x] Detect repeated previous ending hooks as deterministic continuity issues.
- [x] Write failing fixture tests for locked location and future timeline continuity conflicts.
- [x] Detect `CharacterState.metadata.locked_location=true` conflicts against known locations.
- [x] Detect locked future timeline events appearing before their planned chapter sequence.
- [x] Write failing fixture tests for explicit style drift rules.
- [x] Detect active style guide `forbidden_phrases`.
- [x] Detect active style guide `required_phrases` only when required phrase enforcement is enabled.
- [x] Export structured `CanonConsistencyIssue` results.
- [x] Integrate deterministic issues into `run_review_agents` as `canon_consistency_precheck`.
- [x] Prove review aggregation counts deterministic blocking issues.
- [x] Show deterministic canon issues in the Reviews UI with severity, rule type, message, and evidence.
- [x] Expand timeline and location contradiction checks beyond explicitly locked fixture data.
- [x] Expand style drift checks beyond explicit JSON style guide rules with explicit plain-text labels.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test benchmark benchmark_full_pipeline -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `git diff --check`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests review_pipeline_flags_timeline_and_location_continuity -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] `git diff --check`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests review_pipeline_flags_explicit_style_drift -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `npm run build`
- [x] `git diff --check`
- [x] Edge visual check for the Reviews UI canon precheck panel.
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests precheck_flags_unlocked_latest_location_and_planned_future_timeline -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests -- --nocapture`
- [x] `cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test canon_consistency_tests precheck_flags_plain_text_style_rules -- --nocapture`
