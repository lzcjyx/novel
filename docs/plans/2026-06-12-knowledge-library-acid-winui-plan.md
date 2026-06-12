# Knowledge Library, ACID Recovery, and WinUI 3 UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Knowledge Library participation in generation auditable, make interrupted generation jobs fail and roll back project data, and restyle the desktop shell toward WinUI 3 / Fluent.

**Architecture:** Keep the existing Tauri + React + SQLite architecture. Add a small Rust task transaction module that records task-owned rows in generation job metadata and performs recovery as a compensation transaction; add focused metadata tests before touching production code. Refresh the UI through tokens, shell structure, and component classes in the existing React/CSS files, with a small Node test for Fluent token invariants.

**Tech Stack:** Rust 2021, Tauri 2, rusqlite, serde_json, React 19, TypeScript, Vite, Node built-in test runner, CSS.

---

## File Map

- Create `tauri-app/src-tauri/src/workflow/task_transaction.rs`: task snapshot structs, metadata merge helpers, task-owned row recording, recovery transaction.
- Modify `tauri-app/src-tauri/src/workflow/mod.rs`: export `task_transaction`.
- Modify `tauri-app/src-tauri/src/db/generation_jobs.rs`: initialize task snapshot on job creation, expose recovery using task rollback, keep phase summary.
- Modify `tauri-app/src-tauri/src/db/chapters.rs`: return created IDs and support metadata updates needed for job provenance.
- Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`: pass `job_id` into context metadata, record selected learning entries, tag task-created rows, and use rollback recovery on error.
- Modify `tauri-app/src-tauri/src/workflow/canon_updater.rs`: tag task-created canon rows and graph edges with `generation_job_id` or record their IDs.
- Modify `tauri-app/src-tauri/src/db/knowledge_graph.rs`: allow insert/create edge metadata for task ownership.
- Modify `tauri-app/src-tauri/src/lib.rs`: make `reset_running` call rollback recovery and keep tray Quit startup recovery paths aligned.
- Modify `tauri-app/src-tauri/prompts/draft_writer.md`: explicitly require applying learned patterns and returning `learning_entry:<id>` in `used_context_ids`.
- Modify `tauri-app/src/App.tsx`: WinUI-like shell structure, source marker text badges, command/status bar markup.
- Modify `tauri-app/src/index.css`: Fluent token set, NavigationView shell, InfoBar, command bar, button/input/tabs/list states, responsive and reduced-motion rules.
- Create `tauri-app/src/fluentTokens.test.mjs`: CSS token and no-emoji structural UI checks.
- Modify tests in `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`.
- Modify tests in `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`.

## Tasks

### Task 1: Prove Knowledge Library generation provenance

**Files:**
- Modify test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/prompts/draft_writer.md`

- [ ] **Step 1: Write the failing test**

Add this test near the existing learning/writing-context tests in `core_writing_loop_tests.rs`:

```rust
#[tokio::test]
async fn knowledge_library_context_is_persisted_in_generation_metadata() {
    let db = setup_db();
    let project_id = insert_project_with_plan(&db, "plan-learned-meta");
    insert_minimal_bible(&db, &project_id);
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO learning_entries
             (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
             VALUES ('learn-meta-1', ?1, 'manual_file', 'Style Notes', 'style_pattern', 'Object pressure', 'Use objects to carry emotional pressure.', 0.91)",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    let provider = RecordingProvider::with_publish_ready_draft();
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(32);
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel(32);

    let result = tauri_app_lib::workflow::chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project_id,
        true,
        &log_tx,
        &event_tx,
        None,
    )
    .await
    .expect("generation should succeed");

    let chapter_id = result.chapter_id.expect("chapter id");
    let latest = tauri_app_lib::db::chapters::get_latest_version(&db, &chapter_id)
        .unwrap()
        .expect("latest version");
    let metadata: serde_json::Value = serde_json::from_str(&latest.metadata).unwrap();

    assert_eq!(
        metadata["selected_learning_entry_ids"],
        serde_json::json!(["learn-meta-1"])
    );
    assert_eq!(
        metadata["selected_learning_entries"][0]["pattern_name"],
        "Object pressure"
    );
    assert!(metadata["learning_context_hash"].as_str().unwrap_or("").len() >= 16);

    let systems = provider.systems.lock().unwrap().join("\n");
    assert!(systems.contains("Object pressure"));
    assert!(systems.contains("learning_entry:learn-meta-1"));

    let entries = tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
        .expect("learning entries");
    assert_eq!(entries[0].usage_count, 1);
}
```

