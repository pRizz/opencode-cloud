//! Start command implementation
//!
//! Starts the opencode service, building the image if needed.

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use futures_util::stream::StreamExt;
use opencode_cloud_core::bollard::container::{LogOutput, LogsOptions};
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, DockerError, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT,
    ProgressReporter, build_image, container_exists, container_is_running, image_exists,
    setup_and_start, stop_service,
};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// Arguments for the start command
#[derive(Args)]
pub struct StartArgs {
    /// Port to bind on host (default: 3000)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Open browser after starting
    #[arg(long)]
    pub open: bool,

    /// Run in foreground (for service managers like systemd/launchd)
    /// Note: This is the default behavior; flag exists for compatibility
    #[arg(long)]
    pub no_daemon: bool,

    /// Rebuild Docker image using cache (fast, picks up Dockerfile changes)
    #[arg(long)]
    pub cached_rebuild: bool,

    /// Rebuild Docker image from scratch without cache (slow, for troubleshooting)
    #[arg(long)]
    pub full_rebuild: bool,
}

/// Start the opencode service
///
/// This command:
/// 1. Connects to Docker
/// 2. Checks if service is already running (idempotent)
/// 3. Checks port availability
/// 4. Builds image if needed (first run)
/// 5. Creates and starts the container
/// 6. Shows URL and container info
pub async fn cmd_start(args: &StartArgs, quiet: bool, verbose: u8) -> Result<()> {
    let client = connect_docker(verbose)?;

    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{}", msg)
    })?;

    // Load config for port and bind_address
    let config = opencode_cloud_core::config::load_config()?;
    let port = args.port.unwrap_or(config.opencode_web_port);
    let bind_addr = &config.bind_address;

    // Validate config before starting
    match opencode_cloud_core::config::validate_config(&config) {
        Ok(warnings) => {
            for warning in warnings {
                opencode_cloud_core::config::display_validation_warning(&warning);
            }
        }
        Err(error) => {
            opencode_cloud_core::config::display_validation_error(&error);
            return Err(anyhow::anyhow!(
                "Configuration invalid. Fix the error above and try again."
            ));
        }
    }

    let any_rebuild = args.cached_rebuild || args.full_rebuild;

    // Security check: block first start without security configured
    let is_first_start = !container_exists(&client, CONTAINER_NAME).await?;

    if is_first_start && config.users.is_empty() && !config.allow_unauthenticated_network {
        return Err(anyhow!(
            "{}\n\n\
             No users are configured for authentication.\n\
             The service cannot start without security configured.\n\n\
             Quick setup:\n  {}\n\n\
             Or allow unauthenticated access (not recommended):\n  {}",
            style("Security not configured").red().bold(),
            style("occ setup").cyan(),
            style("occ config set allow_unauthenticated_network true").dim()
        ));
    }

    // Handle rebuild: remove existing container so a new one is created from the new image
    if any_rebuild {
        handle_rebuild(&client, verbose).await?;
    } else if container_is_running(&client, CONTAINER_NAME).await? {
        // Already running (idempotent behavior) - only when not rebuilding
        return show_already_running(port, bind_addr, config.is_network_exposed(), quiet);
    }

    // Security check: warn if network exposed without authentication
    if !quiet
        && config.is_network_exposed()
        && config.users.is_empty()
        && !config.allow_unauthenticated_network
    {
        eprintln!();
        eprintln!(
            "{} {}",
            style("WARNING:").yellow().bold(),
            style("Network exposed without authentication!").yellow()
        );
        eprintln!();
        eprintln!(
            "The service is bound to {} but no users are configured.",
            style(bind_addr).cyan()
        );
        eprintln!("Anyone on your network can access the web UI without authentication.");
        eprintln!();
        eprintln!("To add a user: {}", style("occ user add").cyan());
        eprintln!(
            "To suppress this warning: {}",
            style("occ config set allow_unauthenticated_network true").cyan()
        );
        eprintln!();
    }

    // Pre-check port availability
    if !check_port_available(port) {
        return Err(port_in_use_error(port));
    }

    // Build image if needed (first run or rebuild)
    let needs_build =
        any_rebuild || !image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await?;

    if needs_build {
        // full_rebuild uses no_cache, cached_rebuild uses cache
        build_docker_image(&client, args.full_rebuild, verbose).await?;
    }

    // Start container
    let spinner = CommandSpinner::new_maybe("Starting container...", quiet);
    let container_id = match start_container(
        &client,
        port,
        bind_addr,
        config.cockpit_port,
        config.cockpit_enabled,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            spinner.fail("Failed to start container");
            show_docker_error(&e);
            show_logs_if_container_exists(&client).await;
            return Err(e.into());
        }
    };

    // Wait for service to be ready
    if let Err(e) = wait_for_service_ready(&client, port, &spinner).await {
        spinner.fail("Service failed to become ready");
        eprintln!();
        eprintln!("{}", style("Recent container logs:").yellow());
        show_recent_logs(&client, 20).await;
        return Err(e);
    }

    spinner.success("Service started and ready");

    // Show result and optionally open browser
    show_start_result(
        &container_id,
        port,
        bind_addr,
        config.is_network_exposed(),
        quiet,
    );
    open_browser_if_requested(args.open, port, bind_addr);

    Ok(())
}

