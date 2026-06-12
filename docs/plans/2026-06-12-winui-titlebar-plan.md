# WinUI 3 Custom Title Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the remaining native Tauri title bar with a tested, WinUI 3-style custom title bar that preserves close-to-tray behavior.

**Architecture:** Disable native window decorations in Tauri config, then render a lightweight React title bar above the existing navigation view. Window controls call Tauri v2 window APIs, while CSS draws stable glyphs and reserves shell height so the navigation/content layout does not overlap.

**Tech Stack:** Tauri v2, React 19, TypeScript, CSS, Node test runner, Cargo tests.

---

## File Structure

- Modify `tauri-app/src/fluentTokens.test.mjs`: add regression tests for Tauri decoration config, custom titlebar markup, accessible window controls, and CSS sizing.
- Modify `tauri-app/src-tauri/tauri.conf.json`: set the main window `decorations` to `false`.
- Modify `tauri-app/src/App.tsx`: import `getCurrentWindow`, add window control handlers, and wrap the existing shell in `.app-shell` with a `.app-titlebar`.
- Modify `tauri-app/src/index.css`: add titlebar/window-control styles and update `.app-navigation-view` height handling.
- Create `docs/specs/2026-06-12-winui-titlebar-spec.md`: record behavior and acceptance criteria.
- Create `docs/plans/2026-06-12-winui-titlebar-plan.md`: record this implementation plan.

## Task 1: Lock The Regression With Tests

**Files:**
- Modify: `tauri-app/src/fluentTokens.test.mjs`

- [ ] **Step 1: Add titlebar tests**

Add tests that read `tauri.conf.json`, `App.tsx`, and `index.css`, then assert:

```js
const tauriConfig = readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8");

test("tauri window uses a custom Fluent titlebar instead of native decorations", () => {
  const config = JSON.parse(tauriConfig);
  assert.equal(config.app.windows[0].decorations, false);
  assert.match(app, /@tauri-apps\/api\/window/);
  assert.match(app, /className="app-shell"/);
  assert.match(app, /className="app-titlebar"/);
  assert.match(app, /data-tauri-drag-region/);
  assert.match(app, /aria-label="Minimize window"/);
  assert.match(app, /aria-label="Maximize or restore window"/);
  assert.match(app, /aria-label="Close to tray"/);
  assert.match(app, /\.hide\(\)/);
  assert.doesNotMatch(app, /🗕|🗖|🗙|❌/);
});

test("fluent titlebar CSS defines stable WinUI-style window controls", () => {
  assert.match(css, /\.app-shell/);
  assert.match(css, /grid-template-rows:\s*48px minmax\(0,\s*1fr\)/);
  assert.match(css, /\.app-titlebar/);
  assert.match(css, /height:\s*48px/);
  assert.match(css, /\.window-controls/);
  assert.match(css, /\.window-control/);
  assert.match(css, /width:\s*46px/);
  assert.match(css, /\.window-control-close:hover/);
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected result: the new tests fail because native decorations are still enabled and the custom title bar does not exist.

## Task 2: Implement The Custom Title Bar

**Files:**
- Modify: `tauri-app/src-tauri/tauri.conf.json`
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`

- [ ] **Step 1: Disable native decorations**

Change the main window config:

```json
"decorations": false
```

- [ ] **Step 2: Add Tauri window handlers**

Import the Tauri window API:

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";
```

Add handlers in `App`:

```ts
const currentWindow = getCurrentWindow();

const handleMinimizeWindow = async () => {
  await currentWindow.minimize();
};

const handleToggleMaximizeWindow = async () => {
  if (await currentWindow.isMaximized()) {
    await currentWindow.unmaximize();
  } else {
    await currentWindow.maximize();
  }
};

