---
created: 2026-01-18T16:06
title: Make README sync more robust
area: tooling
files:
  - .githooks/pre-commit
  - .github/workflows/ci.yml
---

## Problem

The pre-commit hook in `.githooks/pre-commit` syncs the root README.md to npm package directories, but:

1. **No CI validation** - If someone bypasses the hook or uses a different git client, package READMEs could drift out of sync with no automated detection.

2. **Cross-platform concerns** - The current `git config core.hooksPath` approach requires manual setup and may have edge cases on Windows (though Windows is deferred to v2).

3. **Discoverability** - Developers must remember to run `just setup` or manually configure hooks after cloning.

## Solution

Consider these improvements:

1. **Add CI check** - In `.github/workflows/ci.yml`, add a step that diffs the READMEs:
   ```yaml
   - name: Check README sync
     run: |
       diff README.md packages/core/README.md
       diff README.md packages/cli-node/README.md
   ```

2. **Evaluate husky** - For cross-platform hook management, husky auto-installs hooks via npm postinstall. Trade-off: adds a dev dependency.

3. **Alternative: lefthook** - Rust-based, faster, no Node dependency. Could be a better fit for this Rust-heavy project.

TBD: Decide priority based on when Windows support is needed.