/// Handle rebuild flags: remove existing container so a new one is created from the new image
async fn handle_rebuild(client: &DockerClient, verbose: u8) -> Result<()> {
    let exists =
        opencode_cloud_core::docker::container::container_exists(client, CONTAINER_NAME).await?;

    if !exists {
        return Ok(());
    }

    if verbose > 0 {
        eprintln!(
            "{} Removing existing container for rebuild...",
            style("[info]").cyan()
        );
    }

    // Ignore errors if container doesn't exist
    stop_service(client, true).await.ok();
    Ok(())
}

/// Show message when service is already running
fn show_already_running(port: u16, bind_addr: &str, is_exposed: bool, quiet: bool) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let url = format!("http://{}:{}", bind_addr, port);
    println!("{}", style("Service is already running").dim());
    println!();
    println!("URL:        {}", style(&url).cyan());

    // Show security status
    if is_exposed {
        println!("Security:   {}", style("[NETWORK EXPOSED]").yellow().bold());
    } else {
        println!("Security:   {}", style("[LOCAL ONLY]").green().bold());
    }
    Ok(())
}

/// Create error message for port already in use
fn port_in_use_error(port: u16) -> anyhow::Error {
    let mut msg = format!("Port {} is already in use", port);
    if let Some(p) = find_next_available_port(port) {
        msg.push_str(&format!(". Try: occ start --port {}", p));
    }
    anyhow!(msg)
}

/// Build the Docker image with progress reporting
///
/// If `no_cache` is true, builds from scratch ignoring Docker layer cache.
/// Otherwise uses cached layers for faster builds.
async fn build_docker_image(client: &DockerClient, no_cache: bool, verbose: u8) -> Result<()> {
    if verbose > 0 {
        let action = if no_cache {
            "Full rebuilding Docker image"
        } else {
            "Building Docker image"
        };
        let cache_note = if no_cache {
            " (no cache)"
        } else {
            " (using cache)"
        };
        eprintln!(
            "{} {} from embedded Dockerfile{}",
            style("[info]").cyan(),
            action,
            cache_note
        );
    }

    let context = if no_cache {
        "Full rebuilding Docker image (no cache)"
    } else {
        "Building Docker image"
    };

    let mut progress = ProgressReporter::with_context(context);
    build_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress, no_cache).await?;
    Ok(())
}

/// Start the container, returning the container ID or error
async fn start_container(
    client: &DockerClient,
    port: u16,
    bind_address: &str,
    cockpit_port: u16,
    cockpit_enabled: bool,
) -> Result<String, DockerError> {
    setup_and_start(
        client,
        Some(port),
        None,
        Some(bind_address),
        Some(cockpit_port),
        Some(cockpit_enabled),
    )
    .await
}

/// Show recent logs if the container exists (for debugging failures)
async fn show_logs_if_container_exists(client: &DockerClient) {
    let Ok(true) =
        opencode_cloud_core::docker::container::container_exists(client, CONTAINER_NAME).await
    else {
        return;
    };

    eprintln!();
    eprintln!("{}", style("Recent container logs:").yellow());
    show_recent_logs(client, 20).await;
}

/// Display the start result
fn show_start_result(
    container_id: &str,
    port: u16,
    bind_addr: &str,
    is_exposed: bool,
    quiet: bool,
) {
    let url = format!("http://{}:{}", bind_addr, port);

    if quiet {
        println!("{}", url);
        return;
    }

    println!();
    println!("URL:        {}", style(&url).cyan());
    println!(
        "Container:  {}",
        style(&container_id[..12.min(container_id.len())]).dim()
    );
    println!("Port:       {} -> 3000", port);

    // Show security status
    if is_exposed {
        println!("Security:   {}", style("[NETWORK EXPOSED]").yellow().bold());
        println!(
            "            {}",
            style("Accessible on all network interfaces").dim()
        );
    } else {
        println!("Security:   {}", style("[LOCAL ONLY]").green().bold());
    }

    println!();
    println!("{}", style("Open in browser: occ start --open").dim());
}

/// Open browser if requested
fn open_browser_if_requested(should_open: bool, port: u16, bind_addr: &str) {
    if !should_open {
        return;
    }

    // For network-exposed addresses like 0.0.0.0, use localhost for browser
    let browser_addr = if bind_addr == "0.0.0.0" || bind_addr == "::" {
        "127.0.0.1"
    } else {
        bind_addr
    };
    let url = format!("http://{}:{}", browser_addr, port);
    if let Err(e) = webbrowser::open(&url) {
        eprintln!(
            "{} Failed to open browser: {}",
            style("Warning:").yellow(),
            e
        );
    }
}

