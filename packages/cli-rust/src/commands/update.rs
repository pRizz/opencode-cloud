//! Update command implementation
//!
//! Updates the opencode image to the latest version or rolls back to previous version.

use crate::commands::{RestartArgs, cmd_restart};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::update::tag_current_as_previous;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, IMAGE_TAG_DEFAULT, ImageState, ProgressReporter, build_image,
    container_exists, container_is_running, exec_command, exec_command_with_status,
    get_cli_version, has_previous_image, pull_image, rollback_image, save_state, setup_and_start,
    stop_service,
};

/// Arguments for the update command
#[derive(Args)]
pub struct UpdateArgs {
    /// Update the opencode runtime inside the container
    #[command(subcommand)]
    pub command: Option<UpdateCommand>,

    /// Restore previous version instead of updating
    #[arg(long)]
    pub rollback: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum UpdateCommand {
    /// Update the opencode-cloud CLI binary
    Cli(UpdateCliArgs),
    /// Update the opencode-cloud container image
    Container,
    /// Update opencode inside the running container
    Opencode(UpdateOpencodeArgs),
}

/// Arguments for updating the opencode-cloud CLI binary
#[derive(Args)]
pub struct UpdateCliArgs {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

/// Arguments for updating opencode inside the container
#[derive(Args)]
pub struct UpdateOpencodeArgs {
    /// Use a specific branch (default: dev)
    #[arg(long, conflicts_with = "commit")]
    pub branch: Option<String>,

    /// Use a specific commit SHA
    #[arg(long, conflicts_with = "branch")]
    pub commit: Option<String>,

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
/// 5. Restores persisted users and passwords
/// 6. Starts the service
///
/// Or with --rollback:
/// 1. Stops the service
/// 2. Restores previous image
/// 3. Recreates container
/// 4. Restores persisted users and passwords
/// 5. Starts the service
pub async fn cmd_update(
    args: &UpdateArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    match args.command.as_ref() {
        Some(UpdateCommand::Cli(cli_args)) => {
            return cmd_update_cli(cli_args, maybe_host, quiet, verbose).await;
        }
        Some(UpdateCommand::Opencode(opencode_args)) => {
            return cmd_update_opencode(opencode_args, maybe_host, quiet, verbose).await;
        }
        Some(UpdateCommand::Container) => {}
        None => {
            if !quiet {
                eprintln!(
                    "{} Missing subcommand. Use one of:\n  occ update cli\n  occ update container\n  occ update opencode",
                    style("Error:").red().bold()
                );
            }
            return Ok(());
        }
    }

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
    let config = load_config_or_default()?;

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

async fn cmd_update_cli(
    args: &UpdateCliArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    if is_dev_binary() {
        let message = [
            "You're running the dev build of opencode-cloud.",
            "Nice try, but I can't update myself while I'm still in the lab!",
            "If you meant to update a released install, use:",
            "  - cargo install opencode-cloud",
            "  - npm install -g opencode-cloud",
        ]
        .join("\n");

        return Err(anyhow!(message));
    }

    let install_method = match detect_install_method() {
        Some(method) => method,
        None => {
            let guidance = [
                "Unable to detect how opencode-cloud was installed.",
                "Try one of the following:",
                "  - cargo install opencode-cloud",
                "  - npm install -g opencode-cloud",
                "If you used another package manager, re-run its update command.",
            ]
            .join("\n");

            return Err(anyhow!(guidance));
        }
    };

    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will update the opencode-cloud CLI and restart the service.",
            style("Warning:").yellow().bold()
        );
        eprintln!("Install:    {}", style(install_method.label()).dim());
        eprintln!();
    }

    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt("Continue with opencode-cloud update?")
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Update cancelled.");
            }
            return Ok(());
        }
    }

    let spinner = CommandSpinner::new_maybe("Updating opencode-cloud...", quiet);
    install_method.run_update().map_err(|e| anyhow!("{e}"))?;
    spinner.success("opencode-cloud updated");

    let restart_args = RestartArgs {};
    cmd_restart(&restart_args, maybe_host, quiet, verbose).await?;

    if !quiet {
        eprintln!();
        eprintln!(
            "{} opencode-cloud updated successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
    }

    Ok(())
}

