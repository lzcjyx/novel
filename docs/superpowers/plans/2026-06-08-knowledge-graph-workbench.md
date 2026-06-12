# Knowledge Graph Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a usable knowledge graph workbench that exposes Bible-derived nodes, persisted graph edges, and manual relationship editing.

**Architecture:** Derive graph nodes from existing Bible tables and store only edges in `knowledge_graph_edges`. Add backend snapshot/CRUD functions and Tauri commands, then add a dense React page that renders a stable radial graph, filters nodes, and manages edges.

**Tech Stack:** Rust 2021, Tauri v2, rusqlite SQLite, serde, React 19, TypeScript, Vite, CSS.

---

## File Map

- Modify: `tauri-app/src-tauri/src/db/mod.rs`
  - Export `knowledge_graph`.
- Modify: `tauri-app/src-tauri/src/db/knowledge_graph.rs`
  - Add graph node/snapshot types, node derivation, validation, edge readback, and tests-facing helpers.
- Modify: `tauri-app/src-tauri/src/lib.rs`
  - Add Tauri commands and register them.
- Create: `tauri-app/src-tauri/tests/knowledge_graph_tests.rs`
  - Cover snapshot, degree, insert validation, and delete.
- Modify: `tauri-app/src/App.tsx`
  - Add Graph navigation and `KnowledgeGraphPage`.
- Modify: `tauri-app/src/index.css`
  - Add graph workbench layout, stable canvas, node, edge, inspector, and responsive styles.
- Create: `docs/superpowers/specs/2026-06-08-knowledge-graph-workbench-design.md`
  - Written design spec.
- Create: `docs/superpowers/plans/2026-06-08-knowledge-graph-workbench.md`
  - This implementation plan.

---

### Task 1: Backend Failing Tests

**Files:**
- Create: `tauri-app/src-tauri/tests/knowledge_graph_tests.rs`

- [ ] **Step 1: Add tests for snapshot and edge behavior**

Create `knowledge_graph_tests.rs` with:

```rust
use tauri_app_lib::db::connection::Database;

fn setup_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("knowledge-graph.db");
    let db = Database::open(&db_path).unwrap();
    tauri_app_lib::db::run_migrations(&db).unwrap();
    std::mem::forget(dir);
    db
}

fn insert_project(db: &Database) -> String {
    tauri_app_lib::db::projects::create_project(
        db,
        "图谱测试",
        Some("测试关系图"),
        Some("悬疑"),
        None,
        Some("成人"),
        Some("冷峻"),
        Some("克制"),
        Some(500000),
        Some(3000),
    )
    .unwrap()
    .id
}

fn seed_nodes(db: &Database, project_id: &str) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO characters (id, project_id, name, role, personality, status)
         VALUES ('char-a', ?1, '林白', '主角', '克制', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO locations (id, project_id, name, type, description, status)
         VALUES ('loc-a', ?1, '旧车站', '地点', '雨夜案发地', 'active')",
        rusqlite::params![project_id],
    )
    .unwrap();
}

#[test]
fn graph_snapshot_derives_bible_nodes_and_degrees() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);

    tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "appears_at",
        Some("林白在旧车站找到线索"),
    )
    .unwrap();

    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert_eq!(snapshot.edges.len(), 1);
    assert!(snapshot.nodes.iter().any(|n| n.id == "char-a" && n.node_type == "character" && n.degree == 1));
    assert!(snapshot.nodes.iter().any(|n| n.id == "loc-a" && n.node_type == "location" && n.degree == 1));
    assert_eq!(snapshot.orphan_count, 0);
}

#[test]
fn graph_edge_creation_validates_and_delete_removes_edge() {
    let db = setup_db();
    let project_id = insert_project(&db);
    seed_nodes(&db, &project_id);

    let invalid = tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "char-a",
        "character",
        "self",
        None,
    );
    assert!(invalid.is_err());

    let edge = tauri_app_lib::db::knowledge_graph::create_edge(
        &db,
        &project_id,
        "char-a",
        "character",
        "loc-a",
        "location",
        "investigates",
        Some("手动添加关系"),
    )
    .unwrap();
    assert_eq!(edge.edge_type, "investigates");
    assert!(!edge.auto_inferred);

    tauri_app_lib::db::knowledge_graph::delete_edge(&db, &edge.id).unwrap();
    let snapshot = tauri_app_lib::db::knowledge_graph::get_snapshot(&db, &project_id).unwrap();
    assert!(snapshot.edges.is_empty());
}
```

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml graph_snapshot_derives_bible_nodes_and_degrees -- --nocapture
```

Expected: FAIL because `db::knowledge_graph` is not exported and `get_snapshot`/`create_edge` do not exist.

---

### Task 2: Backend Graph Module

**Files:**
- Modify: `tauri-app/src-tauri/src/db/mod.rs`
- Modify: `tauri-app/src-tauri/src/db/knowledge_graph.rs`

- [ ] **Step 1: Export the module**

Add:

```rust
pub mod knowledge_graph;
```

- [ ] **Step 2: Add snapshot types and helpers**

In `knowledge_graph.rs`, add `KnowledgeGraphNode`, `KnowledgeGraphSnapshot`, `get_snapshot`, and `create_edge`. Derive nodes from `crate::db::bible::get_bible`, load edges with existing `get_edges`, compute degree by counting source/target references, and return orphan count.

- [ ] **Step 3: Run GREEN for backend tests**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml knowledge_graph -- --nocapture
```

