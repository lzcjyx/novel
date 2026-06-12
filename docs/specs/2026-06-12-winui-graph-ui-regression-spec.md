# WinUI Graph UI Regression Spec

Date: 2026-06-12

## Problem

The latest WinUI 3 shell commit introduced a custom Tauri title bar, but the main window cannot be dragged and the minimize, maximize, and close buttons do not respond. The Graph page also still carries the previous dark/gold relationship visualization, which conflicts with the current light Fluent token system. Graph nodes render full labels directly on the canvas, making dense graphs bulky and hard to scan.

## Root Cause Evidence

- `tauri-app/src-tauri/capabilities/default.json` grants `core:default` but does not grant explicit Tauri v2 window permissions for minimize, maximize, unmaximize, hide, or start dragging.
- `tauri-app/src/App.tsx` puts `data-tauri-drag-region` on the full `.app-titlebar`, whose descendants include the interactive `.window-control` buttons. Drag regions must be limited to non-interactive title-bar surfaces.
- `tauri-app/src/index.css` maps the app to light Fluent tokens, but Graph styles still use dark-only strokes such as `rgba(255,255,255,...)`, gold active edges, heavy shadows, and circular guide rings.
- `KnowledgeGraphPage` renders `node.label` inside each `.graph-node`, so long canon labels become canvas content instead of being reserved for title/aria/inspector detail.
- The `ui-ux-pro-max` skill was invoked. Its search script target resolves to a missing path, so the implementation uses the loaded skill rules directly: semantic color tokens, WinUI/Fluent consistency, visible focus states, no emoji structural icons, stable hit targets, accessible contrast, and reduced-motion support.

## Goals

- Restore custom title-bar behavior: drag the window from non-interactive title-bar areas, minimize, maximize/restore, and close-to-tray.
- Keep close semantics unchanged: the close button hides the window to tray and does not quit the runtime.
- Align Graph Nodes and Edges with the light WinUI 3 / Fluent shell.
- Make Graph nodes compact by showing only the most important canvas content: type abbreviation and relationship degree. Full label, type, status, subtitle, description, and relationships stay available through tooltips, accessible labels, and the inspector.
- Improve text and control readability across the shell by removing dark-theme leftover colors from visible light-theme components touched by this slice.
- Add regression tests that fail on the current implementation and pass after the fix.

## Non-Goals

- Do not introduce a native WinUI dependency.
- Do not add a new graph visualization framework.
- Do not change the knowledge graph database schema or generation logic.
- Do not replace the entire React UI or split `App.tsx` in this slice.

## Requirements

### Title Bar

- Tauri capabilities must explicitly allow the window APIs used by the custom title bar:
  - `core:window:allow-start-dragging`
  - `core:window:allow-minimize`
  - `core:window:allow-is-maximized`
  - `core:window:allow-maximize`
  - `core:window:allow-unmaximize`
  - `core:window:allow-hide`
- `.app-titlebar` must not be marked as a drag region when it contains buttons.
- Non-interactive title-bar surfaces must be marked with `data-tauri-drag-region`.
- The app must provide an explicit `onMouseDown`/pointer-safe drag handler that calls `getCurrentWindow().startDragging()` on primary-button drags from the title area.
- `.window-controls` and `.window-control` must not be drag regions.
- Window control buttons must remain semantic buttons with 46px-wide stable hit targets, visible hover/active/focus states, and existing accessible labels.

### Graph UI

- Graph canvas, nodes, edges, empty state, inspector, relationship rows, and edge form must use light Fluent-compatible tokens:
  - neutral surfaces from `--surface-solid`, `--surface-subtle`, and `--control-fill`
  - strokes from `--control-stroke`
  - text from `--text-primary`, `--text-secondary`, and `--text-tertiary`
  - accent from `--accent`
- Graph edges must be visible on a light background, with selected edges using accent blue rather than the previous gold treatment.
- Graph nodes must render compact canvas content:
  - a short type abbreviation
  - the numeric relationship degree
  - no inline full `node.label` text on the canvas
- Full node label remains available in:
  - `title`
  - `aria-label`
  - Graph inspector heading
  - relationship and form selectors
- Graph nodes must keep button semantics, focus visibility, drag behavior, and reduced-motion behavior.
- Dense labels must not overflow or enlarge the graph canvas.

### Readability

- UI colors touched by this slice must avoid dark-theme-only foreground/background pairings in the light WinUI shell.
- Interactive states must remain visible without layout shift.
- Decorative graph rings/orbs must not dominate the surface; graph guides should be subtle grid/selection cues only.

## Acceptance Criteria

- `node --test tauri-app/src/fluentTokens.test.mjs` fails before implementation and passes after implementation.
- The test file verifies title-bar permissions, limited drag-region markup, explicit `startDragging()`, Graph node compact rendering, and light Fluent graph tokens.
- `npm run build` passes from `tauri-app`.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml` passes.
- `git diff --check` passes.
- Manual smoke checks in a Tauri runtime confirm:
  - drag from the app title/title-bar spacer moves the window,
  - minimize minimizes,
  - maximize toggles maximize/restore,
  - close hides to tray,
  - Graph nodes are compact and Graph text/edges are readable in the WinUI 3 shell.
