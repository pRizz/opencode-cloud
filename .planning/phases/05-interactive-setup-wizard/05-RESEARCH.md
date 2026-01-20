# Phase 5: Interactive Setup Wizard - Research

**Researched:** 2026-01-20
**Domain:** Interactive CLI prompts, configuration management, credential generation
**Confidence:** HIGH

## Summary

This phase implements a setup wizard for first-time configuration and extends the existing `occ config` command with full CRUD operations. The project already uses dialoguer 0.11 for interactive prompts (established in Phase 4 for install confirmation) and console 0.16 for colored output.

The research confirms that dialoguer provides all required prompt types: Input (with validation), Password (with confirmation and hidden input), Confirm (for yes/no), and Select (for menu choices). The config schema needs to be extended with auth credentials and container environment variables. Port availability checking is already implemented in start.rs and can be reused.

**Primary recommendation:** Use the existing dialoguer + console stack. Add comfy-table for config display and rand for secure credential generation. Handle Ctrl+C by catching IoError::Interrupted and restoring terminal state.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| dialoguer | 0.11 | Interactive prompts (Input, Password, Confirm, Select) | Already in project, full-featured, cross-platform |
| console | 0.16 | Terminal styling (colors, clearing) | Already in project, same author as dialoguer |
| rand | 0.9 | Secure random credential generation | Standard Rust RNG, Alphanumeric distribution for passwords |
| comfy-table | 7.x | Table formatting for config display | Best-in-class for terminal tables, no-unsafe, fast |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::io::IsTerminal | 1.70+ | TTY detection | Check if running interactively (stdin.is_terminal()) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| dialoguer | inquire | More features but different API, would require rewrite of Phase 4 code |
| comfy-table | tabled | tabled uses derive macros, comfy-table is more flexible for dynamic content |
| rand | getrandom | getrandom is more conservative for cryptography, but rand's Alphanumeric is fine for passwords |

**Installation:**
```bash
# Add to packages/cli-rust/Cargo.toml
[dependencies]
rand = "0.9"
comfy-table = "7"

# Already present:
dialoguer = "0.11"
console = "0.16"
```

## Architecture Patterns

### Recommended Project Structure
```
packages/cli-rust/src/
├── commands/
│   ├── mod.rs           # Add setup, config submodules
│   ├── setup.rs         # Wizard implementation (NEW)
│   └── config/          # Config subcommands (NEW)
│       ├── mod.rs       # Config command router
│       ├── show.rs      # occ config (show all)
│       ├── get.rs       # occ config get <key>
│       ├── set.rs       # occ config set <key> <value>
│       ├── reset.rs     # occ config reset
│       └── env.rs       # occ config env set/list/remove
├── wizard/              # Wizard step implementations (NEW)
│   ├── mod.rs           # Wizard state machine
│   ├── auth.rs          # Username/password prompts
│   ├── network.rs       # Port/hostname prompts
│   └── summary.rs       # Final summary display
└── output/
    └── table.rs         # Table formatting helpers (NEW)

packages/core/src/config/
├── schema.rs            # Extend with auth_username, auth_password, container_env
└── mod.rs               # Add config field accessors
```

### Pattern 1: Wizard State Machine
**What:** Linear wizard flow with step tracking
**When to use:** Multi-step configuration process
**Example:**
```rust
// Source: dialoguer docs + project patterns
pub struct WizardState {
    current_step: usize,
    total_steps: usize,
    auth: Option<AuthConfig>,
    port: Option<u16>,
    hostname: Option<String>,
}

impl WizardState {
    pub fn step_prompt(&self, prompt: &str) -> String {
        format!("[{}/{}] {}", self.current_step, self.total_steps, prompt)
    }
}
```

### Pattern 2: Password Prompt with Confirmation
**What:** Hidden password input with verification
**When to use:** Collecting password credentials
**Example:**
```rust
// Source: https://docs.rs/dialoguer/latest/dialoguer/struct.Password.html
use dialoguer::Password;

let password = Password::new()
    .with_prompt("Password")
    .with_confirmation("Confirm password", "Passwords do not match")
    .interact()?;
```

### Pattern 3: Input with Validation
**What:** Text input with custom validation
**When to use:** Username, port, hostname entry
**Example:**
```rust
// Source: https://docs.rs/dialoguer/latest/dialoguer/struct.Input.html
use dialoguer::Input;

let username: String = Input::new()
    .with_prompt("Username")
    .validate_with(|input: &String| -> Result<(), &str> {
        if input.len() >= 3 && input.len() <= 32 &&
           input.chars().all(|c| c.is_alphanumeric() || c == '_') {
            Ok(())
        } else {
            Err("Username must be 3-32 chars, alphanumeric or underscore")
        }
    })
    .interact_text()?;
```

