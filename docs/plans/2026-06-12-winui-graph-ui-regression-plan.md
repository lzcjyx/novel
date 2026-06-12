# WinUI Graph UI Regression Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore WinUI 3 custom title-bar interaction and align the Knowledge Graph canvas with the light Fluent shell while keeping graph nodes compact.

**Architecture:** Keep the existing Tauri v2 + React + CSS architecture. Add source-level regression tests in `fluentTokens.test.mjs`, fix Tauri capability permissions, constrain drag behavior to non-interactive title-bar regions, and restyle the existing Graph markup/CSS with semantic Fluent tokens.

**Tech Stack:** Tauri v2, React 19, TypeScript, CSS, Node built-in test runner, Cargo tests.

---

## File Map

- Modify `tauri-app/src/fluentTokens.test.mjs`: add failing regression tests for Tauri window permissions, title-bar drag-region boundaries, explicit start-dragging behavior, compact graph node markup, and Fluent graph token usage.
- Modify `tauri-app/src-tauri/capabilities/default.json`: grant the exact Tauri window API permissions used by the title bar.
- Modify `tauri-app/src/App.tsx`: add a title-bar drag handler, remove drag-region from the interactive title-bar parent, keep buttons outside drag regions, add a compact graph node display helper, and render compact Graph node content.
- Modify `tauri-app/src/index.css`: adjust title-bar affordance styles and replace dark/gold graph styling with light Fluent graph surfaces, edges, nodes, empty states, inspector, and forms.
- Create `docs/specs/2026-06-12-winui-graph-ui-regression-spec.md`: record the bug, root cause evidence, requirements, and acceptance criteria.
- Create `docs/plans/2026-06-12-winui-graph-ui-regression-plan.md`: record this plan.

## Task 1: Lock The Regression With Tests

**Files:**
- Modify: `tauri-app/src/fluentTokens.test.mjs`

- [ ] **Step 1: Add title-bar interaction tests**

Add tests that assert the custom title bar has explicit window permissions, limited drag regions, and a start-dragging handler:

```js
const capabilities = JSON.parse(readFileSync(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const requiredWindowPermissions = [
  "core:window:allow-start-dragging",
  "core:window:allow-minimize",
  "core:window:allow-is-maximized",
  "core:window:allow-maximize",
  "core:window:allow-unmaximize",
  "core:window:allow-hide",
];

test("custom titlebar has Tauri window permissions and does not make controls draggable", () => {
  const config = JSON.parse(tauriConfig);
  assert.equal(config.app.windows[0].decorations, false);
for (const permission of requiredWindowPermissions) {
  assert.ok(capabilities.permissions.includes(permission), `${permission} permission missing`);
}
assert.doesNotMatch(app, /<header className="app-titlebar" data-tauri-drag-region>/);
assert.match(app, /handleTitlebarDrag/);
assert.match(app, /\.startDragging\(\)/);
assert.match(app, /className="titlebar-drag-zone" data-tauri-drag-region/);
assert.doesNotMatch(app, /className="window-controls" data-tauri-drag-region/);
});
```

- [ ] **Step 2: Add compact Graph node and Fluent graph token tests**

Add tests:

```js
test("graph nodes render compact canvas content while preserving labels off-canvas", () => {
  assert.match(app, /const graphNodeInitial/);
  assert.match(app, /className="graph-node-initial"/);
  assert.match(app, /className="graph-node-degree"/);
  assert.doesNotMatch(app, /<span>\{node\.label\}<\/span>/);
  assert.match(app, /title=\{\`\$\{node\.label\}/);
  assert.match(app, /aria-label=\{\`\$\{node\.label\}/);
});

test("graph workbench uses light Fluent graph tokens", () => {
  assert.match(css, /\.graph-canvas[\s\S]*var\(--surface-solid\)/);
  assert.match(css, /\.graph-edge-base[\s\S]*rgba\(0,\s*0,\s*0,\s*0\.\d+\)/);
  assert.match(css, /\.graph-edge\.active \.graph-edge-base[\s\S]*var\(--accent\)/);
  assert.match(css, /\.graph-node[\s\S]*var\(--control-fill\)/);
  assert.match(css, /\.graph-node-initial/);
  assert.match(css, /\.graph-node-degree/);
  assert.doesNotMatch(css, /\.graph-edge\.active \.graph-edge-base[\s\S]*208,\s*168,\s*92/);
});
```

- [ ] **Step 3: Run RED**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: FAIL because capabilities are missing, the title-bar parent is a drag region, no `startDragging()` handler exists, graph nodes still render `node.label`, and graph styles still use dark/gold tokens.

## Task 2: Restore Title-Bar Interaction

