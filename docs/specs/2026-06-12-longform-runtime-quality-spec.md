# AI Novel Factory Longform Runtime Quality Spec

Date: 2026-06-12

## Goal

Fix the current generation-time failures without weakening quality gates:

- Reviewer output containing Chinese text must never panic while logging or parsing.
- Dashboard running state and Jobs state must agree during active SQLite generation jobs.
- Continuity and Style review scores must reflect the model's actual JSON output and a clear 0-100 rubric, not parser failures or inconsistent prompt scales.
- New longform projects must treat the first 10 chapter plans as opening-arc plans, not a full-story synopsis that resolves the novel.
- Successful chapter generation must write the chapter markdown into the configured project storage directory, and project deletion must remove both SQLite rows and that project's markdown directory.

## Root Causes

1. `workflow/review_agents.rs` logs low scores with `&raw[..raw.len().min(300)]`. This slices UTF-8 by byte offset and panics when byte 300 lands inside a Chinese character.
2. The same reviewer parser only accepts full-string JSON or simple fenced JSON. Valid JSON surrounded by prose or malformed fences falls back to score 0, which makes Reviews show false low scores.
3. `get_status` reports `state.running` only. Jobs are persisted in `generation_jobs`, so after a command finishes, panics, or stale recovery occurs, Dashboard can show IDLE while Jobs still has a running status.
4. `prompts/bible_generation.md` says the bootstrap output "必须是 10 章章节计划" with no longform pacing constraints, causing the first 10 plans to compress and finish the whole story.
5. Markdown export uses the current global `settings.data_dir` every time and project deletion computes the current path again. A project has no persisted paper directory, so changing Settings can orphan generated files. Export failure is also logged and ignored even though markdown is a required artifact.

## Requirements

### Reviewer Runtime Safety

- Low-score warning previews must truncate by character, not byte.
- Reviewer JSON extraction must accept:
  - raw JSON object;
  - fenced JSON object;
  - JSON object surrounded by non-JSON text.
- If parsing fails, the failure must be represented as an agent failure with raw preview metadata, not a panic.
- Scores must remain 0-100 and pass/fail must come from model output after valid parsing. Do not inflate low scores.

### Review Quality Prompts

- `style_reviewer` must define a consistent 0-100 scale.
- `pass=true` must mean the chapter is publishable for that reviewer, normally score >= 75.
- Score 0-20 must be reserved for unreadable, unusable, or structurally broken prose.
- Minor style issues may reduce score but must not collapse a good chapter into single digits.

### Running State Consistency

- `get_status` must report running if either the in-memory command guard is running or SQLite has an active job in `started`, `draft_created`, `reviewing`, `revising`, or `publishing`.
- Failed, completed, skipped, and needs-human-review jobs must not keep Dashboard in running state.
- Generation command errors must continue marking the latest active job failed.

### Longform Bootstrap Pacing

- `bible_generation` must explicitly ask for the first 10 immediate chapter plans only.
- These first 10 plans must cover only the opening movement of a long novel, roughly the first 2-5% of the target word count when the target is hundreds of thousands of words.
- Plans must introduce conflict, characters, constraints, and hooks without resolving the central conflict, final villain, core mystery, final romance, or power-system endpoint.
- Long-range arcs still belong in `main_plot_threads`; chapter plans are near-term execution beats.

### Markdown Storage Lifecycle

- On project bootstrap, persist the concrete paper directory in project metadata.
- Export commands and generation pipeline must use the persisted project paper directory when present, falling back to the current settings-derived directory for older projects.
- A successful generation result must include a markdown filename/path; if chapter markdown cannot be written, the job must fail instead of silently reporting success.
- Deleting a project must remove the persisted paper directory and the current fallback directory if different, then delete SQLite rows through existing cascade behavior.

## Acceptance Criteria

- Regression tests cover UTF-8 reviewer preview, JSON extraction from wrapped output, status running derived from SQLite, longform prompt pacing guardrails, markdown export to persisted paper directory, and project deletion cleanup path selection.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml` passes.
- `npm run build` in `tauri-app` passes.
- `git diff --check` exits 0 apart from existing CRLF warnings.
