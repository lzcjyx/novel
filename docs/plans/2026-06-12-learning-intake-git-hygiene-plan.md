# Learning Intake and Git Hygiene Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace manual Learn input with file intake, make Web Learn backend-owned and UTF-8 safe, and add root git hygiene rules that prevent accidental local or secret commits.

**Architecture:** Keep the Learn UI small and move source normalization/extraction to Rust helpers under `workflow::learning_intake`. Reuse the existing `workflow::learning::extract_knowledge` model path, but make truncation character-safe and persist source URLs for web entries.

**Tech Stack:** Rust 2021, Tauri v2 commands, reqwest, regex, React 19, TypeScript, Vite, existing CSS.

---

## File Map

- Create `.gitignore`: root ignore rules for dependencies, build output, secrets, logs, local data, Playwright output, installers, and local experiment folders.
- Create `tauri-app/src-tauri/src/workflow/learning_intake.rs`: URL validation, source title extraction, HTML cleanup, file validation, and character-safe truncation helpers.
- Modify `tauri-app/src-tauri/src/workflow/mod.rs`: export `learning_intake`.
- Modify `tauri-app/src-tauri/src/workflow/learning.rs`: use character-safe truncation for extraction and reflection prompts.
- Modify `tauri-app/src-tauri/src/lib.rs`: add `learn_from_file_text` and `learn_from_url`, persist source URLs, reuse a shared learning-entry persistence helper, and register the commands.
- Modify `tauri-app/src/App.tsx`: replace `Manual Input` textarea with file selection and call backend-owned Web Learn.
- Modify `tauri-app/src/index.css` if needed for file-intake affordances.
- Create `tauri-app/src-tauri/tests/learning_intake_tests.rs`: Rust regression tests for URL validation, HTML cleanup, file validation, and UTF-8 truncation.

## Tasks

### Task 1: Add Root Git Hygiene

- [x] **Step 1: Create root `.gitignore`**

Add rules for:

```gitignore
.env
.env.*
!.env.example
**/node_modules/
**/dist/
**/target/
*.log
*.sqlite
*.db
playwright-report/
test-results/
*.msi
writer-service/
orchestrator/
autoresearch/
DESIGN.md
```

- [x] **Step 2: Verify ignored local files**

Run:

```powershell
git status --short
```

Expected: `.env`, `node_modules/`, logs, `gh_install.msi`, `writer-service/`, `orchestrator/`, and `autoresearch/` are not listed as untracked files.

### Task 2: Write Learning Intake Regression Tests

- [x] **Step 1: Create failing helper tests**

Create `tauri-app/src-tauri/tests/learning_intake_tests.rs` with tests that reference:

```rust
tauri_app_lib::workflow::learning_intake::normalize_learning_url;
tauri_app_lib::workflow::learning_intake::extract_meaningful_text_from_html;
tauri_app_lib::workflow::learning_intake::validate_user_file_text;
tauri_app_lib::workflow::learning::extract_knowledge;
```

Test cases:

- `rejects_private_or_unsafe_learning_urls`
- `extracts_article_text_without_page_boilerplate`
- `validates_file_learning_sources`
- `extract_knowledge_truncates_multibyte_input_safely`

- [x] **Step 2: Run tests and require RED**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture
```

Expected: FAIL because `workflow::learning_intake` does not exist yet and `extract_knowledge` still slices UTF-8 by bytes.

### Task 3: Implement Learning Intake Helpers

- [x] **Step 1: Add `learning_intake.rs`**

Implement:

```rust
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
pub const MAX_LEARNING_CHARS: usize = 15_000;

pub struct ValidatedLearningText {
    pub source_title: String,
    pub text: String,
}

pub fn truncate_chars(input: &str, max_chars: usize) -> String;
pub fn normalize_learning_url(input: &str) -> Result<String, String>;
pub fn extract_source_title(url: &str, html: &str) -> String;
pub fn extract_meaningful_text_from_html(html: &str) -> Result<String, String>;
pub fn validate_user_file_text(file_name: &str, byte_len: usize, text: &str) -> Result<ValidatedLearningText, String>;
```

Implementation requirements:

- Only allow `http` and `https`.
- Reject loopback/private/local hosts.
- Drop script/style/noscript/svg/nav/header/footer/aside/form blocks.
- Decode common HTML entities and numeric entities.
- Filter cookie, subscribe, privacy, login, share, ad, and ICP/copyright lines.
- Return an error if cleaned content has fewer than 200 characters.

- [x] **Step 2: Export the module**

Add:

```rust
pub mod learning_intake;
```

to `tauri-app/src-tauri/src/workflow/mod.rs`.

- [x] **Step 3: Run helper tests**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture
```

