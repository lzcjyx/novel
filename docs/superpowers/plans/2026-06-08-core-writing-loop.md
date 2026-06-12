# Core Writing Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make chapter generation coherent, less generic, and more user-directed by repairing the writing context, prompt rendering, learning loop, finalization state, and minimal Dashboard controls.

**Architecture:** Add focused Rust helpers for strict prompt rendering, writing context assembly, and learning-entry selection/usage. Keep `chapter_production` as the orchestrator, but make it consume the context package, emit truthful pipeline events, and finalize against the selected final version. Add a small React control panel that passes optional operator controls without breaking the existing `force` flow.

**Tech Stack:** Rust 2021, Tauri v2, rusqlite SQLite, serde/serde_json, tokio, React 19, TypeScript, Vite.

---

## File Map

- Create: `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`
  - Owns strict prompt rendering and unresolved-placeholder detection.
- Create: `tauri-app/src-tauri/src/workflow/writing_context.rs`
  - Builds the `WritingContextPackage` consumed by the draft writer.
- Create: `tauri-app/src-tauri/prompts/weekly_planner.md`
  - Dedicated prompt for weekly chapter planning.
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
  - Exposes the new workflow modules.
- Modify: `tauri-app/src-tauri/src/prompts/mod.rs`
  - Registers `weekly_planner`.
- Modify: `tauri-app/src-tauri/prompts/draft_writer.md`
  - Replaces mixed placeholders with `{{WRITING_CONTEXT_JSON}}`.
- Modify: `tauri-app/src-tauri/src/workflow/weekly_planner.rs`
  - Loads the dedicated planner prompt through strict rendering.
- Modify: `tauri-app/src-tauri/src/workflow/learning.rs`
  - Adds top-entry selection, usage marking, and reflection persistence helpers.
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`
  - Returns the real inserted or existing job ID.
- Modify: `tauri-app/src-tauri/src/db/chapters.rs`
  - Adds explicit chapter-plan completion helper.
- Modify: `tauri-app/src-tauri/src/workflow/canon_updater.rs`
  - Renders canon extractor prompts strictly and accepts final chapter JSON.
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
  - Integrates context, learning, final canon, status transitions, and complete pipeline events.
- Modify: `tauri-app/src-tauri/src/lib.rs`
  - Extends `generate_next_chapter` command with optional operator controls and adds context preview command.
- Modify: `tauri-app/src/App.tsx`
  - Adds Dashboard chapter controls and context preview.
- Modify: `docs/operations.md`
  - Marks Neon/n8n content as legacy and adds local SQLite operation notes.
- Modify: `docs/troubleshooting.md`
  - Adds desktop-specific troubleshooting entries.
- Create: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`
  - Regression tests for prompt rendering, planning prompt, context, learning, finalization, jobs, plan status, and pipeline events.

---

### Task 1: Strict Prompt Rendering

**Files:**
- Create: `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write the failing prompt rendering tests**

Add this test file with the first two tests:

```rust
use std::collections::HashMap;

use tauri_app_lib::workflow::prompt_rendering::{
    find_unresolved_placeholders, render_prompt_strict,
};

#[test]
fn strict_prompt_rendering_rejects_unresolved_placeholders() {
    let vars = HashMap::from([("KNOWN", "value".to_string())]);

    let err = render_prompt_strict("demo", "A {{KNOWN}} and {{MISSING}}", &vars)
        .expect_err("unresolved placeholders must fail before model calls");

    assert!(err.contains("demo"));
    assert!(err.contains("MISSING"));
}

#[test]
fn strict_prompt_rendering_replaces_all_known_placeholders() {
    let vars = HashMap::from([
        ("PROJECT", "镜城".to_string()),
        ("CHAPTER", "第七章".to_string()),
    ]);

    let rendered = render_prompt_strict("demo", "{{PROJECT}} / {{CHAPTER}}", &vars)
        .expect("all placeholders are supplied");

    assert_eq!(rendered, "镜城 / 第七章");
    assert!(find_unresolved_placeholders(&rendered).is_empty());
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml strict_prompt_rendering -- --nocapture
```

Expected: FAIL because `workflow::prompt_rendering` does not exist.

- [ ] **Step 3: Implement the strict renderer**

Create `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`:

```rust
use std::collections::HashMap;

use regex::Regex;

pub fn find_unresolved_placeholders(text: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*([A-Za-z0-9_]+)\s*\}\}").expect("valid placeholder regex");
    let mut names = re
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

pub fn render_prompt_strict(
    prompt_name: &str,
    template: &str,
    vars: &HashMap<&str, String>,
) -> Result<String, String> {
    let mut rendered = template.to_string();
    for (key, value) in vars {
        rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
    }

    let unresolved = find_unresolved_placeholders(&rendered);
    if unresolved.is_empty() {
        Ok(rendered)
    } else {
        Err(format!(
            "Prompt '{}' has unresolved placeholders: {}",
            prompt_name,
            unresolved.join(", ")
        ))
    }
}
```

Modify `tauri-app/src-tauri/src/workflow/mod.rs`:

```rust
pub mod lock;
pub mod chapter_production;
pub mod novel_bootstrap;
pub mod bible_ingestion;
pub mod review_repair;
pub mod weekly_planner;
pub mod review_agents;
pub mod review_arbiter;
pub mod canon_updater;
pub mod learning;
pub mod prompt_rendering;
```

- [ ] **Step 4: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml strict_prompt_rendering -- --nocapture
```