### Pattern 4: Secure Random Credentials
**What:** Generate cryptographically secure random password
**When to use:** User selects "generate credentials" option
**Example:**
```rust
// Source: https://rust-lang-nursery.github.io/rust-cookbook/algorithms/randomness.html
use rand::Rng;
use rand::distr::Alphanumeric;

fn generate_password(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}
```

### Pattern 5: Table Output for Config Display
**What:** Aligned table with masked passwords
**When to use:** `occ config` output
**Example:**
```rust
// Source: https://docs.rs/comfy-table/latest/comfy_table/
use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};

fn display_config(config: &Config) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(vec!["Key", "Value"]);
    table.add_row(vec!["username", &config.auth_username]);
    table.add_row(vec!["password", "********"]);
    table.add_row(vec!["port", &config.opencode_web_port.to_string()]);
    table.to_string()
}
```

### Anti-Patterns to Avoid
- **Accepting password as command argument:** Never `occ config set password secret123`. Always prompt interactively.
- **Storing password in environment variable during wizard:** Keep password in memory only, write directly to config file.
- **Using `interact()` when cancellation needed:** Use `interact_opt()` for cancellable operations.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Password input | Custom char-by-char reading | `dialoguer::Password` | Handles hidden input, confirmation, cross-platform |
| Input validation | Manual loop with println | `dialoguer::Input::validate_with` | Built-in retry, error display |
| Random generation | `SystemTime::now()` seeding | `rand::rng()` with Alphanumeric | Proper entropy, no timing attacks |
| TTY detection | Platform-specific code | `std::io::IsTerminal` | Stable since Rust 1.70, cross-platform |
| Table formatting | Manual string padding | `comfy-table` | Handles Unicode, wrapping, alignment |
| Port checking | Manual socket creation | Existing `check_port_available()` | Already in start.rs, tested |

**Key insight:** dialoguer handles the complex terminal state management (cursor hiding, raw mode, cleanup) that would be error-prone to implement manually.

## Common Pitfalls

### Pitfall 1: Ctrl+C During Prompt Corrupts Terminal
**What goes wrong:** Pressing Ctrl+C while dialoguer prompt is active can leave terminal in bad state (no cursor, raw mode).
**Why it happens:** SIGINT interrupts read operation, cleanup code may not run.
**How to avoid:** Wrap prompt in match on Result, handle IoError::Interrupted, manually reset terminal with `console::Term::stdout().show_cursor()`.
**Warning signs:** Tests pass but manual testing shows cursor disappearing.

```rust
// Handle Ctrl+C gracefully
match Password::new().with_prompt("Password").interact() {
    Ok(pass) => pass,
    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
        console::Term::stdout().show_cursor()?;
        return Err(anyhow!("Setup cancelled"));
    }
    Err(e) => return Err(e.into()),
}
```

### Pitfall 2: Port Availability Race Condition
**What goes wrong:** Port appears available during check but is taken by another process before binding.
**Why it happens:** TOCTOU (time-of-check to time-of-use) race.
**How to avoid:** Accept that this can happen, provide clear error message suggesting next port.
**Warning signs:** "Address already in use" despite check passing.

### Pitfall 3: Config Schema Migration
**What goes wrong:** Adding new fields breaks existing config files.
**Why it happens:** `deny_unknown_fields` is set, new fields cause deserialization failure.
**How to avoid:** New fields must have `#[serde(default)]` or version migration logic.
**Warning signs:** Existing installations fail after CLI update.

### Pitfall 4: Password Not Persisted Securely
**What goes wrong:** Password visible in config.json, shell history, or process list.
**Why it happens:** Storing plaintext, echoing to stdout, or using command args.
**How to avoid:** Store in config file (user-protected), never echo, always prompt interactively.
**Warning signs:** `ps aux` shows password in arguments.

### Pitfall 5: Wizard Partial Completion
**What goes wrong:** User cancels mid-wizard, config is left in inconsistent state.
**Why it happens:** Writing config after each step.
**How to avoid:** Build complete WizardState in memory, write config atomically at end.
**Warning signs:** Missing required fields after cancelled wizard.

## Code Examples

Verified patterns from official sources:

### Quick Setup Flow
```rust
// Source: CONTEXT.md decision + dialoguer patterns
use dialoguer::Confirm;

fn run_wizard(quick_mode: bool) -> Result<WizardState> {
    // Offer quick setup first
    let quick = Confirm::new()
        .with_prompt("Use defaults for everything except credentials?")
        .default(false)
        .interact()?;

    if quick {
        // Only prompt for auth, use defaults for rest
        let auth = prompt_auth()?;
        return Ok(WizardState::quick(auth));
    }

    // Full wizard flow
    run_full_wizard()
}
```

