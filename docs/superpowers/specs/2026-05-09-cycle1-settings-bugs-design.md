# Cycle 1 — Settings + Bug Fixes

> Settings inline editing, full novel deletion, quality threshold revise loop, character reviewer fix, RAG status visibility.

## 1. Settings — Inline Editing

Replace read-only "Current Config" card with editable fields. Each saves on blur via `update_settings()`.

### Fields

| Field | Control | Default |
|-------|---------|---------|
| Provider | select | deepseek |
| Model | text input | deepseek-v4-pro[1m] |
| Base URL | text input | https://api.deepseek.com |
| Embedding Provider | select (openai/deepseek/openai_compat/none) | none |
| Embedding Model | text input | text-embedding-3-small |
| Data Directory | text input + Browse button (Tauri dialog) | Documents/AI-Novels |
| Quality Threshold | number (0-100) | 85 |
| Auto Publish | toggle | false |
| Max Revise Count | number (1-5) | 2 |
| Daily Target Words | number | 3000 |
| Debug Mode | toggle | false |

### New Command

```rust
#[tauri::command]
async fn pick_directory() -> Result<String, String>
```

Opens native folder picker dialog via `tauri-plugin-dialog`, returns selected path.

## 2. Delete Novel — Full Cleanup

`delete_project` currently only deletes DB rows via FK cascade. Files on disk are orphaned.

### New flow

1. Read project slug + data_dir before deletion
2. `DELETE FROM projects WHERE id = ?1` (FK cascade cleans all related DB rows)
3. After DB success: `rm -rf {data_dir}/{slug}/`
4. File deletion failure is logged as warning, doesn't fail the operation

### Code change

`lib.rs` `delete_project` command: fetch slug first, call `db::projects::delete_project_full()` that handles both DB and filesystem.

## 3. Quality Threshold — Revise Loop

### Root cause

After one revision, code publishes regardless of score. No re-review after revision.

### Fix

Wrap review+revise in a `loop {}` with revise_count tracking:

```
loop {
    run 7 review agents
    aggregate → decision
    
    if decision != "revise" OR revise_count >= max_revise_count:
        break (final)
    
    call revision writer
    save revision version
    revise_count++
}
```

Max revisions = `settings.max_revise_count` (default 2). Each revision costs 1 draft call + 7 review calls.

## 4. Character Reviewer Score=0

### Root cause

`CHARACTER_STATES_JSON` is empty `"[]"` for first chapter. AI sees no character data, returns score 0.

### Fix

- For first chapter: use bible character data as baseline state
- For subsequent chapters: query `character_states` table for latest per-character state
- Add WARN-level log when any reviewer returns score 0 with raw AI response

## 5. RAG Status Visibility

### Root cause

`embedding_provider = "none"` (default for DeepSeek) silently disables vector search with no user notification.

### Fix

- Settings page: show warning banner when embedding_provider is "none"
- Dashboard: add RAG status indicator (enabled/disabled)
- Text: "RAG vector search is disabled. Configure an embedding provider to enable. Without RAG, the AI has less continuity context."

## Files Changed

| File | Change |
|------|--------|
| `src/App.tsx` | Settings inline editing, delete confirmation, RAG banner, Dashboard RAG indicator |
| `src/lib.rs` | `delete_project` full cleanup, `pick_directory` command |
| `src/db/projects.rs` | `delete_project_full` with filesystem cleanup |
| `src/workflow/chapter_production.rs` | Revise loop (while loop around review+revise) |
| `src/workflow/review_agents.rs` | Character states from DB, score=0 warning log |
| `src/db/bible.rs` | `get_character_states` already exists, verify correct |

## Verification

1. Settings: change data directory → Browse → new path saved
2. Settings: change quality threshold to 90 → next chapter respects 90
3. Delete novel → check `Documents/AI-Novels/novel-XXXXX/` is deleted
4. Write chapter with threshold 90 → score 81 → revises twice → needs_human_review (not published)
5. Character reviewer shows score > 0 (not 0)
6. RAG banner shows when embedding_provider is "none"
