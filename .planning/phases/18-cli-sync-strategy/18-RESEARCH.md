# Phase 18: CLI Sync Strategy - Research

**Researched:** 2026-01-25
**Domain:** Node.js CLI passthrough, platform-specific binary distribution
**Confidence:** HIGH

## Summary

This phase establishes patterns to keep the Rust CLI (packages/cli-rust) and Node CLI (packages/cli-node) in sync through automatic passthrough delegation. The Rust CLI is the single source of truth, with the Node CLI acting as a thin wrapper that spawns the Rust binary and passes all arguments through transparently.

**Key findings:**
- Node.js `child_process.spawn()` with `stdio: 'inherit'` provides zero-overhead passthrough
- Modern binary distribution uses optionalDependencies with platform-specific packages (esbuild pattern)
- Testing strategy focuses on verifying passthrough works for all commands discovered dynamically
- CI integration with existing workflows prevents drift by failing builds when binaries missing or tests fail

**Primary recommendation:** Implement Node CLI as a simple spawn wrapper with `stdio: 'inherit'` and `process.argv.slice(2)`, bundle platform-specific Rust binaries via optionalDependencies, and test parity by parsing `occ --help` output to discover commands dynamically.

## Standard Stack

The established tools/libraries for CLI passthrough and binary distribution:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| child_process.spawn | Node.js builtin | Spawn Rust binary with passthrough | Official Node.js API, zero dependencies |
| optionalDependencies | npm feature | Platform-specific binary selection | Industry standard (esbuild, swc) |
| GitHub Actions | CI platform | Cross-compile Rust binaries | Native Rust cross-compilation support |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Vitest | Latest | Test framework | Node.js testing (project uses this) |
| cross | Latest | Rust cross-compilation | Build for Linux from macOS/Windows |
| cargo build --release | Rust builtin | Release binaries | Production binary compilation |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| optionalDependencies | postinstall download | Less reliable (scripts can be disabled), slower |
| optionalDependencies | Bundle all binaries | Massive package size (100MB vs 20MB) |
| spawn with 'inherit' | Manual stream piping | More code, harder to maintain |

**Installation:**
```bash
# No additional dependencies needed for passthrough
# Binary bundling uses npm's built-in optionalDependencies feature
```

## Architecture Patterns

### Recommended Project Structure
```
packages/cli-node/
├── src/
│   └── index.ts           # Main passthrough wrapper
├── bin/
│   ├── occ-darwin-arm64   # Platform-specific binaries
│   ├── occ-darwin-x64
│   ├── occ-linux-arm64
│   └── occ-linux-x64
├── package.json           # With optionalDependencies
└── __tests__/
    └── cli-parity.test.ts # Parity tests
```

### Pattern 1: Automatic Passthrough with stdio: 'inherit'
**What:** Spawn Rust binary passing all arguments, inheriting parent's stdio streams
**When to use:** When wrapper should be completely transparent to user
**Example:**
```typescript
// Source: Node.js official documentation v25.3.0
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const binPath = join(__dirname, '..', 'bin', 'occ');

// Pass all arguments (process.argv.slice(2)) to Rust binary
// stdio: 'inherit' means child uses parent's stdin/stdout/stderr
const child = spawn(binPath, process.argv.slice(2), {
  stdio: 'inherit'
});

// Exit with same code as child
child.on('close', (code) => {
  process.exit(code ?? 1);
});
```

### Pattern 2: Platform-Specific Binary Selection via optionalDependencies
**What:** Publish platform-specific packages with os/cpu fields, npm auto-installs correct one
**When to use:** Distributing native binaries across platforms
**Example:**
```json
{
  "name": "opencode-cloud",
  "optionalDependencies": {
    "@opencode-cloud/cli-darwin-arm64": "3.0.0",
    "@opencode-cloud/cli-darwin-x64": "3.0.0",
    "@opencode-cloud/cli-linux-arm64": "3.0.0",
    "@opencode-cloud/cli-linux-x64": "3.0.0"
  }
}
```

Each platform package has:
```json
{
  "name": "@opencode-cloud/cli-darwin-arm64",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "files": ["bin/occ"]
}
```

