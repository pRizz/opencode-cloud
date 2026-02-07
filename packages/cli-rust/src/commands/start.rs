//! Start command implementation
//!
//! Starts the opencode service, building the image if needed.

use crate::cli_platform::cli_platform_label;
use crate::commands::iotp::{IOTP_FALLBACK_COMMAND, IotpSnapshot, IotpState, fetch_iotp_snapshot};
use crate::commands::runtime_shared::backend::HostBackend;
use crate::commands::runtime_shared::mounts::{collect_bind_mounts, mounts_equal};
use crate::commands::runtime_shared::{
    broker_is_ready as broker_ready_from_status, probe_broker_health,
};
use crate::commands::service::{StopSpinnerMessages, stop_service_with_spinner};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::{
    CommandSpinner, format_cockpit_url, format_docker_error, format_service_url,
    localhost_display_addr, normalize_bind_addr, resolve_remote_addr, show_docker_error,
};
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use futures_util::stream::StreamExt;
use opencode_cloud_core::bollard::container::LogOutput;
use opencode_cloud_core::bollard::query_parameters::LogsOptions;
use opencode_cloud_core::config::save_config;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DEFAULT_STOP_TIMEOUT_SECS, DOCKERFILE, DockerClient, DockerError,
    IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT, ImageState, ParsedMount, ProgressReporter,
    active_resource_names, build_image, container_exists, container_is_running,
    docker_supports_systemd, get_cli_version, get_container_bind_mounts, get_container_ports,
    get_image_version, image_exists, pull_image, save_state, setup_and_start, versions_compatible,
};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::update_signal::run_update_command_listener;

/// Arguments for the start command
#[derive(Args, Default)]
pub struct StartArgs {
    /// Port to bind on host (default: 3000)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Open browser after starting
    #[arg(long)]
    pub open: bool,

    /// Run in foreground (for service managers like systemd/launchd)
    /// Keeps the process running and listens for update commands
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

