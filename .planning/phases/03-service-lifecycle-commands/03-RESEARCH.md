# Phase 3: Service Lifecycle Commands - Research

**Researched:** 2026-01-19
**Domain:** CLI commands with terminal UI (spinners, colors, logs) wrapping Docker operations
**Confidence:** HIGH

## Summary

This phase builds user-facing CLI commands (start, stop, restart, status, logs) on top of the Docker operations implemented in Phase 2. The key challenges are:

1. Creating an excellent terminal UX with spinners, elapsed time, and color-coded output
2. Streaming container logs with follow mode, filtering, and proper handling of container lifecycle events
3. Providing actionable error messages that help users self-diagnose problems
4. Implementing idempotent command behavior (e.g., `start` when already running just shows status)

The project already uses `clap` for argument parsing, `console` for terminal colors, and `indicatif` for progress bars. These are the right tools. The main additions needed are:
- `webbrowser` crate for `--open` flag (browser opening)
- `humantime` for human-readable durations in status output
- Extending the existing `ProgressReporter` for command-specific spinner patterns

**Primary recommendation:** Leverage the existing indicatif/console infrastructure from Phase 2; add clap subcommands with global flags (`-q`, `-v`, `--no-color`); implement log streaming using Bollard's `LogsOptions` with futures-util `StreamExt`.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| [clap](https://docs.rs/clap) | 4.5+ | CLI argument parsing | Already in workspace; derive macros for subcommands |
| [console](https://docs.rs/console) | 0.16+ | Terminal colors/styles | Already in workspace; automatic TTY detection |
| [indicatif](https://docs.rs/indicatif) | 0.17+ | Spinners/progress bars | Already in workspace; ProgressReporter exists |
| [bollard](https://docs.rs/bollard) | 0.18+ | Docker API | Already in workspace; logs/inspect APIs |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| [webbrowser](https://crates.io/crates/webbrowser) | 1.0+ | Open URLs in browser | `--open` flag, config option for auto-open |
| [humantime](https://crates.io/crates/humantime) | 2.1+ | Human-readable durations | Status uptime display ("2h 37m") |
| [futures-util](https://docs.rs/futures-util) | 0.3 | Stream utilities | Log streaming with `StreamExt::next()` |
| [tokio](https://docs.rs/tokio) | 1.43+ | Async runtime | Signal handling for Ctrl+C |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| console | owo-colors | owo-colors has better NO_COLOR support but console is already in workspace |
| webbrowser | open | webbrowser guarantees browser (not editor) opens for URLs |
| humantime | chrono-humanize | humantime is lighter; no chrono dependency needed |

**Installation:**
```bash
# Add to packages/cli-rust/Cargo.toml
cargo add webbrowser
cargo add humantime
```

## Architecture Patterns

### Recommended Project Structure
```
packages/cli-rust/src/
├── bin/
│   ├── opencode-cloud.rs  # Existing: main entry point
│   └── occ.rs             # Existing: alias binary
├── lib.rs                 # Existing: CLI implementation
├── commands/              # NEW: Command implementations
│   ├── mod.rs             # Module exports
│   ├── start.rs           # Start command logic
│   ├── stop.rs            # Stop command logic
│   ├── restart.rs         # Restart command logic
│   ├── status.rs          # Status command logic
│   └── logs.rs            # Logs command logic
└── output/                # NEW: Terminal output utilities
    ├── mod.rs             # Module exports
    ├── spinner.rs         # Command-specific spinner patterns
    └── colors.rs          # Color/style definitions
```

### Pattern 1: Global CLI Flags with Clap
**What:** Define `-q`, `-v`, `--no-color` as global flags available to all subcommands
**When to use:** All commands need consistent output control
**Example:**
```rust
// Source: https://docs.rs/clap/latest/clap/
use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "opencode-cloud")]
#[command(version, about)]
struct Cli {
    #[command(flatten)]
    global: GlobalOpts,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args)]
struct GlobalOpts {
    /// Suppress non-error output (for scripting)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Increase verbosity level (can repeat: -v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the opencode service
    Start(StartArgs),
    /// Stop the opencode service
    Stop(StopArgs),
    /// Restart the opencode service
    Restart(RestartArgs),
    /// Show service status
    Status(StatusArgs),
    /// View service logs
    Logs(LogsArgs),
    // ... existing Config subcommand
}
```

### Pattern 2: Spinner with Elapsed Time
**What:** Show operation progress with spinner animation and elapsed time
**When to use:** Start, stop, restart operations
**Example:**
```rust
// Source: https://docs.rs/indicatif/latest/indicatif/struct.ProgressBar.html
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct CommandSpinner {
    bar: ProgressBar,
}

impl CommandSpinner {
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} ({elapsed})")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));
        Self { bar }
    }

    pub fn update(&self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    pub fn success(self, message: &str) {
        self.bar.finish_with_message(format!("{} {}",
            console::style("✓").green(), message));
    }

    pub fn fail(self, message: &str) {
        self.bar.finish_with_message(format!("{} {}",
            console::style("✗").red(), message));
    }
}
```

### Pattern 3: Log Streaming with Bollard
**What:** Stream container logs with follow mode, line count, timestamps, and filtering
**When to use:** `logs` command implementation
**Example:**
```rust
// Source: https://docs.rs/bollard/latest/bollard/container/struct.LogsOptions.html
use bollard::container::{LogsOptions, LogOutput};
use futures_util::stream::StreamExt;

pub async fn stream_logs(
    client: &DockerClient,
    container_name: &str,
    follow: bool,
    tail: Option<&str>,
    timestamps: bool,
    grep: Option<&str>,
) -> Result<(), DockerError> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow,
        tail: tail.unwrap_or("50"),
        timestamps,
        ..Default::default()
    };

    let mut stream = client.inner().logs(container_name, Some(options));

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                let line = match output {
                    LogOutput::StdOut { message } |
                    LogOutput::StdErr { message } => {
                        String::from_utf8_lossy(&message).to_string()
                    }
                    _ => continue,
                };

                // Apply grep filter if specified
                if let Some(pattern) = grep {
                    if !line.contains(pattern) {
                        continue;
                    }
                }

                // Color-code by log level (if TTY)
                print_log_line(&line);
            }
            Err(e) => {
                // Container stopped during follow
                if follow {
                    eprintln!("\nContainer stopped");
                }
                break;
            }
        }
    }

    Ok(())
}
```

### Pattern 4: Idempotent Command Behavior
**What:** Commands succeed gracefully when already in target state
**When to use:** All lifecycle commands
**Example:**
```rust
// Source: Project decision - idempotent behavior
pub async fn cmd_start(
    client: &DockerClient,
    args: &StartArgs,
    opts: &GlobalOpts,
) -> Result<(), anyhow::Error> {
    // Check current state
    if container_is_running(client, CONTAINER_NAME).await? {
        if !opts.quiet {
            // Show current status instead of error
            show_status(client, opts).await?;
            println!("\n{}", console::style("Service is already running").dim());
        }
        return Ok(()); // Exit 0, not error
    }

    // Proceed with start...
}
```

### Pattern 5: Color Control with NO_COLOR Support
**What:** Respect NO_COLOR env var and --no-color flag, auto-detect TTY
**When to use:** All colored output
**Example:**
```rust
// Source: https://docs.rs/console/latest/console/
use console::{Style, style};

pub fn init_colors(no_color: bool) {
    // Check NO_COLOR environment variable (standard)
    let env_no_color = std::env::var("NO_COLOR").is_ok();

    if no_color || env_no_color {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
    // console auto-detects TTY for non-forced cases
}

// Color definitions for consistent styling
pub fn state_style(state: &str) -> console::StyledObject<&str> {
    match state.to_lowercase().as_str() {
        "running" => style(state).green().bold(),
        "stopped" | "exited" => style(state).red(),
        "starting" | "restarting" => style(state).yellow(),
        _ => style(state).dim(),
    }
}

pub fn log_level_style(line: &str) -> console::StyledObject<&str> {
    if line.contains("ERROR") || line.contains("error") {
        style(line).red()
    } else if line.contains("WARN") || line.contains("warn") {
        style(line).yellow()
    } else if line.contains("INFO") || line.contains("info") {
        style(line).cyan()
    } else if line.contains("DEBUG") || line.contains("debug") {
        style(line).dim()
    } else {
        style(line)
    }
}
```

### Pattern 6: Status Display Format
**What:** Key-value line format for status output
**When to use:** `status` command and success messages
**Example:**
```rust
// Source: Project decision - key-value format
use humantime::format_duration;

pub fn print_status(info: &ContainerInfo, config_path: &Path) {
    let state = info.state.as_deref().unwrap_or("unknown");

    println!("State:       {}", state_style(state));

    if state == "running" {
        println!("URL:         {}", style(format!("http://localhost:{}", info.port)).cyan());
        println!("Container:   {} ({})", info.name, &info.id[..12]);
        println!("Image:       {}", info.image);

        if let Some(started) = &info.started_at {
            let uptime = calculate_uptime(started);
            println!("Uptime:      {} (since {})",
                format_duration(uptime),
                format_timestamp(started));
        }

        println!("Port:        {} -> container:3000", info.port);

        if let Some(health) = &info.health_status {
            println!("Health:      {}", health_style(health));
        }
    } else {
        if let Some(finished) = &info.finished_at {
            println!("Last run:    {}", format_timestamp(finished));
        }
        println!();
        println!("{}", style("Run 'occ start' to start the service").dim());
    }

    println!("Config:      {}", config_path.display());
}
```

### Anti-Patterns to Avoid
- **Swallowing errors silently:** Always show error messages unless `-q` is used
- **Blocking on Ctrl+C:** Use `tokio::select!` with `signal::ctrl_c()` for graceful interrupt handling
- **Hardcoded timeouts:** Use config values or reasonable defaults (30s shutdown timeout)
- **Mixing stdout/stderr:** Errors go to stderr, normal output to stdout
- **Forgetting exit codes:** Return exit code 1 on error, 0 on success

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Browser opening | `std::process::Command("open")` | webbrowser crate | Cross-platform, handles edge cases |
| Duration formatting | Manual arithmetic | humantime | Handles all units, proper localization |
| Spinner animation | Raw terminal escape codes | indicatif | Handles terminal width, cleanup on Ctrl+C |
| Color detection | Manual isatty checks | console | Handles TTY, NO_COLOR, Windows |
| Log streaming | Polling container logs | Bollard LogsOptions with follow | Proper stream handling, container events |

**Key insight:** Terminal UI has many edge cases (terminal width, non-TTY, Windows, Ctrl+C cleanup). The console-rs ecosystem (console, indicatif, dialoguer) handles these consistently.

## Common Pitfalls

### Pitfall 1: Spinner Not Clearing on Error
**What goes wrong:** Spinner animation continues or leaves artifacts when error occurs
**Why it happens:** Error thrown before spinner.finish() called
**How to avoid:** Use RAII pattern or explicit cleanup in error paths:
```rust
let spinner = CommandSpinner::new("Starting...");
match do_operation().await {
    Ok(_) => spinner.success("Started"),
    Err(e) => {
        spinner.fail("Failed to start");
        return Err(e);
    }
}
```
**Warning signs:** Garbled terminal output, spinner artifacts after errors

### Pitfall 2: Log Stream Hanging After Container Stop
**What goes wrong:** `logs -f` never exits when container stops
**Why it happens:** Bollard stream doesn't automatically close on container exit
**How to avoid:** Handle stream errors and check container state:
```rust
while let Some(result) = stream.next().await {
    match result {
        Ok(output) => print_log(output),
        Err(e) => {
            // Check if container stopped
            if !container_is_running(client, name).await.unwrap_or(false) {
                println!("\nContainer stopped");
                break;
            }
            return Err(e.into());
        }
    }
}
```
**Warning signs:** Need to Ctrl+C to exit logs even after container stops

### Pitfall 3: Port Conflict Not Detected Until Start
**What goes wrong:** User sees cryptic Docker error about port already allocated
**Why it happens:** Port check happens at container create, not before
**How to avoid:** Pre-check port availability and suggest alternatives:
```rust
use std::net::TcpListener;

fn check_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn find_next_available_port(start: u16) -> Option<u16> {
    (start..start + 100).find(|&p| check_port_available(p))
}

// In start command:
if !check_port_available(config.port) {
    if let Some(available) = find_next_available_port(config.port) {
        eprintln!("Port {} is in use. Use --port {} instead",
            config.port, available);
    }
    return Err(anyhow!("Port {} is already in use", config.port));
}
```
**Warning signs:** Users see Docker error messages instead of actionable guidance

### Pitfall 4: Colors Displayed in Piped Output
**What goes wrong:** ANSI escape codes appear when output is piped to file or another command
**Why it happens:** Not checking if stdout is a terminal
**How to avoid:** console crate handles this automatically if you use `style()`:
```rust
// console auto-detects TTY and disables colors when piped
println!("{}", style("text").green()); // Green in terminal, plain when piped
```
**Warning signs:** `occ status | grep running` shows escape codes

### Pitfall 5: Quiet Mode Still Produces Output
**What goes wrong:** Scripts parsing output get unexpected text
**Why it happens:** Forgetting to check quiet flag in all output paths
**How to avoid:** Create output helper that respects quiet flag:
```rust
struct Output {
    quiet: bool,
}

impl Output {
    fn info(&self, msg: &str) {
        if !self.quiet {
            println!("{}", msg);
        }
    }

    fn error(&self, msg: &str) {
        // Errors always go to stderr, even in quiet mode
        eprintln!("{}", style("Error:").red().bold());
        eprintln!("  {}", msg);
    }

    fn quiet_result(&self, msg: &str) {
        // For -q mode: just the essential result (e.g., URL)
        if self.quiet {
            println!("{}", msg);
        }
    }
}
```
**Warning signs:** `occ start -q` produces unexpected output

### Pitfall 6: Container Crash Gives No Context
**What goes wrong:** "Container exited" with no explanation
**Why it happens:** Not fetching logs on startup failure
**How to avoid:** On startup timeout or crash, automatically show recent logs:
```rust
async fn start_with_crash_detection(client: &DockerClient) -> Result<(), Error> {
    start_container(client, CONTAINER_NAME).await?;

    // Wait for container to be running (with timeout)
    let timeout = Duration::from_secs(30);
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            // Show last 20 lines of logs for debugging
            eprintln!("\nContainer failed to start. Recent logs:");
            show_recent_logs(client, 20).await?;
            return Err(anyhow!("Timeout waiting for container to start"));
        }

        if container_is_running(client, CONTAINER_NAME).await? {
            return Ok(());
        }

        // Check if container exited (crashed)
        let state = container_state(client, CONTAINER_NAME).await?;
        if state == "exited" {
            eprintln!("\nContainer crashed during startup. Recent logs:");
            show_recent_logs(client, 20).await?;
            return Err(anyhow!("Container exited unexpectedly"));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```
**Warning signs:** Users can't diagnose why container won't start

## Code Examples

Verified patterns from official sources:

### Start Command with Full UX
```rust
// Source: Combining patterns from indicatif, console, bollard
pub async fn cmd_start(
    client: &DockerClient,
    args: &StartArgs,
    opts: &GlobalOpts,
) -> Result<(), anyhow::Error> {
    // Idempotent: if already running, just show status
    if container_is_running(client, CONTAINER_NAME).await? {
        if !opts.quiet {
            show_status(client, opts).await?;
            println!("\n{}", style("Service is already running").dim());
        }
        return Ok(());
    }

    // Pre-check port availability
    let port = args.port.unwrap_or(config.port);
    if !check_port_available(port) {
        let suggestion = find_next_available_port(port);
        let mut msg = format!("Port {} is already in use", port);
        if let Some(p) = suggestion {
            msg.push_str(&format!(". Try: occ start --port {}", p));
        }
        return Err(anyhow!(msg));
    }

    let spinner = if opts.quiet {
        None
    } else {
        Some(CommandSpinner::new("Starting container..."))
    };

    // Auto-build image if needed
    if !image_exists(client, IMAGE_NAME, IMAGE_TAG).await? {
        if let Some(ref s) = spinner {
            s.update("Building image (first run)...");
        }
        build_image(client, DOCKERFILE, IMAGE_TAG).await?;
    }

    // Setup and start
    if let Some(ref s) = spinner {
        s.update("Starting container...");
    }

    let container_id = match setup_and_start(client, Some(port), None).await {
        Ok(id) => id,
        Err(e) => {
            if let Some(s) = spinner {
                s.fail("Failed to start");
            }
            // Show crash logs if container exited
            if let Ok(true) = container_exists(client, CONTAINER_NAME).await {
                eprintln!("\nRecent container logs:");
                show_recent_logs(client, 20).await.ok();
            }
            return Err(e.into());
        }
    };

    // Wait for healthy
    if let Some(ref s) = spinner {
        s.update("Waiting for service to be ready...");
    }
    wait_for_ready(client, Duration::from_secs(30)).await?;

    if let Some(s) = spinner {
        s.success("Service started");
    }

    // Show result
    let url = format!("http://localhost:{}", port);
    if opts.quiet {
        println!("{}", url);
    } else {
        println!();
        println!("URL:         {}", style(&url).cyan());
        println!("Container:   {}", &container_id[..12]);
        println!("Port:        {} -> 3000", port);
        println!();
        println!("{}", style("Open in browser: occ start --open").dim());
    }

    // Auto-open browser if requested
    if args.open || config.auto_open {
        webbrowser::open(&url)?;
    }

    Ok(())
}
```

### Logs Command with All Options
```rust
// Source: https://docs.rs/bollard/latest/bollard/container/struct.LogsOptions.html
#[derive(Args)]
pub struct LogsArgs {
    /// Number of lines to show (default: 50)
    #[arg(short = 'n', long = "lines", default_value = "50")]
    lines: String,

    /// Don't follow logs (one-shot)
    #[arg(long = "no-follow")]
    no_follow: bool,

    /// Show timestamps
    #[arg(long)]
    timestamps: bool,

    /// Filter lines containing pattern
    #[arg(long)]
    grep: Option<String>,
}

pub async fn cmd_logs(
    client: &DockerClient,
    args: &LogsArgs,
    opts: &GlobalOpts,
) -> Result<(), anyhow::Error> {
    // Check container exists
    if !container_exists(client, CONTAINER_NAME).await? {
        return Err(anyhow!("No container found. Run 'occ start' first."));
    }

    let follow = !args.no_follow;

    if !opts.quiet && follow {
        eprintln!("{}", style("Following logs (Ctrl+C to exit)...").dim());
    }

    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow,
        tail: args.lines.clone(),
        timestamps: args.timestamps,
        ..Default::default()
    };

    let mut stream = client.inner().logs(CONTAINER_NAME, Some(options));
    let use_colors = console::colors_enabled();

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                let line = match output {
                    LogOutput::StdOut { message } |
                    LogOutput::StdErr { message } => {
                        String::from_utf8_lossy(&message).to_string()
                    }
                    _ => continue,
                };

                // Apply grep filter
                if let Some(ref pattern) = args.grep {
                    if !line.contains(pattern) {
                        continue;
                    }
                }

                // Print with optional color coding
                if use_colors && !opts.quiet {
                    print!("{}", log_level_style(&line));
                } else {
                    print!("{}", line);
                }
            }
            Err(_) => {
                // Container stopped or error
                if follow {
                    if !container_is_running(client, CONTAINER_NAME).await.unwrap_or(false) {
                        if !opts.quiet {
                            eprintln!("\n{}", style("Container stopped").dim());
                        }
                        break;
                    }
                }
                break;
            }
        }
    }

    Ok(())
}
```

### Stop Command with Graceful Shutdown
```rust
// Source: Project decision - 30s graceful shutdown
pub async fn cmd_stop(
    client: &DockerClient,
    _args: &StopArgs,
    opts: &GlobalOpts,
) -> Result<(), anyhow::Error> {
    // Idempotent: if not running, just confirm
    if !container_is_running(client, CONTAINER_NAME).await? {
        if !opts.quiet {
            println!("{}", style("Service is already stopped").dim());
        }
        return Ok(());
    }

    let spinner = if opts.quiet {
        None
    } else {
        Some(CommandSpinner::new("Stopping service..."))
    };

    // 30 second graceful shutdown timeout
    let timeout_secs = 30;

    if let Some(ref s) = spinner {
        s.update(&format!("Stopping service ({}s timeout)...", timeout_secs));
    }

    match stop_container(client, CONTAINER_NAME, Some(timeout_secs)).await {
        Ok(()) => {
            if let Some(s) = spinner {
                s.success("Service stopped");
            }
        }
        Err(e) => {
            if let Some(s) = spinner {
                s.fail("Failed to stop");
            }
            return Err(e.into());
        }
    }

    Ok(())
}
```

### Actionable Error Messages
```rust
// Source: Project decision - actionable errors with docs links
use opencode_cloud_core::docker::DockerError;

pub fn format_docker_error(e: &DockerError) -> String {
    match e {
        DockerError::NotRunning => {
            format!(
                "{}\n\n  {}\n  {}\n\n  {}: {}",
                style("Docker is not running").red().bold(),
                "Start Docker Desktop or the Docker daemon:",
                style("  sudo systemctl start docker").cyan(),
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::PermissionDenied => {
            format!(
                "{}\n\n  {}\n  {}\n  {}\n\n  {}: {}",
                style("Permission denied accessing Docker").red().bold(),
                "Add your user to the docker group:",
                style("  sudo usermod -aG docker $USER").cyan(),
                "Then log out and back in.",
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::Connection(msg) => {
            format!(
                "{}\n\n  {}\n\n  {}: {}",
                style("Cannot connect to Docker").red().bold(),
                msg,
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        DockerError::Container(msg) if msg.contains("port") => {
            format!(
                "{}\n\n  {}\n  {}\n\n  {}: {}",
                style("Port conflict").red().bold(),
                msg,
                style("  Try: occ start --port <different-port>").cyan(),
                style("Docs").dim(),
                style("https://github.com/pRizz/opencode-cloud#troubleshooting").dim()
            )
        }
        _ => e.to_string(),
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Raw terminal output | console/indicatif | - | Auto TTY detection, colors, spinners |
| Manual arg parsing | clap derive | clap 3.0+ (2021) | Type-safe, auto-help generation |
| Blocking log reads | Async streams | - | Non-blocking, proper Ctrl+C handling |
| `println!` for errors | stderr with color | - | Proper stream separation |

**Deprecated/outdated:**
- `clap::App` - deprecated, use `clap::Command`
- `structopt` - merged into clap; use clap derive directly
- Manual `isatty()` checks - use console's auto-detection

## Open Questions

Things that couldn't be fully resolved:

1. **Exact quiet mode output for start**
   - What we know: Should output minimal info for scripting
   - What's unclear: URL only vs nothing vs just exit code
   - Recommendation: Output URL only in quiet mode (useful for scripts like `open $(occ start -q)`)

2. **Status -q behavior**
   - What we know: Should be script-friendly
   - What's unclear: Text output vs exit code only
   - Recommendation: Exit 0 if running, exit 1 if stopped; no output

3. **Multiple container conflict**
   - What we know: User might have stale containers from old runs
   - What's unclear: Auto-remove vs error vs prompt
   - Recommendation: Error with message to run `occ stop --remove` first

## Sources

### Primary (HIGH confidence)
- [clap docs.rs](https://docs.rs/clap/latest/clap/) - Argument parsing, derive macros
- [console docs.rs](https://docs.rs/console/latest/console/) - Colors, TTY detection
- [indicatif docs.rs](https://docs.rs/indicatif/latest/indicatif/) - Spinners, progress bars
- [bollard LogsOptions](https://docs.rs/bollard/latest/bollard/container/struct.LogsOptions.html) - Container logs API
- [Rust CLI Book](https://rust-cli.github.io/book/) - Exit codes, error handling patterns

### Secondary (MEDIUM confidence)
- [webbrowser crate](https://docs.rs/webbrowser) - Browser opening
- [humantime docs](https://docs.rs/humantime) - Duration formatting
- [Rain's Rust CLI recommendations](https://rust-cli-recommendations.sunshowers.io/) - Global flags, colors

### Tertiary (LOW confidence)
- Various blog posts on terminal UX patterns (cross-verified with official docs)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Using existing workspace dependencies
- Architecture: HIGH - Patterns are well-established in Rust CLI ecosystem
- Pitfalls: HIGH - Based on actual Docker CLI behavior and common issues
- Output formatting: MEDIUM - Some UX decisions need user feedback

**Research date:** 2026-01-19
**Valid until:** 90 days (stable ecosystem; console-rs, clap changes slowly)
