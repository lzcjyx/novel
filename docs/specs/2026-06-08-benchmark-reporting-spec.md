# Benchmark Reporting Spec

## Goal

Make benchmark output comparable between runs by separating pipeline latency, quality, token usage, and cost signals.

## Current Gap

The benchmark test validates the end-to-end chapter pipeline, but its output is a linear set of test-step logs. That makes it hard to compare whether a prompt or workflow change improved latency, quality, or token footprint.

## Scope

Add a lightweight benchmark report inside the existing Rust benchmark test. Do not add new benchmark infrastructure or external dependencies.

Report sections:

- `Latency`: elapsed time for major benchmark phases and total suite time.
- `Quality`: review count, average score, and final decision.
- `Token`: model call count, provider-reported call count, estimated call count, and prompt/completion/total token summary from generation job metadata.
- `Cost`: estimated cost when pricing is configured, or `n/a` when pricing is not available.

## Acceptance Criteria

- A test proves the report renders `Latency`, `Quality`, `Token`, and `Cost` sections in order.
- The full pipeline benchmark prints the structured report after completing the generated chapter flow.
- Token metrics come from persisted generation job usage metadata.
- Provider-reported usage and configured pricing produce a non-`n/a` cost row in the full pipeline benchmark.
- Full Rust tests and whitespace checks pass.

## Follow-Up

- Add configurable benchmark budgets once stable baselines exist.
