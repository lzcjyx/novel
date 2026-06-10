# Canon Consistency Fixtures Spec

## Goal

Add deterministic canon consistency fixtures so regressions can be caught without relying on model reviewer behavior.

## Current Gap

Canon and continuity review currently depend on LLM reviewer output. That is useful for nuanced judgment, but it is not deterministic enough to prove hard canon violations are caught in tests.

## Scope

Add a pure Rust canon consistency checker with fixture-backed tests.

Initial deterministic checks:

- Active canon rules may define `metadata.forbidden_terms`; matching terms in chapter text produce issues.
- Characters whose latest physical state or status indicates death should not appear as present chapter action.
- Current chapter text should not repeat a substantial previous ending hook verbatim.
- Characters with `CharacterState.metadata.locked_location=true` should not appear at a different known location.
- Locked future timeline events should not appear before their planned chapter sequence.
- Characters whose latest known location differs from the only explicit chapter location are flagged as location continuity warnings unless the location is locked, in which case the issue is blocking.
- Future scheduled timeline events should not appear before their planned chapter sequence even when the event is not explicitly locked.
- Active style guides with explicit JSON `forbidden_phrases` should flag matching phrases.
- Active style guides with explicit JSON `required_phrases` and `enforce_required_phrases=true` should flag missing phrases.
- Plain-text style guides with explicit `Forbidden phrases:`, `Required phrases:`, `Enforce required phrases:`, and severity labels should be parsed as deterministic style rules.
- Review runs include a `canon_consistency_precheck` agent result derived from the deterministic checker.
- The Reviews UI parses that deterministic precheck payload and highlights rule type, severity, message, and evidence.

Excluded:

- Natural-language parsing of arbitrary canon rule text.
- Full timeline contradiction detection without explicit locked metadata.
- Free-form travel inference between locations.
- Free-form style analysis when style guides are plain prose rather than explicit JSON rules.

## Acceptance Criteria

- Tests prove a hard forbidden term produces a blocking issue.
- Tests prove a dead character appearing in chapter text produces a blocking issue.
- Tests prove a clean chapter has no deterministic canon issues.
- Tests prove repeated previous ending hooks are caught as deterministic continuity issues.
- Tests prove locked location conflicts are caught as deterministic continuity issues.
- Tests prove locked future timeline events are caught before their planned sequence.
- Tests prove latest-location warnings and non-locked planned future timeline events are caught.
- Tests prove explicit style forbidden phrases and required phrases are caught.
- Tests prove explicit plain-text style labels are caught without requiring JSON style guide rules.
- The checker is exported under `workflow::canon_consistency` for future review pipeline integration.
- `run_review_agents` includes deterministic canon issues as a persisted review result that aggregation can count.
- Reviews UI shows deterministic canon issues separately from raw reviewer JSON.
