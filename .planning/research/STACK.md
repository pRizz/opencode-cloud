# Technology Stack

**Project:** opencode-cloud-service
**Researched:** 2026-01-18
**Overall Confidence:** HIGH

---

## Executive Summary

This document recommends the 2025/2026 standard stack for building a cross-platform cloud service installer with both npm/npx and Rust/cargo distribution paths. The recommendations prioritize:

1. **Battle-tested libraries** with high download counts and active maintenance
2. **Cross-platform compatibility** as a first-class requirement
3. **Modern async patterns** for Docker and service management
4. **Excellent developer and user experience** for interactive CLI wizards

---

## Recommended Stack

### Node.js/npm Installer

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **Node.js** | >= 20 LTS | Runtime | Commander 14 requires Node 20+; LTS ensures stability | HIGH |
| **TypeScript** | 5.x | Type safety | Catches errors at compile time, better DX | HIGH |
| **Commander.js** | 14.x | CLI framework | Industry standard, 14M+ weekly downloads, excellent subcommand support | HIGH |
| **@inquirer/prompts** | latest | Interactive prompts | Modern rewrite of Inquirer with smaller bundle, better performance | HIGH |
| **conf** | latest | Config persistence | Atomic writes, proper cross-platform config directories, built for CLIs | HIGH |
| **dockerode** | 4.x | Docker API | Most popular Node Docker client, both callback and promise APIs | HIGH |
| **ora** | 9.x | Spinners/progress | Elegant terminal spinners, industry standard | HIGH |
| **chalk** | 5.x | Terminal colors | Most popular terminal styling, 200M+ weekly downloads | MEDIUM |
| **execa** | latest | Process spawning | Superior to native child_process, cross-platform, promise-based | HIGH |

#### Installation Command (npm)

```bash
npm install commander @inquirer/prompts conf dockerode ora chalk execa
npm install -D typescript @types/node @types/dockerode
```

### Rust/cargo Installer

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **clap** | 4.5.x | CLI framework | Undisputed standard for Rust CLIs, derive macro for clean code | HIGH |
| **tokio** | 1.43+ LTS | Async runtime | Required for bollard, LTS releases maintain stability | HIGH |
| **bollard** | 0.19.x | Docker API | Only mature async Docker client for Rust, uses Docker API 1.49 | HIGH |
| **dialoguer** | latest | Interactive prompts | Part of console-rs ecosystem, 38M+ downloads | HIGH |
| **indicatif** | 0.18.x | Progress bars | 90M+ downloads, works seamlessly with dialoguer | HIGH |
| **console** | 0.16.x | Terminal abstraction | Foundation for dialoguer/indicatif, handles cross-platform terminal | HIGH |
| **serde** | 1.0.x | Serialization | Universal standard for Rust serialization | HIGH |
| **serde_json** | 1.0.x | JSON handling | 600M+ downloads, standard JSON library | HIGH |
| **directories** | latest | Config paths | Cross-platform config/data/cache directory resolution | HIGH |
| **tracing** | 0.1.x | Logging | Structured logging standard, maintained by Tokio team | HIGH |
| **thiserror** | 2.0 | Error types | Clean error type definitions for library code | HIGH |
| **anyhow** | 2.0 | Error handling | Ergonomic error handling for application code | HIGH |

#### Cargo.toml Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
tokio = { version = "1.43", features = ["full"] }
bollard = "0.19"
dialoguer = { version = "0.11", features = ["fuzzy-select"] }
indicatif = "0.18"
console = "0.16"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
directories = "5"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2.0"
anyhow = "2.0"
```

---

## Cross-Platform Service Installation

This is the most complex area with the least standardization. Recommendation: **build a thin abstraction layer** over platform-specific libraries.

### Linux (systemd)

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| **Template-based unit files** | - | Generate .service files | HIGH |
| **libsystemd-rs** (Rust) | latest | systemd integration | MEDIUM |

**Approach:** Generate systemd unit files from templates, use `systemctl` commands via process spawning for installation. This is more reliable than D-Bus wrappers.

```ini
# Template: opencode.service
[Unit]
Description=OpenCode Cloud Service
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
ExecStart=/usr/bin/docker start -a opencode
ExecStop=/usr/bin/docker stop opencode
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### macOS (launchd)

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| **plist templates** | - | Generate .plist files | HIGH |
| **launchctl** (Rust crate) | latest | launchd wrapper | MEDIUM |

**Approach:** Generate plist XML files, use `launchctl` commands via process spawning.

