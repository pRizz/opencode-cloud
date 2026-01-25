# Phase 16: Code Quality Audit - Research

**Researched:** 2026-01-24
**Domain:** Rust code quality, refactoring patterns, nesting analysis
**Confidence:** HIGH

## Summary

The codebase is in good health - clippy passes with zero warnings and all 147 core library tests pass. However, there are concrete opportunities for improvement:

1. **Duplicated code patterns**: `format_docker_error()` is implemented 5 times across different command files with slight variations
2. **Similar URL display logic**: Remote host URL resolution and Cockpit URL display code is duplicated in `start.rs` and `status.rs`
3. **Large files**: `start.rs` (942 lines) could benefit from extraction into smaller modules
4. **Spinner creation patterns**: `ProgressBar::new_spinner()` setup is repeated with similar configuration

**Primary recommendation:** Extract common patterns into shared helper modules rather than reducing nesting (nesting levels are generally acceptable).

## Standard Stack

### Core (Already in Use)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `clippy` | built-in | Static analysis | Official Rust linting tool |
| `rustfmt` | built-in | Code formatting | Official formatter |

### Supporting Tools
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `cargo clippy -- -W clippy::pedantic` | Stricter lints | Optional deeper analysis |
| `cargo tree --duplicates` | Dependency audit | Check for duplicate deps |

## Architecture Patterns

### Current Project Structure (Unchanged)
```
packages/
├── core/              # Library with Docker/platform logic
│   └── src/
│       ├── config/    # Configuration management
│       ├── docker/    # Docker operations
│       ├── host/      # Remote host management
│       └── platform/  # Service managers (launchd, systemd)
└── cli-rust/          # CLI application
    └── src/
        ├── commands/  # Command implementations
        ├── output/    # Terminal output utilities
        └── wizard/    # Interactive setup
```

### Recommended Extraction Pattern
When extracting duplicated code:

1. **Identify the duplication** - Find 3+ instances of similar code
2. **Abstract the common parts** - Create a function/trait in the appropriate module
3. **Parameterize differences** - Use function parameters or trait implementations
4. **Update call sites** - Replace duplicated code with calls to shared function
5. **Verify tests still pass** - Run `just test` after each extraction

### Anti-Patterns to Avoid
- **Over-extraction**: Don't create abstractions for code used only 1-2 times
- **God modules**: Don't create a single "utils" module; keep helpers near their domain

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Linting | Custom lint rules | `cargo clippy` | Comprehensive, maintained |
| Formatting | Manual formatting | `cargo fmt` | Consistent, zero-config |
| Complexity metrics | LOC counting | `cargo clippy` warnings | Catches real issues |

## Identified Issues

### Issue 1: Duplicated `format_docker_error()` Functions
**What:** Five implementations of Docker error formatting exist
**Files affected:**
- `packages/cli-rust/src/commands/start.rs:722` (returns String)
- `packages/cli-rust/src/commands/status.rs:422` (returns anyhow::Error)
- `packages/cli-rust/src/commands/stop.rs:77` (returns String)
- `packages/cli-rust/src/commands/restart.rs:120` (returns String)
- `packages/cli-rust/src/commands/logs.rs:174` (returns anyhow::Error)

**Recommendation:** Extract to `packages/cli-rust/src/output/errors.rs`:
```rust
pub fn format_docker_error(e: &DockerError) -> String {
    match e {
        DockerError::NotRunning => // ...
        DockerError::PermissionDenied => // ...
        DockerError::Connection(msg) => // ...
        _ => e.to_string(),
    }
}

pub fn docker_error_to_anyhow(e: &DockerError) -> anyhow::Error {
    anyhow!(format_docker_error(e))
}
```

### Issue 2: Duplicated Remote URL Resolution
**What:** `maybe_remote_addr` resolution pattern appears 4+ times
**Files affected:**
- `packages/cli-rust/src/commands/start.rs:408`, `628`
- `packages/cli-rust/src/commands/status.rs:136`

**Recommendation:** Extract to `packages/cli-rust/src/lib.rs` or create a helper:
```rust
/// Resolve remote host address for display URLs
pub fn resolve_remote_addr(host_name: Option<&str>) -> Option<String> {
    host_name.and_then(|name| {
        load_hosts()
            .ok()
            .and_then(|h| h.get_host(name).map(|cfg| cfg.hostname.clone()))
    })
}
```

### Issue 3: Duplicated Cockpit URL Display
**What:** Cockpit URL formatting logic duplicated 4 times in `start.rs`
**Lines:** 432, 442, 665, 675

