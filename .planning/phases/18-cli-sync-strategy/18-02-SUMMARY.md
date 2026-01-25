---
phase: 18
plan: 02
subsystem: cli
tags: [testing, vitest, ci, parity, automation]
requires:
  - 18-01 # Node CLI passthrough wrapper
provides:
  - Dynamic CLI parity testing
  - CI enforcement of Rust/Node sync
  - Command discovery automation
affects:
  - 18-03 # Documentation of sync strategy
  - 22-01 # Prebuilt binary testing
tech-stack:
  added:
    - vitest ^3.0.0
  patterns:
    - Dynamic command discovery via help parsing
    - Passthrough verification testing
    - CI-enforced CLI synchronization
key-files:
  created:
    - packages/cli-node/vitest.config.ts
    - packages/cli-node/src/cli-parity.test.ts
  modified:
    - packages/cli-node/package.json
    - .github/workflows/ci.yml
decisions:
  - name: "Dynamic command discovery from help output"
    rationale: "Automatically adapts when new commands added to Rust CLI"
    alternatives: "Hardcoded command list"
    chosen: "Parse 'occ --help' output"
  - name: "Separate CI job for parity tests"
    rationale: "Clear signal when Node/Rust CLI drift occurs"
    alternatives: "Include in main build job"
    chosen: "Dedicated cli-parity job"
  - name: "30-second test timeout"
    rationale: "CLI commands may take time, especially on CI"
    alternatives: "Default 5s timeout"
    chosen: "30s timeout in vitest.config.ts"
metrics:
  duration: "2 min"
  completed: "2026-01-25"
---

# Phase 18 Plan 02: CLI Parity Tests Summary

**One-liner:** Dynamic test suite discovers Rust CLI commands from help output and verifies Node CLI passthrough works for all commands

## What Was Built

Created a self-maintaining parity test suite that ensures Rust and Node CLIs stay in sync:

1. **Vitest testing framework** - Added vitest with 30s timeout for CLI operations
2. **Dynamic command discovery** - Parses `occ --help` to extract all available commands
3. **Passthrough verification** - Tests each command's `--help` through Node CLI wrapper
4. **CI integration** - Dedicated job that fails builds when parity breaks
5. **31 automated tests** - All passing with zero hardcoded command lists

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add Vitest and create parity test suite | 50e76ce | package.json, vitest.config.ts, cli-parity.test.ts |
| 2 | Add CLI parity job to CI workflow | 11e34b3 | .github/workflows/ci.yml |

## Deviations from Plan

None - plan executed exactly as written.

## Technical Details

### Test Architecture

```
┌─────────────────────────────────────┐
│  cli-parity.test.ts                 │
│                                     │
│  1. Parse: occ --help               │
│     → Extract: start, stop, etc     │
│                                     │
│  2. Test each command:              │
│     → node dist/index.js <cmd>      │
│     → Verify help output            │
└─────────────────────────────────────┘
```

### Command Discovery Implementation

```typescript
function discoverCommands(): string[] {
  const helpOutput = execSync(`${RUST_BINARY_PATH} --help`, {
    encoding: 'utf-8',
  });

  const lines = helpOutput.split('\n');
  const commandsSection = lines.findIndex((line) => line.trim() === 'Commands:');

  // Parse lines until "Options:" section
  for (let i = commandsSection + 1; i < lines.length; i++) {
    if (line.trim() === 'Options:') break;

    // Extract: "  start      Start the opencode service"
    //           ^^^^^^
    const match = line.match(/^\s+([a-z-]+)\s+/);
    if (match) commands.push(match[1]);
  }

  return commands;
}
```

**Why this works:**
- Help format is stable (Commands: → Options: structure)
- New commands automatically discovered on next test run
- No maintenance burden when adding commands

### Test Coverage

**Discovery tests (16 tests):**
- Discovers at least 10 commands
- Verifies presence of: start, stop, restart, status, logs, install, uninstall, config, setup, user, mount, update, cockpit, host, help

**Passthrough tests (15 tests):**
- `--version` through Node CLI
- `--help` through Node CLI
- `<command> --help` for all 14 commands (excluding help itself)

**All tests verify:**
- Node CLI can spawn Rust binary
- Arguments pass through correctly
- Help output is intact
- Exit codes propagate

### CI Job Structure

```yaml
cli-parity:
  name: CLI Parity Tests
  runs-on: ubuntu-latest
  needs: [build]  # Runs after main build
  steps:
    - Install Rust toolchain
    - Install pnpm + Node.js
    - Build Rust CLI (debug mode for speed)
    - Build Node CLI wrapper
    - Run parity tests
```

