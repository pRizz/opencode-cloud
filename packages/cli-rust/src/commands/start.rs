//! Start command implementation
//!
//! Starts the opencode service, building the image if needed.

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use bollard::container::{LogOutput, LogsOptions};
use clap::Args;
use console::style;
use futures_util::stream::StreamExt;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DEFAULT_PORT, DockerClient, DockerError, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT,
    ProgressReporter, build_image, container_is_running, image_exists, setup_and_start,
};
use std::net::TcpListener;

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
    // Connect to Docker
    let client = connect_docker(verbose)?;

    // Verify connection
    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{}", msg)
    })?;

    let port = args.port.unwrap_or(DEFAULT_PORT);

    // Check if already running (idempotent behavior)
    if container_is_running(&client, CONTAINER_NAME).await? {
        if !quiet {
            let url = format!("http://127.0.0.1:{}", port);
            println!("{}", style("Service is already running").dim());
            println!();
            println!("URL:        {}", style(&url).cyan());
        }
        return Ok(());
    }

    // Pre-check port availability
    if !check_port_available(port) {
        let suggestion = find_next_available_port(port);
        let mut msg = format!("Port {} is already in use", port);
        if let Some(p) = suggestion {
            msg.push_str(&format!(". Try: occ start --port {}", p));
        }
        return Err(anyhow!(msg));
    }

    // Create spinner
    let spinner = CommandSpinner::new_maybe("Starting service...", quiet);

    // Check if image exists - build if not
    if !image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await? {
        spinner.update("Building image (first run)...");

        if verbose > 0 {
            eprintln!(
                "{} Building from embedded Dockerfile",
                style("[info]").cyan()
            );
        }

        // Create a progress reporter for the build
        let mut progress = ProgressReporter::new();
        build_image(&client, Some(IMAGE_TAG_DEFAULT), &mut progress).await?;
    }

    // Start container
    spinner.update("Starting container...");

    let container_id = match setup_and_start(&client, Some(port), None).await {
        Ok(id) => id,
        Err(e) => {
            spinner.fail("Failed to start");
            show_docker_error(&e);
            // Try to show last logs if container exists
            if let Ok(true) =
                opencode_cloud_core::docker::container::container_exists(&client, CONTAINER_NAME)
                    .await
            {
                eprintln!();
                eprintln!("{}", style("Recent container logs:").yellow());
                show_recent_logs(&client, 20).await;
            }
            return Err(e.into());
        }
    };

    spinner.success("Service started");

    // Show result
    let url = format!("http://127.0.0.1:{}", port);
    if quiet {
        println!("{}", url);
    } else {
        println!();
        println!("URL:        {}", style(&url).cyan());
        println!(
            "Container:  {}",
            style(&container_id[..12.min(container_id.len())]).dim()
        );
        println!("Port:       {} -> 3000", port);
        println!();
        println!("{}", style("Open in browser: occ start --open").dim());
    }

    // Open browser if requested
    if args.open {
        if let Err(e) = webbrowser::open(&url) {
            eprintln!(
                "{} Failed to open browser: {}",
                style("Warning:").yellow(),
                e
            );
        }
    }

    Ok(())
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

    while let Some(result) = stream.next().await {
        if count >= lines {
            break;
        }
        match result {
            Ok(output) => {
                let line = match output {
                    LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                        String::from_utf8_lossy(&message).to_string()
                    }
                    _ => continue,
                };
                eprint!("  {}", line);
                count += 1;
            }
            Err(_) => break,
        }
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
