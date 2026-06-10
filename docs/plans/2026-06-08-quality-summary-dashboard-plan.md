# Quality Summary Dashboard Plan

## Goal

Add project-level quality evaluation summaries to the Reviews page.

## Tasks

- [x] Write failing tests for project score and decision aggregation.
- [x] Write failing tests for agent score/pass/blocking aggregation.
- [x] Add `AgentQualityScore` and `ProjectQualitySummary`.
- [x] Add `db::reviews::get_project_quality_summary`.
- [x] Add Tauri command `get_project_quality_summary`.
- [x] Add Reviews page quality dashboard with project metrics and agent rows.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test quality_summary_tests -- --nocapture`
- [x] `npm run build`
- [x] Full Rust test suite.
- [x] Edge visual verification.
- [x] `git diff --check`.

## Follow-Up

- [ ] Add trend chart once there are stable timestamped quality runs.
- [x] Split benchmark reports into latency, quality, token, and cost sections.
- [x] Add deterministic canon consistency fixtures.
