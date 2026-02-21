# Claude Code Instructions

## Pre-Commit Requirements

Before creating any git commit, you MUST run `just pre-commit`.

Only proceed with the commit if it passes. If it fails, fix the issues first.

**Exception:** If the commit contains only documentation and markdown changes (`.md` files), you may skip `just pre-commit`.

`just pre-commit` intentionally runs generation and formatting steps that may modify files you did not edit directly (including markdown). Treat these diffs as expected outputs of the check pipeline.

If those diffs are mechanical (generation/format/lint output only), they should be committed with the related change. Do not treat them as noise and do not leave them uncommitted.

Common expected examples (non-exhaustive):
- `packages/opencode/packages/sdk/openapi.json`
- `packages/opencode/README.md` (including normalized markdown formatting / generated sections)
- `packages/core/README.md` (synced by hook)
- `bun.lock` files

Required post-`just pre-commit` checklist:
1. Check `git status` (superproject) and `git -C packages/opencode status` (submodule).
2. Review all newly changed files and confirm they are mechanical (no unintended behavior changes).
3. Commit/push submodule generated changes first when present.
4. Run `just update-opencode-commit`.
5. Commit/push the superproject pointer + Dockerfile pin update when changed.

## Bun Lockfile Updates

`bun.lock` file updates are expected in this repository across all `bun.lock` files when changing versions of our own packages (for example `packages/cli-node`, `packages/core`, and related workspace packages). They can also be produced by normal `just pre-commit`/build flows.

When these `bun.lock` changes are tied to intended package/version updates, they are valid and should be committed with the related change. Do not treat them as unexpected noise.
This same expected-and-commit rule applies to other mechanical formatter/generator outputs from `just pre-commit`, including markdown rewrites.

## Project Structure

This is a polyglot monorepo with Rust and TypeScript:

- `packages/core/` - Rust core library with NAPI-RS bindings
- `packages/cli-rust/` - Rust CLI binary
- `packages/cli-node/` - Node.js CLI wrapper
- `packages/opencode/` - Git submodule pointing to our fork of the opencode repository. We own this fork and can freely modify, commit, and push changes to it.

## Fork Isolation (Upstream Modification Policy)

The `packages/opencode/` submodule is a fork that syncs from upstream every 2 hours. To minimize merge conflicts:

- **Prefer putting new code in `fork-*` packages** (`fork-auth`, `fork-config`, `fork-ui`, `fork-security`, `fork-terminal`, `fork-cli`, `fork-provider`, `fork-tests`). These are ours and never conflict with upstream.
- **Minimize modifications to non-fork packages** (`app`, `opencode`, `sdk`, `ui`, `util`). When changes to upstream code are needed, keep the diff as small as possible — typically just an import and a single function call that delegates to a `fork-*` package.
- **Do not refactor upstream code.** Even if upstream code has bugs or poor patterns, fix only what directly blocks our users. Leave broader cleanups to upstream — they will fix their own bugs over time. Every line we change in upstream files is a potential merge conflict.
- **Tolerate imperfect upstream code.** If an upstream bug doesn't affect our users, leave it alone. If it does affect our users, fix it with the smallest possible diff (inline changes > new imports > new files).
- Use thin re-export shims in upstream packages that delegate to `fork-*` implementations when hooks are needed.

## Key Commands

```bash
just setup       # First command after clone/worktree init (hooks + deps + submodule bootstrap)
just build       # Build all packages
just test        # Run all tests
just e2e         # Run e2e tests (Playwright, boots server in-process)
just fmt         # Format all code
just lint        # Lint all code
just pre-commit  # Format, lint, build, test, and e2e
just clean       # Clean build artifacts
just run <args>  # Run CLI with arguments (e.g., just run status)
just dev         # Start local runtime (local submodule + cached sandbox rebuild)
```

Run `bash scripts/check-dev-prereqs.sh` to verify all required and optional tools are installed. Then run `just setup` before any other project command after cloning this repo. In new worktrees, initialize the submodule first with `git submodule update --init --recursive`, then run `just setup`.

Setup reference:
- Rust toolchain: `1.89` (from `rust-toolchain.toml`)
- Bun: `1.3.9+`
- Optional tools (`docker`, `jq`, `shellcheck`, `actionlint`, `cfn-lint`) are required only for specific flows (`just dev`, `just lint`, and CloudFormation hook checks).
- Rerun `just setup` for new clones/worktrees, if hooks are reset, or if dependency bootstrap looks stale.

## Git Hooks

This repo uses git hooks (wired via `git config core.hooksPath .githooks`, set up by `just setup`):

- **pre-commit** — Syncs README to npm packages, guards against unpublished submodule pins, runs cfn-lint on CloudFormation changes.
- **pre-push** — Validates the opencode submodule commit is published.
- **post-merge** — After every `git pull`, automatically syncs the submodule and runs `bun install`. No manual action needed.

## README Badge Sync

