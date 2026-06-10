# Runtime Stability and Writing UX Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden AI Novel Factory's Windows desktop runtime and make chapter production visibly live.

**Architecture:** Keep the existing Tauri + React + SQLite architecture. Add focused backend recovery functions and event payload fields, wire startup/tray lifecycle through existing `lib.rs`, and update the dashboard to consume live events without rewriting provider APIs.

**Tech Stack:** Rust 2021, Tauri 2, Tokio, rusqlite, React, TypeScript, Vite, CSS.

---

## File Structure

- Modify `tauri-app/src-tauri/src/db/generation_jobs.rs`: stale job recovery and active job interruption helpers.
- Modify `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`: recovery regression tests.
- Modify `tauri-app/src-tauri/src/models/generation_job.rs`: optional preview fields on `PipelineEvent`.
- Modify `tauri-app/src-tauri/src/workflow/chapter_production.rs`: emit preview events after draft/revision saves.
- Modify `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`: assert preview events are emitted by real workflow tests.
- Modify `tauri-app/src-tauri/src/lib.rs`: startup recovery, tray icon, restore helper, live event relay.
- Modify `tauri-app/src/App.tsx`: active-page rendering, resume refresh, live writer panel, preview animation.
- Modify `tauri-app/src/index.css`: editorial palette, local font stack, live writer styling, reduced radii.

## Tasks

### Task 1: Recover Stale Generation Jobs

**Files:**
- Modify: `tauri-app/src-tauri/tests/generation_job_observability_tests.rs`
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`

- [ ] **Step 1: Write the failing stale-job recovery test**

Add a test that creates a job, moves it to `reviewing`, ages `updated_at`, runs recovery, and expects `failed` with a useful error message:

```rust
#[test]
fn stale_running_jobs_are_marked_failed_on_recovery() {
    let db = setup_db();
    let project_id = seed_project(&db);
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-stale",
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None)
        .unwrap();

    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE generation_jobs
             SET updated_at = datetime('now', '-2 hours')
             WHERE id = ?1",
            rusqlite::params![job_id],
        )
        .unwrap();
    }

    let recovered = tauri_app_lib::db::generation_jobs::recover_stale_running_jobs(
        &db,
        600,
        "Application restarted while this generation job was still running.",
    )
    .unwrap();

    assert_eq!(recovered, 1);
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(
        jobs[0]
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("Application restarted")
    );
}
```

- [ ] **Step 2: Write the fresh-job guard test**

Add a second test proving fresh running work is not failed:

```rust
#[test]
fn fresh_running_jobs_are_not_recovered_as_stale() {
    let db = setup_db();
    let project_id = seed_project(&db);
    let job_id = tauri_app_lib::db::generation_jobs::create_generation_job(
        &db,
        &project_id,
        "plan-fresh",
    )
    .unwrap();
    tauri_app_lib::db::generation_jobs::update_job_status(&db, &job_id, "reviewing", None)
        .unwrap();

    let recovered = tauri_app_lib::db::generation_jobs::recover_stale_running_jobs(
        &db,
        600,
        "Application restarted while this generation job was still running.",
    )
    .unwrap();

    assert_eq!(recovered, 0);
    let jobs = tauri_app_lib::db::generation_jobs::get_generation_jobs(&db, &project_id).unwrap();
    assert_eq!(jobs[0].status, "reviewing");
}
```

- [ ] **Step 3: Verify RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests stale_running_jobs_are_marked_failed_on_recovery fresh_running_jobs_are_not_recovered_as_stale -- --nocapture
```

Expected: fails because `recover_stale_running_jobs` does not exist.

- [ ] **Step 4: Implement `recover_stale_running_jobs`**

Add a public function that selects stale non-terminal jobs by `updated_at`, then calls `update_job_status(..., "failed", Some(reason))` for each id:

```rust
pub fn recover_stale_running_jobs(
    db: &Database,
    timeout_secs: i64,
    reason: &str,
) -> Result<usize, String> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(timeout_secs.max(0));
    let cutoff_sql = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id FROM generation_jobs
             WHERE status IN ('started','draft_created','reviewing','revising','publishing')
               AND datetime(updated_at) <= datetime(?1)
             ORDER BY updated_at ASC",
        )
        .map_err(|e| format!("Prepare stale job recovery: {}", e))?;
    let ids = stmt
        .query_map(params![cutoff_sql], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Query stale jobs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect stale jobs: {}", e))?;
    drop(stmt);
    drop(conn);

    for id in &ids {
        update_job_status(db, id, "failed", Some(reason))?;
    }

    Ok(ids.len())
}
```

- [ ] **Step 5: Verify GREEN**

Run the same targeted command. Expected: both tests pass.

### Task 2: Wire Startup Recovery and Tray Lifecycle

