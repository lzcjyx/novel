# Resilient Auto Publish And Pet Visibility Design

Date: 2026-06-28

## Goal

Add a portable, ACID-style scheduled writing and automatic publishing system, and fix the desktop pet so enabling it reliably makes it visible. The implementation must keep Firefly support first-class without hard-coding the whole feature to one user's website.

## Evidence

- The app is a Tauri v2 desktop app with React, Rust, rusqlite, `generation_jobs`, `blog_posts`, and an existing `publication_queue` table.
- Current auto publish only creates a local `blog_posts` draft after a chapter reaches `publish_ready`; it does not write a website post, commit, push, recover interrupted publication, or expose a scheduler control in the navigation.
- Existing `task_transaction` already tracks `publication_queue`, so publication queue rows can participate in the current recovery model.
- The local Firefly site at `D:\Learning\Code\Git\website\Firefly` is an Astro 6 project deployed through Cloudflare Workers assets.
- Firefly writes posts under `src/content/posts`, validates frontmatter in `src/content.config.ts`, builds with `pnpm build`, and uses `wrangler.toml` / `wrangler.jsonc` for Cloudflare Workers assets.
- The Firefly quick-start docs confirm Node.js 22+, pnpm, Git, `src/content`, and `pnpm build`.
- The pet is declared as a transparent always-on-top Tauri window, but enabling it can still be invisible if the saved position is off-screen, the window is shown before layout readiness, or the transparent window has no obvious fallback visibility contract.

## Architecture

### 1. Publication Target Layer

Publishing is configured through portable target settings, not hard-coded paths. A target has:

- `provider`: initially `firefly_git`.
- `workspace_path`: local website repository path.
- `remote_name`: default `origin`.
- `branch`: optional branch override; empty means current branch.
- `posts_dir`: default `src/content/posts`.
- `build_command`: default `pnpm build`.
- `commit_template`: default `publish: add {title}`.
- `push_enabled`: whether the app may push to the target remote.
- `dry_run`: validate and write preview metadata without touching the external repository.

Firefly is one adapter that implements a generic file-based static-site contract. Future adapters can use the same queue and scheduler while changing only post rendering and validation rules.

### 2. Publication Outbox Layer

`publication_queue` becomes the authoritative outbox. A chapter that reaches `publish_ready` and is allowed to publish creates or updates one queue row keyed by project/chapter/provider. The queue row stores target snapshot, generated slug, target file path, content hash, command log, commit id, and recovery attempts in metadata.

The allowed states are:

- `pending`: ready to publish when `scheduled_at <= now`.
- `publishing`: claimed by the current process.
- `published`: file committed and optional push succeeded.
- `failed`: terminal for the current attempt; retry can move it back to `pending`.
- `cancelled`: user disabled the schedule before publication.
- `needs_human_review`: generated chapter was not safe to publish.

Startup recovery must move stale `publishing` rows back to `pending` with a recovery note, unless they already have evidence of a completed commit. This handles closing the app mid-publish.

### 3. ACID And Privacy Rules

SQLite remains the source of truth for internal state. External Git writes cannot be part of the same database transaction, so the app uses a durable outbox plus idempotent file writes:

- queue creation and local draft metadata are persisted before external side effects;
- publishing claims one row at a time;
- file paths are deterministic and slug-based;
- if the same queue item is retried, the adapter rewrites the same post path instead of creating duplicates;
- secrets and tokens are never stored in settings, metadata, commit messages, logs, or generated frontmatter;
- only the generated post file is staged for commit; unrelated target repository changes are detected and block publishing unless the queue item is in dry-run mode.

### 4. Firefly Adapter

For Firefly, a published chapter becomes a Markdown file:

```yaml
---
title: <title>
published: <YYYY-MM-DD>
description: <excerpt>
image: ""
tags: [tag1, tag2]
category: <category>
draft: false
lang: zh-CN
comment: true
---
```

The adapter validates:

- target repository exists and has `.git`;
- `src/content.config.ts` exists;
- posts directory exists or can be created;
- slug contains only lower-case letters, numbers, dashes, underscores, or path separators after sanitization;
- no frontmatter key receives raw prompt, review JSON, API key, local database path, or provider secret;
- `pnpm build` passes before commit when build validation is enabled.

The adapter commits only the generated post file, then pushes the configured branch when enabled.

### 5. Scheduler UI

The navigation area gets a compact toggle labeled `定时写作`. It controls whether scheduled writing/publishing is active globally. Settings hold the detailed target configuration. Runtime remains the place to view queue status and retry failures.

The toggle must not start a duplicate generation if a project is already running. When the app starts after a missed time window, the scheduler enqueues the next eligible pending chapter rather than silently skipping it.

### 6. Pet Visibility Fix

Showing the pet must:

- set `pet_enabled = true`;
- clamp the saved position into the current monitor work area;
- show, unminimize, and focus the pet window when the user explicitly enables it;
- emit a ready/default status so the transparent window has visible content even before the main shell sends runtime state;
- keep drag persistence best-effort but never save absurd off-screen coordinates.

The pet window should remain independent of the main window and preserve the lightweight CSS animation approach.

## Non-Goals

- Do not store GitHub, Cloudflare, or API tokens in the app database.
- Do not implement a hosted publishing API in this slice.
- Do not push when the target repository has unrelated dirty changes.
- Do not turn the pet into a high-frequency canvas/WebGL widget.

## Acceptance Criteria

- Specs and plans exist under `docs/superpowers`.
- Settings expose portable publication target fields and scheduler enablement.
- Navigation exposes an `开启/关闭定时写作` control.
- A publish-ready chapter can create a durable queue item.
- Stale `publishing` rows recover on startup or explicit recovery.
- Firefly rendering writes valid Markdown under `src/content/posts`.
- Publishing validates the Firefly build command before commit when enabled.
- Publishing stages and commits only the generated post file.
- Push is optional and privacy-preserving.
- Pet show clamps position and makes the window visible after the user enables it.
- Automated Rust and Node tests cover the queue state machine, Firefly adapter, scheduler UI contract, and pet visibility regression.

## Verification Plan

- Rust tests for target settings persistence, publication queue upsert/recovery, Firefly Markdown rendering, dirty repo blocking, and pet position clamping.
- Node contract tests for navigation scheduled-writing toggle and Tauri client publication commands.
- Existing full backend test suite.
- Existing frontend contract tests.
- Production build.
- `git diff --check`.