/// Connect to Docker with actionable error messages
fn connect_docker(verbose: u8) -> Result<DockerClient> {
    if verbose > 0 {
        eprintln!("{} Connecting to Docker...", style("[info]").cyan());
    }

    DockerClient::new().map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{}", msg)
    })
}

/// Format Docker errors with actionable guidance
fn format_docker_error(e: &DockerError) -> String {
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

/// Show Docker error in a rich format
fn show_docker_error(e: &DockerError) {
    let msg = format_docker_error(e);
    eprintln!();
    eprintln!("{}", msg);
}

/// Check if a port is available for binding
fn check_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find the next available port starting from the given port
fn find_next_available_port(start: u16) -> Option<u16> {
    (start..start.saturating_add(100)).find(|&p| check_port_available(p))
}

/// Configuration for health check waiting
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 10;
const HEALTH_CHECK_INTERVAL_MS: u64 = 500;
const HEALTH_CHECK_CONSECUTIVE_REQUIRED: u32 = 3;

/// Known fatal error patterns in container logs that indicate immediate failure
const FATAL_ERROR_PATTERNS: &[&str] = &[
    "exec opencode failed",      // tini: binary not found
    "exec failed",               // general exec failure
    "[FATAL tini",               // tini fatal errors
    "No such file or directory", // missing binary
    "permission denied",         // permission issues
    "cannot execute binary",     // exec format error
];

/// Check container logs for fatal errors that indicate the service cannot start
async fn check_for_fatal_errors(client: &DockerClient) -> Option<String> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: "20".to_string(),
        ..Default::default()
    };

    let mut stream = client.inner().logs(CONTAINER_NAME, Some(options));
    let mut logs = Vec::new();

    while let Some(Ok(output)) = stream.next().await {
        let line = match output {
            LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                String::from_utf8_lossy(&message).to_string()
            }
            _ => continue,
        };
        logs.push(line);
    }

    // Check for fatal error patterns
    logs.iter().find_map(|log_line| {
        let lower = log_line.to_lowercase();
        FATAL_ERROR_PATTERNS
            .iter()
            .any(|pattern| lower.contains(&pattern.to_lowercase()))
            .then(|| log_line.trim().to_string())
    })
}

/// Wait for the service to be ready by checking TCP connectivity
///
/// Returns Ok(()) when the service is ready, or Err if timeout is reached or fatal error detected.
/// Requires multiple consecutive successful connections to avoid false positives.
/// Also monitors container logs for fatal errors to fail fast.
async fn wait_for_service_ready(
    client: &DockerClient,
    port: u16,
    spinner: &CommandSpinner,
) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS);
    let interval = Duration::from_millis(HEALTH_CHECK_INTERVAL_MS);
    let log_check_interval = Duration::from_secs(1);

    let mut consecutive_success = 0;
    let mut last_log_check = Instant::now();

    spinner.update("Waiting for service to be ready...");

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Service did not become ready within {} seconds. Check logs with: occ logs",
                HEALTH_CHECK_TIMEOUT_SECS
            ));
        }

        // Periodically check logs for fatal errors (every 1 second)
        if last_log_check.elapsed() > log_check_interval {
            if let Some(error) = check_for_fatal_errors(client).await {
                return Err(anyhow!(
                    "Fatal error detected in container:\n  {}\n\nThe service cannot start. Try rebuilding the Docker image: occ start --full-rebuild",
                    error
                ));
            }
            last_log_check = Instant::now();
        }

        // Try to connect to the service
        let addr = format!("127.0.0.1:{}", port).parse().unwrap();
        let connected = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok();

        if connected {
            consecutive_success += 1;
            if consecutive_success >= HEALTH_CHECK_CONSECUTIVE_REQUIRED {
                return Ok(());
            }
            spinner.update(&format!(
                "Service responding ({}/{})",
                consecutive_success, HEALTH_CHECK_CONSECUTIVE_REQUIRED
            ));
        } else {
            consecutive_success = 0;
            spinner.update(&format!(
                "Waiting for service to be ready... ({}s)",
                start.elapsed().as_secs()
            ));
        }

        tokio::time::sleep(interval).await;
    }
}

/// Show recent container logs for debugging
async fn show_recent_logs(client: &DockerClient, lines: usize) {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: lines.to_string(),
        ..Default::default()
    };

    let mut stream = client.inner().logs(CONTAINER_NAME, Some(options));
    let mut count = 0;

    while let Some(Ok(output)) = stream.next().await {
        if count >= lines {
            break;
        }

        let line = match output {
            LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                String::from_utf8_lossy(&message).to_string()
            }
            _ => continue,
        };

        eprint!("  {}", line);
        count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_check_returns_false_for_privileged_ports() {
        // Port 1 is privileged and typically unavailable
        // This test may pass if run as root, but that's unlikely in dev
        // Instead, test the logic with a known available port
        assert!(!check_port_available(1)); // Privileged, should fail on non-root
    }

    #[test]
    fn find_next_port_finds_available_port() {
        // This should find something in the 49152-49252 range (dynamic ports)
        let result = find_next_available_port(49152);
        assert!(result.is_some());
    }
}