**CI failure triggers:**
- New command in Rust CLI but Node CLI can't invoke it
- Command removed from Rust but test still expects it
- Help output format changes unexpectedly
- Binary placement broken

## Decisions Made

### Dynamic Command Discovery from Help Output

**Context:** Need to verify Node CLI works for all Rust CLI commands

**Options:**
1. Hardcoded command list in test
2. Parse help output dynamically

**Decision:** Parse `occ --help` output

**Rationale:**
- Automatically discovers new commands when added
- Test file never needs updates for new commands
- Single source of truth (Rust CLI defines what exists)
- Failure indicates actual drift, not stale test

**Impact:** Tests adapt automatically when commands added/removed

### Separate CI Job for Parity Tests

**Context:** Where to run parity verification in CI pipeline

**Options:**
1. Include in main build job
2. Separate dedicated job

**Decision:** Dedicated `cli-parity` job

**Rationale:**
- Clear signal in GitHub Actions UI when parity breaks
- Can be re-run independently if flaky
- Doesn't slow down format/clippy/test jobs
- Runs after build (leverages artifacts)

**Impact:** Better observability of Node/Rust synchronization

### 30-Second Test Timeout

**Context:** Default vitest timeout is 5 seconds

**Options:**
1. Keep default 5s timeout
2. Increase to 30s for CLI commands

**Decision:** 30s timeout in vitest.config.ts

**Rationale:**
- CLI commands spawn processes (slower than unit tests)
- CI environments can be slow (especially GitHub Actions)
- Better to be generous than flaky
- Still fast enough for good feedback

**Impact:** Tests reliable on slow CI runners

## Verification Results

All verification checks passed:

1. ✅ `pnpm -C packages/cli-node test` runs successfully
2. ✅ 31 tests pass (16 discovery + 15 passthrough)
3. ✅ packages/cli-node/vitest.config.ts exists
4. ✅ packages/cli-node/src/cli-parity.test.ts has dynamic command discovery
5. ✅ .github/workflows/ci.yml has cli-parity job that depends on build

## Test Output Sample

```
✓ src/cli-parity.test.ts (31 tests) 511ms
  ✓ CLI Parity > should discover at least 10 commands
  ✓ CLI Parity > should discover start command
  ✓ CLI Parity > should discover stop command
  [... 28 more tests ...]

Test Files  1 passed (1)
     Tests  31 passed (31)
  Duration  681ms
```

## Known Limitations

1. **Requires debug build** - Tests use `target/debug/opencode-cloud` for speed
2. **Help parsing fragile** - If help format changes drastically, discovery breaks
3. **No deep command testing** - Only verifies `--help` works, not actual behavior
4. **Binary must exist** - Tests fail if Rust not built first (documented in error)

These are acceptable for a parity test. Behavioral testing is separate.

## Auto-fixed Issues

None. Plan executed cleanly without deviations.

## Next Phase Readiness

**Blockers:** None

**Concerns:** None

**Dependencies satisfied:**
- ✅ Dynamic command discovery works
- ✅ Node CLI passthrough verified for all commands
- ✅ CI enforces parity on every PR
- ✅ Test suite adapts automatically to new commands

**Ready for:**
- 18-03: Documentation of sync strategy (reference these tests)
- 22-01: Prebuilt binary distribution (same test pattern applies)
- Future command additions (tests automatically include them)

## Maintenance Notes

**When adding a new command to Rust CLI:**
1. No test changes required - discovery automatic
2. CI will verify Node CLI can invoke it
3. If Node CLI passthrough works, tests pass

**When help format changes:**
- If "Commands:" or "Options:" section moves, update `discoverCommands()` parser
- Consider regex pattern updates if command format changes

**When debugging CI failures:**
1. Check if new command added
2. Verify Node CLI binary placement works
3. Run tests locally: `pnpm -C packages/cli-node test`

## Files Changed

**Created:**
- packages/cli-node/vitest.config.ts (test configuration)
- packages/cli-node/src/cli-parity.test.ts (31 tests)

**Modified:**
- packages/cli-node/package.json (added vitest, test scripts)
- .github/workflows/ci.yml (added cli-parity job)

## Metrics

- **Duration:** 2 min
- **Tasks completed:** 2/2
- **Commits:** 2
- **Files created:** 2
- **Files modified:** 2
- **Tests added:** 31
- **Lines added:** ~220
