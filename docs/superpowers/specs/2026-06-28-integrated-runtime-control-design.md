# Integrated Runtime Control Design

Date: 2026-06-28

## Goal

Turn AI Novel Factory from a collection of powerful but scattered tools into a chapter-centered runtime control console. The work must confirm and strengthen RAG with SiliconFlow `bge-m3`, make the desktop pet persist outside the main window, redesign the Runtime page around the chapter production flow, and reduce duplicated or contradictory feature entry points without lowering the existing product target.

## Current Context

The app is a Tauri v2 desktop application with a React 19 frontend and Rust/rusqlite backend. Existing code already includes vector storage, content-hash dedupe, cosine retrieval, Graph-RAG reranking, model profiles, prompt runtime, context rules, operator recipes, import/export packages, extension packages, and a first shell-embedded status pet.

The current gaps are:

- RAG uses a single `embed(texts)` contract, so it cannot express BGE-M3's query/document asymmetry.
- The pet is rendered inside the main React shell, so it disappears when the main window is hidden or minimized.
- The Runtime page presents many unrelated panels in one long surface, which makes high-value capabilities feel like loose tools rather than a coherent writing runtime.
- Model settings, RAG readiness, prompt presets, package IO, extensions, and pet state can appear as separate concerns even when they affect the same chapter workflow.

## Architecture

### 1. RAG Context Layer

The RAG Context Layer owns embedding, vector indexing, retrieval queries, Graph-RAG reranking, context rules, hard facts, learning entries, style assets, and retrieval trace output.

The embedding interface must distinguish `Document` and `Query` inputs. Indexing content such as bible entries, chapters, learning entries, hard facts, and style assets uses `Document`. Runtime retrieval queries built from chapter plans and operator controls use `Query`.

The SiliconFlow `bge-m3` path must centralize its asymmetric encoding policy in the provider layer. Callers request an input kind; they do not hand-roll prefixes. For the OpenAI-compatible BGE-M3 request path, the implementation prepares document inputs as `passage: ...` and query inputs as `query: ...` before sending the `input` array.

### 2. Runtime Orchestration Layer

The Runtime page becomes the control console for the selected project and selected chapter plan. It organizes features by production phase:

- `准备`: chapter plan selection, workflow model bindings, RAG health, index freshness.
- `上下文`: context rules, Lorebook import, Graph/RAG trace, manual context controls.
- `生成`: operator recipes, prompt runtime, draft candidates.
- `审阅`: quality results, feedback decisions, revision candidates.
- `交付`: project packages, bible packages, prompt packages, extension packages.

The top task bar must answer the user's first operational question: whether the current chapter is ready to run and what needs attention.

### 3. Desktop Pet Layer

The pet becomes a separate transparent Tauri webview window. It is borderless, always on top, skipped from the taskbar, and independent of the main window's visibility. Hiding or minimizing the main window must not remove the pet.

The pet is a status assistant with light companionship:

- Shows `waiting`, `working`, `idle`, `attention`, and `context` states.
- Can be dragged and remembers its last valid screen position.
- Single-click expands a small status bubble with project, RAG state, and latest message.
- Double-click shows and focuses the main window.
- Right-click offers open main window, hide pet, and open settings.
- Does not expose high-impact actions such as generating a chapter.

Animations remain CSS-based and respect `prefers-reduced-motion` plus the existing `static`, `subtle`, and `lively` setting.

### 4. Capability Hygiene Layer

Each capability has one primary entry point and one clear responsibility:

- RAG is configured in Settings and monitored in Runtime.
- Model defaults are configured in Settings and bound to workflows in Runtime.
- Prompt presets live in the Runtime `生成` phase.
- Package import/export lives in the Runtime `交付` phase.
- Extensions live in the Runtime `交付` or enhancement area.
- Pet settings live in Settings; pet status lives in the pet window.

Existing high-value features should not be removed during this slice. The work should first consolidate UI placement and shared state. Fully obsolete or unreachable entry points can be listed for later cleanup only after tests prove the replacement path exists.

## Scope And Sequencing

This spec covers one integrated product slice because RAG readiness, model routing, Runtime layout, and pet status all depend on the same chapter workflow state. Implementation should still proceed in small testable increments:

1. RAG semantics and health.
2. Pet window independence and status mapping.
3. Runtime workflow layout and model/RAG panels.
4. Feature consistency cleanup and contract tests.

Each increment must leave the app runnable and must not require a live SiliconFlow network call for automated verification.

## RAG Requirements

### Embedding Semantics

