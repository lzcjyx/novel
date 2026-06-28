# Release Sync Privacy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Safely synchronize tracked local branches with their remotes, harden Firefly auto-publish privacy, and produce a verified Tauri release build.

**Architecture:** Treat Git synchronization, Firefly publishing privacy, and release artifacts as separate gates. Git synchronization uses fetch plus ahead/behind checks without history rewriting. Firefly privacy is enforced in the static-site publish adapter with tests before production changes.

**Tech Stack:** Git, PowerShell, Rust 2021, rusqlite, Tauri v2, React 19, TypeScript, Node test runner, npm/Tauri CLI.

---

## File Structure

- Create `docs/superpowers/specs/2026-06-28-release-sync-privacy-design.md`: release synchronization and privacy contract.
- Create `docs/superpowers/plans/2026-06-28-release-sync-privacy.md`: executable plan and verification checklist.
- Modify `tauri-app/src-tauri/src/workflow/static_site_publish.rs`: centralize publish text redaction for Firefly Markdown and Git commit messages.
- Modify `tauri-app/src-tauri/tests/publication_queue_tests.rs`: add privacy regression tests for frontmatter/body/commit-message redaction.

## Task 1: Repository Synchronization Audit

- [ ] Run `git fetch --all --prune`.
  Expected: command exits 0.

- [ ] Run `git branch -vv`.
  Expected: tracked branches show upstream state; local-only branches are visible.

- [ ] Run tracked-branch ahead/behind audit:

```powershell
$branches = git for-each-ref --format='%(refname:short) %(upstream:short)' refs/heads
foreach ($line in $branches) {
  $parts = $line -split ' '
  if ($parts.Length -ge 2 -and $parts[1]) {
    $counts = git rev-list --left-right --count "$($parts[1])...$($parts[0])"
    "$($parts[0]) <= $($parts[1]) : $counts"
  } else {
    "$($parts[0]) : no upstream"
  }
}
```

Expected for this run:

```text
codex/integrated-runtime-control <= origin/codex/integrated-runtime-control : 0 0
learn-intake-git-hygiene <= origin/learn-intake-git-hygiene : 0 0
main <= origin/main : 0 5
feat/sillytavern-runtime-improvements : no upstream
feature/knowledge-acid-winui : no upstream
```

- [ ] Push `main` because it is ahead-only:

```powershell
git push origin main
```

Expected: `origin/main` advances to local `main`. If push is rejected, stop and run `git fetch --all --prune` plus the ahead/behind audit again before deciding.

- [ ] Do not push `feat/sillytavern-runtime-improvements` or `feature/knowledge-acid-winui` automatically because they have no upstream.

## Task 2: Firefly Privacy Regression Test

- [ ] Add a failing Rust test in `tauri-app/src-tauri/tests/publication_queue_tests.rs` named `firefly_publish_redacts_secrets_paths_and_commit_messages`.

Test body:

```rust
#[test]
fn firefly_publish_redacts_secrets_paths_and_commit_messages() {
    let post = StaticSitePost {
        title: "发布 sk-123456789012345678901234".to_string(),
        slug: "privacy-check".to_string(),
        published: "2026-06-28".to_string(),
        description: "本地路径 D:\\novel\\private.db 与 Bearer abcdefghijklmnopqrstuvwxyz".to_string(),
        tags: vec!["token=abcdefghijklmnopqrstuvwxyz".to_string()],
        category: Some("C:\\Users\\secret\\drafts".to_string()),
        lang: Some("zh-CN".to_string()),
        body_markdown: "正文可见\napi_key=abcdefghijklmnopqrstuvwxyz\n路径 D:\\novel\\private.db\nBearer abcdefghijklmnopqrstuvwxyz".to_string(),
    };

    let markdown = render_firefly_markdown(&post);
    assert!(!markdown.contains("sk-123456789012345678901234"));
    assert!(!markdown.contains("D:\\novel"));
    assert!(!markdown.contains("C:\\Users"));
    assert!(!markdown.contains("abcdefghijklmnopqrstuvwxyz"));
    assert!(markdown.contains("***REDACTED***"));
    assert!(markdown.contains("[LOCAL_PATH_REDACTED]"));

    let redacted_commit =
        tauri_app_lib::workflow::static_site_publish::redact_publish_commit_message(
            "publish: add sk-123456789012345678901234 from D:\\novel\\private.db",
        );
    assert!(!redacted_commit.contains("sk-123456789012345678901234"));
    assert!(!redacted_commit.contains("D:\\novel"));
}
```

