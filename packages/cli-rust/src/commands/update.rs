//! Update command implementation
//!
//! Updates the opencode image to the latest version or rolls back to previous version.

use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::load_config;
use opencode_cloud_core::docker::update::tag_current_as_previous;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, IMAGE_TAG_DEFAULT, ImageState, ProgressReporter, build_image,
    create_user, get_cli_version, has_previous_image, pull_image, rollback_image, save_state,
    setup_and_start, stop_service,
};

/// Arguments for the update command
#[derive(Args)]
pub struct UpdateArgs {
    /// Restore previous version instead of updating
    #[arg(long)]
    pub rollback: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

/// Update the opencode image to the latest version
///
/// This command:
/// 1. Stops the service (brief downtime)
/// 2. Backs up current image (for rollback)
/// 3. Pulls latest image from registry
/// 4. Recreates container with new image
/// 5. Recreates users (passwords NOT preserved - must reset)
/// 6. Starts the service
///
/// Or with --rollback:
/// 1. Stops the service
/// 2. Restores previous image
/// 3. Recreates container
/// 4. Recreates users
/// 5. Starts the service
pub async fn cmd_update(
    args: &UpdateArgs,
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

    client
        .verify_connection()
        .await
        .map_err(|e| anyhow!("Docker connection error: {e}"))?;

    // Load config
    let config = load_config()?;

    if args.rollback {
        // Rollback flow
        handle_rollback(
            &client,
            &config,
            args.yes,
            quiet,
            verbose,
            host_name.as_deref(),
        )
        .await
    } else {
        // Update flow
        handle_update(
            &client,
            &config,
            args.yes,
            quiet,
            verbose,
            host_name.as_deref(),
        )
        .await
    }
}

/// Handle the normal update flow
async fn handle_update(
    client: &DockerClient,
    config: &opencode_cloud_core::config::Config,
    skip_confirm: bool,
    quiet: bool,
    verbose: u8,
    _host_name: Option<&str>,
) -> Result<()> {
    let port = config.opencode_web_port;
    let bind_addr = &config.bind_address;

    // Show warning about downtime
    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will briefly stop the service to apply the update.",
            style("Warning:").yellow().bold()
        );
        eprintln!();
    }

    // Confirm with user unless --yes
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt("Continue with update?")
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Update cancelled.");
            }
            return Ok(());
        }
    }

    // Step 1: Stop service
    if verbose > 0 {
        eprintln!("{} Stopping service...", style("[1/5]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
    if let Err(e) = stop_service(client, true).await {
        spinner.fail("Failed to stop service");
        return Err(anyhow!("Failed to stop service: {e}"));
    }
    spinner.success("Service stopped");

    // Step 2: Get new image based on config.image_source
    if verbose > 0 {
        eprintln!("{} Getting new image...", style("[2/5]").cyan());
    }

    if config.image_source == "build" {
        // Building from source
        if !quiet {
            println!();
            println!(
                "{} Rebuilding image from source (per config.image_source=build)",
                style("Info:").cyan()
            );
            println!(
                "{}",
                style("To use prebuilt images: occ config set image_source prebuilt").dim()
            );
            println!();
        }

        // First, tag current as previous for rollback (same as update_image does)
        tag_current_as_previous(client)
            .await
            .map_err(|e| anyhow!("Failed to backup current image: {e}"))?;

        // Then build new image
        let mut progress = if quiet {
            ProgressReporter::new()
        } else {
            ProgressReporter::with_context("Building image")
        };

        build_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress, false)
            .await
            .map_err(|e| anyhow!("Failed to build image: {e}"))?;

        // Save provenance
        save_state(&ImageState::built(get_cli_version())).ok();
    } else {
        // Pulling prebuilt (default)
        if !quiet {
            println!();
            println!(
                "{} Pulling prebuilt image (per config.image_source=prebuilt)",
                style("Info:").cyan()
            );
            println!(
                "{}",
                style("To build from source: occ config set image_source build").dim()
            );
            println!();
        }

        // First, tag current as previous for rollback
        tag_current_as_previous(client)
            .await
            .map_err(|e| anyhow!("Failed to backup current image: {e}"))?;

        // Then pull new image
        let mut progress = if quiet {
            ProgressReporter::new()
        } else {
            ProgressReporter::with_context("Updating image")
        };

        let full_image = pull_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress)
            .await
            .map_err(|e| anyhow!("Failed to pull image: {e}"))?;

        // Determine registry and save provenance
        let registry = if full_image.starts_with("ghcr.io") {
            "ghcr.io"
        } else {
            "docker.io"
        };
        save_state(&ImageState::prebuilt(get_cli_version(), registry)).ok();
    }

    // Step 3: Recreate container
    if verbose > 0 {
        eprintln!("{} Recreating container...", style("[3/5]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Recreating container...", quiet);
    if let Err(e) = setup_and_start(
        client,
        Some(port),
        None,
        Some(bind_addr),
        Some(config.cockpit_port),
        Some(config.cockpit_enabled),
    )
    .await
    {
        spinner.fail("Failed to recreate container");
        return Err(anyhow!("Failed to recreate container: {e}"));
    }
    spinner.success("Container recreated");

    // Step 4: Recreate users
    if verbose > 0 {
        eprintln!("{} Recreating users...", style("[4/5]").cyan());
    }
    recreate_users(client, config, quiet).await?;

    // Step 5: Show success
    if verbose > 0 {
        eprintln!("{} Update complete", style("[5/5]").cyan());
    }
    if !quiet {
        eprintln!();
        eprintln!(
            "{} Update completed successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
        eprintln!(
            "URL:      {}",
            style(format!("http://{bind_addr}:{port}")).cyan()
        );
        if !config.users.is_empty() {
            eprintln!();
            eprintln!(
                "{} User accounts were recreated but passwords were NOT preserved.",
                style("Note:").yellow()
            );
            eprintln!(
                "      You must reset passwords with: {}",
                style("occ user passwd <username>").cyan()
            );
        }
        eprintln!();
    }

    Ok(())
}

/// Handle the rollback flow
async fn handle_rollback(
    client: &DockerClient,
    config: &opencode_cloud_core::config::Config,
    skip_confirm: bool,
    quiet: bool,
    verbose: u8,
    _host_name: Option<&str>,
) -> Result<()> {
    let port = config.opencode_web_port;
    let bind_addr = &config.bind_address;

    // Check if previous image exists
    if !has_previous_image(client).await? {
        return Err(anyhow!(
            "No previous image available for rollback.\n\
             You must update at least once before using --rollback."
        ));
    }

    // Show warning about downtime
    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will briefly stop the service to rollback to the previous version.",
            style("Warning:").yellow().bold()
        );
        eprintln!();
    }

    // Confirm with user unless --yes
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt("Continue with rollback?")
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Rollback cancelled.");
            }
            return Ok(());
        }
    }

    // Step 1: Stop service
    if verbose > 0 {
        eprintln!("{} Stopping service...", style("[1/4]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
    if let Err(e) = stop_service(client, true).await {
        spinner.fail("Failed to stop service");
        return Err(anyhow!("Failed to stop service: {e}"));
    }
    spinner.success("Service stopped");

    // Step 2: Rollback image
    if verbose > 0 {
        eprintln!("{} Rolling back image...", style("[2/4]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Rolling back to previous image...", quiet);
    if let Err(e) = rollback_image(client).await {
        spinner.fail("Failed to rollback image");
        return Err(anyhow!("Failed to rollback: {e}"));
    }
    spinner.success("Rolled back to previous image");

    // Step 3: Recreate container
    if verbose > 0 {
        eprintln!("{} Recreating container...", style("[3/4]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Recreating container...", quiet);
    if let Err(e) = setup_and_start(
        client,
        Some(port),
        None,
        Some(bind_addr),
        Some(config.cockpit_port),
        Some(config.cockpit_enabled),
    )
    .await
    {
        spinner.fail("Failed to recreate container");
        return Err(anyhow!("Failed to recreate container: {e}"));
    }
    spinner.success("Container recreated");

    // Step 4: Recreate users
    if verbose > 0 {
        eprintln!("{} Recreating users...", style("[4/4]").cyan());
    }
    recreate_users(client, config, quiet).await?;

    // Show success
    if !quiet {
        eprintln!();
        eprintln!(
            "{} Rollback completed successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
        eprintln!(
            "URL:      {}",
            style(format!("http://{bind_addr}:{port}")).cyan()
        );
        if !config.users.is_empty() {
            eprintln!();
            eprintln!(
                "{} User accounts were recreated but passwords were NOT preserved.",
                style("Note:").yellow()
            );
            eprintln!(
                "      You must reset passwords with: {}",
                style("occ user passwd <username>").cyan()
            );
        }
        eprintln!();
    }

    Ok(())
}

/// Recreate users from config
///
/// Note: Passwords are NOT stored in config, so they cannot be preserved.
/// Users must reset their passwords after update/rollback.
async fn recreate_users(
    client: &DockerClient,
    config: &opencode_cloud_core::config::Config,
    quiet: bool,
) -> Result<()> {
    if config.users.is_empty() {
        return Ok(());
    }

    let spinner = CommandSpinner::new_maybe(
        &format!("Recreating {} user(s)...", config.users.len()),
        quiet,
    );

    for username in &config.users {
        // Create user (ignore errors if already exists)
        if let Err(e) = create_user(client, CONTAINER_NAME, username).await {
            // Only fail if error is not "already exists"
            let err_msg = e.to_string();
            if !err_msg.contains("already exists") {
                spinner.fail(&format!("Failed to recreate user: {username}"));
                return Err(anyhow!("Failed to recreate user {username}: {e}"));
            }
        }
    }

    spinner.success(&format!("{} user(s) recreated", config.users.len()));
    Ok(())
}
