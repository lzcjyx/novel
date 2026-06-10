# Token And Cost Observability Spec

## Goal

Make model usage visible per generation job so operators can identify expensive phases and compare prompt changes against token growth.

## Current Gap

The app has structured generation jobs and persisted phase events. Model usage is now recorded from provider-reported usage when available, with deterministic estimates used as the fallback when a provider does not expose usage metadata.

## Scope

Use the existing `generation_jobs.metadata` JSON column. Do not add a migration for this slice.

Persist:

- `model_usage_events`: ordered model-call records with `phase`, `provider`, `model`, `prompt_tokens`, `completion_tokens`, `total_tokens`, `usage_source`, optional pricing rates, optional `estimated_cost_usd`, and `timestamp`.
- `usage_summary`: derived `call_count`, `provider_reported_call_count`, `estimated_call_count`, total prompt/completion/total tokens, optional total estimated cost, and `updated_at`.

Record:

- Provider-reported draft-generation usage when the `ModelClient` response carries usage.
- Estimated draft-generation usage from rendered system/user prompts and generated JSON output when provider usage is unavailable.
- Estimated revision usage from rendered revision prompts and generated JSON output.
- Estimated cost when both input and output per-million-token rates are configured in Settings. Each usage event stores the rates used for that call so historical jobs keep their original cost snapshot.

Expose:

- Total tokens in the Jobs metric row.
- Estimated cost in the Jobs metric row, or `n/a` when no cost is available.

## Acceptance Criteria

- Backend tests prove usage events are persisted and summarized.
- Chapter production records provider-reported usage when available and estimated usage otherwise.
- Chapter production applies configured input/output token rates to usage events when both rates are present.
- Jobs UI parses usage metadata and renders token/cost metrics without breaking existing phase timeline behavior.
- Core pipeline tests prove provider-reported usage is preferred over estimates when available.
- Full Rust tests, frontend build, whitespace check, and Edge visual verification pass.

## Follow-Up

- Add project-specific or per-model override tables if operators need multiple simultaneous model price profiles.
- Split benchmark output into latency, quality, token, and cost sections.