- [ ] Run:

```powershell
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml --test publication_queue_tests firefly_publish_redacts_secrets_paths_and_commit_messages -- --nocapture
```

Expected red result before implementation: failure because the redaction helper is missing or the sensitive strings are still present.

## Task 3: Implement Firefly Publish Redaction

- [ ] Modify `tauri-app/src-tauri/src/workflow/static_site_publish.rs`.

Implementation requirements:

- Add `pub fn redact_publish_commit_message(message: &str) -> String`.
- Add a private `fn redact_publish_text(value: &str) -> String`.
- Apply `redact_publish_text` before YAML string escaping for title, description, tags, category, and lang.
- Apply `redact_publish_text` to body lines that are not fully removed.
- Apply `redact_publish_commit_message` immediately before the `git commit -m` command in `publish_firefly_git`.
- Redact common secret tokens using the existing `redact_secrets`.
- Redact local absolute paths such as `D:\novel\file.db`, `C:\Users\name\file`, `/Users/name/file`, and `/home/name/file`.

- [ ] Run targeted test again.
  Expected: test passes.

- [ ] Run all publication queue tests:

```powershell
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml --test publication_queue_tests -- --nocapture
```

Expected: all publication queue tests pass.

## Task 4: Full Verification

- [ ] Run full Rust suite:

```powershell
cargo test -j 1 --manifest-path tauri-app/src-tauri/Cargo.toml -- --nocapture
```

Expected: all tests pass.

- [ ] Run full Node contract suite:

```powershell
node --test tauri-app/src/*.test.mjs tauri-app/src/lib/*.test.mjs
```

Expected: all tests pass.

- [ ] Run frontend production build:

```powershell
npm run build
```

Working directory: `D:\novel\tauri-app`.
Expected: TypeScript and Vite build exit 0.

- [ ] Run Tauri release build:

```powershell
npm run tauri build
```

Working directory: `D:\novel\tauri-app`.
Expected: release bundle exits 0 and creates artifacts under `tauri-app/src-tauri/target/release/bundle`.

- [ ] Run:

```powershell
git diff --check
git status -sb
```

Expected: no whitespace errors; only intentional files are changed before commit.

## Task 5: Commit And Push

- [ ] Stage only intentional files:

```powershell
git add docs/superpowers/specs/2026-06-28-release-sync-privacy-design.md docs/superpowers/plans/2026-06-28-release-sync-privacy.md tauri-app/src-tauri/src/workflow/static_site_publish.rs tauri-app/src-tauri/tests/publication_queue_tests.rs
```

- [ ] Commit:

```powershell
git commit -m "fix: harden firefly publish privacy"
```

- [ ] Push current branch:

```powershell
git push origin codex/integrated-runtime-control
```

- [ ] Confirm final status:

```powershell
git status -sb
```

Expected: current branch has no uncommitted changes and no ahead/behind difference.

## Self-Review

- Spec coverage: repository synchronization, local-only branch handling, Firefly privacy, release build, full test verification, commit, and push are covered.
- Placeholder scan: no task uses TBD, TODO, or vague "handle edge cases" language.
- Type consistency: function names in tests and implementation tasks match `render_firefly_markdown`, `redact_publish_commit_message`, and `StaticSitePost`.
