# Runtime Workflow UI Bugs Spec

## Goal

Fix the reported runtime bugs without lowering quality gates or changing the product goal. The app must keep generated novel state visible after human review edits, show valid learned patterns, make the graph interactive, report remaining chapter plans truthfully, and preserve publication metadata for future blog/social publishing work.

## Bugs And Required Behavior

### Learn intake stores usable patterns

File Learn and Web Learn must not persist library rows with `pattern_name = "Unknown"` or an empty `pattern_description` when the model returns common aliases such as `name`, `title`, `description`, `summary`, `type`, or `notes`.

Accepted behavior:

- `workflow::learning::extract_knowledge` accepts strict fields and common aliases.
- Category names are normalized to the UI/backend vocabulary: `plot_pattern`, `character_archetype`, `dialogue_style`, `style_pattern`, `pacing_pattern`, `narrative_device`, or `improvement_note`.
- Empty or unusable model items are skipped instead of saved as blank library content.
- The extraction prompt/schema explicitly asks for the normalized categories and field names.

### Knowledge Graph is live and draggable

The Graph page must no longer feel fixed. Edges can keep the existing animated flow, but node positions must also have visible motion and users must be able to drag nodes with the mouse or pointer.

Accepted behavior:

- Visible nodes get deterministic base positions.
- Non-dragged nodes receive a small bounded live offset so the graph reads as active.
- Pointer drag updates node coordinates inside the graph canvas and edges follow the moved nodes.
- Dragging a node selects it and does not reset after pointer release.
- Reduced-motion users keep edge/node animation disabled by CSS, but drag still works.

### Publication reviewer and future publishing seam

Publication review must score on a clear 0-100 rubric instead of letting valid publishable chapters drift to single-digit scores because the prompt is underspecified. The publication review output must also be usable by future blog/social publishing work.

Accepted behavior:

- `publication_reviewer.md` defines score bands and says a clean publishable draft should normally score 85-100.
- Reviewer JSON can include `blog_metadata`.
- Blog draft creation preserves latest `publication_reviewer.blog_metadata` into `blog_posts.metadata`.
- No external blog/social network call is added in this fix. The seam remains local and testable.

### Dashboard Plans Left decreases after a chapter is written

`plans_left` means plans not yet drafted. A plan that already produced a chapter, even one in `needs_human_review`, must not count as left on Dashboard.

Accepted behavior:

- Project stats count only `chapter_plans.status = 'planned'`.
- Existing generation state can keep human-review plans `in_progress` for workflow visibility.
- After a needs-human-review chapter is generated from one planned plan, stats report `plans_left = 0`.

### Human review edit/save does not blank the app

Saving a human edit for a chapter in `needs_human_review` must not deadlock or block Tauri commands. After save, Chapters, Plans, Jobs, Bible, Graph, Learn, and Dashboard Context Preview should remain queryable.

Accepted behavior:

- `save_edited_chapter` does not hold the database mutex while calling helper functions that lock the same database.
- A saved human edit creates a new `chapter_versions` row with `version_type = 'revised'` and updates the chapter final version.
- It is safe to immediately call chapter, plan, job, bible, graph, learning, and writing context commands after save.

## Out Of Scope

- Implementing real external blog/social publishing APIs.
- Replacing the current review pipeline or lowering review thresholds.
- Rewriting the Graph page as a new visualization framework.

## Verification

- Add failing Rust regression tests for learning normalization, plan stats, human edit persistence, and publication metadata.
- Add a lightweight frontend helper test for graph layout/drag coordinate behavior.
- Run targeted tests first, then full Rust tests, TypeScript/Vite build, and whitespace checks.