- [ ] **Step 2: Run the targeted RED command**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests knowledge_library_context_is_persisted_in_generation_metadata -- --nocapture
```

Expected: FAIL because `chapter_versions.metadata` does not contain `selected_learning_entry_ids` or `learning_context_hash`, and the prompt does not mention `learning_entry:<id>`.

- [ ] **Step 3: Implement compact learning provenance**

In `chapter_production.rs`, extend `build_context_metadata`:

```rust
fn compact_learning_entries(context: &writing_context::WritingContextPackage) -> Vec<serde_json::Value> {
    context
        .learned_patterns
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "category": entry.category,
                "pattern_name": entry.pattern_name,
                "source_type": entry.source_type,
                "confidence": entry.confidence,
            })
        })
        .collect()
}

fn learning_context_hash(entries: &[serde_json::Value]) -> String {
    let payload = serde_json::to_string(entries).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}
```

Add these fields to the metadata JSON:

```rust
let selected_learning_entries = compact_learning_entries(context);
let selected_learning_entry_ids = context
    .learned_patterns
    .iter()
    .map(|entry| entry.id.clone())
    .collect::<Vec<_>>();
let learning_context_hash = learning_context_hash(&selected_learning_entries);

serde_json::json!({
    "selected_retrieval_source_keys": selected_retrieval_source_keys,
    "selected_retrieval_document_ids": selected_retrieval_document_ids,
    "retrieval_trace": context.retrieval_trace,
    "graph_context": context.graph_context,
    "selected_learning_entry_ids": selected_learning_entry_ids,
    "selected_learning_entries": selected_learning_entries,
    "learning_context_hash": learning_context_hash,
})
```

- [ ] **Step 4: Update the draft prompt**

In `tauri-app/src-tauri/prompts/draft_writer.md`, add this under the input context section:

```markdown
## Knowledge Library 使用要求
- 如果 writing_context.learned_patterns 非空，必须优先吸收其中与本章相关的技巧。
- 不要机械复述 pattern_name；要把技巧落实到场景、动作、对话、节奏和叙事选择里。
- 对实际使用的条目，在 used_context_ids 中写入 `learning_entry:<id>`。
```

- [ ] **Step 5: Run GREEN verification**

Run the same targeted command. Expected: PASS.

### Task 2: Add task transaction snapshot and rollback tests

**Files:**
- Modify test: `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`
- Create: `tauri-app/src-tauri/src/workflow/task_transaction.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`

- [ ] **Step 1: Write the failing interrupted recovery test**

Add to `generation_job_observability_tests.rs`:

```rust
#[test]
fn interrupted_generation_recovery_rolls_back_task_owned_rows() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-rollback");
    let job_id =
        tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-rollback")
            .unwrap();

    tauri_app_lib::workflow::task_transaction::begin_generation_task_snapshot(
        &db,
        &job_id,
        &project_id,
        "plan-rollback",
    )
    .unwrap();

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE chapter_plans SET status = 'in_progress' WHERE id = 'plan-rollback'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapters (id, project_id, chapter_plan_id, sequence, title, status)
             VALUES ('chapter-owned', ?1, 'plan-rollback', 1, 'Owned Draft', 'draft')",
            rusqlite::params![project_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chapter_versions (id, chapter_id, project_id, version_number, version_type, title, metadata)
             VALUES ('version-owned', 'chapter-owned', ?1, 1, 'draft', 'Owned Draft', '{\"generation_job_id\":\"job-owned\"}')",
            rusqlite::params![project_id],
        )
        .unwrap();
    }

    tauri_app_lib::workflow::task_transaction::record_task_owned_row(
        &db,
        &job_id,
        "chapters",
        "chapter-owned",
    )
    .unwrap();
    tauri_app_lib::workflow::task_transaction::record_task_owned_row(
        &db,
        &job_id,
        "chapter_versions",
        "version-owned",
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    let recovered = tauri_app_lib::db::generation_jobs::recover_interrupted_generation_jobs(
        &db,
        0,
        "Application quit before this generation job completed.",
    )
    .unwrap();

    assert_eq!(recovered, 1);
    let conn = db.conn.lock().unwrap();
    let chapter_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM chapters WHERE id = 'chapter-owned'", [], |row| row.get(0))
        .unwrap();
    let version_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM chapter_versions WHERE id = 'version-owned'", [], |row| row.get(0))
        .unwrap();
    let plan_status: String = conn
        .query_row("SELECT status FROM chapter_plans WHERE id = 'plan-rollback'", [], |row| row.get(0))
        .unwrap();
    let job_status: String = conn
        .query_row("SELECT status FROM generation_jobs WHERE id = ?1", rusqlite::params![job_id], |row| row.get(0))
        .unwrap();

    assert_eq!(chapter_count, 0);
    assert_eq!(version_count, 0);
    assert_eq!(plan_status, "planned");
    assert_eq!(job_status, "failed");
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests interrupted_generation_recovery_rolls_back_task_owned_rows -- --nocapture
```

Expected: FAIL because `workflow::task_transaction` and `recover_interrupted_generation_jobs` do not exist.

- [ ] **Step 3: Create task transaction module**

Create `task_transaction.rs` with:

```rust
use crate::db::connection::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskOwnedRows {
    pub chapters: Vec<String>,
    pub chapter_versions: Vec<String>,
    pub agent_reviews: Vec<String>,
    pub review_scores: Vec<String>,
    pub blog_posts: Vec<String>,
    pub publication_queue: Vec<String>,
    pub vector_document_metadata: Vec<String>,
    pub character_states: Vec<String>,
    pub timeline_events: Vec<String>,
    pub foreshadowing: Vec<String>,
    pub knowledge_graph_edges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTaskSnapshot {
    pub job_id: String,
    pub project_id: String,
    pub chapter_plan_id: String,
    pub plan_status_before: String,
    pub plan_metadata_before: String,
    pub started_at: String,
    pub owned_rows: TaskOwnedRows,
}
```

Implement:

```rust
pub fn begin_generation_task_snapshot(
    db: &Database,
    job_id: &str,
    project_id: &str,
    chapter_plan_id: &str,
) -> Result<(), String>;

pub fn record_task_owned_row(
    db: &Database,
    job_id: &str,
    table: &str,
    row_id: &str,
) -> Result<(), String>;

pub fn rollback_generation_task(
    db: &Database,
    job_id: &str,
    reason: &str,
) -> Result<bool, String>;
```

Use `generation_jobs.metadata.task_snapshot` as the storage location. `record_task_owned_row` must de-duplicate IDs. `rollback_generation_task` must run a single SQLite transaction and delete child rows before parent rows:

```rust
const DELETE_ORDER: &[(&str, &str)] = &[
    ("agent_reviews", "agent_reviews"),
    ("review_scores", "review_scores"),
    ("blog_posts", "blog_posts"),
    ("publication_queue", "publication_queue"),
    ("vector_document_metadata", "vector_document_metadata"),
    ("knowledge_graph_edges", "knowledge_graph_edges"),
    ("character_states", "character_states"),
    ("timeline_events", "timeline_events"),
    ("foreshadowing", "foreshadowing"),
    ("chapter_versions", "chapter_versions"),
    ("chapters", "chapters"),
];
```

- [ ] **Step 4: Export module**

Add to `workflow/mod.rs`:

```rust
pub mod task_transaction;
```

- [ ] **Step 5: Add recovery wrapper**

In `db/generation_jobs.rs`, add:

```rust
pub fn recover_interrupted_generation_jobs(
    db: &Database,
    timeout_secs: i64,
    reason: &str,
) -> Result<usize, String> {
    let cutoff_modifier = format!("-{} seconds", timeout_secs.max(0));
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let job_ids = query_active_jobs_older_than(&conn, &cutoff_modifier)?;
    drop(conn);

    let mut recovered = 0;
    for job_id in &job_ids {
        if crate::workflow::task_transaction::rollback_generation_task(db, job_id, reason)? {
            recovered += 1;
        }
    }
    Ok(recovered)
}
```

Keep `recover_stale_running_jobs` as a compatibility wrapper that calls `recover_interrupted_generation_jobs`.

- [ ] **Step 6: Run GREEN**

Run the targeted command again. Expected: PASS.

### Task 3: Wire task ownership into chapter generation

**Files:**
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/src/db/chapters.rs`
- Modify: `tauri-app/src-tauri/src/workflow/canon_updater.rs`
- Modify: `tauri-app/src-tauri/src/db/knowledge_graph.rs`

- [ ] **Step 1: Begin snapshot immediately after job creation**

In `chapter_production.rs`, after:

```rust
let job_id = generation_jobs::create_generation_job(db, project_id, &plan.id)?;
```

add:

```rust
crate::workflow::task_transaction::begin_generation_task_snapshot(
    db,
    &job_id,
    project_id,
    &plan.id,
)?;
```

- [ ] **Step 2: Record created chapter and version**

After `save_draft_version`, add:

```rust
crate::workflow::task_transaction::record_task_owned_row(db, &job_id, "chapters", &chapter_id)?;
crate::workflow::task_transaction::record_task_owned_row(
    db,
    &job_id,
    "chapter_versions",
    &version_id,
)?;
```

When saving revision versions, record each `rev_version_id` the same way.

- [ ] **Step 3: Record review rows**

Change `db::reviews::save_agent_review` to return the inserted review ID, or add a new `save_agent_review_returning_id` helper. After saving a review, record:

```rust
crate::workflow::task_transaction::record_task_owned_row(db, &job_id, "agent_reviews", &review_id)?;
```

Do the same for `review_scores` if `save_review_scores` is updated to return an ID.

- [ ] **Step 4: Record local blog draft row**

When `create_local_blog_draft` or `blog_posts::create_blog_post_with_metadata` returns an ID during generation, record it under `blog_posts`.

- [ ] **Step 5: Tag/record canon updater rows**

Change canon updater signature:

```rust
pub async fn update_canon_after_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    project_id: &str,
    chapter_id: &str,
    chapter_draft: &serde_json::Value,
    generation_job_id: Option<&str>,
) -> Result<(), String>
```

Whenever it inserts `character_states`, `timeline_events`, introduced `foreshadowing`, or graph edges, call `record_task_owned_row` when `generation_job_id` is `Some(job_id)`.

- [ ] **Step 6: Update graph edge insert metadata**

In `db/knowledge_graph.rs`, add a metadata-aware helper:

```rust
pub fn insert_edge_with_metadata(
    db: &Database,
    project_id: &str,
    source_id: &str,
    source_type: &str,
    target_id: &str,
    target_type: &str,
    edge_type: &str,
    description: Option<&str>,
    auto_inferred: bool,
    confidence: f64,
    metadata: &serde_json::Value,
) -> Result<String, String>
```

Have existing `insert_edge` call it with `{}`.

- [ ] **Step 7: Run recovery tests**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture
```

Expected: PASS.

### Task 4: Align startup, Quit, and Reset Stuck Job recovery

**Files:**
- Modify test: `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`

- [ ] **Step 1: Write Reset recovery test**

Add:

```rust
#[test]
fn reset_stuck_job_uses_recovery_and_restores_plan() {
    let db = setup_db();
    let project_id = insert_project(&db);
    insert_plan(&db, &project_id, "plan-reset-rollback");
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-reset-rollback",
    )
    .unwrap();
    tauri_app_lib::workflow::task_transaction::begin_generation_task_snapshot(
        &db,
        &job_id,
        &project_id,
        "plan-reset-rollback",
    )
    .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute("UPDATE chapter_plans SET status = 'in_progress' WHERE id = 'plan-reset-rollback'", []).unwrap();
    }
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None).unwrap();

    tauri_app_lib::db::generation_jobs::recover_project_interrupted_jobs(
        &db,
        &project_id,
        "operator reset stuck job",
    )
    .unwrap();

    let conn = db.conn.lock().unwrap();
    let plan_status: String = conn
        .query_row("SELECT status FROM chapter_plans WHERE id = 'plan-reset-rollback'", [], |row| row.get(0))
        .unwrap();
    let job_status: String = conn
        .query_row("SELECT status FROM generation_jobs WHERE id = ?1", rusqlite::params![job_id], |row| row.get(0))
        .unwrap();
    assert_eq!(plan_status, "planned");
    assert_eq!(job_status, "failed");
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests reset_stuck_job_uses_recovery_and_restores_plan -- --nocapture
```

Expected: FAIL because `recover_project_interrupted_jobs` does not exist or reset does not call it.

- [ ] **Step 3: Add project-scoped recovery helper**

In `generation_jobs.rs`:

```rust
pub fn recover_project_interrupted_jobs(
    db: &Database,
    project_id: &str,
    reason: &str,
) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let job_ids = query_active_jobs_for_project(&conn, project_id)?;
    drop(conn);
    let mut recovered = 0;
    for job_id in job_ids {
        if crate::workflow::task_transaction::rollback_generation_task(db, &job_id, reason)? {
            recovered += 1;
        }
    }
    Ok(recovered)
}
```

- [ ] **Step 4: Update Tauri command and tray Quit**

In `lib.rs`, change `reset_running` and tray Quit to call the new rollback recovery helpers:

```rust
let _ = db::generation_jobs::recover_project_interrupted_jobs(
    &state.db,
    &project_id,
    "operator reset stuck job",
);
```

Startup recovery continues using:

```rust
db::generation_jobs::recover_interrupted_generation_jobs(
    &db,
    600,
    "Application restarted while this generation job was still running.",
);
```

- [ ] **Step 5: Run GREEN**

Run the targeted reset test and all generation job observability tests. Expected: PASS.

### Task 5: Fluent token and UI structure tests

**Files:**
- Create: `tauri-app/src/fluentTokens.test.mjs`
- Modify: `tauri-app/src/index.css`
- Modify: `tauri-app/src/App.tsx`

- [ ] **Step 1: Write failing Node UI token test**

Create `tauri-app/src/fluentTokens.test.mjs`:

```javascript
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("./index.css", import.meta.url), "utf8");
const app = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

