//! Start command implementation
//!
//! Starts the opencode service, building the image if needed.

use crate::output::{
    CommandSpinner, format_cockpit_url, format_docker_error, normalize_bind_addr,
    resolve_remote_addr, show_docker_error,
};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use futures_util::stream::StreamExt;
use opencode_cloud_core::bollard::container::{LogOutput, LogsOptions};
use opencode_cloud_core::config::save_config;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, DockerError, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT, ImageState,
    ParsedMount, ProgressReporter, build_image, check_container_path_warning, container_exists,
    container_is_running, get_cli_version, get_container_bind_mounts, get_container_ports,
    get_image_version, image_exists, pull_image, save_state, setup_and_start, stop_service,
    validate_mount_path, versions_compatible,
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

    /// Pull prebuilt image from registry (fast, ~2 min)
    #[arg(long)]
    pub pull_sandbox_image: bool,

    /// Rebuild Docker image using cache (picks up Dockerfile changes)
    #[arg(long)]
    pub cached_rebuild_sandbox_image: bool,

    /// Rebuild Docker image from scratch without cache (slow, 30-60 min)
    #[arg(long)]
    pub full_rebuild_sandbox_image: bool,

    /// Skip version compatibility check between CLI and Docker image
    #[arg(long)]
    pub ignore_version: bool,

    /// Skip checking for updates on start
    #[arg(long)]
    pub no_update_check: bool,

    /// Add one-time bind mount (can be specified multiple times)
    /// Format: /host/path:/container/path[:ro]
    #[arg(long = "mount", action = clap::ArgAction::Append)]
    pub mounts: Vec<String>,

    /// Skip configured mounts (only use --mount flags if specified)
    #[arg(long)]
    pub no_mounts: bool,
}

/// Collect and validate bind mounts from config and CLI flags
fn collect_bind_mounts(
    config: &opencode_cloud_core::Config,
    cli_mounts: &[String],
    no_mounts: bool,
    quiet: bool,
) -> Result<Vec<ParsedMount>> {
    let mut all_mounts = Vec::new();

    // Add config mounts unless --no-mounts
    if !no_mounts {
        for mount_str in &config.mounts {
            let parsed = ParsedMount::parse(mount_str)
                .map_err(|e| anyhow!("Invalid config mount '{mount_str}': {e}"))?;
            all_mounts.push(parsed);
        }
    }

    // Add CLI mounts (always, even with --no-mounts)
    for mount_str in cli_mounts {
        let parsed = ParsedMount::parse(mount_str)
            .map_err(|e| anyhow!("Invalid mount '{mount_str}': {e}"))?;
        all_mounts.push(parsed);
    }

    // Validate all mount paths exist
    for parsed in &all_mounts {
        if let Err(e) = validate_mount_path(&parsed.host_path) {
            return Err(anyhow!(
                "Mount path validation failed for '{}':\n  {}\n\nDid the directory move? Run: occ mount remove {}",
                parsed.host_path.display(),
                e,
                parsed.host_path.display()
            ));
        }

        // Show warnings for system paths (non-blocking)
        if !quiet {
            if let Some(warning) = check_container_path_warning(&parsed.container_path) {
                eprintln!("{}", style(&warning).yellow());
            }
        }
    }

    Ok(all_mounts)
}