Expected: PASS for both strict prompt rendering tests.

---

### Task 2: Dedicated Weekly Planner Prompt

**Files:**
- Create: `tauri-app/src-tauri/prompts/weekly_planner.md`
- Modify: `tauri-app/src-tauri/src/prompts/mod.rs`
- Modify: `tauri-app/src-tauri/src/workflow/weekly_planner.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write the failing planner prompt test**

Append:

```rust
#[test]
fn weekly_planner_prompt_is_registered_and_dedicated() {
    let prompt = tauri_app_lib::prompts::load_prompt("weekly_planner")
        .expect("weekly planner prompt should be registered");

    assert!(prompt.contains("weekly_planner"));
    assert!(prompt.contains("WEEKLY_PLANNER_CONTEXT_JSON"));
    assert!(!prompt.contains("style_reviewer"));
    assert!(!prompt.contains("review_arbiter"));
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml weekly_planner_prompt_is_registered_and_dedicated -- --nocapture
```

Expected: FAIL because `weekly_planner` is not registered.

- [ ] **Step 3: Add and register the prompt**

Create `tauri-app/src-tauri/prompts/weekly_planner.md`:

```markdown
你是 weekly_planner，负责为连载小说生成下一组章节计划。

你只输出合法 JSON，不输出解释、寒暄或 Markdown 代码块。

## 规划目标

1. 承接已完成章节、未完成计划、活跃剧情线、未回收伏笔和人物状态。
2. 每章必须有明确冲突、信息释放、人物推进和章末钩子。
3. 避免空泛标题、日常流水账、模板化升级、无代价胜利和反派降智。
4. 计划必须服务长期连载连续性，不能为了单章爽点破坏 canon。

## 输入

{{WEEKLY_PLANNER_CONTEXT_JSON}}

## 输出 JSON schema

{
  "weekly_plan": {
    "chapters": [
      {
        "sequence": 1,
        "title": "string",
        "outline": "string",
        "target_word_count": 3000,
        "pov_character": "string",
        "plot_goals": ["string"],
        "required_characters": ["string"],
        "required_locations": ["string"],
        "required_foreshadowing": ["string"],
        "ending_hook": "string"
      }
    ],
    "weekly_summary": "string"
  }
}
```

Modify `tauri-app/src-tauri/src/prompts/mod.rs` by adding:

```rust
prompts.insert("weekly_planner", include_str!("../../prompts/weekly_planner.md"));
```

- [ ] **Step 4: Use the dedicated prompt in weekly planner**

In `weekly_planner.rs`, replace:

```rust
let system = prompts::load_prompt("review_agents").unwrap_or_else(|_| "Plan the next 7-14 chapters.".into());
```

with strict rendering:

```rust
let template = prompts::load_prompt("weekly_planner")?;
let context_json = serde_json::to_string_pretty(&context).unwrap_or_default();
let vars = std::collections::HashMap::from([
    ("WEEKLY_PLANNER_CONTEXT_JSON", context_json.clone()),
]);
let system = crate::workflow::prompt_rendering::render_prompt_strict(
    "weekly_planner",
    &template,
    &vars,
)?;
```

Keep `context_json` as the user prompt or replace the user prompt with a short instruction:

```rust
let user = "请基于 system prompt 中的上下文生成下一组章节计划，只输出 JSON。";
```

- [ ] **Step 5: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml weekly_planner_prompt_is_registered_and_dedicated -- --nocapture
```

Expected: PASS.

---

### Task 3: Learning Entry Selection and Usage Metadata

**Files:**
- Modify: `tauri-app/src-tauri/src/workflow/learning.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write failing learning usage test**

Append a lightweight in-memory DB setup and this test:

```rust
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("core-loop.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "核心闭环测试",
        Some("测试项目"),
        Some("悬疑"),
        None,
        Some("成人"),
        Some("冷峻"),
        Some("克制、具体、少套话"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

#[test]
fn learning_entries_can_be_selected_and_marked_used() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, example_text, application_notes, confidence, usage_count)
         VALUES
         ('learn-1', ?1, 'manual', '样章', 'style', '冷处理冲突', '用克制动作替代情绪解释', '他把杯口转向墙角。', '用于高压对话', 0.95, 0),
         ('learn-2', ?1, 'manual', '样章', 'dialogue', '半句台词', '角色说一半留一半', '你来晚了。', '用于悬念揭示', 0.90, 0)",
        rusqlite::params![project_id],
    )
    .unwrap();
    drop(conn);

    let entries = tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
        .expect("learning entries should load");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "learn-1");

    tauri_app_lib::workflow::learning::mark_learning_entries_used(&db, &["learn-1".to_string()])
        .expect("usage metadata should update");

    let updated = tauri_app_lib::workflow::learning::get_top_learning_entries(&db, &project_id, 8)
        .expect("updated entries should load");
    let used = updated.iter().find(|entry| entry.id == "learn-1").unwrap();
    assert_eq!(used.usage_count, 1);
    assert!(used.last_used_at.is_some());
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml learning_entries_can_be_selected_and_marked_used -- --nocapture
```

Expected: FAIL because the helper functions do not exist.

- [ ] **Step 3: Implement learning helpers**

Add to `learning.rs`:

```rust
use crate::db::connection::Database;

