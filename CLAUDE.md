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

Git worktrees and submodules work together, but support is incomplete and requires care.

Submodules are not automatically initialized per worktree. After every `git worktree add`, run this in the new worktree:

```bash
git submodule sync --recursive
git submodule update --init --recursive
git submodule status --recursive
```

Submodule metadata under `.git/modules/...` (via the repo's common git dir) is shared across worktrees. Changing a submodule branch or commit in one worktree can affect other worktrees.

When multiple worktrees exist, treat submodules as read-only and detached at the superproject-pinned commit unless you explicitly intend to update the submodule pointer and understand the impact.

Check for drift or dirty submodule state before commits and when switching worktrees:

```bash
git submodule status --recursive
git submodule foreach --recursive 'git status --short --branch'
git diff --submodule=log
```

Removing a worktree that contains initialized submodules can require force or manual cleanup. If normal removal fails, use `git worktree remove --force <path>` and then `git worktree prune`.

### Do / Don't

- Do run `git submodule update --init --recursive` in every new worktree.
- Do keep submodules detached and pinned unless you are intentionally updating them.
- Do check `git submodule status` and submodule dirtiness before committing.
- Don't assume submodules are ready in a fresh worktree.
- Don't switch submodule branches casually while multiple worktrees are active.
- Don't ignore failed worktree removal; clean up stale metadata promptly.
