# Cycle 2 — Quality Overhaul + Self-Learning

## 1. Quality Overhaul

### 1.1 Fix 6 Missing Canon Fields in Reviews

**File**: `src/workflow/review_agents.rs`

Add to render_vars (all 6 are defined in CanonContext but never rendered):
```rust
vars.insert("LOCATIONS_JSON", canon.locations_json);
vars.insert("ORGANIZATIONS_JSON", canon.organizations_json);
vars.insert("ITEMS_JSON", canon.items_json);
vars.insert("MAGIC_SYSTEMS_JSON", canon.magic_systems_json);
vars.insert("TIMELINE_JSON", canon.timeline_json);
```

### 1.2 Selective Context Per Reviewer

Each agent gets only the data it needs:

| Reviewer | Context fields |
|----------|---------------|
| continuity | timeline, locations, items, character_states, prev_chapters |
| character | characters, character_states |
| plot_logic | plot_threads, foreshadowing, writing_brief |
| pacing | style_guide, writing_brief |
| style | style_guide |
| safety | project_policy |
| publication | blog_config |

### 1.3 Enhanced Coherence + Literary Quality Prompts

**continuity_reviewer.md**: Add explicit methodology — check location transitions, item consistency, timeline ordering.

**plot_logic_reviewer.md**: Add anti-formula checks — predictability, non-protagonist plotlines, Chekhov's Gun.

**draft_writer.md**: Add subtext requirement (≥3 instances), sentence rhythm, multi-perspective (≥1 non-MC POV), sensory depth (≥2 senses per scene).

### 1.4 RAG Retrieval Enhancement

Query from `outline + title + required_characters + required_locations` instead of just `outline + title + genre`.

---

## 2. Self-Learning Module

### 2.1 Migration

```sql
CREATE TABLE learning_entries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_url TEXT,
    source_title TEXT,
    category TEXT NOT NULL,
    pattern_name TEXT NOT NULL,
    pattern_description TEXT NOT NULL,
    example_text TEXT,
    application_notes TEXT,
    confidence REAL DEFAULT 0.7,
    usage_count INTEGER DEFAULT 0,
    last_used_at TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 2.2 Core Function

`extract_knowledge(provider, text, source_info) -> Vec<LearningEntry>`: AI analyzes text, extracts writing techniques, outputs structured JSON. Each entry auto-vectorized into RAG.

### 2.3 Tauri Commands

- `learn_from_text(project_id, text, source_title)` — manual paste
- `learn_from_url(project_id, url)` — web fetch + extract
- `get_learning_entries(project_id)` — list
- `delete_learning_entry(id)` — remove
- `reflect_on_chapter(project_id, chapter_id)` — auto self-analysis

### 2.4 Self-Reflection

After canon_updater: load chapter + reviews + top learning entries → AI compares → stores improvement notes. Notes injected into next chapter's writing_brief.

### 2.5 Frontend: Learn Tab

3 sub-tabs: Manual Input, Web Learn, Knowledge Library (cards grid).

## Verification

1. Reviewer scores include location/timeline/item data
2. Draft shows subtext instances, multi-POV, sensory detail
3. Paste sample text → extract patterns → visible in Knowledge Library
4. Write chapter → reflection auto-runs → improvement notes appear
5. Next chapter's writing_brief includes improvement notes