`README.md` (superproject) and `packages/opencode/README.md` (submodule) use distinct generated badge blocks for different audiences.
The source of truth is `packages/opencode/packages/fork-ui/src/readme-badge-catalog.ts`.

- Run `just sync-readme-badges` after badge catalog changes.
- `just lint` and `just pre-commit` run `just check-readme-badges` and fail on drift.
- Do not manually edit badge lines between generated marker comments.

## UAT Testing

When performing manual UAT tests with the user, use justfile commands instead of the installed `occ` binary:

- Use `just run mount add /path:/container` instead of `occ mount add /path:/container`
- Use `just run status` instead of `occ status`
- Use `just dev` instead of `occ start`

This ensures tests run against the locally-built development version.

## Architecture Notes

- npm package uses compile-on-install (no prebuilt binaries)
- Users need Rust 1.89+ installed for npm install
- Config stored at `~/.config/opencode-cloud/config.json`
- Data stored at `~/.local/share/opencode-cloud/`

## Version and Metadata Sync

**Important:** `packages/core/Cargo.toml` must use explicit values (not `workspace = true`) because it's published to npm where users install it standalone without the workspace root.

When updating versions or metadata, keep these files in sync:
- `Cargo.toml` (workspace root) - `[workspace.package]` section
- `packages/core/Cargo.toml` - explicit values for version, edition, rust-version, license, repository, homepage, documentation, keywords, categories

The `scripts/set-all-versions.sh` script handles version updates automatically. For other metadata changes, update both files manually.

## Git Workflow

- Always use rebase pulls (e.g., `git pull --rebase`).
- For pushes in the `packages/opencode/` submodule (our own fork), default to the `dev` flow unless explicitly requested otherwise:
  - Rebase pull from `dev` first (for example: `git pull --rebase origin dev`).
  - Then push to the effective `dev` branch/tip for that repository (for example: `git push origin HEAD:dev`).
- For pushes in the `opencode-cloud` superproject, default to `main` unless explicitly requested otherwise:
  - Rebase pull from `main` first (for example: `git pull --rebase origin main`).
  - Then push to `main`.
- After committing and pushing changes in the `packages/opencode/` submodule, run `just update-opencode-commit` in the superproject to update the Dockerfile's `OPENCODE_COMMIT` pin. Commit and push the resulting Dockerfile change to `main`.
- After pulling the superproject, the `post-merge` hook automatically syncs the submodule and runs `bun install`. Then run `just update-opencode-commit` — if the Dockerfile changed, commit and push the update.
- **Resolving `OPENCODE_COMMIT` merge conflicts:** If a rebase or merge produces a conflict in the Dockerfile's `OPENCODE_COMMIT` line, do not manually pick a SHA. Instead, accept either side to clear the conflict markers, then run `just update-opencode-commit` to set the correct value from the current submodule state.

## Working with Git Worktrees and Submodules

Git worktrees and submodules interact poorly. Submodule metadata is partially shared across worktrees, so careless commands in one worktree can break others.

### After `git worktree add`, init the submodule

Submodules are not automatically initialized per worktree. After every `git worktree add`, run this **in the new worktree**:

```bash
git submodule update --init --recursive
```

### Never run `git submodule sync` in worktrees

`git submodule sync` rewrites `submodule.<name>.url` in the **shared** `.git/config`, affecting all worktrees. It also mutates `core.worktree` paths in shared submodule metadata. Only run `sync` if the submodule remote URL has actually changed, and only from the main worktree with no other worktrees active.

### Never run submodule commands concurrently across worktrees

If two worktree sessions run `git submodule update` at the same time, they race on shared metadata under `.git/modules/` and `.git/config`. This causes intermittent "cannot be used without a working tree" errors and stale `core.worktree` paths. Serialize submodule operations across worktrees.

### Submodules must stay detached

The submodule should always be detached at the superproject-pinned commit. If `git submodule status` shows a branch name instead of a detached SHA, something has gone wrong (likely a `sync` or manual `checkout` inside the submodule). Fix with:

```bash
git submodule update --recursive
```

### Fixing stale `core.worktree` errors

If you see errors like "cannot be used without a working tree" or submodule commands fail mysteriously, the submodule's `core.worktree` config is pointing to a wrong or deleted worktree path. Fix from the affected worktree:

```bash
git submodule deinit -f packages/opencode
git submodule update --init --recursive
```

### Checking submodule health before commits

```bash
git submodule status --recursive
git diff --submodule=log
```

### Removing worktrees

Worktrees with initialized submodules may resist normal removal. Use `git worktree remove --force <path>` followed by `git worktree prune`.

### Do / Don't

- Do run `git submodule update --init --recursive` in every new worktree.
- Do keep submodules detached and pinned unless intentionally updating the pointer.
- Do check `git submodule status` before committing.
- Do serialize submodule operations — never run them concurrently across worktrees.
- Don't run `git submodule sync` unless the remote URL has changed.
- Don't run `git checkout <branch>` inside the submodule working tree.
- Don't assume submodules are ready in a fresh worktree.
- Don't ignore failed worktree removal; clean up stale metadata promptly.