```xml
<!-- Template: com.opencode.service.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.opencode.service</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/docker</string>
        <string>start</string>
        <string>-a</string>
        <string>opencode</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

### Windows (Windows Services)

| Technology | Version | Purpose | Confidence |
|------------|---------|---------|------------|
| **windows-service** (Rust) | latest | Windows Service API | HIGH |
| **node-windows** (npm) | latest | Windows service wrapper | MEDIUM |

**Approach for Node.js:** Use `node-windows` package or spawn `sc.exe` commands.

**Approach for Rust:** Use `windows-service` crate from Mullvad (battle-tested in production VPN software).

### Cross-Platform Abstraction Libraries

| Library | Platform Support | Status | Recommendation |
|---------|-----------------|--------|----------------|
| **cross-platform-service** (Rust) | Win/Linux/macOS | Active | Consider, but verify maturity |
| **uni_service** (Rust) | Win/Linux/macOS/Unix | Active | Alternative option |

**Recommendation:** Due to the complexity and platform-specific nature of service installation, **prefer spawning native tools** (`systemctl`, `launchctl`, `sc.exe`) over library abstractions. This provides:
- Better error messages from native tools
- Easier debugging
- No dependency on library maintenance
- More predictable behavior

---

## Docker Management

### Node.js

| Option | Recommendation | Rationale |
|--------|---------------|-----------|
| **dockerode** | USE | Mature, promise-based, handles streams well |
| **@docker/sdk** | AVOID | Last published 5 years ago, appears abandoned |
| **docker-cli-js** | AVOID | CLI wrapper, less reliable than API client |

**dockerode Example:**
```typescript
import Docker from 'dockerode';
const docker = new Docker();

// List containers
const containers = await docker.listContainers();

// Start a container
const container = docker.getContainer('opencode');
await container.start();

// Stream logs
const logStream = await container.logs({ follow: true, stdout: true, stderr: true });
```

### Rust

| Option | Recommendation | Rationale |
|--------|---------------|-----------|
| **bollard** | USE | Only mature async Docker client, Docker API 1.49 |
| **docker** crate | AVOID | Less maintained than bollard |
| **shiplift** | AVOID | Predecessor to bollard, no longer maintained |

**bollard Example:**
```rust
use bollard::Docker;
use bollard::container::{StartContainerOptions, LogsOptions};

let docker = Docker::connect_with_local_defaults()?;

// Start container
docker.start_container("opencode", None::<StartContainerOptions<String>>).await?;

// Stream logs
let logs = docker.logs("opencode", Some(LogsOptions::<String> {
    follow: true,
    stdout: true,
    stderr: true,
    ..Default::default()
}));
```

---

## Interactive CLI Wizards

### Node.js

| Library | Use For | Example |
|---------|---------|---------|
| **@inquirer/prompts** | All interactive prompts | `import { input, select, confirm } from '@inquirer/prompts'` |
| **ora** | Loading spinners | `ora('Installing...').start()` |
| **chalk** | Colored output | `chalk.green('Success!')` |

**Wizard Flow Example:**
```typescript
import { input, select, confirm, password } from '@inquirer/prompts';
import ora from 'ora';
import chalk from 'chalk';

// Step 1: Collect config
const port = await input({ message: 'Web UI port:', default: '3000' });
const provider = await select({
  message: 'AI Provider:',
  choices: [
    { name: 'Anthropic', value: 'anthropic' },
    { name: 'OpenAI', value: 'openai' },
  ],
});
const apiKey = await password({ message: 'API Key:' });

// Step 2: Confirm
const proceed = await confirm({ message: 'Install service?' });

