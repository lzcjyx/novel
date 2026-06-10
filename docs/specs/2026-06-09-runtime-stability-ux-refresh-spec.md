# Runtime Stability and Writing UX Refresh Spec

## Goal

Make the desktop app recover cleanly after hidden-window, quit, crash, and restart paths; make generation progress visible while a chapter is being produced; and replace the one-note dark/AI-styled shell with a calmer editorial production UI.

## Current Findings

- The main window close button currently hides the window to the system tray. That behavior is not visible to the user, so it looks like the app exited while the process and any active generation work continue.
- The tray builder does not set an explicit icon, even though the bundle has icon files. On Windows this can produce a transparent tray icon.
- Startup only calls `cleanup_stale_locks`. A stale `generation_jobs` row with status `started`, `draft_created`, `reviewing`, `revising`, or `publishing` can still block the next writing run after a killed process or forced quit.
- The Tauri command creates a `PipelineEvent` channel but drains it only after the workflow returns. The workflow itself emits useful phase events, but the UI does not receive them live during long model calls.
- `ModelClient` has non-streaming `generate_json` and `generate_text` methods. True token streaming would require changing every provider adapter. This slice should not do that provider-wide rewrite.
- The dashboard mounts every page and hides inactive pages with CSS. Hidden pages still allocate React state and can run effects, which increases startup and tab-switch work.
- The current visual system is mostly pure black, PlayStation blue, pill buttons, and large generic cards. It reads like a generic AI dashboard rather than a writing operations tool.

## Scope

### 1. Runtime Recovery

- Add a deterministic generation-job recovery function in `db::generation_jobs`.
- On app startup, mark stale non-terminal generation jobs as failed/interrupted before the UI can start a new chapter run.
- Preserve phase summaries and failure reasons when a stale job is recovered.
- Keep the in-memory `RunningGuard` reset behavior for command-level errors.

### 2. Window and Tray Lifecycle

- Keep close-button behavior as "hide to tray", but make that behavior explicit through tray tooltip/menu labels and app status text.
- Set the tray icon from the bundled application icon so Windows does not show a transparent tray entry.
- Reuse one helper for tray click, menu open, and menu write actions:
  - show the main window,
  - unminimize it when supported,
  - set focus,
  - emit a frontend resume event.
- On full Quit, mark active jobs interrupted where possible before exiting. Startup recovery remains the fallback for forced termination.

### 3. Live Generation Feedback

- Start a background event relay before awaiting `workflow::chapter_production::generate_next_chapter`, so pipeline events reach the frontend while the workflow is running.
- Extend `PipelineEvent` with optional transient preview fields:
  - `preview_title`
  - `preview_text`
  - `preview_kind`
- Emit a draft preview after the draft is saved and a revision preview after each successful revision. Keep previews transient; do not store large body text in `generation_jobs.metadata`.
- In the dashboard, show a live writer panel with:
  - current phase,
  - progress,
  - phase timeline,
  - animated cursor/activity while waiting for a model call,
  - progressive reveal of the latest draft/revision preview once available.

### 4. Resume and Black-Screen Resilience

- Frontend listens for:
  - `app-resume` from Tauri,
  - browser `visibilitychange`,
  - window `focus`.
- On resume, refresh status/logs and re-fetch the context preview for the selected project.
- Keep root backgrounds and app surfaces defined so a WebView repaint never exposes a blank black page without UI state.

### 5. UI and Performance Refresh

- Render only the active page instead of pre-mounting every page.
- Remove remote font import from CSS to avoid network-dependent font loading in the desktop shell.
- Replace the pure black/blue visual language with an editorial operations palette:
  - deep ink background,
  - warm paper text,
  - muted slate surfaces,
  - brass/teal accents,
  - restrained warnings/success colors.
- Reduce pill-shaped controls to 8px-or-less radius except small badges.
- Keep information density high: dashboard first screen remains the working console, not a landing page.

## Non-Goals

- Do not implement provider-wide token streaming in this slice. The accepted behavior is live stage events plus progressive reveal of completed draft/revision snapshots.
- Do not introduce a new frontend dependency solely for icons or animation.
- Do not restructure the whole React app into many files unless required by the specific fixes.
- Do not claim the entire product has zero bugs. Verification covers the regressions and workflows changed in this slice.

## Acceptance Criteria

- Stale running jobs are recovered on startup and no longer block the next generation run.
- Fresh running jobs are not incorrectly failed by recovery.
- Startup calls both stale lock cleanup and stale job recovery.
- The tray icon is explicitly set from bundled assets.
- Restore/open/write tray actions show and focus the main window and emit `app-resume`.
- Pipeline events are relayed live while generation is running.
- Draft/revision preview events include transient preview text.
- Dashboard shows a live writer panel and progressive preview reveal.
- Frontend resume handlers refresh app state after restore/focus.
- Inactive pages are no longer all mounted at once.
- CSS no longer depends on Google Fonts.
- Frontend build passes.
- Rust full test suite passes.
- Whitespace check passes.