**Files:**
- Modify: `tauri-app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add startup recovery**

In `setup`, after `workflow::lock::cleanup_stale_locks(&db, 600);`, call:

```rust
let _ = db::generation_jobs::recover_stale_running_jobs(
    &db,
    600,
    "Application restarted while this generation job was still running.",
);
```

- [ ] **Step 2: Add restore helper**

Add a small helper near `run()`:

```rust
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = app.emit_to("main", "app-resume", serde_json::json!({
            "reason": "tray_restore",
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        }));
    }
}
```

- [ ] **Step 3: Set explicit tray icon and clearer labels**

Use the bundled default icon and clearer menu labels:

```rust
let open = MenuItemBuilder::with_id("open", "Open AI Novel Factory").build(app)?;
let write = MenuItemBuilder::with_id("write", "Open Writing Console").build(app)?;
let quit = MenuItemBuilder::with_id("quit", "Quit Completely").build(app)?;

let mut tray_builder = TrayIconBuilder::new()
    .menu(&menu)
    .tooltip("AI Novel Factory - close hides to tray; Quit Completely exits");

if let Some(icon) = app.default_window_icon() {
    tray_builder = tray_builder.icon(icon.clone());
}
```

- [ ] **Step 4: Use restore helper and interrupt on quit**

Replace repeated `window.show()` / `window.set_focus()` blocks with `show_main_window(app)`. In the quit branch:

```rust
if let Some(state) = app.try_state::<AppState>() {
    let _ = db::generation_jobs::recover_stale_running_jobs(
        &state.db,
        0,
        "Application quit before this generation job completed.",
    );
}
app.exit(0);
```

- [ ] **Step 5: Compile check**

Run:

```powershell
cargo check --manifest-path tauri-app\src-tauri\Cargo.toml
```

Expected: compiles. If `try_state` or `unminimize` signatures differ in Tauri 2, adjust to the compiling API while preserving behavior.

### Task 3: Relay Pipeline Events Live

**Files:**
- Modify: `tauri-app/src-tauri/src/lib.rs`

- [ ] **Step 1: Start the relay before awaiting workflow**

In the `generate_next_chapter` command, replace post-run `try_recv` draining with an event relay task:

```rust
let (event_tx, mut event_rx) = mpsc::channel::<PipelineEvent>(50);
let app_for_events = app.clone();
let event_relay = tokio::spawn(async move {
    while let Some(ev) = event_rx.recv().await {
        let _ = app_for_events.emit_to("main", "pipeline-step", &ev);
    }
});
```

- [ ] **Step 2: Drop the sender and join relay after workflow**

After the workflow future resolves:

```rust
drop(event_tx);
let _ = event_relay.await;
```

Expected: events are emitted as soon as workflow sends them. No terminal drain loop remains.

- [ ] **Step 3: Compile check**

Run:

```powershell
cargo check --manifest-path tauri-app\src-tauri\Cargo.toml
```

Expected: compiles.

### Task 4: Emit Draft and Revision Preview Events

**Files:**
- Modify: `tauri-app/src-tauri/src/models/generation_job.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Extend `PipelineEvent`**

Add optional preview fields:

```rust
#[serde(default)]
pub preview_title: Option<String>,
#[serde(default)]
pub preview_text: Option<String>,
#[serde(default)]
pub preview_kind: Option<String>,
```

Update the basic `emit` constructor with `None` for these fields.

- [ ] **Step 2: Add a preview helper**

In `chapter_production.rs`, add:

```rust
fn preview_excerpt(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn emit_preview(
    event_tx: &mpsc::Sender<PipelineEvent>,
    step: &str,
    title: &str,
    body: &str,
    preview_kind: &str,
    progress_pct: f64,
) {
    let _ = event_tx.try_send(PipelineEvent {
        step: step.into(),
        status: "preview".into(),
        elapsed_ms: None,
        detail: Some(format!("{} preview ready", preview_kind)),
        progress_pct,
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        preview_title: Some(title.to_string()),
        preview_text: Some(preview_excerpt(body, 8000)),
        preview_kind: Some(preview_kind.to_string()),
    });
}
```

- [ ] **Step 3: Emit previews after saves**

After draft save:

```rust
emit_preview(event_tx, "draft_preview", &title, &body, "draft", 38.0);
```

After each successful revision save:

```rust
emit_preview(event_tx, "revision_preview", &title, &rev_body, "revision", 72.0);
```

- [ ] **Step 4: Write/extend workflow test**

In the existing core writing loop test that already collects events, assert a preview event exists:

```rust
let preview_event = events
    .iter()
    .find(|event| event["preview_kind"].as_str() == Some("draft"))
    .expect("draft preview event should be emitted");
assert!(
    preview_event["preview_text"]
        .as_str()
        .unwrap_or("")
        .contains("chapter")
);
```