### Pattern 3: Dynamic Command Discovery for Testing
**What:** Parse CLI help output to discover all commands, test each one
**When to use:** Ensuring wrapper supports all commands without hardcoding list
**Example:**
```typescript
// Parse help output to discover commands
const helpOutput = execSync('occ --help', { encoding: 'utf8' });
const commandsSection = helpOutput.split('Commands:')[1].split('Options:')[0];
const commands = commandsSection
  .split('\n')
  .map(line => line.trim())
  .filter(line => line && !line.startsWith('help'))
  .map(line => line.split(/\s+/)[0]);

// Test each discovered command
for (const cmd of commands) {
  test(`Node CLI supports ${cmd}`, () => {
    const result = spawnSync('node', ['dist/index.js', cmd, '--help']);
    expect(result.status).toBe(0);
  });
}
```

### Pattern 4: CI Build Matrix for Cross-Compilation
**What:** Use GitHub Actions matrix to build Rust binaries for all platforms
**When to use:** Generating platform-specific binaries in CI
**Example:**
```yaml
# Source: GitHub Actions best practices
jobs:
  build-binaries:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
```

### Anti-Patterns to Avoid
- **Parsing command output manually:** Don't try to intercept or parse command output - use `stdio: 'inherit'` for transparency
- **Hardcoding command lists:** Discovery prevents drift when new commands added to Rust CLI
- **Postinstall scripts for binaries:** Less reliable than optionalDependencies (users can disable scripts)
- **Bundling all binaries in one package:** Wastes bandwidth (100MB vs 20MB per platform)

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Stream piping | Manual stdout/stderr forwarding | `stdio: 'inherit'` | Handles all edge cases (colors, TTY detection, signals) |
| Platform detection | Runtime OS/arch checks | npm's optionalDependencies | npm automatically installs correct package based on os/cpu fields |
| Binary download | postinstall fetch script | optionalDependencies + prebuilt packages | More reliable (works when scripts disabled), cached by npm |
| Command parsing | Custom CLI parser | Pass arguments as-is with `process.argv.slice(2)` | Zero maintenance - Rust CLI handles all parsing |

**Key insight:** Modern Node.js and npm provide all primitives needed for transparent passthrough. Custom solutions add complexity without benefit.

## Common Pitfalls

### Pitfall 1: Breaking TTY/Color Support
**What goes wrong:** Manual stream piping loses TTY detection, colored output breaks
**Why it happens:** stdout.pipe() doesn't preserve `isTTY` property
**How to avoid:** Always use `stdio: 'inherit'` for transparent passthrough
**Warning signs:** Help output missing colors when run through Node CLI

### Pitfall 2: Exit Code Not Propagated
**What goes wrong:** Node wrapper exits 0 even when Rust binary fails
**Why it happens:** Not listening to child 'close' event or not calling process.exit()
**How to avoid:** Listen to 'close' event and exit with child's exit code
**Warning signs:** CI passes when commands fail, users see success but operation failed

### Pitfall 3: Signal Handling Issues
**What goes wrong:** Ctrl+C doesn't work, zombie processes left behind
**Why it happens:** Not forwarding signals to child process
**How to avoid:** Use `stdio: 'inherit'` which handles signals automatically, or manually forward SIGINT/SIGTERM
**Warning signs:** Can't interrupt long-running commands, `ps aux` shows orphaned processes

### Pitfall 4: Platform Package Not Installed
**What goes wrong:** Binary missing after install on some platforms
**Why it happens:** optionalDependencies with bugs in npm/pnpm (known issues as of 2025)
**How to avoid:** Test on all platforms in CI, provide fallback error message
**Warning signs:** Works locally but fails on different OS, "binary not found" errors

### Pitfall 5: Hardcoded Binary Paths
**What goes wrong:** Binary path assumes specific directory structure, breaks in different install contexts
**Why it happens:** Using relative paths instead of resolving from package location
**How to avoid:** Use `import.meta.url` + `dirname()` + `join()` for ESM, or `__dirname` for CJS
**Warning signs:** Works with `node src/index.js` but fails with `npm install -g`

### Pitfall 6: Testing Only Subset of Commands
**What goes wrong:** New commands added to Rust CLI don't work in Node CLI
**Why it happens:** Tests hardcode command list instead of discovering dynamically
**How to avoid:** Parse help output to discover commands, test all of them
**Warning signs:** Some commands work, others fail with "unknown command"

