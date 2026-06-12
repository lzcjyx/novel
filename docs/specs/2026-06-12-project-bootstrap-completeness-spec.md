# Project Bootstrap Completeness Spec

Date: 2026-06-12

## Goal

Creating a project must not leave a successful-looking partial project. A successful bootstrap creates the project row, project paper directory metadata, Bible data, initial chapter plans, and a readable knowledge graph snapshot derived from Bible entities. If any required bootstrap step fails, the command must return an error and remove the partial project row through the existing cascade cleanup path.

## Root Causes

1. `workflow/novel_bootstrap.rs` creates the project before asking the model for Bible JSON. If Bible generation fails, it logs the error and returns `Ok(project)`, so the UI reports success even though Bible, graph nodes, and chapter plans are empty.
2. Bible persistence uses ignored insert results in several places. A database error can silently drop required Bible rows while bootstrap still reports success.
3. The success path does not validate required bootstrap artifacts after persistence. There is no final guard that the project has at least the required Bible entities, 10 planned chapters, and graph nodes.
4. Cleanup is not part of the failure path. Once the project row is inserted, failed bootstrap steps can leave an active project with no usable writing context.

## Requirements

### Atomic Bootstrap Semantics

- `bootstrap_novel` must return `Err` when Bible generation fails.
- `bootstrap_novel` must delete the just-created project if any required bootstrap step fails after project insertion.
- Cleanup must use the existing project deletion cascade for SQLite rows.
- Cleanup failure must be included in the returned error message, but must not convert a failed bootstrap into success.

### Required Bootstrap Artifacts

A successful bootstrap must persist:

- At least 6 characters.
- At least 4 locations.
- At least 2 organizations.
- At least 1 world lore entry.
- At least 1 power or magic system.
- At least 5 canon rules.
- At least 3 plot threads.
- Exactly 10 initial chapter plans.
- At least one graph node in the derived knowledge graph snapshot.

The graph requirement is satisfied by `db::knowledge_graph::get_snapshot`, because graph nodes are derived from Bible tables. Edges can remain optional during bootstrap.

### Persistence Errors Are Fatal

- Inserts for required Bible rows and chapter plans must propagate database errors instead of ignoring them.
- Optional vector indexing remains best-effort. Embedding failures may be logged and skipped, because the existing app supports rebuilding the vector index later.
- The required artifact validation must run after Bible persistence and before returning success.

### User-Facing Behavior

- The create project command should either return a complete project or an error explaining which bootstrap phase failed.
- The project list must not contain the just-created project after a required bootstrap failure.
- The implementation must not downgrade the goal by accepting empty Bible, empty graph, or empty plans as a valid project.

## Acceptance Criteria

- A regression test with a failing Bible provider proves `bootstrap_novel` returns `Err` and leaves no project rows.
- A regression test with an incomplete Bible provider proves missing required artifacts cause `Err` and cleanup.
- A success regression test proves a valid mock provider creates Bible data, 10 plans, and a graph snapshot with nodes.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test project_bootstrap_tests -- --nocapture` passes.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml` passes.
- `npm run build` in `tauri-app` passes.
- `git diff --check` exits 0 apart from pre-existing CRLF warnings if any appear.
