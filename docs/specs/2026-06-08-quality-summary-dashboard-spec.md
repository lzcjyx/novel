# Quality Summary Dashboard Spec

## Goal

Make long-form quality regressions visible at the project level. Operators should not need to open every chapter to see whether the factory is trending toward publish-ready output, repeated revisions, human review, or recurring agent failures.

## Current Gap

The app already stores per-chapter `review_scores` and per-agent `agent_reviews`. The Reviews page can inspect one selected chapter, but it does not summarize quality across the project.

## Scope

Add a project quality summary derived from existing review tables.

Included:

- Count reviewed chapters.
- Count publish-ready, revise, and human-review decisions.
- Compute average raw score and final score.
- Surface total blocking issue count.
- Surface latest decision and latest final score.
- Group agent reviews by reviewer with average score, pass rate, review count, and blocking issue count.
- Show these metrics on the Reviews page above single-chapter details.

Excluded:

- AI quality scoring beyond the existing review agents.
- New database tables.
- Trend charts over time.
- Token/cost metrics.

## Acceptance Criteria

- Tests prove project-level decision and score aggregation.
- Tests prove agent-level score/pass/blocking aggregation.
- Tauri exposes `get_project_quality_summary`.
- Reviews UI shows project quality metrics and keeps single-chapter review inspection.
- Full Rust tests, frontend build, whitespace check, and Edge visual verification pass.