Expected: PASS.

---

### Task 3: Tauri Commands

**Files:**
- Modify: `tauri-app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add graph commands**

Add commands:

```rust
#[tauri::command]
async fn get_knowledge_graph(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<db::knowledge_graph::KnowledgeGraphSnapshot, String> {
    db::knowledge_graph::get_snapshot(&state.db, &project_id)
}

#[tauri::command]
async fn create_knowledge_graph_edge(
    state: tauri::State<'_, AppState>,
    project_id: String,
    source_id: String,
    source_type: String,
    target_id: String,
    target_type: String,
    edge_type: String,
    description: Option<String>,
) -> Result<db::knowledge_graph::KnowledgeGraphEdge, String> {
    db::knowledge_graph::create_edge(
        &state.db,
        &project_id,
        &source_id,
        &source_type,
        &target_id,
        &target_type,
        &edge_type,
        description.as_deref(),
    )
}

#[tauri::command]
async fn delete_knowledge_graph_edge(
    state: tauri::State<'_, AppState>,
    edge_id: String,
) -> Result<(), String> {
    db::knowledge_graph::delete_edge(&state.db, &edge_id)
}
```

- [ ] **Step 2: Register commands**

Add the three commands to `tauri::generate_handler!`.

- [ ] **Step 3: Compile**

Run:

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml knowledge_graph -- --nocapture
```

Expected: PASS.

---

### Task 4: Graph Page

**Files:**
- Modify: `tauri-app/src/App.tsx`
- Modify: `tauri-app/src/index.css`

- [ ] **Step 1: Add TypeScript interfaces**

Add:

```ts
interface KnowledgeGraphNode { id: string; node_type: string; label: string; subtitle?: string; description?: string; status: string; degree: number; }
interface KnowledgeGraphEdge { id: string; source_node_id: string; source_node_type: string; target_node_id: string; target_node_type: string; edge_type: string; description?: string; auto_inferred: boolean; confidence: number; }
interface KnowledgeGraphSnapshot { nodes: KnowledgeGraphNode[]; edges: KnowledgeGraphEdge[]; orphan_count: number; }
```

- [ ] **Step 2: Add navigation**

Add `graph: <KnowledgeGraphPage />` and sidebar label `graph: "Graph"`.

- [ ] **Step 3: Add `KnowledgeGraphPage`**

Implement a page that loads `get_knowledge_graph`, filters nodes by search/type, renders CSS-positioned nodes, shows the selected node inspector, and creates/deletes edges via the new commands.

- [ ] **Step 4: Add CSS**

Add graph layout classes for:

- `.graph-workbench`
- `.graph-toolbar`
- `.graph-type-filter`
- `.graph-layout`
- `.graph-canvas`
- `.graph-node`
- `.graph-inspector`
- `.edge-list`
- `.edge-form`

- [ ] **Step 5: Build**

Run:

```powershell
npm run build
```

Expected: PASS.

---

### Task 5: Verification

**Files:**
- All touched files.

- [ ] **Step 1: Run targeted Rust tests**

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml knowledge_graph -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run full Rust tests**

```powershell
cargo test --manifest-path tauri-app\src-tauri\Cargo.toml
```

Expected: PASS. Existing warnings are acceptable only if unrelated to this change.

- [ ] **Step 3: Run frontend build**

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Run diff whitespace check**

```powershell
git diff --check
```

Expected: no whitespace errors.

## Self-Review

Spec coverage: backend graph snapshot, graph CRUD commands, UI navigation, graph canvas, inspector, manual edge editing, and verification are mapped to tasks.

Placeholder scan: no `TBD`, `TODO`, or unspecified implementation steps remain.

Type consistency: `KnowledgeGraphNode`, `KnowledgeGraphEdge`, `KnowledgeGraphSnapshot`, `get_snapshot`, and `create_edge` are named consistently across tests, backend commands, and frontend interfaces.