test("fluent shell tokens replace the old dark gold shell", () => {
  assert.match(css, /--accent:\s*#0067c0/i);
  assert.match(css, /--mica-bg:/);
  assert.match(css, /--control-fill:/);
  assert.match(css, /font-family:\s*var\(--font-system\)/);
  assert.doesNotMatch(css, /--primary:\s*#d0a85c/i);
  assert.doesNotMatch(css, /--canvas-dark:\s*#111417/i);
});

test("app shell uses WinUI-like navigation and command bar classes", () => {
  assert.match(app, /className="app-navigation-view"/);
  assert.match(app, /className="app-command-bar"/);
  assert.match(app, /className="nav-icon"/);
  assert.match(app, /className="info-bar/);
});

test("learn source markers do not use emoji as structural icons", () => {
  assert.doesNotMatch(app, /🌐|🔄|📝/);
  assert.match(app, /source-marker/);
});
```

- [ ] **Step 2: Run RED**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: FAIL because Fluent tokens/classes are not present and emoji markers still exist.

- [ ] **Step 3: Update App shell markup**

In `App.tsx`, rename shell classes:

```tsx
<div className="app-navigation-view">
  <aside className="navigation-pane">
    <div className="navigation-brand">AI Novel Factory</div>
    ...
    <button className={`navigation-item ${page === p ? "active" : ""}`}>
      <span className="nav-icon" aria-hidden="true">{navIcon[p]}</span>
      <span>{navLabels[p]}</span>
    </button>
  </aside>
  <section className="app-frame">
    <div className="app-command-bar">
      ...
    </div>
    <main className="app-main">
      {renderPage()}
    </main>
  </section>
</div>
```

Use text badges or CSS markers for icons for now, for example `D`, `P`, `C`, not emoji.

- [ ] **Step 4: Replace Learn emoji source markers**

Replace:

```tsx
{e.source_type === "web" ? "🌐" : e.source_type === "self_reflection" ? "🔄" : "📝"}
```

with:

```tsx
<span className={`source-marker source-marker-${e.source_type || "file"}`} aria-hidden="true">
  {sourceMarkerLabel(e.source_type)}
</span>
```

and define:

```ts
const sourceMarkerLabel = (sourceType?: string) =>
  sourceType === "web" ? "Web" : sourceType === "self_reflection" ? "Review" : "File";
```

- [ ] **Step 5: Rewrite CSS tokens and core controls**

At the top of `index.css`, replace token definitions with:

```css
:root {
  --font-system: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  --accent: #0067c0;
  --accent-hover: #005a9e;
  --accent-pressed: #004578;
  --accent-subtle: #e5f1fb;
  --on-accent: #ffffff;
  --mica-bg: #f3f3f3;
  --app-bg: #f7f7f7;
  --surface: rgba(255, 255, 255, 0.82);
  --surface-solid: #ffffff;
  --surface-subtle: #f9f9f9;
  --control-fill: #ffffff;
  --control-fill-hover: #f5f5f5;
  --control-stroke: rgba(0, 0, 0, 0.12);
  --text-primary: #1a1a1a;
  --text-secondary: #5f5f5f;
  --text-tertiary: #777777;
  --danger: #c42b1c;
  --success: #107c10;
  --warning: #9d5d00;
  --focus-ring: #005fb8;
  --radius-control: 4px;
  --radius-surface: 8px;
  --shadow-flyout: 0 8px 24px rgba(0, 0, 0, 0.12);
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;
}
```

Then remap legacy variable names for incremental compatibility:

```css
:root {
  --primary: var(--accent);
  --primary-pressed: var(--accent-pressed);
  --primary-active: var(--accent-pressed);
  --on-primary: var(--on-accent);
  --canvas-dark: var(--mica-bg);
  --surface-dark-elevated: var(--surface);
  --surface-dark-card: var(--surface-solid);
  --on-dark: var(--text-primary);
  --on-dark-body: var(--text-secondary);
  --on-dark-mute: var(--text-tertiary);
  --hairline-dark: var(--control-stroke);
  --font-display: var(--font-system);
  --font-body: var(--font-system);
}
```

- [ ] **Step 6: Run Node test**

Run:

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
```

Expected: PASS.

### Task 6: Finish UI responsive and accessibility pass

**Files:**
- Modify: `tauri-app/src/index.css`
- Modify: `tauri-app/src/App.tsx`

- [ ] **Step 1: Add focus and motion rules**

Ensure:

```css
:where(button, input, select, textarea, [tabindex]):focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
  }
}
```

- [ ] **Step 2: Add InfoBar classes**

Map existing message markup to:

```css
.info-bar,
.msg-banner {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: 1px solid var(--control-stroke);
  border-radius: var(--radius-surface);
  background: var(--surface-solid);
  color: var(--text-primary);
}
.msg-error { border-color: rgba(196, 43, 28, 0.36); background: #fdf3f2; color: var(--danger); }
.msg-success { border-color: rgba(16, 124, 16, 0.32); background: #f1faf1; color: var(--success); }
```

- [ ] **Step 3: Add responsive shell rules**

At the existing responsive breakpoint, keep the navigation reachable and avoid horizontal overflow:

```css
@media (max-width: 980px) {
  .app-navigation-view {
    grid-template-columns: 1fr;
  }
  .navigation-pane {
    position: sticky;
    top: 0;
    z-index: 20;
    width: 100%;
    flex-direction: row;
    overflow-x: auto;
  }
  .app-frame,
  .app-main {
    min-width: 0;
  }
}
```

- [ ] **Step 4: Build frontend**

Run from `tauri-app`:

```powershell
npm run build
```

Expected: PASS.

### Task 7: Full verification

**Files:**
- All files touched by previous tasks.

- [ ] **Step 1: Run Knowledge Library targeted test**

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests knowledge_library_context_is_persisted_in_generation_metadata -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run recovery targeted tests**

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests interrupted_generation_recovery_rolls_back_task_owned_rows -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests reset_stuck_job_uses_recovery_and_restores_plan -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run focused test files**

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test generation_job_observability_tests -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Run frontend checks**

```powershell
node --test tauri-app/src/fluentTokens.test.mjs
npm run build
```

Expected: PASS.

- [ ] **Step 5: Run full Rust and whitespace checks**

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml
git diff --check
```

Expected: PASS. If `git diff --check` reports only existing CRLF conversion warnings with exit code 0, record that.

## Self-Review

Spec coverage: Task 1 covers Knowledge Library prompt and metadata provenance. Tasks 2-4 cover ACID/Saga recovery, failed job visibility, plan restoration, and reset/quit/startup paths. Tasks 5-6 cover WinUI 3 / Fluent shell, token, emoji removal, motion, focus, and responsive behavior. Task 7 covers required verification.

Incomplete-marker scan: no incomplete tasks remain; every task names concrete files, commands, and expected outcomes.

Type consistency: `GenerationTaskSnapshot`, `TaskOwnedRows`, `begin_generation_task_snapshot`, `record_task_owned_row`, `rollback_generation_task`, `recover_interrupted_generation_jobs`, and `recover_project_interrupted_jobs` are named consistently across tests and implementation tasks.