**Recommendation:** Extract to a helper function:
```rust
fn format_cockpit_url(
    maybe_remote_addr: Option<&str>,
    bind_addr: &str,
    cockpit_port: u16,
) -> String {
    if let Some(remote_addr) = maybe_remote_addr {
        format!("http://{}:{}", remote_addr, cockpit_port)
    } else {
        let cockpit_addr = if bind_addr == "0.0.0.0" || bind_addr == "::" {
            "127.0.0.1"
        } else {
            bind_addr
        };
        format!("http://{}:{}", cockpit_addr, cockpit_port)
    }
}
```

### Issue 4: Large Files
**What:** Some files exceed 500 lines
**Files:**
- `packages/cli-rust/src/commands/start.rs` - 942 lines
- `packages/core/src/config/schema.rs` - 673 lines (mostly tests, acceptable)

**Recommendation for start.rs:**
Consider extracting into submodules:
- `start/image.rs` - Image building/pulling logic
- `start/health.rs` - Health check and waiting logic
- `start/display.rs` - Result display functions

### Issue 5: Spinner Configuration Duplication
**What:** `ProgressBar::new_spinner()` setup repeated with same configuration
**Files:** `packages/cli-rust/src/commands/host/add.rs` (3 instances)

**Recommendation:** Already have `CommandSpinner` - ensure all commands use it consistently.

## Nesting Analysis

### Current Nesting Levels
The codebase generally has acceptable nesting (1-3 levels). Reviewed files:

| File | Max Nesting | Notes |
|------|-------------|-------|
| `start.rs` | 3 levels | `if let` chains in Cockpit URL display |
| `status.rs` | 3 levels | Acceptable match arms |
| `host/add.rs` | 3 levels | Error handling branches |
| `container.rs` | 2 levels | Clean with early returns |

**Finding:** Most code uses early returns well. The `if let Some` chains for optional values are idiomatic Rust and don't constitute problematic nesting.

### Already Using Early Returns
Examples of good patterns already in place:
- `packages/core/src/platform/launchd.rs:197` - Early return on error
- `packages/cli-rust/src/commands/start.rs:403` - Guard clause for quiet mode
- `packages/core/src/host/provision.rs:82` - Early return on empty ID

## Common Pitfalls (For This Audit)

### Pitfall 1: Over-Refactoring
**What goes wrong:** Creating abstractions that add complexity without benefit
**How to avoid:** Only extract patterns appearing 3+ times with significant code

### Pitfall 2: Breaking Tests
**What goes wrong:** Refactoring changes behavior
**How to avoid:** Run `just test` after each extraction; tests should remain unchanged

### Pitfall 3: Changing Public APIs
**What goes wrong:** Breaking downstream consumers
**How to avoid:** Keep all public interfaces stable; internal refactoring only

## Code Examples

### Pattern: Extracting Duplicated Functions
```rust
// Before: In each command file
fn format_docker_error(e: &DockerError) -> String {
    // Duplicated implementation
}

// After: In output/errors.rs
pub fn format_docker_error(e: &DockerError) -> String {
    // Single implementation
}

// Call sites
use crate::output::format_docker_error;
```

### Pattern: Using let...else for Early Returns
```rust
// Good (already present in codebase)
let Some(home) = dirs::home_dir() else {
    return Vec::new();
};

// Also good
let Ok(config) = load_config() else {
    return Err(anyhow!("Config not found"));
};
```

## Verification Checklist

Before marking each refactoring task complete:

- [ ] `just lint` passes (includes clippy)
- [ ] `just test` passes (all 147+ tests)
- [ ] `just build` succeeds
- [ ] No new warnings introduced
- [ ] Public APIs unchanged
- [ ] Code is more readable, not just different

## Priority Order

1. **High Priority** - Extract `format_docker_error()` (5 duplications)
2. **Medium Priority** - Extract remote URL resolution helper (4 duplications)
3. **Medium Priority** - Extract Cockpit URL display helper (4 duplications)
4. **Low Priority** - Consider splitting `start.rs` (only if other refactoring reveals clean boundaries)
5. **Low Priority** - Review remaining spinner creation patterns

## Open Questions

1. **Should `start.rs` be split?**
   - What we know: It's 942 lines, large but not unmanageable
   - What's unclear: Whether natural module boundaries exist
   - Recommendation: Defer to Phase 16 execution; extract duplications first, then reassess

## Sources

### Primary (HIGH confidence)
- Direct codebase analysis via grep and file reading
- `cargo clippy` output: 0 warnings
- `cargo test` output: 147 tests passing

### Methodology
- Analyzed all Rust files in `packages/core/` and `packages/cli-rust/`
- Searched for pattern duplication using regex
- Measured file lengths and identified largest files
- Reviewed nesting levels in key files

## Metadata

**Confidence breakdown:**
- Duplication findings: HIGH - Direct code search verification
- Nesting analysis: HIGH - Manual review of key files
- Extraction recommendations: MEDIUM - Based on Rust best practices

**Research date:** 2026-01-24
**Valid until:** Indefinite (internal codebase analysis)
