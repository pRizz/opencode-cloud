# Rust Modularization Plan (Top 3 Largest Files)

This document proposes a safe, incremental modularization plan for the three largest Rust source files in this repository.

## Scope and line counts

Measured with:

```bash
rg --files -g '*.rs' | xargs wc -l | sort -nr | head
```

Largest files:

1. `packages/cli-rust/src/commands/update.rs` (~2162 lines)
2. `packages/core/src/docker/image.rs` (~1845 lines)
3. `packages/cli-rust/src/commands/start.rs` (~1791 lines)

---

## 1) `packages/cli-rust/src/commands/update.rs`

### Problem shape

`update.rs` currently mixes:

- argument orchestration and command flow
- image/version decision logic
- container update/rollback logic
- runtime status and messaging
- helper structs/utilities and tests

This makes review/debugging harder and increases coupling between unrelated concerns.

### Proposed module split

Keep `update.rs` as the command entry/root and move logic into:

- `commands/update/args.rs`
  - CLI option parsing helpers and argument validation
- `commands/update/image_resolution.rs`
  - image/tag selection, version candidate resolution, compatibility checks
- `commands/update/container_update.rs`
  - update execution flow, candidate container creation, handoff/rollback operations
- `commands/update/rollback.rs`
  - rollback-specific pathways and error recovery policy
- `commands/update/output.rs`
  - user-facing progress messages + summary formatting
- `commands/update/tests/` (or inline `#[cfg(test)]` submodules)
  - focused test files by domain (resolution/update/rollback)

### Migration order

1. Extract pure helpers first (`image_resolution`), no behavior changes.
2. Extract container update/rollback paths.
3. Extract output formatting and final orchestration cleanup.
4. Split tests last to preserve confidence throughout.

---

## 2) `packages/core/src/docker/image.rs`

### Problem shape

`image.rs` currently combines:

- pull/build/tag primitives
- local cache/discovery logic
- provenance/metadata handling
- progress and stream parsing
- fallback behavior and registry differences

This inflates cognitive load and makes subtle behavior regressions easier to introduce.

### Proposed module split

Keep `image.rs` as public façade/re-export module and split internals into:

- `docker/image/pull.rs`
  - pull flows, pull-source fallbacks, post-pull retag policy
- `docker/image/build.rs`
  - build flows, build args, build output handling
- `docker/image/tag.rs`
  - retagging, canonical naming, migration helpers
- `docker/image/inspect.rs`
  - image existence/inspection/local lookup helpers
- `docker/image/provenance.rs`
  - provenance labels and extraction utilities
- `docker/image/progress.rs`
  - stream/progress translation utilities (if not already covered in `docker/progress.rs`)

### Migration order

1. Move inspect/tag helpers (lowest risk, often pure).
2. Move pull path with comprehensive tests.
3. Move build/provenance paths.
4. Keep `image.rs` as thin API surface delegating to submodules.

---

## 3) `packages/cli-rust/src/commands/start.rs`

### Problem shape

`start.rs` currently blends:

- preflight checks and environment detection
- image selection/pull/build decisions
- mount and runtime composition
- prompt/interaction UX branches
- startup orchestration and post-start reporting

Large control-flow depth makes changes risky and slow to reason about.

### Proposed module split

Keep `start.rs` as orchestration root and split into:

- `commands/start/preflight.rs`
  - host checks, dependency checks, config validation
- `commands/start/image_strategy.rs`
  - local-vs-remote image decision logic, fallback rules
- `commands/start/mounts.rs`
  - mount derivation, validation, normalization
- `commands/start/interaction.rs`
  - prompts/confirmations and non-interactive defaults
- `commands/start/runtime.rs`
  - container launch/runtime start sequence
- `commands/start/output.rs`
  - user messaging/status summary

### Migration order

1. Extract preflight + mount normalization.
2. Extract image strategy.
3. Extract interaction prompts.
4. Extract runtime launch flow and reduce `start.rs` to top-level pipeline.

---

## Cross-cutting standards for all three refactors

- Keep behavior stable: **refactor-only** PRs per module extraction.
- Prefer early returns/guard clauses to reduce nesting.
- Avoid `unwrap()`; propagate errors with context.
- Keep each commit small and reversible.
- Maintain tests for each extracted concern before moving to next.

## Suggested PR sequence

1. `update.rs` extraction (resolution + rollback first)
2. `image.rs` extraction (inspect/tag first)
3. `start.rs` extraction (preflight/image strategy first)

This order starts with highest line-count and highest complexity hotspots while keeping runtime risk manageable.
