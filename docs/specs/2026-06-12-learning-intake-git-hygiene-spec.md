# Learning Intake and Git Hygiene Spec

Date: 2026-06-12

## Goal

Make the Learn page safe and non-blocking for real user sources while preventing local artifacts, secrets, browser output, and dependency folders from being committed accidentally.

## Findings

- The repository has no root `.gitignore`, so `.env`, logs, installer files, dependency folders, and local experiment folders show up as untracked files.
- The current Learn page has a `Manual Input` textarea. Users must paste source text by hand, which is not the desired workflow.
- Web Learn fetches pages from the React renderer, falls back to a backend HTML fetch, then strips HTML in the renderer. This can hit browser/CORS failures, pull boilerplate text, and process large pages on the UI side.
- `workflow::learning::extract_knowledge` truncates input with byte indices. Long Chinese text can panic if the truncation point lands inside a UTF-8 character.
- Playwright verification in this workspace must use Microsoft Edge, not Chrome.

## Scope

### Git Hygiene

- Add a root `.gitignore`.
- Ignore dependency folders, build output, Rust targets, Tauri generated schemas, logs, local env files, local databases, local exported novel data, Playwright/browser reports, screenshots, installers, and local legacy/experiment folders.
- Keep source files, tracked docs, `package-lock.json`, `Cargo.lock`, Tauri icons, migrations, prompts, and tests trackable.
- Do not remove or revert existing user changes.
- Do not commit `.env`, API keys, keychain material, SQLite runtime databases, local machine paths, logs, or `node_modules`.

### Learn File Intake

- Rename the Learn tab from `Manual Input` to file-based intake.
- Remove the visible free-text sample textarea from the Learn UI.
- Let the user choose a local text-like file and have the app read and learn from it.
- Supported file extensions for this slice: `.txt`, `.md`, `.markdown`, `.text`, `.log`, `.csv`, `.json`, and `.html`.
- Reject empty files, unsupported extensions, and files larger than 1 MiB with clear UI feedback.
- Source title defaults to the chosen file name, with an optional editable title field.

### Web Learn Stability

- Move URL normalization, HTTP fetch, HTML cleanup, boilerplate filtering, source-title extraction, text-size limits, and learning invocation behind a backend command.
- The frontend must not fetch remote HTML directly and must not strip large HTML strings in React.
- Only `http` and `https` URLs are accepted.
- Local/private URL forms such as `localhost`, loopback, `file:`, `data:`, and `javascript:` are rejected.
- Web HTML extraction removes script/style/nav/header/footer/aside/form boilerplate and common cookie/subscribe/privacy lines before sending text to the model.
- The cleaned learning text is truncated by characters, not bytes, to a maximum of 15,000 characters.
- If meaningful page content is below 200 characters, Web Learn returns a clear error instead of storing junk.

### Runtime Safety

- All learning input truncation must be UTF-8 safe.
- Existing reflection truncation in `workflow::learning` must also be UTF-8 safe because it uses the same pattern.
- Backend fetches must have bounded timeouts and size limits.
- The app must remain responsive while file or web learning is running.

### Playwright/Edge Constraint

- Any Playwright smoke check in this workspace must target Edge, for example by using an installed `msedge` executable or an Edge channel, because Chrome is not installed locally.
- Do not add a Chrome-only verification assumption.

## Non-Goals

- Do not add PDF or DOCX parsing in this slice.
- Do not add new network-only frontend dependencies.
- Do not rewrite the whole React app into separate pages.
- Do not weaken model extraction standards or reduce review/learning quality to hide failures.

## Acceptance Criteria

- Root `git status --short` no longer lists `.env`, `node_modules`, logs, installers, or ignored local experiment folders after `.gitignore` is added.
- A regression test proves long Chinese learning text no longer panics during extraction.
- A regression test proves HTML extraction drops scripts, navigation, cookie/subscribe noise, and keeps article prose.
- A regression test proves unsafe/private URL forms are rejected.
- A regression test proves file-intake validation rejects unsupported extensions and oversized files.
- The Learn page no longer exposes a manual sample textarea and presents a file chooser instead.
- Web Learn invokes one backend command for URL learning and does not call browser `fetch()` for the page.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test learning_intake_tests -- --nocapture` passes.
- `cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml --test core_writing_loop_tests -- --nocapture` passes for learning-related regressions.
- `npm run build` from `tauri-app` passes.
- `git diff --check` exits 0.