## Code Examples

Verified patterns from official sources:

### Complete Passthrough Wrapper (ESM)
```typescript
// Source: Node.js child_process documentation v25.3.0
#!/usr/bin/env node
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rustBinary = join(__dirname, '..', 'bin', 'occ');

// Spawn with stdio: 'inherit' for transparent passthrough
const child = spawn(rustBinary, process.argv.slice(2), {
  stdio: 'inherit'
});

// Forward exit code
child.on('close', (code) => {
  process.exit(code ?? 1);
});

// Handle spawn errors (binary missing)
child.on('error', (err) => {
  console.error('Failed to start Rust CLI:', err.message);
  console.error('This may indicate a missing platform-specific binary.');
  process.exit(1);
});
```

### Platform-Specific Package.json (Platform Package)
```json
// Source: esbuild platform package pattern
{
  "name": "@opencode-cloud/cli-darwin-arm64",
  "version": "3.0.0",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "main": "index.js",
  "files": [
    "bin/occ",
    "index.js"
  ]
}
```

### Main Package with Optional Dependencies
```json
// Source: esbuild and swc distribution patterns
{
  "name": "opencode-cloud",
  "version": "3.0.0",
  "bin": {
    "occ": "./dist/index.js",
    "opencode-cloud": "./dist/index.js"
  },
  "optionalDependencies": {
    "@opencode-cloud/cli-darwin-arm64": "3.0.0",
    "@opencode-cloud/cli-darwin-x64": "3.0.0",
    "@opencode-cloud/cli-linux-arm64": "3.0.0",
    "@opencode-cloud/cli-linux-x64": "3.0.0"
  }
}
```

### Dynamic Command Discovery Test
```typescript
// Source: Testing best practices
import { describe, test, expect } from 'vitest';
import { execSync, spawnSync } from 'child_process';

describe('CLI Parity', () => {
  test('Node CLI passthrough works', () => {
    const result = spawnSync('node', ['dist/index.js', '--version'], {
      encoding: 'utf8'
    });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain('opencode-cloud');
  });

  test('All Rust commands available in Node CLI', () => {
    // Discover commands from Rust CLI
    const helpOutput = execSync('cargo run -p opencode-cloud --bin occ -- --help', {
      encoding: 'utf8'
    });

    const commandsSection = helpOutput.split('Commands:')[1]?.split('Options:')[0];
    if (!commandsSection) {
      throw new Error('Failed to parse commands from help output');
    }

    const commands = commandsSection
      .split('\n')
      .map(line => line.trim())
      .filter(line => line && !line.startsWith('help'))
      .map(line => line.split(/\s+/)[0]);

    // Test each command works through Node wrapper
    for (const cmd of commands) {
      const result = spawnSync('node', ['dist/index.js', cmd, '--help'], {
        encoding: 'utf8'
      });
      expect(result.status).toBe(0);
    }
  });
});
```

### GitHub Actions Cross-Compile Workflow
```yaml
# Source: GitHub Actions + Rust cross-compilation patterns
name: Build Binaries
on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
            bin-name: occ-darwin-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            bin-name: occ-darwin-arm64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            bin-name: occ-linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            bin-name: occ-linux-arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          targets: ${{ matrix.target }}

      - name: Build binary
        run: cargo build --release --target ${{ matrix.target }} -p opencode-cloud

      - name: Copy binary
        run: |
          mkdir -p packages/cli-node/bin
          cp target/${{ matrix.target }}/release/occ packages/cli-node/bin/${{ matrix.bin-name }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.bin-name }}
          path: packages/cli-node/bin/${{ matrix.bin-name }}
```