enum InstallMethod {
    Cargo,
    Npm,
}

impl InstallMethod {
    fn label(&self) -> &'static str {
        match self {
            InstallMethod::Cargo => "cargo install",
            InstallMethod::Npm => "npm install -g",
        }
    }

    fn run_update(&self) -> Result<(), String> {
        let (program, args) = match self {
            InstallMethod::Cargo => ("cargo", vec!["install", "opencode-cloud"]),
            InstallMethod::Npm => ("npm", vec!["install", "-g", "opencode-cloud"]),
        };

        let status = std::process::Command::new(program)
            .args(args)
            .status()
            .map_err(|e| format!("Failed to execute {program}: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} update failed with status {status}"))
        }
    }
}

fn detect_install_method() -> Option<InstallMethod> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_str = exe_path.to_string_lossy();

    if exe_str.contains("node_modules") || exe_str.contains("@opencode-cloud") {
        return Some(InstallMethod::Npm);
    }

    if exe_str.contains(".cargo") || exe_str.contains("cargo/bin") {
        return Some(InstallMethod::Cargo);
    }

    None
}

fn is_dev_binary() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let exe_str = exe_path.to_string_lossy();

    exe_str.contains("/target/debug/")
}

pub(crate) async fn cmd_update_opencode(
    args: &UpdateOpencodeArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
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

    let config = load_config_or_default()?;

    if !container_exists(&client, CONTAINER_NAME).await? {
        return Err(anyhow!(
            "Container does not exist. Start it first with:\n  occ start"
        ));
    }

    if !container_is_running(&client, CONTAINER_NAME).await? {
        if !quiet {
            eprintln!();
            eprintln!(
                "{} Container is stopped. It must be running to update opencode.",
                style("Note:").yellow()
            );
        }

        if !args.yes {
            let confirmed = Confirm::new()
                .with_prompt("Start container now?")
                .default(true)
                .interact()?;

            if !confirmed {
                return Err(anyhow!(
                    "Container not started. Run:\n  occ start\nthen retry the update."
                ));
            }
        }

        setup_and_start(
            &client,
            Some(config.opencode_web_port),
            None,
            Some(&config.bind_address),
            Some(config.cockpit_port),
            Some(config.cockpit_enabled && COCKPIT_EXPOSED),
            None,
        )
        .await
        .map_err(|e| anyhow!("Failed to start container: {e}"))?;
    }

    let target_ref = args
        .commit
        .clone()
        .or_else(|| args.branch.clone())
        .unwrap_or_else(|| "dev".to_string());
    let checkout_cmd = if args.commit.is_some() {
        "git checkout \"$OPENCODE_REF\"".to_string()
    } else {
        "git checkout -B \"$OPENCODE_REF\" \"origin/$OPENCODE_REF\"".to_string()
    };

    let current_version = get_current_opencode_version(&client).await;
    let current_commit = get_current_opencode_commit(&client).await;
    let next_commit = if let Some(commit) = args.commit.as_deref() {
        Some(short_commit(commit))
    } else {
        resolve_remote_commit(&client, &target_ref).await
    };

    if current_commit.is_some() && current_commit == next_commit {
        if !quiet {
            let check = style("✓").green();
            eprintln!(
                "{} Opencode is already up to date (hash: {}).",
                check,
                style(current_commit.unwrap_or_else(|| "unknown".to_string())).dim()
            );
        }
        return Ok(());
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will stop the opencode service, update from {target_ref}, rebuild, and restart.",
            style("Warning:").yellow().bold()
        );
        eprintln!(
            "Current:    version={}, hash={}",
            style(current_version.unwrap_or_else(|| "unknown".to_string())).dim(),
            style(current_commit.unwrap_or_else(|| "unknown".to_string())).dim()
        );
        let next_hash = next_commit.as_deref().unwrap_or("unknown");
        eprintln!("Next hash:  {}", style(next_hash).dim());
        eprintln!();
    }

    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt("Continue with opencode update?")
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Update cancelled.");
            }
            return Ok(());
        }
    }

    let spinner = CommandSpinner::new_maybe("Updating opencode...", quiet);

    // Stop opencode processes inside the container
    let stop_cmd = r#"set -euo pipefail
