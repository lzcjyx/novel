# Agent Workbench Memory Pet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the app shell into an agent-style writing workbench, make the desktop pet an efficient phase-aware status assistant, and persist deterministic continuity memory after every completed chapter.

**Architecture:** Keep existing Tauri, React, and SQLite boundaries. Reframe top-level navigation and dashboard copy in React, add pet status fields through the existing Tauri event channel, and reuse `learning_entries` for deterministic chapter memory without adding another model call.

**Tech Stack:** Rust 2021, rusqlite, Tauri v2 events, React 19, TypeScript, CSS, Node test runner.

---

## File Structure

- Modify `tauri-app/src/App.tsx`: agent navigation surfaces, command bar copy, dashboard status payload to pet.
- Modify `tauri-app/src/pages/PetWindow.tsx`: merge partial status, render phase/progress/RAG indicators, reduce status reset churn.
- Modify `tauri-app/src/index.css`: agent shell styling and lightweight pet indicators.
- Modify `tauri-app/src/fluentTokens.test.mjs`: UI contract tests for agent shell and pet status.
- Modify `tauri-app/src-tauri/src/workflow/learning.rs`: add deterministic `remember_chapter_for_continuity`.
- Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`: call chapter memory after final chapter content is known and record task-owned rows best-effort.
- Modify `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`: assert chapter memory is saved while self-reflection remains.

## Task 1: Agent Shell Contract

- [ ] Add failing Node assertions in `tauri-app/src/fluentTokens.test.mjs`:

```js
test("top-level shell is reframed as an agent workbench with consolidated surfaces", () => {
  for (const label of ["Agent 总控", "流程编排", "记忆中枢", "质量审稿", "发布运维", "项目设置"]) {
    assert.ok(app.includes(label), `missing agent nav label ${label}`);
  }
  for (const legacy of ["章节计划", "小说圣经", "关系图谱", "学习库"]) {
    assert.ok(!app.includes(`${legacy}",`), `legacy rail label still exposed: ${legacy}`);
  }
  assert.match(app, /Agent 总控台/);
  assert.match(app, /agentSurfaceMap/);
});
```

- [ ] Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: fails because the agent labels and `agentSurfaceMap` are not implemented.

- [ ] Modify `App.tsx`:
  - Replace top-level rail labels with six surfaces.
  - Add `agentSurfaceMap` that routes surfaces to existing page components:
    - `agent`: dashboard
    - `orchestrate`: runtime
    - `memory`: learn
    - `quality`: reviews
    - `ops`: jobs
    - `settings`: settings
  - Keep project selection and scheduled-writing toggle.
  - Change dashboard headline copy to `Agent 总控台`.

- [ ] Run the Node test again.
  Expected: passes.

## Task 2: Pet Phase-Aware Status

- [ ] Add failing Node assertions in `tauri-app/src/fluentTokens.test.mjs`:

```js
test("desktop pet receives throttled agent phase status", () => {
  assert.match(app, /emitPetPipelineStatus/);
  assert.match(app, /lastPetEmitRef/);
  assert.match(app, /phaseLabel/);
  assert.match(app, /progressPct/);
  assert.match(css, /\.pet-progress/);
  assert.match(css, /\.pet-rag-indicator/);
});
```

- [ ] Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: fails until pet fields and CSS exist.

- [ ] Modify `App.tsx`:
  - Add `lastPetEmitRef`.
  - Add `emitPetPipelineStatus(ev)` that sends `phaseLabel`, `progressPct`, `statusText`, `running`, `loading` to the pet.
  - Throttle emits to 250ms unless progress is 100 or the step status is failed.
  - Call it from the existing `pipeline-step` listener.

- [ ] Modify `PetWindow.tsx`:
  - Add optional `phaseLabel`, `progressPct`, `statusText`.
  - Merge new pet event payload with previous status instead of `{...defaultStatus, ...payload}`.
  - Use phase/progress to choose copy when available.
  - Render `.pet-progress` and `.pet-rag-indicator`.

- [ ] Modify `index.css`:
  - Add small stable progress bar.
  - Add low-motion RAG indicator styles.
  - Keep CSS animations only, no `requestAnimationFrame`.

- [ ] Run the Node test again.
  Expected: passes.

## Task 3: Deterministic Chapter Continuity Memory

- [ ] Add failing Rust assertions in `tauri-app/src-tauri/tests/core_writing_loop_tests.rs` inside `chapter_pipeline_uses_writing_context_and_finalizes_plan`:

```rust
let chapter_memory_count: i64 = {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM learning_entries WHERE project_id = ?1 AND source_type = 'chapter_memory'",
        rusqlite::params![project_id],
        |row| row.get(0),
    ).unwrap()
};
assert_eq!(chapter_memory_count, 1);

let chapter_memory: (String, String, String) = {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT category, pattern_name, application_notes FROM learning_entries WHERE project_id = ?1 AND source_type = 'chapter_memory' LIMIT 1",
        rusqlite::params![project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap()
};
assert_eq!(chapter_memory.0, "continuity_memory");
assert!(chapter_memory.1.contains("章节记忆"));
assert!(chapter_memory.2.contains("下一章"));
```

- [ ] Run:

```powershell
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture
```

Expected: fails because no `chapter_memory` row exists.

- [ ] Modify `tauri-app/src-tauri/src/workflow/learning.rs`:
  - Add `pub fn remember_chapter_for_continuity(...) -> Result<Option<String>, String>`.
  - Build a `LearningEntry` with `source_type = "chapter_memory"` and `category = "continuity_memory"`.
  - Use `save_learning_entries_with_style_drafts` and return the saved id.
  - Keep text deterministic; no model call.

- [ ] Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`:
  - After self-reflection attempt and before final completion, call `remember_chapter_for_continuity`.
  - Record the row in `task_transaction::record_task_owned_row` when an id is returned.
  - Log success or noncritical skip.

- [ ] Run the targeted Rust test again.
  Expected: passes.

## Task 4: Full Verification

- [ ] Run targeted suites:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture
```

- [ ] Run full backend:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture
```

- [ ] Run full frontend contracts:

```powershell
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
```

- [ ] Run production build:

```powershell
npm run build
```

Working directory: `D:\novel\tauri-app`.

- [ ] Run:

```powershell
git diff --check
git status -sb
```

Expected: no whitespace errors; only planned files changed.

## Task 5: Commit And Push

- [ ] Stage planned files:

```powershell
git add docs/superpowers/specs/2026-06-29-agent-workbench-memory-pet-design.md docs/superpowers/plans/2026-06-29-agent-workbench-memory-pet.md tauri-app/src/App.tsx tauri-app/src/pages/PetWindow.tsx tauri-app/src/index.css tauri-app/src/fluentTokens.test.mjs tauri-app/src-tauri/src/workflow/learning.rs tauri-app/src-tauri/src/workflow/chapter_production.rs tauri-app/src-tauri/tests/core_writing_loop_tests.rs
```

- [ ] Commit:

```powershell
git commit -m "feat: reframe app as agent workbench"
```

- [ ] Push current branch:

```powershell
git push origin codex/integrated-runtime-control
```

## Self-Review

- Spec coverage: agent references, UI consolidation, pet status/performance, deterministic chapter memory, and full verification are covered.
- Placeholder scan: no placeholders or vague tasks remain.
- Type consistency: `phaseLabel`, `progressPct`, `statusText`, `remember_chapter_for_continuity`, and `chapter_memory` are consistent across tests and implementation tasks.