pub fn get_top_learning_entries(
    db: &Database,
    project_id: &str,
    limit: usize,
) -> Result<Vec<LearningEntry>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, source_type, source_url, source_title, category,
                pattern_name, pattern_description, example_text, application_notes,
                confidence, usage_count, last_used_at, metadata, created_at, updated_at
         FROM learning_entries
         WHERE project_id = ?1
         ORDER BY confidence DESC, usage_count ASC, created_at DESC
         LIMIT ?2",
    ).map_err(|e| format!("Prepare learning entries: {}", e))?;

    stmt.query_map(rusqlite::params![project_id, limit as i64], |row| {
        Ok(LearningEntry {
            id: row.get(0)?,
            project_id: row.get(1)?,
            source_type: row.get(2)?,
            source_url: row.get(3)?,
            source_title: row.get(4)?,
            category: row.get(5)?,
            pattern_name: row.get(6)?,
            pattern_description: row.get(7)?,
            example_text: row.get(8)?,
            application_notes: row.get(9)?,
            confidence: row.get(10)?,
            usage_count: row.get(11)?,
            last_used_at: row.get(12)?,
            metadata: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })
    .map_err(|e| format!("Query learning entries: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Collect learning entries: {}", e))
}

pub fn mark_learning_entries_used(db: &Database, ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for id in ids {
        conn.execute(
            "UPDATE learning_entries
             SET usage_count = usage_count + 1,
                 last_used_at = datetime('now'),
                 updated_at = datetime('now')
             WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Mark learning entry used: {}", e))?;
    }
    Ok(())
}

pub fn save_reflection_entries(
    db: &Database,
    project_id: &str,
    entries: &[LearningEntry],
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for entry in entries {
        conn.execute(
            "INSERT INTO learning_entries
             (id, project_id, source_type, source_url, source_title, category,
              pattern_name, pattern_description, example_text, application_notes,
              confidence, metadata)
             VALUES (?1, ?2, 'self_reflection', NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                Database::new_uuid(),
                project_id,
                entry.source_title,
                entry.category,
                entry.pattern_name,
                entry.pattern_description,
                entry.example_text,
                entry.application_notes,
                entry.confidence,
                entry.metadata,
            ],
        )
        .map_err(|e| format!("Save reflection entry: {}", e))?;
    }
    Ok(())
}
```

- [ ] **Step 4: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml learning_entries_can_be_selected_and_marked_used -- --nocapture
```

Expected: PASS.

---

### Task 4: Writing Context Package

**Files:**
- Create: `tauri-app/src-tauri/src/workflow/writing_context.rs`
- Modify: `tauri-app/src-tauri/src/workflow/mod.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write failing writing context test**

Append:

```rust
#[test]
fn writing_context_includes_recent_bodies_and_learning_patterns() {
    let db = setup_db();
    let project_id = insert_project(&db);

    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-1', ?1, 1, '门后的人', '主角发现旧案线索', 3000, 'planned')",
        rusqlite::params![project_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO chapters (id, project_id, sequence, title, status, word_count, summary)
         VALUES ('chapter-prev', ?1, 0, '雨夜旧案', 'final', 1200, '上一章主角找到带血钥匙')",
        rusqlite::params![project_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO chapter_versions
         (id, chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count)
         VALUES ('version-prev', 'chapter-prev', ?1, 1, 'final', '雨夜旧案', '雨水敲在铁门上。钥匙在掌心发冷。最后，他听见门后有人叫出了他的旧名。', '上一章主角找到带血钥匙', 1200)",
        rusqlite::params![project_id],
    ).unwrap();
    conn.execute(
        "UPDATE chapters SET final_version_id = 'version-prev' WHERE id = 'chapter-prev'",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
         VALUES ('learn-style', ?1, 'manual', '样章', 'style', '冷硬细节', '用物件触感承载压力', 0.92)",
        rusqlite::params![project_id],
    ).unwrap();
    drop(conn);

    let project = tauri_app_lib::db::projects::get_project(&db, &project_id).unwrap();
    let plan = tauri_app_lib::db::chapters::get_next_chapter_plan(&db, &project_id).unwrap().unwrap();
    let canon = tauri_app_lib::db::bible::get_bible(&db, &project_id).unwrap();
    let settings = tauri_app_lib::db::settings::get_settings(&db).unwrap();

    let package = tauri_app_lib::workflow::writing_context::build_writing_context(
        &db,
        &project,
        &plan,
        &canon,
        &settings,
        vec![],
        None,
    ).unwrap();

    let json = serde_json::to_value(&package).unwrap();
    assert!(json["continuity"]["recent_body_excerpts"].to_string().contains("门后有人叫出了他的旧名"));
    assert!(json["continuity"]["previous_ending_hook"].as_str().unwrap_or("").contains("旧名"));
    assert!(json["learned_patterns"].to_string().contains("冷硬细节"));
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml writing_context_includes_recent_bodies_and_learning_patterns -- --nocapture
```

Expected: FAIL because `writing_context` does not exist.

- [ ] **Step 3: Implement the context builder**

Create `writing_context.rs` with serializable structs:

```rust
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::connection::Database;
use crate::models::{
    AppSettings, BibleData, ChapterPlan, LearningEntry, Project, VectorDocument,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperatorControls {
    pub generation_mode: Option<String>,
    pub chapter_intent: Option<String>,
    pub must_include_beats: Option<String>,
    pub forbidden_moves: Option<String>,
    pub style_emphasis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingContextPackage {
    pub project: serde_json::Value,
    pub chapter_plan: serde_json::Value,
    pub continuity: serde_json::Value,
    pub canon: serde_json::Value,
    pub retrieval: Vec<VectorDocument>,
    pub style: serde_json::Value,
    pub learned_patterns: Vec<LearningEntry>,
    pub operator_controls: OperatorControls,
}

pub fn build_writing_context(
    db: &Database,
    project: &Project,
    plan: &ChapterPlan,
    canon: &BibleData,
    settings: &AppSettings,
    retrieval: Vec<VectorDocument>,
    controls: Option<OperatorControls>,
) -> Result<WritingContextPackage, String> {
    let chapters = crate::db::chapters::get_chapters(db, &project.id)?;
    let mut recent_summaries = Vec::new();
    let mut recent_body_excerpts = Vec::new();

    for chapter in chapters.iter().rev().take(5) {
        recent_summaries.push(json!({
            "sequence": chapter.sequence,
            "title": chapter.title,
            "summary": chapter.summary,
            "status": chapter.status,
        }));
    }

    for chapter in chapters.iter().rev().take(2) {
        if let Some(version) = crate::db::chapters::get_latest_version(db, &chapter.id)? {
            if let Some(body) = version.body_markdown {
                let excerpt = body.chars().take(2400).collect::<String>();
                recent_body_excerpts.push(json!({
                    "sequence": chapter.sequence,
                    "title": chapter.title,
                    "excerpt": excerpt,
                }));
            }
        }
    }

    let previous_ending_hook = recent_body_excerpts
        .first()
        .and_then(|item| item["excerpt"].as_str())
        .map(|text| {
            let chars = text.chars().collect::<Vec<_>>();
            let start = chars.len().saturating_sub(160);
            chars[start..].iter().collect::<String>()
        })
        .unwrap_or_default();

    let character_states = crate::db::bible::get_character_states(db, &project.id)
        .unwrap_or_default();
    let learned_patterns = crate::workflow::learning::get_top_learning_entries(db, &project.id, 8)
        .unwrap_or_default();

    Ok(WritingContextPackage {
        project: json!({
            "id": project.id,
            "name": project.name,
            "genre": project.genre,
            "target_audience": project.target_audience,
            "quality_threshold": project.quality_threshold,
        }),
        chapter_plan: json!({
            "id": plan.id,
            "sequence": plan.sequence,
            "title": plan.title,
            "outline": plan.outline,
            "target_word_count": plan.target_word_count.unwrap_or(settings.daily_target_words),
            "pov_character": plan.pov_character_id,
            "required_characters": plan.required_characters,
            "required_locations": plan.required_locations,
            "plot_goals": plan.plot_goals,
            "required_foreshadowing": plan.required_foreshadowing,
        }),
        continuity: json!({
            "recent_summaries": recent_summaries,
            "recent_body_excerpts": recent_body_excerpts,
            "previous_ending_hook": previous_ending_hook,
            "timeline_events": canon.timeline_events,
            "character_states": character_states,
        }),
        canon: json!({
            "characters": canon.characters,
            "locations": canon.locations,
            "organizations": canon.organizations,
            "items": canon.items,
            "magic_systems": canon.magic_systems,
            "canon_rules": canon.canon_rules,
            "active_plot_threads": canon.plot_threads,
            "unresolved_foreshadowing": canon.foreshadowing,
            "world_lore": canon.world_lore,
        }),
        retrieval,
        style: json!({
            "project_style_profile": project.style_profile,
            "style_guides": canon.style_guides,
        }),
        learned_patterns,
        operator_controls: controls.unwrap_or_default(),
    })
}
```

Expose it in `workflow/mod.rs`:

```rust
pub mod writing_context;
```

- [ ] **Step 4: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml writing_context_includes_recent_bodies_and_learning_patterns -- --nocapture
```

Expected: PASS.

---

### Task 5: Job and Chapter Plan State Fixes

**Files:**
- Modify: `tauri-app/src-tauri/src/db/generation_jobs.rs`
- Modify: `tauri-app/src-tauri/src/db/chapters.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write failing job/plan tests**

Append:

```rust
#[test]
fn conflicting_generation_job_returns_existing_job_id() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, status)
         VALUES ('plan-job', ?1, 1, 'planned')",
        rusqlite::params![project_id],
    ).unwrap();
    drop(conn);

    let first = tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-job").unwrap();
    let second = tauri_app_lib::db::generation_jobs::create_generation_job(&db, &project_id, "plan-job").unwrap();

    assert_eq!(first, second);
}

#[test]
fn chapter_plan_can_be_marked_completed() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, status)
         VALUES ('plan-complete', ?1, 1, 'in_progress')",
        rusqlite::params![project_id],
    ).unwrap();
    drop(conn);

    tauri_app_lib::db::chapters::mark_chapter_plan_completed(&db, "plan-complete").unwrap();

    let plans = tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id).unwrap();
    assert_eq!(plans[0].status, "completed");
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml "generation_job|chapter_plan_can_be_marked_completed" -- --nocapture
```

Expected: one test fails because `create_generation_job` returns a new ignored ID; one fails because completion helper does not exist.

- [ ] **Step 3: Fix job ID return and plan completion helper**

In `generation_jobs.rs`, replace `create_generation_job` with an insert-then-select implementation:

```rust
pub fn create_generation_job(db: &Database, project_id: &str, chapter_plan_id: &str) -> Result<String, String> {
    let id = Database::new_uuid();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR IGNORE INTO generation_jobs (id, project_id, chapter_plan_id, job_date, status)
         VALUES (?1, ?2, ?3, ?4, 'started')",
        params![id, project_id, chapter_plan_id, today],
    ).map_err(|e| format!("Create job: {}", e))?;

    conn.query_row(
        "SELECT id FROM generation_jobs
         WHERE project_id = ?1 AND chapter_plan_id = ?2 AND job_date = ?3
         ORDER BY created_at ASC LIMIT 1",
        params![project_id, chapter_plan_id, today],
        |row| row.get::<_, String>(0),
    ).map_err(|e| format!("Load generation job id: {}", e))
}
```

In `chapters.rs`, add:

```rust
pub fn mark_chapter_plan_completed(db: &Database, chapter_plan_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "UPDATE chapter_plans SET status = 'completed', updated_at = datetime('now') WHERE id = ?1",
        params![chapter_plan_id],
    ).map_err(|e| format!("Complete plan: {}", e))?;
    Ok(())
}
```

- [ ] **Step 4: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml "generation_job|chapter_plan_can_be_marked_completed" -- --nocapture
```