// Step 3: Install with spinner
const spinner = ora('Installing OpenCode service...').start();
// ... installation logic
spinner.succeed(chalk.green('OpenCode installed successfully!'));
```

### Rust

| Library | Use For | Example |
|---------|---------|---------|
| **dialoguer** | All interactive prompts | `Input`, `Select`, `Confirm`, `Password` |
| **indicatif** | Progress bars/spinners | `ProgressBar`, `MultiProgress` |
| **console** | Terminal styling | `style()`, `Term` |

**Wizard Flow Example:**
```rust
use dialoguer::{Input, Select, Confirm, Password, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use console::style;

// Step 1: Collect config
let port: u16 = Input::with_theme(&ColorfulTheme::default())
    .with_prompt("Web UI port")
    .default(3000)
    .interact()?;

let providers = vec!["Anthropic", "OpenAI"];
let provider_idx = Select::with_theme(&ColorfulTheme::default())
    .with_prompt("AI Provider")
    .items(&providers)
    .interact()?;

let api_key = Password::with_theme(&ColorfulTheme::default())
    .with_prompt("API Key")
    .interact()?;

// Step 2: Confirm
let proceed = Confirm::with_theme(&ColorfulTheme::default())
    .with_prompt("Install service?")
    .interact()?;

// Step 3: Install with spinner
let pb = ProgressBar::new_spinner();
pb.set_style(ProgressStyle::default_spinner());
pb.set_message("Installing OpenCode service...");
pb.enable_steady_tick(std::time::Duration::from_millis(100));
// ... installation logic
pb.finish_with_message(format!("{}", style("OpenCode installed successfully!").green()));
```

---

## Configuration Persistence

### Node.js

| Library | Recommendation | Rationale |
|---------|---------------|-----------|
| **conf** | USE | Atomic writes, proper XDG paths, built for CLIs |
| **configstore** | AVOID | Older, conf is its successor by same author |
| **node-config** | AVOID | Designed for apps, not CLI tools |

**conf Example:**
```typescript
import Conf from 'conf';

interface OpenCodeConfig {
  port: number;
  provider: 'anthropic' | 'openai';
  containerName: string;
  installPath: string;
}

const config = new Conf<OpenCodeConfig>({
  projectName: 'opencode-cloud-service',
  defaults: {
    port: 3000,
    provider: 'anthropic',
    containerName: 'opencode',
    installPath: '/opt/opencode',
  },
});

// Config stored at:
// Linux: ~/.config/opencode-cloud-service/config.json
// macOS: ~/Library/Application Support/opencode-cloud-service/config.json
// Windows: %APPDATA%\opencode-cloud-service\Config\config.json
```

### Rust

| Library | Recommendation | Rationale |
|---------|---------------|-----------|
| **directories + serde_json** | USE | Standard approach, full control |
| **confy** | CONSIDER | Higher-level, uses directories internally |
| **config-rs** | AVOID | Designed for reading config, not writing |

**directories + serde_json Example:**
```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct OpenCodeConfig {
    port: u16,
    provider: String,
    container_name: String,
    install_path: String,
}

fn get_config_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "opencode", "opencode-cloud-service")
        .map(|dirs| dirs.config_dir().join("config.json"))
}

fn save_config(config: &OpenCodeConfig) -> anyhow::Result<()> {
    let path = get_config_path().ok_or_else(|| anyhow::anyhow!("No config directory"))?;
    fs::create_dir_all(path.parent().unwrap())?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json)?;
    Ok(())
}
```

---

## Alternatives Considered

### CLI Frameworks

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Node CLI | Commander 14 | Yargs | Commander has cleaner API, better TypeScript support |
| Node CLI | Commander 14 | Oclif | Oclif is overkill for this use case, heavy framework |
| Rust CLI | clap 4.5 | structopt | structopt merged into clap, use clap derive instead |
| Rust CLI | clap 4.5 | argh | Less feature-rich, smaller community |

### Interactive Prompts

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Node prompts | @inquirer/prompts | prompts | Inquirer has richer features, better maintained |
| Node prompts | @inquirer/prompts | enquirer | Inquirer is more actively developed |
| Node prompts | @inquirer/prompts | inquirer (legacy) | Legacy version, @inquirer/prompts is the rewrite |
| Rust prompts | dialoguer | requestty | dialoguer is more mature, part of console-rs |
| Rust prompts | dialoguer | tui-prompts | tui-prompts for ratatui only, overkill for prompts |

### Docker Clients

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Node Docker | dockerode | @docker/sdk | SDK appears abandoned (5 years old) |
| Node Docker | dockerode | docker-cli-js | CLI wrapper is fragile, API client is better |
| Rust Docker | bollard | docker crate | bollard is more actively maintained, async |
| Rust Docker | bollard | shiplift | shiplift is predecessor, no longer maintained |

### Service Management

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Cross-platform | Native tools | cross-platform-service | Native tools more reliable, better error messages |
| Windows (Rust) | windows-service | windows-services | windows-service from Mullvad, battle-tested |

---

## What NOT to Use

### Node.js

| Library | Reason to Avoid |
|---------|-----------------|
| **inquirer** (legacy) | Use @inquirer/prompts instead - modern rewrite |
| **colors** | Had supply chain attack in 2022; use chalk instead |
| **@docker/sdk** | Appears abandoned, last update 5+ years ago |
| **configstore** | Outdated; use conf (successor by same author) |
| **chalk 4.x** | Chalk 5.x is ESM-only; use 5.x for modern projects or 4.x for CJS |
| **vorpal** | Unmaintained since 2017 |

### Rust

| Crate | Reason to Avoid |
|-------|-----------------|
| **structopt** | Merged into clap; use clap derive instead |
| **shiplift** | Predecessor to bollard, no longer maintained |
| **async-std** | Discontinued as of March 2025; use tokio |
| **app_dirs** | Unmaintained; use directories instead |
| **term** | Old; use console from console-rs |

---

## Version Pinning Strategy

### Node.js (package.json)

```json
{
  "engines": {
    "node": ">=20.0.0"
  },
  "dependencies": {
    "commander": "^14.0.0",
    "@inquirer/prompts": "^7.0.0",
    "conf": "^13.0.0",
    "dockerode": "^4.0.0",
    "ora": "^9.0.0",
    "chalk": "^5.0.0",
    "execa": "^9.0.0"
  }
}
```

### Rust (Cargo.toml)

```toml
[package]
edition = "2021"
rust-version = "1.75"