- Add an embedding input kind that distinguishes document embeddings from query embeddings.
- Existing tests and fake providers should be updated so they can assert which kind was requested.
- All document indexing paths must use document embeddings.
- All retrieval query paths must use query embeddings.
- The old `embed(texts)` behavior may remain as a compatibility shim only if it delegates to a specific kind and cannot be accidentally used by new RAG code.

### SiliconFlow BGE-M3

- `openai_compat` embedding configuration must support SiliconFlow `bge-m3`.
- The default user-facing preset should make the expected base URL and model clear: `https://api.siliconflow.cn/v1` and `BAAI/bge-m3` unless the local app already stores a different explicit value.
- Testing the embedding provider must exercise both document and query embedding paths.
- The test result must report dimensions and whether both paths returned non-empty vectors.

### Index Freshness

Vector metadata must record enough information to detect stale indexes:

- embedding model
- embedding provider or profile
- embedding kind
- embedding dimension
- content hash
- indexed timestamp

If the configured model, provider/profile, kind, or dimension changes, existing vector rows must be treated as stale for that source and reindexed before being presented as healthy.

### Retrieval Trace

The context preview and Runtime RAG panel must explain:

- retrieval query summary
- top-k
- source type and source id
- title and excerpt
- similarity
- Graph-RAG boost or rerank reason
- metadata
- freshness status

Empty index, missing provider, missing API key, dimension mismatch, HTTP failure, and stale index must be distinct Chinese-facing states.

## Runtime UI Requirements

### Layout

The Runtime page must use a stable, workflow-oriented layout:

- A top task bar with chapter plan selection, readiness status, primary action, and latest result.
- A segmented workflow navigation for `准备`, `上下文`, `生成`, `审阅`, and `交付`.
- Phase-specific panels instead of one long grid of unrelated cards.
- Stable dimensions for controls and status elements so text and loading states do not shift the layout.

### Visual Direction

The UI should feel like a serious production console for long-form novel work: dense, calm, readable, and traceable. It should avoid marketing-style hero sections, oversized decorative cards, and one-note color themes. Use restrained contrast, compact labels, and clear state badges.

### Model Routing

Runtime should show workflow model bindings as a table:

- Draft
- Review
- Repair
- Embedding
- Summarization

Each row shows provider, model, base URL/profile label, relevant capabilities, and warnings. Binding is a Runtime action; global defaults remain in Settings.

### RAG Health

RAG health is a first-class Runtime status. It must show whether retrieval is usable before generation starts. The status must distinguish configured-but-empty, configured-but-stale, configured-but-failing, and disabled states.

## Desktop Pet Requirements

- Create or configure a separate `pet` Tauri webview window.
- Main window close-to-tray and minimize behavior must not hide or destroy the pet window.
- The pet can be shown or hidden from Settings and tray menu.
- The pet persists position and animation settings through app settings or a dedicated settings key.
- The pet window receives status updates without polling at a high frequency.
- The pet renders a compact status bubble on click and hides it on outside click or timeout.
- The pet supports reduced-motion users and static animation mode.

## Feature Consistency Requirements

- A feature must not have two visible entry points with different meanings.
- A state must not be represented by two competing fields.
- RAG trace data must have one shared backend structure and multiple UI projections.
- Model profile warnings must come from the existing provider capability layer.
- Runtime contract tests must prevent reintroducing a flat pile of unrelated cards as the primary organization.

## Non-Goals

- Do not replace the whole app shell.
- Do not remove existing advanced capabilities such as Prompt Runtime, extensions, package IO, operator recipes, or context rules.
- Do not introduce a 3D, WebGL, or high-frequency rendered pet.
- Do not depend on live SiliconFlow network calls in automated tests.
- Do not downgrade the existing Chinese-first UI work.

## Acceptance Criteria

- Automated tests prove document and query embedding paths are separate and used by the correct workflows.
- Automated tests prove SiliconFlow/BGE-M3 preparation is centralized and testable.
- RAG health can distinguish disabled, missing key, empty index, stale index, dimension mismatch, and usable states.
- Context preview includes a single shared retrieval trace that Runtime can render.
- The pet stays visible when the main window is hidden or minimized, can be dragged, and can reopen the main window.
- Runtime page source and contract tests show the five workflow phases and do not rely on the old flat panel organization.
- Settings remains the place for global defaults; Runtime is the place for current chapter readiness and workflow binding.

## Verification Plan

- Rust unit/integration tests for embedding input kinds, RAG health, vector freshness, provider routing, and pet window configuration.
- Node contract tests for Runtime workflow phases, RAG status rendering, model binding table, and pet UI state mapping.
- Build verification with `npm run build`.
- Full backend verification with `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture`.
- Frontend verification with `node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs`.
- Diff hygiene with `git diff --check`.
