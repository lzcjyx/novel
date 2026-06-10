# Model Pricing Settings Spec

## Goal

Let operators configure model input/output token prices so generation jobs can report estimated dollar cost without hard-coded rates.

## Scope

Use system settings as the current pricing source:

- `input_cost_per_million`: optional USD price per one million prompt/input tokens.
- `output_cost_per_million`: optional USD price per one million completion/output tokens.

Persist each rate as a nullable setting. Empty or invalid values are treated as unset.

## Behavior

- Settings can save, reload, and clear both rates.
- Chapter production reads the active settings for each generation run.
- Each `model_usage_events` entry stores the rate snapshot used for that call.
- `estimated_cost_usd` is computed only when both input and output rates are configured.
- Historical job metadata remains stable after an operator changes pricing later.

## Acceptance Criteria

- Backend tests prove pricing settings round-trip and clear correctly.
- Core writing-loop tests prove configured rates are applied to provider-reported usage.
- Settings UI exposes editable input/output per-million-token rate fields.
- Frontend build, Rust tests, whitespace checks, and Edge visual verification pass.

## Out Of Scope

- Project-specific or per-model override tables.
- Currency conversion.
- Provider price discovery from external APIs.