Expected: helper tests pass or fail only on UTF-8 extraction until Task 4 is complete.

### Task 4: Make Learning Extraction UTF-8 Safe

- [x] **Step 1: Replace byte slicing**

In `workflow/learning.rs`, replace:

```rust
&text[..text.len().min(4000)]
&chapter_body[..chapter_body.len().min(5000)]
```

with character-safe truncation via `learning_intake::truncate_chars`.

- [x] **Step 2: Run learning tests**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture
```

Expected: PASS.

### Task 5: Add Backend Web Learn Command

- [x] **Step 1: Add shared persistence helper in `lib.rs`**

Persist `source_type`, `source_url`, and `source_title` from each `LearningEntry`.

- [x] **Step 2: Add `learn_from_file_text`**

Command behavior:

- Validate selected project.
- Validate file name, extension, size, and readable text through `learning_intake::validate_user_file_text`.
- Use an optional UI-provided source title, falling back to the file name.
- Call `workflow::learning::extract_knowledge(provider, text, title, "manual_file", None)`.
- Persist entries.

- [x] **Step 3: Add `learn_from_url`**

Command behavior:

- Validate selected project.
- Normalize and validate URL.
- Fetch with reqwest timeout and a browser-like user agent.
- Reject pages over `MAX_SOURCE_BYTES`.
- Extract meaningful text and source title.
- Call `workflow::learning::extract_knowledge(provider, text, title, "web", Some(url))`.
- Persist entries with source URL.

- [x] **Step 4: Register commands**

Add `learn_from_file_text` and `learn_from_url` to the Tauri invoke handler.

- [x] **Step 5: Run targeted compile/test**

Run:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture
```

Expected: PASS.

### Task 6: Replace Manual Input UI With File Intake

- [x] **Step 1: Update Learn state and tab labels**

Change tab type from `"input"` to `"file"` and label from `Manual Input` to `File Learn`.

- [x] **Step 2: Add file chooser path**

Use a hidden `<input type="file">`, accept `.txt,.md,.markdown,.text,.log,.csv,.json,.html`, read the file with `File.text()`, reject files over 1 MiB before reading, then call:

```ts
invoke<any[]>("learn_from_file_text", {
  projectId: selected,
  fileName: file.name,
  byteLen: file.size,
  text,
  sourceTitle: sourceTitle || null
})
```

- [x] **Step 3: Remove visible manual textarea**

The user should not see or use a free-text source editor on Learn.

- [x] **Step 4: Update Web Learn UI call**

Replace frontend `fetch()` and HTML stripping with:

```ts
invoke<any[]>("learn_from_url", { projectId: selected, url: url.trim() })
```

- [x] **Step 5: Update empty library copy**

Reference file import and Web Learn, not paste.

- [x] **Step 6: Run frontend build**

Run from `tauri-app`:

```powershell
npm run build
```

Expected: PASS.

### Task 7: Final Verification

- [x] Run targeted learning tests:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture
```

- [x] Run core loop tests:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture
```

- [x] Run all Rust tests:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml
```

- [x] Run frontend build:

```powershell
npm run build
```

- [x] Run whitespace check:

```powershell
git diff --check
```

- [x] Run final git hygiene check:

```powershell
git status --short
```

Expected: only project source/docs changes relevant to this work remain visible, plus pre-existing tracked modifications unrelated to this task.

## Self-Review

Spec coverage: git hygiene, file-based Learn, backend Web Learn, UTF-8 safety, Edge/Playwright constraint, and verification are each mapped to tasks.

Placeholder scan: no TBD or incomplete task remains.

Type consistency: helper names, command names, source types, and file paths are consistent across the tasks.

## Verification Results

- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture`: PASS, 4 tests.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture`: PASS, 20 tests.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml`: PASS, all Rust unit, integration, and doc tests.
- `npm run build` from `tauri-app`: PASS.
- `git diff --check`: PASS with existing CRLF conversion warnings only.
- `rg -n "Manual Input|fetch\(" tauri-app/src/App.tsx tauri-app/src-tauri/src --glob '!**/target/**'`: no matches.