**Files:**
- Modify: `tauri-app/src-tauri/capabilities/default.json`
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`

- [ ] **Step 1: Add Tauri window permissions**

In `default.json`, extend `permissions`:

```json
"core:window:allow-start-dragging",
"core:window:allow-minimize",
"core:window:allow-is-maximized",
"core:window:allow-maximize",
"core:window:allow-unmaximize",
"core:window:allow-hide"
```

- [ ] **Step 2: Add title-bar drag handler**

In `App`, add:

```ts
const handleTitlebarDrag = async (event: ReactPointerEvent<HTMLElement>) => {
  if (event.button !== 0) return;
  await getCurrentWindow().startDragging();
};
```

- [ ] **Step 3: Limit drag-region markup**

Change the title bar to:

```tsx
<header className="app-titlebar">
  <div
    className="titlebar-drag-zone"
    data-tauri-drag-region
    onPointerDown={handleTitlebarDrag}
  >
    <div className="titlebar-brand" data-tauri-drag-region>
      <span className="titlebar-app-mark" aria-hidden="true">A</span>
      <span className="titlebar-title" data-tauri-drag-region>AI Novel Factory</span>
    </div>
  </div>
  <div className="window-controls">
    ...
  </div>
</header>
```

Do not add `data-tauri-drag-region` to `.window-controls` or `.window-control`.

- [ ] **Step 4: Update title-bar CSS**

Add:

```css
.titlebar-drag-zone {
  min-width: 0;
  height: 100%;
  flex: 1 1 auto;
  display: flex;
  align-items: center;
  cursor: default;
}
.window-controls {
  isolation: isolate;
}
```

- [ ] **Step 5: Run focused GREEN check**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: title-bar tests pass after Task 2, while Graph tests may still fail until Task 3.

## Task 3: Compact And Restyle The Graph Canvas

**Files:**
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`

- [ ] **Step 1: Add compact node helper**

Inside `KnowledgeGraphPage`, add near `typeLabel`:

```ts
const graphNodeInitial = (type: string) =>
  type
    .split("_")
    .map(part => part[0] || "")
    .join("")
    .slice(0, 2)
    .toUpperCase() || "N";
```

- [ ] **Step 2: Render compact node content**

Change graph node button content:

```tsx
<span className="graph-node-initial" aria-hidden="true">{graphNodeInitial(node.node_type)}</span>
<span className="graph-node-degree" aria-hidden="true">{node.degree}</span>
```

Keep:

```tsx
title={`${node.label} (${typeLabel(node.node_type)})`}
aria-label={`${node.label}, ${typeLabel(node.node_type)}, ${node.degree} relationships`}
```

- [ ] **Step 3: Restyle graph surface and edges**

Replace graph dark/gold CSS with light Fluent CSS:

```css
.graph-canvas {
  background:
    linear-gradient(90deg, rgba(0, 0, 0, 0.035) 1px, transparent 1px),
    linear-gradient(0deg, rgba(0, 0, 0, 0.035) 1px, transparent 1px),
    var(--surface-solid);
  border: 1px solid var(--control-stroke);
}
.graph-edge-base {
  stroke: rgba(0, 0, 0, 0.22);
}
.graph-edge-flow {
  stroke: rgba(0, 103, 192, 0.42);
}
.graph-edge.active .graph-edge-base {
  stroke: var(--accent);
}
```

- [ ] **Step 4: Restyle compact graph nodes**

Use stable compact dimensions:

```css
.graph-node {
  min-width: 52px;
  width: 58px;
  min-height: 42px;
  padding: 5px 6px;
  background: var(--control-fill);
  color: var(--text-primary);
  border: 1px solid var(--control-stroke-strong);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
}
.graph-node-initial {
  font-size: 12px;
  font-weight: 700;
}
.graph-node-degree {
  min-width: 18px;
  height: 18px;
  border-radius: 999px;
  background: var(--accent-subtle);
  color: var(--accent-pressed);
}
```

- [ ] **Step 5: Restyle inspector, empty state, edge rows, and forms**

Keep these surfaces on light tokens:

```css
.graph-inspector,
.edge-row,
.edge-form select,
.edge-form input,
.edge-form textarea,
.graph-hint-chips span,
.graph-query-terms span {
  background: var(--surface-solid);
  color: var(--text-primary);
  border-color: var(--control-stroke);
}
```

- [ ] **Step 6: Run focused GREEN check**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: all tests in `fluentTokens.test.mjs` pass.

## Task 4: Full Verification

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run Node regression tests**

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
node --test tauri-app/src/graphLayout.test.mjs
```

Expected: PASS.

- [ ] **Step 2: Run frontend build**

From `tauri-app`:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 3: Run Rust tests**

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run whitespace check**

```powershell
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Manual Tauri smoke**

Run the desktop app and verify:

```powershell
cargo run --manifest-path tauri-app/src-tauri/Cargo.toml
```

Expected:
- dragging the title text/spacer moves the window,
- minimize works,
- maximize toggles,
- close hides to tray,
- Graph nodes are compact,
- Graph edges and inspector text are readable in the light WinUI shell.

## Self-Review

- Spec coverage: Title-bar permissions, drag behavior, close-to-tray, Graph node compactness, Graph edge/node Fluent styling, readability, tests, and manual smoke are covered.
- Placeholder scan: no TBD/TODO placeholders remain.
- Type consistency: `handleTitlebarDrag`, `graphNodeInitial`, `.titlebar-drag-zone`, `.graph-node-initial`, and `.graph-node-degree` are named consistently across tests and implementation steps.
