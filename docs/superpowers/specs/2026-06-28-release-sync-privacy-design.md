# Release Sync Privacy Design

Date: 2026-06-28

## Goal

Prepare the project for a safe release by proving the local repository and remote repository are in sync, building a Tauri release artifact, and strengthening the Firefly auto-publish path so generated website commits do not leak local secrets, API keys, or machine paths.

## Current Evidence

- Workspace: `D:\novel`.
- Current branch: `codex/integrated-runtime-control`.
- Remote: `origin` at `https://github.com/lzcjyx/novel.git`.
- `git fetch --all --prune` completed without conflicts.
- Current branch has tracking branch `origin/codex/integrated-runtime-control`.
- Current branch ahead/behind after fetch: `0 0`.
- `main` tracks `origin/main` and is ahead by 5 commits:
  - `57f2487 feat: route rag embeddings by input kind`
  - `2e44ed5 feat: report rag health and vector freshness`
  - `9ae2b12 feat: distinguish query and document embeddings`
  - `891c8bf docs: add integrated runtime control plan`
  - `7dd9331 docs: add integrated runtime control spec`
- Local branches `feat/sillytavern-runtime-improvements` and `feature/knowledge-acid-winui` have no upstream, so they must be treated as local-only work unless the user explicitly asks to publish those branch names.
- Tauri release entry point is `npm run tauri build` from `D:\novel\tauri-app`.

## Safety Rules

1. Never rewrite history, reset, clean, or force-push.
2. Before pushing, require a clean worktree or only staged changes intentionally created for this task.
3. Push only branches that already have an upstream and are ahead without being behind.
4. Do not push local-only branches automatically.
5. If a tracked branch is both ahead and behind, stop and report divergence instead of merging blindly.
6. Release build artifacts may be generated locally, but generated build output should not be committed unless the repository already tracks it.
7. Firefly auto-publish must stage only the generated post file in the Firefly repository.
8. Firefly auto-publish must not write API keys, bearer tokens, local Windows paths, home-directory paths, internal prompt/debug metadata, database paths, or provider secrets into post frontmatter, post body, command logs, or commit messages.

## Firefly Privacy Contract

The publish adapter must sanitize all content that can enter the Firefly Git history:

- `title`
- `description`
- `tags`
- `category`
- generated Markdown body
- generated commit message
- command logs stored in queue metadata

Sensitive body lines are removed when they look like operational metadata. Secret-like substrings in visible prose are redacted. Local paths are redacted rather than committed. Existing safety behavior remains: unrelated Firefly repository changes block publishing, and push is optional.

## Acceptance Criteria

- Specs and plans exist under `docs/superpowers`.
- Repository ahead/behind state is documented after a fresh fetch.
- Tracked branches that are safely ahead-only are pushed or explicitly reported if pushing fails.
- Local-only branches are listed but not pushed automatically.
- Worktree remains clean except for intentional spec/plan/privacy changes before commit.
- Firefly Markdown rendering redacts secrets and local paths from frontmatter and body.
- Firefly Git commit messages are redacted before being committed.
- Targeted privacy tests pass.
- Full Rust test suite passes.
- Full Node contract test suite passes.
- Frontend production build passes.
- Tauri release build succeeds and the artifact path is reported.
- `git diff --check` passes.
- Final work is committed and pushed to the current remote branch.