/// Compare container bind mounts with configured mounts
///
/// Returns true if the mounts match (ignoring order).
/// Handles macOS path translation (/tmp -> /private/tmp -> /host_mnt/private/tmp).
fn mounts_equal(
    current: &[opencode_cloud_core::docker::ContainerBindMount],
    configured: &[ParsedMount],
) -> bool {
    // Different count means definitely not equal
    if current.len() != configured.len() {
        return false;
    }

    // For each configured mount, check if there's a matching current mount
    for conf in configured {
        let conf_host = conf.host_path.to_string_lossy();
        let conf_container = &conf.container_path;

        let found = current.iter().any(|cur| {
            // Check container path matches
            if cur.target != *conf_container {
                return false;
            }

            // Check read-only flag matches
            if cur.read_only != conf.read_only {
                return false;
            }

            // Check host path matches (handling macOS path translation)
            // Docker on macOS translates paths: /tmp -> /private/tmp -> /host_mnt/private/tmp
            let cur_source = &cur.source;
            let conf_host_str = conf_host.as_ref();

            // Direct match
            if cur_source == conf_host_str {
                return true;
            }

            // Handle /host_mnt prefix from Docker Desktop
            if let Some(stripped) = cur_source.strip_prefix("/host_mnt") {
                if stripped == conf_host_str {
                    return true;
                }
                // Also check /private prefix for /tmp -> /private/tmp
                if let Some(private_stripped) = stripped.strip_prefix("/private") {
                    if private_stripped == conf_host_str {
                        return true;
                    }
                }
            }

            // Handle /private prefix (macOS symlink: /tmp -> /private/tmp)
            if let Some(private_path) = conf_host_str.strip_prefix("/private") {
                if cur_source.ends_with(private_path) {
                    return true;
                }
            }

            false
        });

        if !found {
            return false;
        }
    }

    true
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
pub async fn cmd_start(
    args: &StartArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    // Resolve Docker client (local or remote)
    let (client, host_name) = crate::resolve_docker_client(maybe_host).await?;

    if verbose > 0 {
        let target = host_name.as_deref().unwrap_or("local");
        eprintln!(
            "{} Connecting to Docker on {}...",
            style("[info]").cyan(),
            target
        );
    }

    client.verify_connection().await.map_err(|e| {
        let msg = format_docker_error(&e);
        anyhow!("{msg}")
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

    // Collect and validate bind mounts
    let bind_mounts = collect_bind_mounts(&config, &args.mounts, args.no_mounts, quiet)?;
    let bind_mounts_option = if bind_mounts.is_empty() {
        None
    } else {
        Some(bind_mounts)
    };

    // Check mutual exclusivity of image flags
    let image_flags = [
        args.pull_sandbox_image,
        args.cached_rebuild_sandbox_image,
        args.full_rebuild_sandbox_image,
    ];
    let flag_count = image_flags.iter().filter(|&&f| f).count();
    if flag_count > 1 {
        return Err(anyhow!(
            "Only one of --pull-sandbox-image, --cached-rebuild-sandbox-image, or --full-rebuild-sandbox-image can be specified"
        ));
    }

    let has_image_flag = args.pull_sandbox_image
        || args.cached_rebuild_sandbox_image
        || args.full_rebuild_sandbox_image;

    // If any image flag is used while container is running, prompt to stop
    if has_image_flag && container_is_running(&client, CONTAINER_NAME).await? {
        if quiet {
            return Err(anyhow!(
                "Container is running. Stop it first with: occ stop"
            ));
        }
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Container is running. Stop and apply image change?")
            .default(false)
            .interact()?;
        if !confirm {
            return Err(anyhow!("Aborted. Stop container first with: occ stop"));
        }
        // Stop the container
        stop_service(&client, true, None).await.ok();
    }

    let mut any_rebuild = args.cached_rebuild_sandbox_image || args.full_rebuild_sandbox_image;

    // Determine image source: flag > config default
    let mut use_prebuilt = if args.pull_sandbox_image {
        true
    } else if any_rebuild {
        false
    } else {
        config.image_source == "prebuilt"
    };

    // Version compatibility check (skip if rebuilding, --ignore-version, or --no-update-check)
    let should_check_version = !args.ignore_version
        && !any_rebuild
        && !args.no_update_check
        && config.update_check != "never"
        && !quiet;

    if should_check_version {
        let cli_version = get_cli_version();
        let image_tag = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");

        // Only check if image exists
        if image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await? {
            if let Ok(Some(image_version)) = get_image_version(&client, &image_tag).await {
                if !versions_compatible(cli_version, Some(&image_version)) {
                    println!();
                    println!("{} Version mismatch detected", style("⚠").yellow());
                    println!("  CLI version:   {}", style(cli_version).cyan());
                    println!("  Image version: {}", style(&image_version).cyan());
                    println!();

                    let selection = dialoguer::Select::new()
                        .with_prompt("What would you like to do?")
                        .items(&[
                            "Rebuild image from source (recommended)",
                            "Continue with mismatched versions",
                        ])
                        .default(0)
                        .interact()?;

                    if selection == 0 {
                        any_rebuild = true;
                    }
                    // selection == 1 means continue anyway
                }
            }
        }
    }

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

    // Check for port mismatch on existing container
    if !is_first_start && !any_rebuild {
        let current_ports = get_container_ports(&client, CONTAINER_NAME).await?;
        let current_opencode_port = current_ports.opencode_port.unwrap_or(3000);
        let current_cockpit_port = current_ports.cockpit_port.unwrap_or(9090);

        let port_mismatch = current_opencode_port != port;
        let cockpit_mismatch = current_cockpit_port != config.cockpit_port;

        if port_mismatch || cockpit_mismatch {
            if quiet {
                // In quiet mode, fail with clear error
                return Err(anyhow!(
                    "Port mismatch: container uses port {current_opencode_port} but requested port {port}.\n\
                     Container must be recreated to change ports.\n\
                     Run without --quiet to be prompted, or manually remove with:\n  \
                     occ stop && docker rm {CONTAINER_NAME}"
                ));
            }

            eprintln!();
            eprintln!(
                "{} {}",
                style("Port mismatch detected:").yellow().bold(),
                style("Container must be recreated to change ports.").yellow()
            );
            if port_mismatch {
                eprintln!(
                    "  opencode port: {} (current) → {} (requested)",
                    style(current_opencode_port).red(),
                    style(port).green()
                );
            }
            if cockpit_mismatch {
                eprintln!(
                    "  cockpit port: {} (current) → {} (requested)",
                    style(current_cockpit_port).red(),
                    style(config.cockpit_port).green()
                );
            }
            eprintln!();
            eprintln!(
                "{}",
                style("This will stop and recreate the container from the existing image.").dim()
            );
            eprintln!("{}", style("Your data volumes will be preserved.").dim());
            eprintln!();

            let confirm = dialoguer::Confirm::new()
                .with_prompt("Recreate container with new port(s)?")
                .default(true)
                .interact()?;

            if confirm {
                any_rebuild = true;
            } else {
                return Err(anyhow!(
                    "Container not recreated. To use port {port}, run:\n  occ stop && docker rm {CONTAINER_NAME} && occ start --port {port}"
                ));
            }
        }
    }

    // Check for mount mismatch on existing container (only if not already rebuilding)
    if !is_first_start && !any_rebuild {
        let current_mounts = get_container_bind_mounts(&client, CONTAINER_NAME).await?;

        // Get configured mounts for comparison
        let configured_mounts = bind_mounts_option.as_deref().unwrap_or(&[]);

        // Compare mounts: check if they match (order doesn't matter)
        let mounts_match = mounts_equal(&current_mounts, configured_mounts);

        if !mounts_match {
            if quiet {
                return Err(anyhow!(
                    "Mount configuration changed. Container must be recreated to apply mount changes.\n\
                     Run without --quiet to be prompted, or manually remove with:\n  \
                     occ stop && docker rm {CONTAINER_NAME}"
                ));
            }

            eprintln!();
            eprintln!(
                "{} {}",
                style("Mount configuration changed:").yellow().bold(),
                style("Container must be recreated to apply mount changes.").yellow()
            );
            eprintln!();

            // Show current mounts
            if current_mounts.is_empty() {
                eprintln!("  Current mounts: {}", style("(none)").dim());
            } else {
                eprintln!("  Current mounts:");
                for m in &current_mounts {
                    let ro = if m.read_only { ":ro" } else { "" };
                    eprintln!("    - {}:{}{}", m.source, m.target, ro);
                }
            }

            // Show configured mounts
            if configured_mounts.is_empty() {
                eprintln!("  Configured mounts: {}", style("(none)").dim());
            } else {
                eprintln!("  Configured mounts:");
                for m in configured_mounts {
                    let ro = if m.read_only { ":ro" } else { "" };
                    eprintln!("    - {}:{}{}", m.host_path.display(), m.container_path, ro);
                }
            }

            eprintln!();
            eprintln!(
                "{}",
                style("This will stop and recreate the container from the existing image.").dim()
            );
            eprintln!("{}", style("Your data volumes will be preserved.").dim());
            eprintln!();

            let confirm = dialoguer::Confirm::new()
                .with_prompt("Recreate container with new mount configuration?")
                .default(true)
                .interact()?;

            if confirm {
                any_rebuild = true;
            } else {
                return Err(anyhow!(
                    "Container not recreated. To apply mount changes, run:\n  occ stop && docker rm {CONTAINER_NAME} && occ start"
                ));
            }
        }
    }

    // Handle rebuild: remove existing container so a new one is created from the new image
    if any_rebuild {
        handle_rebuild(&client, verbose).await?;
    } else if container_is_running(&client, CONTAINER_NAME).await? {
        // Already running (idempotent behavior) - only when not rebuilding
        return show_already_running(
            port,
            bind_addr,
            config.is_network_exposed(),
            quiet,
            host_name.as_deref(),
        );
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

    // First-run image source prompt (if no image and no flag specified)
    let image_already_exists = image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await?;
    if !image_already_exists && !has_image_flag && !quiet {
        let (new_use_prebuilt, updated_config) = prompt_image_source_choice(&config)?;
        // Save config with new image_source
        if updated_config.image_source != config.image_source {
            save_config(&updated_config)?;
        }
        // Use the choice for this run
        use_prebuilt = new_use_prebuilt;
    }

    // Acquire image if needed (first run, rebuild, or forced pull)
    let needs_image = any_rebuild
        || args.pull_sandbox_image
        || !image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await?;

    if needs_image {
        if any_rebuild {
            // Build from source
            build_docker_image(&client, args.full_rebuild_sandbox_image, verbose).await?;
            save_state(&ImageState::built(get_cli_version())).ok();
        } else if use_prebuilt {
            // Pull prebuilt image
            match pull_docker_image(&client, verbose).await {
                Ok(registry) => {
                    save_state(&ImageState::prebuilt(get_cli_version(), &registry)).ok();
                }
                Err(e) => {
                    // Pull failed - offer to build instead
                    if !quiet {
                        eprintln!();
                        eprintln!(
                            "{} Failed to pull prebuilt image: {e}",
                            style("Error:").red().bold()
                        );
                        eprintln!();
                        let build_instead = dialoguer::Confirm::new()
                            .with_prompt("Build from source instead? (This takes 30-60 minutes)")
                            .default(true)
                            .interact()?;
                        if build_instead {
                            build_docker_image(&client, false, verbose).await?;
                            save_state(&ImageState::built(get_cli_version())).ok();
                        } else {
                            return Err(anyhow!(
                                "Cannot proceed without image. Run 'occ start --full-rebuild-sandbox-image' to build from source."
                            ));
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            // Build from source (config.image_source == "build")
            build_docker_image(&client, false, verbose).await?;
            save_state(&ImageState::built(get_cli_version())).ok();
        }
    }

    // Start container
    let msg = crate::format_host_message(host_name.as_deref(), "Starting container...");
    let spinner = CommandSpinner::new_maybe(&msg, quiet);
    let container_id = match start_container(
        &client,
        port,
        bind_addr,
        config.cockpit_port,
        config.cockpit_enabled,
        bind_mounts_option,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            spinner.fail(&crate::format_host_message(
                host_name.as_deref(),
                "Failed to start container",
            ));
            show_docker_error(&e);
            show_logs_if_container_exists(&client).await;
            return Err(e.into());
        }
    };

    // Wait for service to be ready
    if let Err(e) = wait_for_service_ready(&client, port, &spinner, host_name.as_deref()).await {
        spinner.fail(&crate::format_host_message(
            host_name.as_deref(),
            "Service failed to become ready",
        ));
        eprintln!();
        eprintln!("{}", style("Recent container logs:").yellow());
        show_recent_logs(&client, 20).await;
        return Err(e);
    }

    spinner.success(&crate::format_host_message(
        host_name.as_deref(),
        "Service started and ready",
    ));

    // Show result and optionally open browser
    show_start_result(
        &container_id,
        port,
        bind_addr,
        config.is_network_exposed(),
        quiet,
        host_name.as_deref(),
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
    stop_service(client, true, None).await.ok();
    Ok(())
}

/// Show message when service is already running
fn show_already_running(
    port: u16,
    bind_addr: &str,
    is_exposed: bool,
    quiet: bool,
    host_name: Option<&str>,
) -> Result<()> {
    if quiet {
        return Ok(());
    }

    // Get remote host address if using --host
    let maybe_remote_addr = resolve_remote_addr(host_name);

    let msg = crate::format_host_message(host_name, "Service is already running");
    println!("{}", style(msg).dim());
    println!();

    // Show URL - use remote address if available
    if let Some(ref remote_addr) = maybe_remote_addr {
        let remote_url = format!("http://{remote_addr}:{port}");
        println!("Remote URL: {}", style(&remote_url).cyan());
    } else {
        let url = format!("http://{bind_addr}:{port}");
        println!("URL:        {}", style(&url).cyan());
    }

    // Show Cockpit URL if enabled
    if let Ok(config) = opencode_cloud_core::config::load_config() {
        if config.cockpit_enabled {
            let cockpit_url =
                format_cockpit_url(maybe_remote_addr.as_deref(), bind_addr, config.cockpit_port);
            println!("Cockpit:    {cockpit_url} (web admin)");
        }
    }

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
    let mut msg = format!("Port {port} is already in use");
    if let Some(p) = find_next_available_port(port) {
        msg.push_str(&format!(". Try: occ start --port {p}"));
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

/// Pull the Docker image with progress reporting
/// Returns the registry name on success (for provenance tracking)
async fn pull_docker_image(client: &DockerClient, verbose: u8) -> Result<String> {
    if verbose > 0 {
        eprintln!(
            "{} Pulling prebuilt Docker image from registry...",
            style("[info]").cyan()
        );
    }

    let mut progress = ProgressReporter::with_context("Pulling prebuilt image");
    let full_image = pull_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress).await?;

    // Extract registry from full image name
    let registry = if full_image.starts_with("ghcr.io") {
        "ghcr.io"
    } else if full_image.starts_with("docker.io") || full_image.starts_with("prizz/") {
        "docker.io"
    } else {
        "unknown"
    };

    Ok(registry.to_string())
}

/// Prompt user to choose between prebuilt and build from source
fn prompt_image_source_choice(
    config: &opencode_cloud_core::Config,
) -> Result<(bool, opencode_cloud_core::Config)> {
    println!();
    println!("{}", style("Docker Image Setup").cyan().bold());
    println!("{}", style("=".repeat(20)).dim());
    println!();
    println!("Choose how to get the opencode-cloud Docker image:");
    println!();
    println!("  {} Pull prebuilt image (~2 min)", style("[1]").bold());
    println!("      Fast download from GitHub Container Registry");
    println!("      Published automatically, verified builds");
    println!();
    println!("  {} Build from source (30-60 min)", style("[2]").bold());
    println!("      Compile locally for customization/auditing");
    println!("      Full transparency, modify Dockerfile if needed");
    println!();
    println!(
        "{}",
        style("Transparency: https://github.com/pRizz/opencode-cloud/actions/workflows/version-bump.yml").dim()
    );
    println!();

    let options = vec![
        "Pull prebuilt image (recommended, ~2 min)",
        "Build from source (30-60 min)",
    ];

    let selection = dialoguer::Select::new()
        .with_prompt("Select image source")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|_| anyhow!("Setup cancelled"))?;

    let use_prebuilt = selection == 0;
    let mut new_config = config.clone();
    new_config.image_source = if use_prebuilt { "prebuilt" } else { "build" }.to_string();

    println!();
    if use_prebuilt {
        println!(
            "{}",
            style("Using prebuilt image. You can change this later with:").dim()
        );
        println!("  {}", style("occ config set image_source build").cyan());
    } else {
        println!(
            "{}",
            style("Building from source. You can change this later with:").dim()
        );
        println!("  {}", style("occ config set image_source prebuilt").cyan());
    }
    println!();

    Ok((use_prebuilt, new_config))
}

/// Start the container, returning the container ID or error
async fn start_container(
    client: &DockerClient,
    port: u16,
    bind_address: &str,
    cockpit_port: u16,
    cockpit_enabled: bool,
    bind_mounts: Option<Vec<ParsedMount>>,
) -> Result<String, DockerError> {
    setup_and_start(
        client,
        Some(port),
        None,
        Some(bind_address),
        Some(cockpit_port),
        Some(cockpit_enabled),
        bind_mounts,
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
    host_name: Option<&str>,
) {
    // Get remote host address if using --host
    let maybe_remote_addr = resolve_remote_addr(host_name);

    if quiet {
        if let Some(ref remote_addr) = maybe_remote_addr {
            println!("http://{remote_addr}:{port}");
        } else {
            println!("http://{bind_addr}:{port}");
        }
        return;
    }

    println!();

    // Show URL - use remote address if available
    if let Some(ref remote_addr) = maybe_remote_addr {
        let remote_url = format!("http://{remote_addr}:{port}");
        println!("Remote URL: {}", style(&remote_url).cyan());
    } else {
        let url = format!("http://{bind_addr}:{port}");
        println!("URL:        {}", style(&url).cyan());
    }

    println!(
        "Container:  {}",
        style(&container_id[..12.min(container_id.len())]).dim()
    );
    println!("Port:       {port} -> 3000");

    // Show Cockpit availability if enabled
    if let Ok(config) = opencode_cloud_core::config::load_config() {
        if config.cockpit_enabled {
            let cockpit_url =
                format_cockpit_url(maybe_remote_addr.as_deref(), bind_addr, config.cockpit_port);
            println!("Cockpit:    {cockpit_url} (web admin)");
        }
    }

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
    if host_name.is_none() {
        println!("{}", style("Open in browser: occ start --open").dim());
    }
}

/// Open browser if requested
fn open_browser_if_requested(should_open: bool, port: u16, bind_addr: &str) {
    if !should_open {
        return;
    }

    // For network-exposed addresses like 0.0.0.0, use localhost for browser
    let browser_addr = normalize_bind_addr(bind_addr);
    let url = format!("http://{browser_addr}:{port}");
    if let Err(e) = webbrowser::open(&url) {
        eprintln!(
            "{} Failed to open browser: {}",
            style("Warning:").yellow(),
            e
        );
    }
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
/// Note: 60 seconds allows time for systemd to boot and start all services
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 60;
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
    _host_name: Option<&str>,
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
                "Service did not become ready within {HEALTH_CHECK_TIMEOUT_SECS} seconds. Check logs with: occ logs"
            ));
        }

        // Periodically check logs for fatal errors (every 1 second)
        if last_log_check.elapsed() > log_check_interval {
            if let Some(error) = check_for_fatal_errors(client).await {
                return Err(anyhow!(
                    "Fatal error detected in container:\n  {error}\n\nThe service cannot start. Try rebuilding the Docker image: occ start --full-rebuild"
                ));
            }
            last_log_check = Instant::now();
        }

        // Try to connect to the service
        let addr = format!("127.0.0.1:{port}").parse().unwrap();
        let connected = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok();

        if connected {
            consecutive_success += 1;
            if consecutive_success >= HEALTH_CHECK_CONSECUTIVE_REQUIRED {
                return Ok(());
            }
            spinner.update(&format!(
                "Service responding ({consecutive_success}/{HEALTH_CHECK_CONSECUTIVE_REQUIRED})"
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

        eprint!("  {line}");
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