[dependencies]
clap = "4.5"
tokio = "1.43"
bollard = "0.19"
dialoguer = "0.11"
indicatif = "0.18"
console = "0.16"
serde = "1.0"
serde_json = "1.0"
directories = "5"
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "2"
anyhow = "2"
```

---

## Sources

### CLI Frameworks
- [clap GitHub](https://github.com/clap-rs/clap) - Rust CLI parser (HIGH confidence)
- [Commander.js GitHub](https://github.com/tj/commander.js) - Node.js CLI framework (HIGH confidence)
- [Commander.js npm](https://www.npmjs.com/package/commander) - Version 14.0.2 confirmed

### Interactive Prompts
- [@inquirer/prompts npm](https://www.npmjs.com/package/@inquirer/prompts) - Modern Inquirer rewrite (HIGH confidence)
- [dialoguer GitHub](https://github.com/console-rs/dialoguer) - Rust prompts (HIGH confidence)
- [indicatif Guide](https://generalistprogrammer.com/tutorials/indicatif-rust-crate-guide) - Progress bars (HIGH confidence)

### Docker Management
- [bollard crates.io](https://crates.io/crates/bollard) - Version 0.19.3 (HIGH confidence)
- [bollard docs.rs](https://docs.rs/bollard/latest/bollard/) - API documentation
- [dockerode npm](https://www.npmjs.com/package/dockerode) - Version 4.0.9 (HIGH confidence)

### Configuration
- [conf npm](https://www.npmjs.com/package/conf) - Node config persistence (HIGH confidence)
- [directories crates.io](https://crates.io/crates/directories) - Cross-platform paths (HIGH confidence)

### Service Management
- [windows-service-rs GitHub](https://github.com/mullvad/windows-service-rs) - Windows services (HIGH confidence)
- [launchctl crates.io](https://crates.io/crates/launchctl) - macOS launchd wrapper (MEDIUM confidence)
- [Apple launchd docs](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html) - Official docs

### Error Handling & Logging
- [anyhow docs.rs](https://docs.rs/anyhow) - Error handling (HIGH confidence)
- [thiserror docs.rs](https://docs.rs/thiserror) - Error type definitions (HIGH confidence)
- [tracing crates.io](https://crates.io/crates/tracing) - Structured logging (HIGH confidence)

### Async Runtime
- [Tokio website](https://tokio.rs/) - Async runtime (HIGH confidence)
- [Tokio LTS releases](https://github.com/tokio-rs/tokio) - 1.43.x LTS until March 2026

---

## Roadmap Implications

Based on this stack analysis:

1. **Phase 1: Core Docker Management** - Both installers can share the same Docker interaction patterns; start with container lifecycle commands (start, stop, status, logs)

2. **Phase 2: Interactive Setup Wizard** - Commander + @inquirer/prompts for Node; clap + dialoguer for Rust. Both ecosystems have mature, well-documented solutions.

3. **Phase 3: Platform Service Installation** - This is the highest-risk area. Recommend template-based approach with native tool spawning over library abstractions. Test extensively on each platform.

4. **Phase 4: Config Persistence** - conf for Node, directories + serde_json for Rust. Both handle cross-platform config directories correctly.

**Key Risk:** Cross-platform service installation has the least standardization and highest platform variance. Budget extra time for testing Windows, macOS, and various Linux distributions.