Expected: PASS.

---

### Task 6: Integrate Context, Final Canon, Learning, and Pipeline Events

**Files:**
- Modify: `tauri-app/src-tauri/prompts/draft_writer.md`
- Modify: `tauri-app/src-tauri/src/workflow/canon_updater.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Test: `tauri-app/src-tauri/tests/core_writing_loop_tests.rs`

- [ ] **Step 1: Write failing pipeline behavior test**

Append a mock provider that captures prompts and canon input, then test:

```rust
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use serde_json::{json, Value};
use tauri_app_lib::ai::client::ModelClient;

#[derive(Default)]
struct CapturingProvider {
    systems: Arc<Mutex<Vec<String>>>,
    users: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ModelClient for CapturingProvider {
    async fn generate_json(&self, system: &str, user: &str, _schema: &Value, _max_tokens: u32) -> Result<Value, String> {
        self.systems.lock().unwrap().push(system.to_string());
        self.users.lock().unwrap().push(user.to_string());

        if system.contains("canon_extractor") {
            return Ok(json!({
                "chapter_summary": "最终修订稿进入圣经",
                "character_state_updates": [],
                "timeline_events": [],
                "new_lore": [],
                "foreshadowing_updates": [],
                "vector_documents": [],
                "human_review_required": []
            }));
        }

        Ok(json!({
            "title": "门后旧名",
            "body_markdown": "最终稿正文。门后的人没有现身，只把他的旧名写在潮湿墙面上。这个版本必须进入 canon。",
            "summary": "最终稿摘要",
            "word_count": 120,
            "pov_character": "主角",
            "major_events": ["旧名出现"],
            "character_state_changes": [],
            "timeline_events": [],
            "foreshadowing_used": [],
            "foreshadowing_planted": [],
            "new_canon_candidates": [],
            "continuity_notes": "下一章回应旧名",
            "used_context_ids": []
        }))
    }

    async fn generate_text(&self, _system: &str, _user: &str, _max_tokens: u32) -> Result<String, String> {
        Ok(r#"{"score":92,"pass":true,"blocking_issues":[],"minor_issues":[],"recommendations":[]}"#.into())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[tokio::test]
async fn chapter_pipeline_uses_writing_context_and_finalizes_plan() {
    let db = setup_db();
    let project_id = insert_project(&db);
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO chapter_plans (id, project_id, sequence, title, outline, target_word_count, status)
         VALUES ('plan-pipe', ?1, 1, '门后旧名', '调查门后声音', 3000, 'planned')",
        rusqlite::params![project_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO learning_entries
         (id, project_id, source_type, source_title, category, pattern_name, pattern_description, confidence)
         VALUES ('learn-pipe', ?1, 'manual', '样章', 'style', '克制悬疑', '少解释，多用动作和物件', 0.91)",
        rusqlite::params![project_id],
    ).unwrap();
    drop(conn);

    let provider = CapturingProvider::default();
    let (log_tx, _log_rx) = tokio::sync::mpsc::channel(100);
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    let result = tauri_app_lib::workflow::chapter_production::generate_next_chapter(
        &db,
        &provider,
        None,
        &project_id,
        true,
        &log_tx,
        &event_tx,
        None,
    ).await.unwrap();

    assert!(result.ok);

    let systems = provider.systems.lock().unwrap().join("\n---\n");
    assert!(systems.contains("WRITING_CONTEXT_JSON") == false);
    assert!(systems.contains("克制悬疑"));
    assert!(systems.contains("门后旧名"));
    assert!(!systems.contains("{{"));

    let plans = tauri_app_lib::db::chapters::get_chapter_plans(&db, &project_id).unwrap();
    assert_eq!(plans[0].status, "completed");

    let mut steps = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        steps.push(event.step);
    }
    for expected in ["acquire_lock", "load_canon", "retrieve_context", "generate_draft", "aggregate_reviews", "export", "update_canon", "complete"] {
        assert!(steps.iter().any(|step| step == expected), "missing pipeline step {expected}; got {steps:?}");
    }
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture
```

Expected: FAIL because `generate_next_chapter` has no `operator_controls` argument, prompt still uses old brief, and plan is not completed.

- [ ] **Step 3: Update draft prompt**

Replace `draft_writer.md` input section with:

```markdown
## 输入 writing_context JSON
{{WRITING_CONTEXT_JSON}}
```

Remove `{{target_word_count}}` and references to `WRITING_BRIEF_JSON`.

- [ ] **Step 4: Integrate writing context in chapter production**

Change `generate_next_chapter` signature to:

```rust
pub async fn generate_next_chapter(
    db: &Database,
    provider: &dyn ModelClient,
    emb_provider: Option<&dyn ModelClient>,
    project_id: &str,
    force: bool,
    log_tx: &mpsc::Sender<String>,
    event_tx: &mpsc::Sender<PipelineEvent>,
    operator_controls: Option<crate::workflow::writing_context::OperatorControls>,
) -> Result<GenerationResult, String>
```

Keep existing behavior by passing `None` from old callers until `lib.rs` is updated.

In the retrieval block:

```rust
emit(event_tx, "retrieve_context", "running", None, 15.0);
let mut retrieval_documents = Vec::new();
...
retrieval_documents = docs;
...
emit(event_tx, "retrieve_context", "done", Some(&format!("{} docs", retrieval_documents.len())), 22.0);
```

Build context:

```rust
let writing_context = crate::workflow::writing_context::build_writing_context(
    db,
    &project,
    &plan,
    &canon_data,
    &settings,
    retrieval_documents,
    operator_controls,
)?;
let writing_context_json = serde_json::to_string_pretty(&writing_context).unwrap_or_default();
let used_learning_ids = writing_context
    .learned_patterns
    .iter()
    .map(|entry| entry.id.clone())
    .collect::<Vec<_>>();
```

Render prompt strictly:

```rust
let draft_template = prompts::load_prompt("draft_writer")?;
let vars = HashMap::from([
    ("WRITING_CONTEXT_JSON", writing_context_json.clone()),
]);
let rendered = crate::workflow::prompt_rendering::render_prompt_strict(
    "draft_writer",
    &draft_template,
    &vars,
)?;
```

After successful draft generation:

```rust
crate::workflow::learning::mark_learning_entries_used(db, &used_learning_ids)?;
```

- [ ] **Step 5: Use final selected chapter JSON for canon**

After the revision loop and after `current_draft` is final, call canon updater with `current_draft`:

```rust
emit(event_tx, "update_canon", "running", None, 90.0);
canon_updater::update_canon_after_chapter(db, provider, project_id, &chapter_id, &current_draft).await?;
emit(event_tx, "update_canon", "done", None, 94.0);
```

Do not pass `&draft` after revisions.

- [ ] **Step 6: Mark plan completed and emit export events**

Before export:

```rust
emit(event_tx, "export", "running", None, 82.0);
```

After export:

```rust
emit(event_tx, "export", "done", filename.as_deref(), 88.0);
```

On terminal publish-ready or final revised decision:

```rust
if final_decision != "needs_human_review" {
    chapters::mark_chapter_plan_completed(db, &plan.id)?;
}
```

- [ ] **Step 7: Store self-reflection notes**

After canon update:

```rust
if let Ok(existing_learning) = crate::workflow::learning::get_top_learning_entries(db, project_id, 8) {
    let review_summary = serde_json::to_string(&current_reviews).unwrap_or_default();
    if let Ok(reflections) = crate::workflow::learning::reflect_on_chapter(
        provider,
        &title,
        current_draft["body_markdown"].as_str().unwrap_or_default(),
        &review_summary,
        &existing_learning,
    ).await {
        let _ = crate::workflow::learning::save_reflection_entries(db, project_id, &reflections);
    }
}
```

Reflection failure should be logged and non-blocking.

- [ ] **Step 8: Run GREEN**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml chapter_pipeline_uses_writing_context_and_finalizes_plan -- --nocapture
```

Expected: PASS.

---

### Task 7: Tauri Command and Dashboard Controls

**Files:**
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src/App.tsx`
- Test: frontend build

- [ ] **Step 1: Update the Tauri command signature**

In `lib.rs`, import `OperatorControls`:

```rust
use workflow::writing_context::OperatorControls;
```

Change command signature:

```rust
async fn generate_next_chapter(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    project_id: String,
    force: bool,
    operator_controls: Option<OperatorControls>,
) -> Result<GenerationResult, String>
```

Call backend with the new argument:

```rust
let result = workflow::chapter_production::generate_next_chapter(
    &state.db,
    provider.as_ref(),
    emb_provider.as_ref().map(|p| p.as_ref()),
    &project_id,
    force,
    &log_tx,
    &event_tx,
    operator_controls,
).await;
```

- [ ] **Step 2: Add a context preview command**

Add:

```rust
#[tauri::command]
async fn get_next_chapter_context_preview(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<serde_json::Value, String> {
    let project = db::projects::get_project(&state.db, &project_id)?;
    let settings = db::settings::get_settings(&state.db)?;
    let plan = db::chapters::get_next_chapter_plan(&state.db, &project_id)?
        .ok_or_else(|| "No planned chapter available.".to_string())?;
    let chapters = db::chapters::get_chapters(&state.db, &project_id)?;
    let learned = workflow::learning::get_top_learning_entries(&state.db, &project_id, 8)
        .unwrap_or_default();
    let last_hook = chapters
        .last()
        .and_then(|chapter| db::chapters::get_latest_version(&state.db, &chapter.id).ok().flatten())
        .and_then(|version| version.body_markdown)
        .map(|body| {
            let chars = body.chars().collect::<Vec<_>>();
            let start = chars.len().saturating_sub(160);
            chars[start..].iter().collect::<String>()
        })
        .unwrap_or_default();

    Ok(serde_json::json!({
        "project": {"name": project.name, "genre": project.genre},
        "next_plan": {
            "sequence": plan.sequence,
            "title": plan.title,
            "outline": plan.outline,
            "target_word_count": plan.target_word_count.unwrap_or(settings.daily_target_words)
        },
        "last_hook": last_hook,
        "learned_pattern_count": learned.len(),
        "rag_enabled": settings.embedding_provider != "none",
    }))
}
```

Register it in `invoke_handler!`.

- [ ] **Step 3: Add Dashboard controls**

In `App.tsx`, add state near `Dashboard`:

```tsx
const [controls, setControls] = useState({
  generation_mode: "continuity-first",
  chapter_intent: "",
  must_include_beats: "",
  forbidden_moves: "",
  style_emphasis: "",
});
const [contextPreview, setContextPreview] = useState<any>(null);
```

Load preview when project changes:

```tsx
useEffect(() => {
  if (!selected) return;
  invoke("get_next_chapter_context_preview", { projectId: selected })
    .then(setContextPreview)
    .catch(() => setContextPreview(null));
}, [selected, status?.plans_left]);
```

Change the write invoke:

```tsx
const r = await invoke<GenerationResult>("generate_next_chapter", {
  projectId: selected,
  force: true,
  operatorControls: controls,
});
```

Render a compact panel above action buttons:

```tsx
<div className="card-feature" style={{ marginBottom: 16 }}>
  <h3 className="section-title">Chapter Control</h3>
  {contextPreview && (
    <div className="text-meta" style={{ marginBottom: 12 }}>
      Next: Ch.{contextPreview.next_plan?.sequence} — {contextPreview.next_plan?.title || "Untitled"}
      {" · "}Learned patterns: {contextPreview.learned_pattern_count}
      {" · "}RAG: {contextPreview.rag_enabled ? "ON" : "OFF"}
    </div>
  )}
  {contextPreview?.last_hook && (
    <div className="content-preview" style={{ marginBottom: 12 }}>
      {contextPreview.last_hook}
    </div>
  )}
  <div className="bible-edit-field">
    <label>Generation Mode</label>
    <select className="select" value={controls.generation_mode} onChange={e => setControls({...controls, generation_mode: e.target.value})}>
      <option value="continuity-first">Continuity-first</option>
      <option value="style-first">Style-first</option>
      <option value="balanced">Balanced</option>
      <option value="experimental">Experimental</option>
    </select>
  </div>
  <div className="bible-edit-field"><label>Chapter Intent</label><textarea value={controls.chapter_intent} onChange={e => setControls({...controls, chapter_intent: e.target.value})} /></div>
  <div className="bible-edit-field"><label>Must Include Beats</label><textarea value={controls.must_include_beats} onChange={e => setControls({...controls, must_include_beats: e.target.value})} /></div>
  <div className="bible-edit-field"><label>Forbidden Moves</label><textarea value={controls.forbidden_moves} onChange={e => setControls({...controls, forbidden_moves: e.target.value})} /></div>
  <div className="bible-edit-field"><label>Style Emphasis</label><textarea value={controls.style_emphasis} onChange={e => setControls({...controls, style_emphasis: e.target.value})} /></div>
</div>
```

- [ ] **Step 4: Run frontend GREEN**

Run:

```powershell
npm run build
```

Working directory: `tauri-app`.

Expected: TypeScript and Vite build pass.

---

### Task 8: Documentation Refresh

**Files:**
- Modify: `docs/operations.md`
- Modify: `docs/troubleshooting.md`

- [ ] **Step 1: Mark old operations content as legacy**

Add to the top of `docs/operations.md`:

```markdown
> Current app note: the active product is the local Tauri + SQLite desktop app described in `README.md`. Sections that mention Neon, n8n, Postgres, or writer-service are legacy deployment notes and should not be used for the current desktop workflow unless explicitly running the old stack.

## Local Desktop Operations

- Database: `Documents/AI-Novels/ai-novel-factory.db`.
- Chapter exports: `Documents/AI-Novels/novel-XXXXXXXX/chNNN.md`.
- Main health checks are available in the app: Dashboard, Jobs, Reviews, Bible, Learn, and Settings.
- If generation appears stuck, first check Dashboard logs and Jobs. Use Reset Stuck Job only after confirming no generation is still running.
- If continuity degrades, check Settings → Embedding Provider, Learn → Knowledge Library, Bible → canon/style guides, and the next chapter context preview.
```

- [ ] **Step 2: Add desktop troubleshooting entries**

Add to `docs/troubleshooting.md`:

```markdown
## Desktop App Issues

### Unresolved prompt placeholder

Symptom: generation fails before calling the model with a message such as `Prompt 'draft_writer' has unresolved placeholders`.

Fix:
- Check the prompt file under `tauri-app/src-tauri/prompts`.
- Check the workflow render variables in `tauri-app/src-tauri/src/workflow`.
- Every `{{NAME}}` in the prompt must be supplied by strict rendering.

### Context still feels thin

Check:
- Settings embedding provider is enabled if RAG is expected.
- Learn page has high-confidence patterns.
- Bible has updated character states, timeline events, and style guides.
- Dashboard Chapter Control preview shows the next plan and last hook.

### Chapter plan remains in progress

This is expected only for `needs_human_review`. A successful terminal chapter should mark the plan `completed`.

### Old Neon/n8n instructions conflict with README

The desktop app does not require Neon, n8n, Docker, or writer-service. Those docs are legacy references for the previous architecture.
```

- [ ] **Step 3: Verify docs have no placeholders**

Run:

```powershell
rg -n "TBD|TODO|\\{\\{" docs\operations.md docs\troubleshooting.md
```

Expected: No unresolved planning placeholders. Existing literal examples are acceptable only if they are clearly explained.

---

### Task 9: Full Regression

**Files:**
- All modified files

- [ ] **Step 1: Run Rust tests**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml
```

Expected: all Rust tests pass. Warnings can remain only if unrelated to changed code.

- [ ] **Step 2: Run frontend build**

Run in `tauri-app`:

```powershell
npm run build
```

Expected: `tsc && vite build` exits 0.

- [ ] **Step 3: Run whitespace/conflict check**

Run:

```powershell
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 4: Review final diff against spec**

Run:

```powershell
git diff --stat
rg -n "review_agents\"\\)|\\{\\{target_word_count\\}\\}|WRITING_BRIEF_JSON|canon_updater::update_canon_after_chapter\\(db, provider, project_id, &chapter_id, &draft\\)" tauri-app\src-tauri
```

Expected:

- Diff touches only planned files.
- `weekly_planner.rs` does not load `review_agents`.
- `draft_writer.md` does not contain `{{target_word_count}}`.
- `chapter_production.rs` does not update canon from `&draft` after revisions.
- Any remaining `WRITING_BRIEF_JSON` is in reviewer prompts or compatibility tests, not in draft generation.

---

## Plan Self-Review

- Spec coverage: prompt rendering, weekly planner, writing context, learning loop, final canon, status machine, UI controls, pipeline events, docs, and verification are each mapped to tasks.
- Placeholder scan: no task says TBD/TODO/implement later. Literal `{{...}}` appears only in prompt examples and placeholder tests.
- Type consistency: `OperatorControls`, `WritingContextPackage`, `render_prompt_strict`, `get_top_learning_entries`, `mark_learning_entries_used`, `save_reflection_entries`, and `mark_chapter_plan_completed` are consistently named across tasks.