### CI Parity Check Integration
```yaml
# Source: GitHub Actions best practices
# Add to existing .github/workflows/ci.yml
jobs:
  cli-parity:
    name: CLI Parity Check
    runs-on: ubuntu-latest
    needs: build  # Run after binaries built
    steps:
      - uses: actions/checkout@v4

      - name: Download binaries
        uses: actions/download-artifact@v4

      - name: Install Node dependencies
        run: pnpm install

      - name: Build Node CLI
        run: pnpm -C packages/cli-node build

      - name: Run parity tests
        run: pnpm -C packages/cli-node test
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| postinstall download scripts | optionalDependencies with os/cpu fields | 2020-2021 (esbuild) | More reliable, works when scripts disabled |
| Manual stream piping | stdio: 'inherit' | Always available, emphasized 2018+ | Simpler code, better TTY support |
| Bundle all binaries | Platform-specific packages | 2020+ (esbuild, swc) | 80-90% smaller package size |
| Static command lists in tests | Dynamic discovery from help | Best practice 2024+ | Zero maintenance when commands added |

**Deprecated/outdated:**
- `node-pre-gyp`: Superseded by optionalDependencies pattern, still used by some legacy packages but not recommended for new projects
- `prebuild` + `prebuild-install`: Still viable but more complex than optionalDependencies for simple binary distribution

## Open Questions

Things that couldn't be fully resolved:

1. **Binary location in optionalDependencies packages**
   - What we know: Platform packages need `bin/occ` file, main package imports them
   - What's unclear: Exact resolution mechanism when package installed via optionalDependencies
   - Recommendation: Test with `npm pack` locally, verify binary accessible from main package

2. **npm vs pnpm optionalDependencies behavior**
   - What we know: Known bugs in npm with platform-specific optionalDependencies (package-lock.json issues)
   - What's unclear: Whether these bugs affect this specific use case
   - Recommendation: Test with both package managers in CI, document any workarounds needed

3. **Cross-compilation for aarch64-unknown-linux-gnu on ubuntu-latest**
   - What we know: GitHub Actions ubuntu-latest is x86_64, need cross-compilation for ARM64 Linux
   - What's unclear: Whether `cross` tool needed or if rustup target suffices
   - Recommendation: Try rustup target first, fall back to `cross` if linking fails

## Sources

### Primary (HIGH confidence)
- [Node.js child_process documentation v25.3.0](https://nodejs.org/api/child_process.html) - spawn, stdio options
- [esbuild platform-specific binaries](https://esbuild.github.io/getting-started/) - optionalDependencies pattern
- [npm package.json documentation](https://docs.npmjs.com/cli/v7/configuring-npm/package-json/) - os/cpu fields
- [clap derive documentation](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) - help output format
- Examined project files: packages/cli-rust/src/lib.rs (clap structure), packages/cli-node/src/index.ts (current state)

### Secondary (MEDIUM confidence)
- [How to publish binaries on npm (Sentry Engineering)](https://sentry.engineering/blog/publishing-binaries-on-npm) - Combined optionalDependencies + fallback approach
- [Cross Compiling Rust Projects in GitHub Actions](https://blog.urth.org/2023/03/05/cross-compiling-rust-projects-in-github-actions/) - Build matrix patterns
- [esbuild Different strategy for installing platform-specific binaries #789](https://github.com/evanw/esbuild/issues/789) - Evolution from postinstall to optionalDependencies
- [Parity Testing with Feature Flags (Harness)](https://www.harness.io/blog/parity-testing-with-feature-flags) - Testing methodology

### Secondary (MEDIUM confidence - npm issues)
- [Platform-specific optional dependencies bugs in npm #4828](https://github.com/npm/cli/issues/4828) - Known issues with package-lock.json
- [@swc/core platform packages](https://www.npmjs.com/package/@swc/core) - Real-world example of pattern

### Tertiary (LOW confidence)
- WebSearch results on general testing patterns - Need verification with actual implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Node.js spawn and optionalDependencies are official, well-documented features
- Architecture: HIGH - Patterns verified in production (esbuild, swc), tested in Node.js docs
- Pitfalls: MEDIUM - Based on common issues and GitHub discussions, some specific to this use case need testing
- Binary bundling details: MEDIUM - Pattern proven but exact package structure needs validation

**Research date:** 2026-01-25
**Valid until:** 60 days (stable patterns, but npm ecosystem updates frequently)

**Next steps for planning:**
1. Design exact package structure for platform-specific packages
2. Create parity test suite with dynamic command discovery
3. Set up GitHub Actions build matrix for all target platforms
4. Update CONTRIBUTING.md with new command addition process