### Docker Pre-check
```rust
// Source: Existing start.rs pattern + CONTEXT.md decision
use opencode_cloud_core::docker::DockerClient;

async fn verify_docker_available() -> Result<()> {
    let client = DockerClient::new().map_err(|_| {
        anyhow!(
            "Docker is not available.\n\n\
             Please start Docker Desktop or the Docker daemon before running setup.\n\
             On Linux: sudo systemctl start docker"
        )
    })?;

    client.verify_connection().await.map_err(|_| {
        anyhow!("Docker is not running. Please start Docker and try again.")
    })?;

    Ok(())
}
```

### Config Set with Restart Warning
```rust
// Source: CONTEXT.md decision
fn cmd_config_set(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;

    match key {
        "port" | "opencode_web_port" => {
            let port: u16 = value.parse()?;
            config.opencode_web_port = port;
        }
        "password" => {
            // Never accept password as argument
            return Err(anyhow!(
                "Password cannot be set via command line.\n\
                 Use: occ config set password  (will prompt securely)"
            ));
        }
        // ... other keys
        _ => return Err(anyhow!("Unknown config key: {}", key)),
    }

    save_config(&config)?;

    // Check if service is running
    if is_service_running()? {
        eprintln!("{}", style("Restart required for changes to take effect").yellow());
    }

    Ok(())
}
```

### Environment Variable Management
```rust
// Source: CONTEXT.md decision - container_env array
fn cmd_env_set(env_var: &str) -> Result<()> {
    // Validate KEY=value format
    if !env_var.contains('=') {
        return Err(anyhow!("Format must be KEY=value"));
    }

    let mut config = load_config()?;

    // Extract key to check for duplicates
    let key = env_var.split('=').next().unwrap();

    // Remove existing entry for same key
    config.container_env.retain(|e| !e.starts_with(&format!("{}=", key)));

    // Add new entry
    config.container_env.push(env_var.to_string());

    save_config(&config)?;
    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| atty crate for TTY detection | std::io::IsTerminal trait | Rust 1.70 (June 2023) | Use stdlib, no dependency |
| rand::thread_rng() | rand::rng() | rand 0.9 (Jan 2025) | Simpler API, same security |
| dialoguer + atty | dialoguer (built-in console) | N/A | dialoguer uses console crate internally |

**Deprecated/outdated:**
- `atty` crate: Unmaintained, replaced by `std::io::IsTerminal`
- `rand::gen_ascii_chars()`: Removed in rand 0.6, use `Alphanumeric` distribution
- `rand::thread_rng()`: Renamed to `rand::rng()` in 0.9

## Open Questions

Things that couldn't be fully resolved:

1. **OpenCode config import scope**
   - What we know: OpenCode config is at `~/.config/opencode/opencode.json`, contains model, provider settings
   - What's unclear: Which fields are "compatible" for import? The config format is different from opencode-cloud
   - Recommendation: Import only API keys and provider settings if present, warn for unknown fields

2. **Test configuration implementation**
   - What we know: CONTEXT.md says "Offer to test configuration after setup"
   - What's unclear: What exactly should the test do? Docker pull? Container start? Health check?
   - Recommendation: Test Docker connectivity and port availability, optionally start container and verify health

3. **Ctrl+C handling completeness**
   - What we know: dialoguer + ctrlc crate has known issue #248 with cursor not restoring
   - What's unclear: Is this fully fixed in dialoguer 0.11/0.12?
   - Recommendation: Implement explicit terminal cleanup in error handler, test manually

## Sources

### Primary (HIGH confidence)
- dialoguer 0.11/0.12 docs: https://docs.rs/dialoguer/latest/dialoguer/
  - Password struct API
  - Input struct with validation
  - Confirm struct with interact_opt
- rand 0.9 docs: https://docs.rs/rand
  - Alphanumeric distribution for passwords
- comfy-table docs: https://docs.rs/comfy-table/latest/comfy_table/
  - Table formatting API
- Rust std::io::IsTerminal: https://doc.rust-lang.org/std/io/trait.IsTerminal.html
  - TTY detection
- OpenCode config docs: https://opencode.ai/docs/config/
  - Config file location and format

### Secondary (MEDIUM confidence)
- Rust Cookbook: https://rust-lang-nursery.github.io/rust-cookbook/algorithms/randomness.html
  - Random string generation patterns
- dialoguer GitHub Issue #248: https://github.com/console-rs/dialoguer/issues/248
  - Ctrl+C handling limitations

### Tertiary (LOW confidence)
- Various CLI prompt library comparisons (multiple blog posts)
  - Confirmed dialoguer is appropriate choice

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - dialoguer already in project, patterns well-documented
- Architecture: HIGH - extends existing CLI patterns, config schema is straightforward
- Pitfalls: MEDIUM - Ctrl+C handling issue is documented but fix status unclear

**Research date:** 2026-01-20
**Valid until:** 2026-02-20 (30 days - stable domain, dialoguer mature)