const handleCloseToTray = async () => {
  await currentWindow.hide();
};
```

- [ ] **Step 3: Render the title bar**

Wrap the existing navigation view in `.app-shell` and add the titlebar before it:

```tsx
<div className="app-shell">
  <header className="app-titlebar" data-tauri-drag-region>
    <div className="titlebar-brand" data-tauri-drag-region>
      <span className="titlebar-app-mark" aria-hidden="true">A</span>
      <span className="titlebar-title" data-tauri-drag-region>AI Novel Factory</span>
    </div>
    <div className="window-controls">
      <button type="button" className="window-control" aria-label="Minimize window" onClick={handleMinimizeWindow}>
        <span className="window-glyph window-glyph-minimize" aria-hidden="true" />
      </button>
      <button type="button" className="window-control" aria-label="Maximize or restore window" onClick={handleToggleMaximizeWindow}>
        <span className="window-glyph window-glyph-maximize" aria-hidden="true" />
      </button>
      <button type="button" className="window-control window-control-close" aria-label="Close to tray" onClick={handleCloseToTray}>
        <span className="window-glyph window-glyph-close" aria-hidden="true" />
      </button>
    </div>
  </header>
  <div className="app-navigation-view">
    ...
  </div>
</div>
```

- [ ] **Step 4: Add CSS titlebar styles**

Add CSS for `.app-shell`, `.app-titlebar`, titlebar brand, window controls, glyphs, hover/active/focus states, and reduced mobile overflow. Update `.app-navigation-view` from `height: 100vh` to `height: 100%; min-height: 0;`.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected result: all tests in `fluentTokens.test.mjs` pass.

## Task 3: Full Verification And Integration

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run frontend build**

Run:

```powershell
npm run build
```

from `tauri-app`.

Expected result: TypeScript and Vite build pass.

- [ ] **Step 2: Run backend test suite**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml
```

Expected result: all Rust tests pass.

- [ ] **Step 3: Run whitespace diff check**

Run:

```powershell
git diff --check
```

Expected result: no whitespace errors.

- [ ] **Step 4: Commit and merge**

Run:

```powershell
git status --short
git add docs/specs/2026-06-12-winui-titlebar-spec.md docs/plans/2026-06-12-winui-titlebar-plan.md tauri-app/src/fluentTokens.test.mjs tauri-app/src-tauri/tauri.conf.json tauri-app/src/App.tsx tauri-app/src/index.css
git add docs/specs/2026-06-12-knowledge-library-acid-winui-spec.md docs/plans/2026-06-12-knowledge-library-acid-winui-plan.md tauri-app/src-tauri/prompts/draft_writer.md tauri-app/src-tauri/src/db/generation_jobs.rs tauri-app/src-tauri/src/db/knowledge_graph.rs tauri-app/src-tauri/src/lib.rs tauri-app/src-tauri/src/workflow/canon_updater.rs tauri-app/src-tauri/src/workflow/chapter_production.rs tauri-app/src-tauri/src/workflow/mod.rs tauri-app/src-tauri/src/workflow/task_transaction.rs tauri-app/src-tauri/src/workflow/writing_context.rs tauri-app/src-tauri/tests/core_writing_loop_tests.rs tauri-app/src-tauri/tests/generation_job_observability_tests.rs tauri-app/src-tauri/tests/knowledge_graph_tests.rs
git commit -m "feat: finish WinUI shell and task recovery"
git switch main
git merge feature/knowledge-acid-winui
```

Expected result: the feature branch is committed and merged locally.

- [ ] **Step 5: Re-run verification after merge**

Run the same focused Node test, frontend build, Rust tests, and diff check on the merged branch.

Expected result: merged `main` remains green.

## Self-Review

- Spec coverage: titlebar rendering, Tauri config, close-to-tray preservation, layout, accessibility, performance, tests, commit, and merge are covered.
- Placeholder scan: no TBD/TODO placeholders remain.
- Type consistency: handlers use Tauri v2 `getCurrentWindow`, `minimize`, `isMaximized`, `maximize`, `unmaximize`, and `hide`.