pkill -f "/opt/opencode/bin/opencode" || true
pkill -f "opencode-broker" || true
"#;
    let (stop_output, stop_status) =
        exec_command_with_status(&client, CONTAINER_NAME, vec!["bash", "-lc", stop_cmd])
            .await
            .map_err(|e| anyhow!("Failed to stop opencode processes: {e}"))?;
    if !quiet && !stop_output.trim().is_empty() {
        eprintln!(
            "{} Stop output:\n{}",
            style("[info]").cyan(),
            stop_output.trim()
        );
    }
    if stop_status != 0 {
        return Err(anyhow!(
            "Failed to stop opencode processes (exit {stop_status}).\n{stop_output}"
        ));
    }

    let update_script = format!(
        r#"set -euo pipefail
REPO="/tmp/opencode-repo"
OPENCODE_REF="{target_ref}"
rm -rf "$REPO"
git clone --depth 1 https://github.com/pRizz/opencode.git "$REPO"
cd "$REPO"
git fetch --depth 1 origin "$OPENCODE_REF"
{checkout_cmd}

mkdir -p /opt/opencode
git rev-parse HEAD > /opt/opencode/COMMIT
chown opencode:opencode /opt/opencode/COMMIT

runuser -u opencode -- bash -lc 'export PATH="/home/opencode/.bun/bin:$PATH"; cd /tmp/opencode-repo; bun install --frozen-lockfile; cd packages/opencode; bun run build-single-ui'
runuser -u opencode -- bash -lc '. /home/opencode/.cargo/env; cd /tmp/opencode-repo/packages/opencode-broker; cargo build --release'

mkdir -p /opt/opencode/bin /opt/opencode/ui
cp /tmp/opencode-repo/packages/opencode/dist/opencode-*/bin/opencode /opt/opencode/bin/opencode
cp -R /tmp/opencode-repo/packages/opencode/dist/opencode-*/ui/. /opt/opencode/ui/
chown -R opencode:opencode /opt/opencode
chmod +x /opt/opencode/bin/opencode
cp /tmp/opencode-repo/packages/opencode-broker/target/release/opencode-broker /usr/local/bin/opencode-broker
chmod 4755 /usr/local/bin/opencode-broker
/opt/opencode/bin/opencode --version
rm -rf "$REPO"
"#
    );

    let (update_output, update_status) =
        exec_command_with_status(&client, CONTAINER_NAME, vec!["bash", "-lc", &update_script])
            .await
            .map_err(|e| anyhow!("Failed to update opencode: {e}"))?;
    if !quiet && !update_output.trim().is_empty() {
        eprintln!(
            "{} Update output:\n{}",
            style("[info]").cyan(),
            update_output.trim()
        );
    }
    if update_status != 0 {
        return Err(anyhow!(
            "Opencode update failed (exit {update_status}).\n{update_output}"
        ));
    }

    if let Some(expected) = next_commit.as_deref() {
        let updated_commit = get_current_opencode_commit(&client).await;
        if updated_commit.as_deref() != Some(expected) {
            let found = updated_commit.unwrap_or_else(|| "unknown".to_string());
            return Err(anyhow!(
                "Opencode update did not apply (expected {expected}, found {found}).\n{update_output}"
            ));
        }
    }

    spinner.success("Opencode updated, restarting service...");

    let restart_args = RestartArgs {};
    cmd_restart(&restart_args, maybe_host, quiet, verbose).await?;

    if !quiet {
        eprintln!();
        eprintln!(
            "{} Opencode updated successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
    }

    Ok(())
}

