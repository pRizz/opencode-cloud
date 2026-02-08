# Claude Code Instructions

## Pre-Commit Requirements

Before creating any git commit, you MUST run `just pre-commit`.

Only proceed with the commit if it passes. If it fails, fix the issues first.

## Project Structure

This is a polyglot monorepo with Rust and TypeScript:

- `packages/core/` - Rust core library with NAPI-RS bindings
- `packages/cli-rust/` - Rust CLI binary
- `packages/cli-node/` - Node.js CLI wrapper
- `packages/opencode/` - Git submodule checkout of the upstream opencode repository

## Key Commands

```bash
just build       # Build all packages
just test        # Run all tests
just fmt         # Format all code
just lint        # Lint all code
just pre-commit  # Format, lint, build, and test
just clean       # Clean build artifacts
just run <args>  # Run CLI with arguments (e.g., just run status)
```

## UAT Testing

When performing manual UAT tests with the user, use justfile commands instead of the installed `occ` binary:

- Use `just run mount add /path:/container` instead of `occ mount add /path:/container`
- Use `just run status` instead of `occ status`
- Use `just run start` instead of `occ start`

This ensures tests run against the locally-built development version.

## Architecture Notes

- npm package uses compile-on-install (no prebuilt binaries)
- Users need Rust 1.82+ installed for npm install
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
- For pushes in the `packages/opencode/` submodule, default to the upstream `dev` flow unless explicitly requested otherwise:
  - Rebase pull from `dev` first (for example: `git pull --rebase origin dev`).
  - Then push to the effective `dev` branch/tip for that repository (for example: `git push origin HEAD:dev`).
- For pushes in the `opencode-cloud` superproject, default to `main` unless explicitly requested otherwise:
  - Rebase pull from `main` first (for example: `git pull --rebase origin main`).
  - Then push to `main`.

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
