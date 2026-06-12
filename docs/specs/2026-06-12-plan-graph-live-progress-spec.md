# AI Novel Factory Plan Graph Live Progress Spec

Date: 2026-06-12

## Goal

Improve the software in three connected areas:

- Every later "Generate Plans" action must continue the current story at longform pacing, not just the initial project bootstrap.
- The Knowledge Graph must evolve as chapters are written, with meaningful edges connecting characters, locations, items, plot threads, foreshadowing, and timeline events.
- The UI must present Graph and generation progress with a professional, flowing desktop-workbench style guided by `ui-ux-pro-max`.

## Current Findings

### Plan Generation

`weekly_planner.rs` sends project target words, chapters written, existing plans, active plot threads, foreshadowing, and characters. The prompt already has a minimal longform pacing warning, but the context lacks explicit progress percent, estimated total chapters, story phase, recent chapter summaries, next sequence, and local movement constraints. The model can still compress the story if it only sees a few active threads and a broad long-term description.

### Knowledge Graph

The backend already derives graph nodes from Bible tables and persists `knowledge_graph_edges`. `canon_extractor.md` asks the model for `knowledge_graph_edges`, and `canon_updater.rs` persists them only when the model provides exact existing node IDs. In practice, models often output names instead of IDs or omit edges. The result is a graph with nodes but few or no edges.

### Graph UI

`KnowledgeGraphPage` lays nodes into static type rings and draws straight SVG lines. It is useful, but it reads as static. It needs a flowing relationship layer, better edge emphasis, motion that communicates activity, and auto-refresh after generation.

### Live Progress UI

Dashboard has pipeline ticks and a preview once draft/revision text arrives. The user wants a text box next to generation content that scrolls in real time and acts as a progress indicator. Current logs are separate and not shaped as a writing-progress feed.

## Design Direction From ui-ux-pro-max

Product type: desktop SaaS/workbench for long-running AI operations. Use a quiet, operational, content-dense interface.

Required UI rules:

- Accessibility: all graph nodes remain buttons with labels; color is not the only edge indicator; focus states remain visible.
- Interaction: touch/click targets stay at least 44px high where practical; controls use clear disabled/loading states.
- Motion: 150-300ms state transitions; ambient graph flow is subtle and disabled under `prefers-reduced-motion`.
- Layout: no horizontal scroll on narrow screens; graph canvas and live progress panels have stable min/max dimensions.
- Style: use semantic node colors, restrained glow, and animated strokes for relationship flow. Avoid decorative orbs and layout-shifting animations.

## Requirements

### 1. Longform Plan Continuation

- Weekly planner context must include:
  - total target words;
  - daily target words;
  - chapters written;
  - completed words;
  - estimated total chapters;
  - story progress percent;
  - next sequence;
  - remaining planned count;
  - story phase: `opening`, `early_development`, `middle_build`, `late_build`, or `endgame`.
- Recent completed chapter summaries must be included so the planner can continue from the actual current story.
- Prompt must require plans to move only the next local movement unless story phase is `endgame`.
- Prompt must forbid resolving final villain/core mystery/final romance/final power endpoint before `endgame`.
- Generated plans must preserve model-provided `plot_goals`, `required_characters`, `required_locations`, `required_foreshadowing`, `pov_character`, `ending_hook`, and pacing metadata where available.

### 2. Graph Edge Evolution

- Canon update must persist valid AI-inferred edges when the model uses exact IDs.
- Canon update must also resolve model-provided node names or labels to existing node IDs when the type is valid and the label is unique.
- Canon update must create deterministic edges from extracted timeline events:
  - character -> timeline_event with `participates_in`;
  - timeline_event -> location with `occurs_at`;
  - timeline_event -> foreshadowing with `introduces` or `resolves` when applicable.
- Deterministic edges must be idempotent and `auto_inferred=true`.
- Invalid or ambiguous edges must be skipped without failing chapter generation.
- The Graph page must auto-refresh after `pipeline-step` complete/update_canon events and window resume/focus.

### 3. Flowing Knowledge Graph UI

- Graph canvas must render:
  - visible edges with animated flow strokes;
  - stronger styling for edges connected to the selected node;
  - node chips that move subtly without changing layout bounds;
  - edge labels or accessible titles where practical.
- Motion must pause under `prefers-reduced-motion`.
- Graph stats must show Nodes, Edges, Orphans, and Selected Degree.
- Empty state must explain whether there are no nodes, no matching nodes, or no edges yet.

### 4. Live Text Progress Box

- Dashboard must have a dedicated "Live Progress Feed" beside/near the live chapter preview.
- Feed must append pipeline events as readable lines with phase, status, progress percent, and detail.
- During generation, feed must auto-scroll to the newest line.
- The feed must continue showing the last run after completion, so the user can inspect what happened.
- The feed must not replace the chapter preview; it complements it as progress text.

## Acceptance Criteria

- Regression tests cover weekly planner pacing context and prompt constraints.
- Regression tests cover graph edge persistence from exact IDs and name/label resolution.
- Regression tests cover deterministic timeline-event graph edges.
- Frontend build passes with the new Graph and Live Progress UI.
- CSS contains reduced-motion handling for graph/live animations.
- Full Rust tests and frontend build pass.