- [ ] **Step 5: Run targeted test**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture
```

Expected: passes after implementation.

### Task 5: Frontend Resume, Active Page Rendering, and Live Writer Panel

**Files:**
- Modify: `tauri-app/src/App.tsx`

- [ ] **Step 1: Extend frontend event type**

Update dashboard pipeline state type:

```ts
type PipelineStep = {
  step: string;
  status: string;
  detail?: string;
  progress_pct: number;
  timestamp: string;
  preview_title?: string;
  preview_text?: string;
  preview_kind?: string;
};
```

- [ ] **Step 2: Render active page only**

Replace the full `Object.entries(pageComponents).map(...)` render with:

```tsx
const renderPage = () => {
  switch (page) {
    case "dashboard": return <Dashboard />;
    case "projects": return <ProjectList refresh={loadProjects} />;
    case "chapters": return <Chapters />;
    case "plans": return <ChapterPlans />;
    case "reviews": return <ReviewPage />;
    case "jobs": return <JobsPage />;
    case "bible": return <BiblePage />;
    case "graph": return <KnowledgeGraphPage />;
    case "learn": return <LearnPage />;
    case "settings": return <SettingsPage refreshSettings={refreshSettings} />;
    default: return <Dashboard />;
  }
};
```

Then render `{renderPage()}` in `app-main`.

- [ ] **Step 3: Add app resume refresh**

In `App`, add an effect that listens to `app-resume`, `visibilitychange`, and `focus`, then calls `loadStatus`, `loadLogs`, and `loadProjects`:

```ts
useEffect(() => {
  const refreshAfterResume = () => {
    loadProjects();
    if (selected) loadStatus();
    loadLogs();
  };
  let unlisten: (() => void) | null = null;
  listen("app-resume", refreshAfterResume).then(u => { unlisten = u; });
  window.addEventListener("focus", refreshAfterResume);
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) refreshAfterResume();
  });
  return () => {
    if (unlisten) unlisten();
    window.removeEventListener("focus", refreshAfterResume);
  };
}, [selected, loadProjects, loadStatus, loadLogs]);
```

- [ ] **Step 4: Track live preview in dashboard**

In `Dashboard`, add `livePreview` and `visiblePreview` state. When a pipeline event has `preview_text`, store it and progressively reveal it with a short interval.

- [ ] **Step 5: Render live writer panel**

Add a dashboard section beside or below the controls that shows:

- phase,
- progress,
- waiting animation,
- latest preview title,
- progressively revealed text.

- [ ] **Step 6: Build check**

Run:

```powershell
npm run build
```

from `tauri-app`. Expected: Vite build passes.

### Task 6: Editorial CSS Refresh

**Files:**
- Modify: `tauri-app/src/index.css`

- [ ] **Step 1: Remove remote font import**

Delete the Google Fonts `@import` line.

- [ ] **Step 2: Replace core tokens**

Use local system stacks and a less generic palette:

```css
:root {
  --primary: #d0a85c;
  --primary-pressed: #b88d42;
  --primary-active: #8f6b2e;
  --accent-teal: #4fb6a6;
  --canvas-dark: #111417;
  --surface-dark-elevated: #191f24;
  --surface-dark-card: #20262c;
  --on-dark: #f4efe4;
  --on-dark-body: rgba(244,239,228,0.76);
  --on-dark-mute: rgba(244,239,228,0.52);
  --warning: #d36b5d;
  --success: #70b77e;
  --font-display: "Segoe UI", system-ui, sans-serif;
  --font-body: "Segoe UI", system-ui, sans-serif;
  --font-content: Georgia, "Times New Roman", serif;
  --radius-full: 8px;
}
```

- [ ] **Step 3: Add live writer styles**

Add classes for `.live-writer`, `.live-writer-preview`, `.live-cursor`, and `.pipeline-timeline` with fixed dimensions and overflow handling so text does not shift the layout.

- [ ] **Step 4: Build check**

Run:

```powershell
npm run build
```

Expected: Vite build passes.

### Task 7: Full Verification

**Files:**
- No new files unless failures require fixes.

- [ ] **Step 1: Rust targeted tests**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test generation_job_observability_tests -- --nocapture
cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml --test core_writing_loop_tests -- --nocapture
```

- [ ] **Step 2: Rust full suite**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app\src-tauri\Cargo.toml
```

- [ ] **Step 3: Frontend production build**

Run:

```powershell
npm run build
```

from `tauri-app`.

- [ ] **Step 4: Whitespace check**

Run:

```powershell
git diff --check
```

Expected: exit code 0. CRLF warnings may appear in this repository and are acceptable when the command exits successfully.

- [ ] **Step 5: Manual Windows smoke**

Run:

```powershell
cd D:\novel\tauri-app
npm run tauri -- dev
```

Smoke checks:

- App launches without migration panic.
- Tray icon is visible.
- Close button hides to tray.
- Tray open restores a usable window.
- Quit fully exits the process.
- Starting a chapter shows live pipeline progress before completion.
- Draft preview appears after the draft writer returns.
