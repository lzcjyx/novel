# Benchmark Reporting Plan

## Goal

Split benchmark output into latency, quality, token, and cost sections.

## Tasks

- [x] Write a failing test for required benchmark report sections and ordering.
- [x] Add a small `BenchmarkReport` helper in the benchmark test.
- [x] Capture bootstrap, plan loading, chapter generation, and total suite latency.
- [x] Capture review quality summary from benchmark outputs.
- [x] Capture token and cost summary from generation job metadata.
- [x] Capture provider-reported and estimated model call counts.
- [x] Configure benchmark pricing so cost output is non-`n/a`.
- [x] Print the structured report at the end of the full pipeline benchmark.

## Verification

- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test benchmark benchmark_report_has_required_sections -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test benchmark benchmark_full_pipeline -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml --test benchmark -- --nocapture`
- [x] `cargo test --manifest-path tauri-app\src-tauri\Cargo.toml`
- [x] `git diff --check`

## Follow-Up

- [ ] Add latency/token budget assertions after baseline collection.
- [x] Add provider-reported usage and cost once `ModelClient` exposes them.