async fn get_current_opencode_version(client: &DockerClient) -> Option<String> {
    let output = exec_command(
        client,
        CONTAINER_NAME,
        vec!["/opt/opencode/bin/opencode", "--version"],
    )
    .await
    .ok()?;

    let version = output.lines().next()?.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

async fn get_current_opencode_commit(client: &DockerClient) -> Option<String> {
    let output = exec_command(client, CONTAINER_NAME, vec!["cat", "/opt/opencode/COMMIT"])
        .await
        .ok()?;

    let commit = output.lines().next()?.trim();
    if commit.is_empty() {
        None
    } else {
        Some(short_commit(commit))
    }
}

async fn resolve_remote_commit(client: &DockerClient, target_ref: &str) -> Option<String> {
    let output = exec_command(
        client,
        CONTAINER_NAME,
        vec![
            "git",
            "ls-remote",
            "https://github.com/pRizz/opencode.git",
            target_ref,
        ],
    )
    .await
    .ok()?;

    let full = output.split_whitespace().next()?;
    Some(short_commit(full))
}

fn short_commit(value: &str) -> String {
    value.chars().take(7).collect()
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
    let use_build = config.image_source == "build";

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
        eprintln!("{} Stopping service...", style("[1/4]").cyan());
    }
    if container_exists(client, CONTAINER_NAME).await? {
        let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
        if let Err(e) = stop_service(client, true, None).await {
            spinner.fail("Failed to stop service");
            return Err(anyhow!("Failed to stop service: {e}"));
        }
        spinner.success("Service stopped");
    } else if !quiet {
        eprintln!(
            "{} Container not found, skipping stop.",
            style("Note:").yellow()
        );
    }

    // Step 2: Get new image based on config.image_source
    if verbose > 0 {
        eprintln!("{} Getting new image...", style("[2/4]").cyan());
    }

    if use_build {
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

        build_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress, false, None)
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
        eprintln!("{} Recreating container...", style("[3/4]").cyan());
    }
    let spinner = CommandSpinner::new_maybe("Recreating container...", quiet);
    if let Err(e) = setup_and_start(
        client,
        Some(port),
        None,
        Some(bind_addr),
        Some(config.cockpit_port),
        Some(config.cockpit_enabled && COCKPIT_EXPOSED),
        None, // bind_mounts: update recreates without bind mounts (user can restart with mounts)
    )
    .await
    {
        spinner.fail("Failed to recreate container");
        return Err(anyhow!("Failed to recreate container: {e}"));
    }
    spinner.success("Container recreated");

    // Step 4: Show success
    if verbose > 0 {
        eprintln!("{} Update complete", style("[4/4]").cyan());
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
    if container_exists(client, CONTAINER_NAME).await? {
        let spinner = CommandSpinner::new_maybe("Stopping service...", quiet);
        if let Err(e) = stop_service(client, true, None).await {
            spinner.fail("Failed to stop service");
            return Err(anyhow!("Failed to stop service: {e}"));
        }
        spinner.success("Service stopped");
    } else if !quiet {
        eprintln!(
            "{} Container not found, skipping stop.",
            style("Note:").yellow()
        );
    }

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
        Some(config.cockpit_enabled && COCKPIT_EXPOSED),
        None, // bind_mounts: rollback recreates without bind mounts (user can restart with mounts)
    )
    .await
    {
        spinner.fail("Failed to recreate container");
        return Err(anyhow!("Failed to recreate container: {e}"));
    }
    spinner.success("Container recreated");

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
        eprintln!();
    }

    Ok(())
}