    /// Build sandbox image from local packages/opencode checkout (dev mode)
    #[arg(long)]
    pub local_opencode_submodule: bool,

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

/// Check if container mounts differ from configured mounts
///
/// Returns `Some(true)` to rebuild, `Some(false)` on user decline (error), `None` if no mismatch.
async fn check_mount_mismatch(
    client: &DockerClient,
    configured_mounts: Option<&[ParsedMount]>,
    quiet: bool,
) -> Result<Option<bool>> {
    let current_mounts = get_container_bind_mounts(client, CONTAINER_NAME).await?;
    let configured = configured_mounts.unwrap_or(&[]);

    if mounts_equal(&current_mounts, configured) {
        return Ok(None);
    }

    if quiet {
        let container_name = active_container_name();
        return Err(anyhow!(
            "Mount configuration changed. Container must be recreated to apply mount changes.\n\
             Run without --quiet to be prompted, or manually remove with:\n  \
             occ stop && docker rm {container_name}"
        ));
    }

    display_mount_mismatch(&current_mounts, configured);

    let confirm = dialoguer::Confirm::new()
        .with_prompt("Recreate container with new mount configuration?")
        .default(true)
        .interact()?;

    if !confirm {
        let container_name = active_container_name();
        return Err(anyhow!(
            "Container not recreated. To apply mount changes, run:\n  \
             occ stop && docker rm {container_name} && occ start"
        ));
    }

    Ok(Some(true))
}

/// Display mount mismatch information to user
fn display_mount_mismatch(
    current: &[opencode_cloud_core::docker::ContainerBindMount],
    configured: &[ParsedMount],
) {
    eprintln!();
    eprintln!(
        "{} {}",
        style("Mount configuration changed:").yellow().bold(),
        style("Container must be recreated to apply mount changes.").yellow()
    );
    eprintln!();

    display_current_mounts(current);
    display_configured_mounts(configured);

    eprintln!();
    eprintln!(
        "{}",
        style("This will stop and recreate the container from the existing image.").dim()
    );
    eprintln!("{}", style("Your data volumes will be preserved.").dim());
    eprintln!();
}

/// Display current container mounts
fn display_current_mounts(mounts: &[opencode_cloud_core::docker::ContainerBindMount]) {
    if mounts.is_empty() {
        eprintln!("  Current mounts: {}", style("(none)").dim());
        return;
    }

    eprintln!("  Current mounts:");
    for m in mounts {
        let ro = if m.read_only { ":ro" } else { "" };
        eprintln!("    - {}:{}{}", m.source, m.target, ro);
    }
}

/// Display configured mounts
fn display_configured_mounts(mounts: &[ParsedMount]) {
    if mounts.is_empty() {
        eprintln!("  Configured mounts: {}", style("(none)").dim());
        return;
    }

    eprintln!("  Configured mounts:");
    for m in mounts {
        let ro = if m.read_only { ":ro" } else { "" };
        eprintln!("    - {}:{}{}", m.host_path.display(), m.container_path, ro);
    }
}

/// Check if image flag is used while container is running, prompt to stop
async fn ensure_container_stopped_for_image_flag(
    client: &DockerClient,
    has_image_flag: bool,
    quiet: bool,
    host_name: Option<&str>,
) -> Result<()> {
    if !has_image_flag {
        return Ok(());
    }

    if !container_is_running(client, CONTAINER_NAME).await? {
        return Ok(());
    }

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

    let _ = stop_service_with_spinner(
        client,
        host_name,
        quiet,
        true,
        DEFAULT_STOP_TIMEOUT_SECS,
        StopSpinnerMessages {
            action_message: "Stopping container for rebuild...",
            update_label: "Stopping container",
            success_base_message: "Container stopped and removed",
            failure_message: "Failed to stop container",
        },
    )
    .await;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionMismatchAction {
    RebuildFromSource,
    PullPrebuilt,
    Continue,
}

/// Check version compatibility and prompt user if mismatch detected.
/// Returns the user-selected action on mismatch.
async fn check_version_compatibility(
    client: &DockerClient,
    config: &opencode_cloud_core::Config,
    args: &StartArgs,
    quiet: bool,
) -> Result<VersionMismatchAction> {
    let should_check = !args.ignore_version
        && !args.cached_rebuild_sandbox_image
        && !args.full_rebuild_sandbox_image
        && !args.no_update_check
        && config.update_check != "never"
        && !quiet;

    if !should_check {
        return Ok(VersionMismatchAction::Continue);
    }

    if !image_exists(client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await? {
        return Ok(VersionMismatchAction::Continue);
    }

    let cli_version = get_cli_version();
    let image_tag = format!("{IMAGE_NAME_GHCR}:{}", active_resource_names().image_tag);

    let Ok(Some(image_version)) = get_image_version(client, &image_tag).await else {
        return Ok(VersionMismatchAction::Continue);
    };

    if versions_compatible(cli_version, Some(&image_version)) {
        return Ok(VersionMismatchAction::Continue);
    }

    println!();
    println!("{} Version mismatch detected", style("⚠").yellow());
    let cli_label = format!("{} version:", cli_platform_label());
    let image_label = "Image version:".to_string();
    let label_width = cli_label.len().max(image_label.len());
    println!(
        "  {:width$} {}",
        cli_label,
        style(cli_version).cyan(),
        width = label_width
    );
    println!(
        "  {:width$} {}",
        image_label,
        style(&image_version).cyan(),
        width = label_width
    );
    println!();

    let selection = dialoguer::Select::new()
        .with_prompt("What would you like to do?")
        .items([
            "Redownload latest prebuilt image (recommended)",
            "Rebuild image from source",
            "Continue with mismatched versions",
        ])
        .default(0)
        .interact()?;

    Ok(match selection {
        0 => VersionMismatchAction::PullPrebuilt,
        1 => VersionMismatchAction::RebuildFromSource,
        _ => VersionMismatchAction::Continue,
    })
}

/// Check for port mismatch and prompt user to recreate container
/// Returns Some(true) if rebuild requested, None if no mismatch
async fn check_port_mismatch(
    client: &DockerClient,
    config: &opencode_cloud_core::Config,
    port: u16,
    quiet: bool,
) -> Result<Option<bool>> {
    let current_ports = get_container_ports(client, CONTAINER_NAME).await?;
    let current_opencode_port = current_ports.opencode_port.unwrap_or(3000);
    let current_cockpit_port = current_ports.cockpit_port.unwrap_or(9090);

    let port_mismatch = current_opencode_port != port;
    let cockpit_mismatch = COCKPIT_EXPOSED && current_cockpit_port != config.cockpit_port;

    if !port_mismatch && !cockpit_mismatch {
        return Ok(None);
    }

    // Check port availability BEFORE prompting - fail fast if new port isn't available
    let bind_addr = config.bind_address.as_str();
    if port_mismatch && !check_port_available(bind_addr, port) {
        return Err(anyhow!(
            "Port mismatch: container uses port {current_opencode_port} but requested port {port}.\n\
             Cannot switch to port {port} - it's already in use.\n\n\
             Options:\n  \
             1. Stop the process using port {port}\n  \
             2. Use a different port: occ start --port <available-port>\n  \
             3. Keep current port: occ start --port {current_opencode_port}"
        ));
    }

    if quiet {
        let container_name = active_container_name();
        return Err(anyhow!(
            "Port mismatch: container uses port {current_opencode_port} but requested port {port}.\n\
             Container must be recreated to change ports.\n\
             Run without --quiet to be prompted, or manually remove with:\n  \
             occ stop && docker rm {container_name}"
        ));
    }

    if COCKPIT_EXPOSED {
        display_port_mismatch(
            port_mismatch,
            cockpit_mismatch,
            current_opencode_port,
            port,
            current_cockpit_port,
            config.cockpit_port,
        );
    } else {
        display_port_mismatch(
            port_mismatch,
            false,
            current_opencode_port,
            port,
            current_cockpit_port,
            config.cockpit_port,
        );
    }

    let confirm = dialoguer::Confirm::new()
        .with_prompt("Recreate container with new port(s)?")
        .default(true)
        .interact()?;

    if !confirm {
        let container_name = active_container_name();
        return Err(anyhow!(
            "Container not recreated. To use port {port}, run:\n  \
             occ stop && docker rm {container_name} && occ start --port {port}"
        ));
    }

    Ok(Some(true))
}

/// Display port mismatch information
#[allow(clippy::too_many_arguments)]
fn display_port_mismatch(
    port_mismatch: bool,
    cockpit_mismatch: bool,
    current_opencode: u16,
    requested_opencode: u16,
    current_cockpit: u16,
    requested_cockpit: u16,
) {
    eprintln!();
    eprintln!(
        "{} {}",
        style("Port mismatch detected:").yellow().bold(),
        style("Container must be recreated to change ports.").yellow()
    );

    if port_mismatch {
        eprintln!(
            "  opencode port: {} (current) → {} (requested)",
            style(current_opencode).red(),
            style(requested_opencode).green()
        );
    }

    if cockpit_mismatch {
        eprintln!(
            "  cockpit port: {} (current) → {} (requested)",
            style(current_cockpit).red(),
            style(requested_cockpit).green()
        );
    }

    eprintln!();
    eprintln!(
        "{}",
        style("This will stop and recreate the container from the existing image.").dim()
    );
    eprintln!("{}", style("Your data volumes will be preserved.").dim());
    eprintln!();
}

/// Check for init mode mismatch (systemd vs tini) on existing container
async fn check_init_mismatch(
    client: &DockerClient,
    systemd_enabled: bool,
    quiet: bool,
) -> Result<Option<bool>> {
    let container_name = active_container_name();
    let info = client
        .inner()
        .inspect_container(&container_name, None)
        .await
        .map_err(|e| anyhow!("Failed to inspect container: {e}"))?;

    let env = info
        .config
        .and_then(|config| config.env)
        .unwrap_or_default();
    let container_systemd_enabled = env.iter().any(|entry| {
        entry == "USE_SYSTEMD=1"
            || entry
                .strip_prefix("USE_SYSTEMD=")
                .map(|val| val == "1")
                .unwrap_or(false)
    });

    if container_systemd_enabled == systemd_enabled {
        return Ok(None);
    }

    if quiet {
        return Err(anyhow!(
            "Init mode mismatch: container uses {} but desired is {}.\n\
             Container must be recreated to change init mode.\n\
             Run:\n  occ stop --remove\n  occ start",
            if container_systemd_enabled {
                "systemd"
            } else {
                "tini"
            },
            if systemd_enabled { "systemd" } else { "tini" }
        ));
    }

    eprintln!();
    eprintln!("{}", style("Init mode mismatch detected:").yellow().bold());
    eprintln!(
        "  container: {}",
        style(if container_systemd_enabled {
            "systemd"
        } else {
            "tini"
        })
        .red()
    );
    eprintln!(
        "  desired:   {}",
        style(if systemd_enabled { "systemd" } else { "tini" }).green()
    );
    eprintln!();
    eprintln!(
        "{}",
        style("Container must be recreated to change init mode.").dim()
    );

    let confirm = dialoguer::Confirm::new()
        .with_prompt("Recreate container with new init mode?")
        .default(true)
        .interact()?;

    if !confirm {
        return Err(anyhow!(
            "Container not recreated. To switch init mode, run:\n  \
             occ stop --remove\n  \
             occ start"
        ));
    }

    Ok(Some(true))
}

/// Acquire Docker image (build or pull) based on configuration
async fn acquire_image(
    client: &DockerClient,
    use_prebuilt: bool,
    full_rebuild: bool,
    quiet: bool,
    verbose: u8,
    local_opencode_submodule: bool,
) -> Result<()> {
    if !use_prebuilt {
        build_docker_image(
            client,
            full_rebuild,
            quiet,
            verbose,
            local_opencode_submodule,
        )
        .await?;
        save_state(&ImageState::built(get_cli_version())).ok();
        return Ok(());
    }

    // Try pulling prebuilt image
    match pull_docker_image(client, verbose).await {
        Ok(registry) => {
            save_state(&ImageState::prebuilt(get_cli_version(), &registry)).ok();
            Ok(())
        }
        Err(e) => handle_pull_failure(client, e, quiet, verbose, local_opencode_submodule).await,
    }
}

/// Handle pull failure by offering to build from source
async fn handle_pull_failure(
    client: &DockerClient,
    error: anyhow::Error,
    quiet: bool,
    verbose: u8,
    local_opencode_submodule: bool,
) -> Result<()> {
    if quiet {
        return Err(error);
    }

    eprintln!();
    eprintln!(
        "{} Failed to pull prebuilt image: {error}",
        style("Error:").red().bold()
    );
    eprintln!();

    let build_instead = dialoguer::Confirm::new()
        .with_prompt("Build from source instead? (This takes 30-60 minutes)")
        .default(true)
        .interact()?;

    if !build_instead {
        return Err(anyhow!(
            "Cannot proceed without image. Run 'occ start --full-rebuild-sandbox-image' to build from source."
        ));
    }

    build_docker_image(client, false, quiet, verbose, local_opencode_submodule).await?;
    save_state(&ImageState::built(get_cli_version())).ok();
    Ok(())
}

fn validate_local_opencode_submodule_args(args: &StartArgs) -> Result<()> {
    // This mode is meant only for explicit source rebuild flows so normal start/pull semantics stay
    // reproducible and unchanged by default.
    if args.local_opencode_submodule
        && !(args.cached_rebuild_sandbox_image || args.full_rebuild_sandbox_image)
    {
        return Err(anyhow!(
            "--local-opencode-submodule requires --cached-rebuild-sandbox-image or --full-rebuild-sandbox-image"
        ));
    }
    Ok(())
}

fn active_container_name() -> String {
    opencode_cloud_core::docker::active_resource_names().container_name
}

/// Display network exposure warning
fn display_network_exposure_warning(bind_addr: &str) {
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
    let config = opencode_cloud_core::config::load_config_or_default()?;
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

    let systemd_enabled = docker_supports_systemd(&client).await?;

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
    validate_local_opencode_submodule_args(args)?;

    let has_image_flag = args.pull_sandbox_image
        || args.cached_rebuild_sandbox_image
        || args.full_rebuild_sandbox_image;

    // If any image flag is used while container is running, prompt to stop
    ensure_container_stopped_for_image_flag(&client, has_image_flag, quiet, host_name.as_deref())
        .await?;

    let mut rebuild_image = args.cached_rebuild_sandbox_image || args.full_rebuild_sandbox_image;
    let mut recreate_container = rebuild_image;
    let mut force_pull = false;

    // Determine image source: flag > config default
    let mut use_prebuilt = if args.pull_sandbox_image {
        true
    } else if rebuild_image {
        false
    } else {
        config.image_source == "prebuilt"
    };

    // Version compatibility check
    match check_version_compatibility(&client, &config, args, quiet).await? {
        VersionMismatchAction::RebuildFromSource => {
            rebuild_image = true;
            recreate_container = true;
        }
        VersionMismatchAction::PullPrebuilt => {
            use_prebuilt = true;
            force_pull = true;
            recreate_container = true;
        }
        VersionMismatchAction::Continue => {}
    }

    // Determine whether this is first container start
    let is_first_start = !container_exists(&client, CONTAINER_NAME).await?;

    // Check for port mismatch on existing container
    if !is_first_start
        && !recreate_container
        && let Some(rebuild) = check_port_mismatch(&client, &config, port, quiet).await?
    {
        recreate_container = rebuild;
    }

    // Check for init mismatch on existing container (systemd vs tini)
    if !is_first_start
        && !recreate_container
        && let Some(rebuild) = check_init_mismatch(&client, systemd_enabled, quiet).await?
    {
        recreate_container = rebuild;
    }

    // Check for mount mismatch on existing container (only if not already rebuilding)
    if !is_first_start
        && !recreate_container
        && let Some(rebuild) =
            check_mount_mismatch(&client, bind_mounts_option.as_deref(), quiet).await?
    {
        recreate_container = rebuild;
    }

    // Handle rebuild: remove existing container so a new one is created from the new image
    if recreate_container {
        handle_rebuild(&client, host_name.as_deref(), quiet, verbose).await?;
    } else if container_is_running(&client, CONTAINER_NAME).await? {
        // Already running (idempotent behavior) - only when not rebuilding
        return show_already_running(
            &client,
            port,
            bind_addr,
            config.is_network_exposed(),
            quiet,
            host_name.as_deref(),
        )
        .await;
    }

    // Security check: warn if network exposed without authentication
    let should_warn_exposure = !quiet
        && config.is_network_exposed()
        && config.users.is_empty()
        && !config.allow_unauthenticated_network;

    if should_warn_exposure {
        display_network_exposure_warning(bind_addr);
    }

    // Pre-check port availability
    if !check_port_available(bind_addr, port) {
        return Err(port_in_use_error(bind_addr, port));
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
    let needs_image = rebuild_image
        || force_pull
        || args.pull_sandbox_image
        || !image_exists(&client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT).await?;

    if needs_image {
        acquire_image(
            &client,
            use_prebuilt && !rebuild_image,
            args.full_rebuild_sandbox_image,
            quiet,
            verbose,
            args.local_opencode_submodule,
        )
        .await?;
    }

    let env_vars = std::env::var("OPENCODE_CLOUD_ENV")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| vec![format!("OPENCODE_CLOUD_ENV={value}")]);

    // Start container
    let msg = crate::format_host_message(host_name.as_deref(), "Starting container...");
    let spinner = CommandSpinner::new_maybe(&msg, quiet);
    let container_id = match start_container(
        &client,
        port,
        env_vars,
        bind_addr,
        config.cockpit_port,
        config.cockpit_enabled && COCKPIT_EXPOSED,
        systemd_enabled,
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
    if let Err(e) = wait_for_service_ready(&client, bind_addr, port, &spinner).await {
        spinner.fail(&crate::format_host_message(
            host_name.as_deref(),
            "Service failed to become ready",
        ));
        eprintln!();
        eprintln!("{}", style("Recent container logs:").yellow());
        show_recent_logs(&client, 20).await;
        return Err(e);
    }

    // Wait for broker to be ready
    if let Err(e) = wait_for_broker_ready(&client, &spinner).await {
        spinner.fail(&crate::format_host_message(
            host_name.as_deref(),
            "Broker failed to become ready",
        ));
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
    maybe_print_iotp_info(&client, host_name.as_deref(), &config).await;
    open_browser_if_requested(args.open, port, bind_addr);

    if args.no_daemon {
        run_update_command_listener(&client, &config, maybe_host, quiet, verbose).await?;
    }

    Ok(())
}

/// Handle rebuild flags: remove existing container so a new one is created from the new image
async fn handle_rebuild(
    client: &DockerClient,
    host_name: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    if !quiet {
        eprintln!();
        eprintln!();
    }

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

    let _ = stop_service_with_spinner(
        client,
        host_name,
        quiet,
        true,
        DEFAULT_STOP_TIMEOUT_SECS,
        StopSpinnerMessages {
            action_message: "Stopping container for rebuild...",
            update_label: "Stopping container",
            success_base_message: "Container stopped and removed",
            failure_message: "Failed to stop container",
        },
    )
    .await;
    Ok(())
}

/// Show message when service is already running
async fn show_already_running(
    client: &DockerClient,
    port: u16,
    bind_addr: &str,
    is_exposed: bool,
    quiet: bool,
    host_name: Option<&str>,
) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let msg = crate::format_host_message(host_name, "Service is already running");
    println!("{}", style(msg).dim());

    let container_name = active_container_name();
    let container_id = match client
        .inner()
        .inspect_container(&container_name, None)
        .await
    {
        Ok(info) => info.id.unwrap_or_else(|| "unknown".to_string()),
        Err(error) => {
            eprintln!(
                "{} Failed to inspect running container: {error}",
                style("Warning:").yellow()
            );
            "unknown".to_string()
        }
    };

    show_start_result(&container_id, port, bind_addr, is_exposed, quiet, host_name);
    Ok(())
}

/// Create error message for port already in use
fn port_in_use_error(bind_addr: &str, port: u16) -> anyhow::Error {
    let mut msg = format!("Port {port} is already in use");
    if let Some(p) = find_next_available_port(bind_addr, port) {
        msg.push_str(&format!(". Try: occ start --port {p}"));
    }
    anyhow!(msg)
}

/// Build the Docker image with progress reporting
///
/// If `no_cache` is true, builds from scratch ignoring Docker layer cache.
/// Otherwise uses cached layers for faster builds.
async fn build_docker_image(
    client: &DockerClient,
    no_cache: bool,
    quiet: bool,
    verbose: u8,
    local_opencode_submodule: bool,
) -> Result<()> {
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

    let build_args = build_opencode_build_args(local_opencode_submodule)?;
    if local_opencode_submodule && !quiet {
        eprintln!(
            "{}",
            style("Dev mode: building sandbox from local packages/opencode checkout.").yellow()
        );
        if let Some(local_ref) = build_args.get("OPENCODE_LOCAL_REF") {
            eprintln!(
                "{} Local ref: {}",
                style("Info:").dim(),
                style(local_ref).cyan()
            );
        }
    }

    let context = if no_cache {
        "Full rebuilding Docker image (no cache)"
    } else {
        "Building Docker image"
    };

    let mut progress = if verbose > 0 {
        ProgressReporter::with_context_plain(context)
    } else {
        ProgressReporter::with_context(context)
    };
    build_image(
        client,
        Some(IMAGE_TAG_DEFAULT),
        &mut progress,
        no_cache,
        Some(build_args),
    )
    .await?;
    Ok(())
}

fn build_opencode_build_args(local_opencode_submodule: bool) -> Result<HashMap<String, String>> {
    let mut build_args = HashMap::new();
    build_args.insert(
        "OPENCODE_SOURCE".to_string(),
        if local_opencode_submodule {
            "local".to_string()
        } else {
            "remote".to_string()
        },
    );
    // Keep the pinned commit in build args so Dockerfile source-selection remains explicit and
    // deterministic even if the shell logic changes later.
    build_args.insert(
        "OPENCODE_COMMIT".to_string(),
        extract_pinned_opencode_commit_from_dockerfile()?,
    );

    if local_opencode_submodule {
        build_args.insert(
            "OPENCODE_LOCAL_REF".to_string(),
            resolve_local_opencode_ref()?,
        );
    }

    Ok(build_args)
}

fn extract_pinned_opencode_commit_from_dockerfile() -> Result<String> {
    const MARKER: &str = "OPENCODE_COMMIT=\"";
    DOCKERFILE
        .lines()
        .find_map(|line| {
            let start = line.find(MARKER)?;
            let rest = &line[start + MARKER.len()..];
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        })
        .filter(|commit| !commit.is_empty())
        .ok_or_else(|| {
            anyhow!("Could not determine pinned OPENCODE_COMMIT from embedded Dockerfile")
        })
}

fn workspace_root_path() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|err| anyhow!("Failed to resolve workspace root: {err}"))
}

fn opencode_submodule_path() -> Result<PathBuf> {
    Ok(workspace_root_path()?.join("packages/opencode"))
}

fn run_git_command(repo_path: &Path, args: &[&str], context: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|err| anyhow!("Failed to run git for {context}: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "git command failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow!("Failed to resolve {context}: {detail}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn resolve_local_opencode_ref() -> Result<String> {
    let submodule_path = opencode_submodule_path()?;
    if !submodule_path.exists() {
        return Err(anyhow!(
            "Local opencode submodule not found at {}. Run from a source checkout with packages/opencode initialized.",
            submodule_path.display()
        ));
    }

    let sha = run_git_command(
        &submodule_path,
        &["rev-parse", "HEAD"],
        "local opencode commit",
    )?;
    let porcelain = run_git_command(
        &submodule_path,
        &["status", "--porcelain"],
        "local opencode dirty status",
    )?;

    if porcelain.is_empty() {
        Ok(sha)
    } else {
        Ok(format!("{sha}-dirty"))
    }
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
#[allow(clippy::too_many_arguments)]
async fn start_container(
    client: &DockerClient,
    port: u16,
    env_vars: Option<Vec<String>>,
    bind_address: &str,
    cockpit_port: u16,
    cockpit_enabled: bool,
    systemd_enabled: bool,
    bind_mounts: Option<Vec<ParsedMount>>,
) -> Result<String, DockerError> {
    setup_and_start(
        client,
        Some(port),
        env_vars,
        Some(bind_address),
        Some(cockpit_port),
        Some(cockpit_enabled),
        Some(systemd_enabled),
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
    // Get remote host address if using --remote-host
    let maybe_remote_addr = resolve_remote_addr(host_name);

    if quiet {
        let url = format_service_url(maybe_remote_addr.as_deref(), bind_addr, port);
        println!("{url}");
        return;
    }

    println!();

    // Show URL - use remote address if available
    if let Some(ref remote_addr) = maybe_remote_addr {
        let remote_url = format_service_url(Some(remote_addr), bind_addr, port);
        println!("Remote URL: {}", style(&remote_url).cyan());
    } else {
        let url = format_service_url(None, bind_addr, port);
        println!("URL:        {}", style(&url).cyan());
    }

    println!(
        "Container:  {}",
        style(&container_id[..12.min(container_id.len())]).dim()
    );
    println!("Port:       {port} -> 3000");

    // Show Cockpit availability if enabled
    if COCKPIT_EXPOSED
        && let Ok(config) = opencode_cloud_core::config::load_config_or_default()
        && config.cockpit_enabled
    {
        let cockpit_url =
            format_cockpit_url(maybe_remote_addr.as_deref(), bind_addr, config.cockpit_port);
        println!("Cockpit:    {cockpit_url} (web admin)");
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

async fn maybe_print_iotp_info(
    client: &DockerClient,
    host: Option<&str>,
    config: &opencode_cloud_core::Config,
) {
    if config.allow_unauthenticated_network || !config.users.is_empty() {
        return;
    }

    // The bootstrap helper is the source of truth after container startup.
    // Do not infer onboarding state from pre-start config heuristics.
    let snapshot = fetch_iotp_snapshot(client).await;

    println!();
    let headline = crate::format_host_message(host, "First-time onboarding");
    println!("{}", style(headline).cyan().bold());

    if matches!(snapshot.state, IotpState::ActiveUnused)
        && let Some(iotp) = snapshot.otp.as_deref()
    {
        println!(
            "Initial One-Time Password (IOTP): {}",
            style(iotp).green().bold()
        );
        println!(
            "Enter this in the web login first-time setup panel, then enroll a passkey for {}.",
            style("opencoder").cyan()
        );
        return;
    }

    println!(
        "{}",
        style("Could not retrieve an IOTP from bootstrap state.").yellow()
    );
    println!("Reason: {}", iotp_unavailable_reason(&snapshot));
    println!("Fetch logs with: {}", style("occ logs").cyan());
    println!("Try extracting IOTP with:\n  {IOTP_FALLBACK_COMMAND}");
}

fn iotp_unavailable_reason(snapshot: &IotpSnapshot) -> String {
    if let Some(reason) = snapshot.detail.as_deref() {
        return reason.to_string();
    }

    format!("IOTP state is {}", snapshot.state_label)
}

/// Open browser if requested
fn open_browser_if_requested(should_open: bool, port: u16, bind_addr: &str) {
    if !should_open {
        return;
    }

    // Use localhost for loopback/wildcard browser URLs to preserve passkey support.
    let browser_addr = localhost_display_addr(bind_addr);
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
fn check_port_available(bind_addr: &str, port: u16) -> bool {
    let bind_target = format_bind_addr(bind_addr, port);
    TcpListener::bind(&bind_target).is_ok()
}

/// Find the next available port starting from the given port
fn find_next_available_port(bind_addr: &str, start: u16) -> Option<u16> {
    (start..start.saturating_add(100)).find(|&p| check_port_available(bind_addr, p))
}

fn format_bind_addr(bind_addr: &str, port: u16) -> String {
    if bind_addr.contains(':') && !bind_addr.starts_with('[') {
        format!("[{bind_addr}]:{port}")
    } else {
        format!("{bind_addr}:{port}")
    }
}

/// Configuration for health check waiting
/// Note: 60 seconds allows time for systemd to boot and start all services
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 60;
const HEALTH_CHECK_INTERVAL_MS: u64 = 500;
const HEALTH_CHECK_CONSECUTIVE_REQUIRED: u32 = 3;
const BROKER_READY_TIMEOUT_SECS: u64 = 30;
const BROKER_READY_INTERVAL_MS: u64 = 500;

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
    let options = LogsOptions {
        stdout: true,
        stderr: true,
        tail: "20".to_string(),
        ..Default::default()
    };

    let container_name = active_container_name();
    let mut stream = client.inner().logs(&container_name, Some(options));
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
pub(crate) async fn wait_for_service_ready(
    client: &DockerClient,
    bind_addr: &str,
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
                "Service did not become ready within {HEALTH_CHECK_TIMEOUT_SECS} seconds. Check logs with: occ logs"
            ));
        }

        // Periodically check logs for fatal errors (every 1 second)
        if last_log_check.elapsed() > log_check_interval {
            if let Some(error) = check_for_fatal_errors(client).await {
                return Err(anyhow!(
                    "Fatal error detected in container:\n  {error}\n\nThe service cannot start. Try rebuilding the Docker image: occ start --full-rebuild-sandbox-image"
                ));
            }
            last_log_check = Instant::now();
        }

        // Try to connect to the service
        let browser_addr = normalize_bind_addr(bind_addr);
        let addr = format_bind_addr(browser_addr, port).parse().unwrap();
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

async fn broker_is_ready(client: &DockerClient) -> Result<bool> {
    let backend = HostBackend::new(client);
    let health = probe_broker_health(&backend).await;
    Ok(broker_ready_from_status(health))
}

/// Wait for the broker to be ready by checking socket + process inside container
pub(crate) async fn wait_for_broker_ready(
    client: &DockerClient,
    spinner: &CommandSpinner,
) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(BROKER_READY_TIMEOUT_SECS);
    let interval = Duration::from_millis(BROKER_READY_INTERVAL_MS);

    spinner.update("Waiting for broker to be ready...");

    loop {
        if start.elapsed() > timeout {
            eprintln!();
            eprintln!("{}", style("Recent container logs:").yellow());
            show_recent_logs(client, 50).await;
            return Err(anyhow!(
                "Broker did not become ready within {BROKER_READY_TIMEOUT_SECS} seconds. Check logs with: occ logs --broker"
            ));
        }

        if broker_is_ready(client).await? {
            return Ok(());
        }

        spinner.update(&format!(
            "Waiting for broker to be ready... ({}s)",
            start.elapsed().as_secs()
        ));
        tokio::time::sleep(interval).await;
    }
}

/// Show recent container logs for debugging
async fn show_recent_logs(client: &DockerClient, lines: usize) {
    let options = LogsOptions {
        stdout: true,
        stderr: true,
        tail: lines.to_string(),
        ..Default::default()
    };

    let container_name = active_container_name();
    let mut stream = client.inner().logs(&container_name, Some(options));
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
    use std::net::TcpListener;

    fn can_bind_localhost() -> bool {
        TcpListener::bind(("127.0.0.1", 0)).is_ok()
    }

    /// Returns true if we're running in an environment where privileged port binding
    /// succeeds (e.g., root, sandboxed environment, or elevated capabilities).
    fn can_bind_privileged_ports() -> bool {
        TcpListener::bind(("127.0.0.1", 1)).is_ok()
    }

    #[test]
    fn port_check_returns_false_for_privileged_ports() {
        // Port 1 is privileged and typically unavailable on non-root systems.
        // Skip this test in sandboxed/elevated environments where it would pass.
        if can_bind_privileged_ports() {
            eprintln!(
                "Skipping test: privileged ports are bindable in this environment (sandbox/root)."
            );
            return;
        }
        assert!(!check_port_available("127.0.0.1", 1));
    }

    #[test]
    fn find_next_port_finds_available_port() {
        if !can_bind_localhost() {
            eprintln!("Skipping test: cannot bind to localhost in this environment.");
            return;
        }
        // This should find something in the 49152-49252 range (dynamic ports)
        let result = find_next_available_port("127.0.0.1", 49152);
        assert!(result.is_some());
    }

    #[test]
    fn iotp_unavailable_reason_prefers_snapshot_detail() {
        let snapshot = IotpSnapshot {
            state: IotpState::Unavailable,
            state_label: "unavailable".to_string(),
            otp: None,
            detail: Some("bootstrap helper unavailable".to_string()),
        };
        let reason = super::iotp_unavailable_reason(&snapshot);
        assert_eq!(reason, "bootstrap helper unavailable");
    }

    #[test]
    fn iotp_unavailable_reason_does_not_invent_bootstrap_hint() {
        let snapshot = IotpSnapshot {
            state: IotpState::Unavailable,
            state_label: "unavailable".to_string(),
            otp: None,
            detail: None,
        };
        let reason = super::iotp_unavailable_reason(&snapshot);
        assert_eq!(reason, "IOTP state is unavailable");
        assert!(!reason.contains("bootstrap mode was requested"));
    }

    #[test]
    fn iotp_unavailable_reason_falls_back_to_state_label() {
        let snapshot = IotpSnapshot {
            state: IotpState::InactiveUsersConfigured,
            state_label: "inactive (users configured)".to_string(),
            otp: None,
            detail: None,
        };
        let reason = super::iotp_unavailable_reason(&snapshot);
        assert_eq!(reason, "IOTP state is inactive (users configured)");
    }

    #[test]
    fn local_opencode_submodule_requires_rebuild_flag() {
        let args = StartArgs {
            local_opencode_submodule: true,
            ..Default::default()
        };
        let error =
            validate_local_opencode_submodule_args(&args).expect_err("should reject invalid args");
        assert!(error.to_string().contains("--cached-rebuild-sandbox-image"));
    }

    #[test]
    fn local_opencode_submodule_allowed_with_cached_rebuild() {
        let args = StartArgs {
            local_opencode_submodule: true,
            cached_rebuild_sandbox_image: true,
            ..Default::default()
        };
        validate_local_opencode_submodule_args(&args).expect("cached rebuild should be accepted");
    }

    #[test]
    fn local_opencode_submodule_allowed_with_full_rebuild() {
        let args = StartArgs {
            local_opencode_submodule: true,
            full_rebuild_sandbox_image: true,
            ..Default::default()
        };
        validate_local_opencode_submodule_args(&args).expect("full rebuild should be accepted");
    }

    #[test]
    fn extracts_pinned_opencode_commit_from_embedded_dockerfile() {
        let commit = extract_pinned_opencode_commit_from_dockerfile()
            .expect("pinned commit should be present");
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
