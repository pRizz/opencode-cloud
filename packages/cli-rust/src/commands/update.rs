//! Update command implementation
//!
//! Updates the opencode image to the latest version or rolls back to previous version.

use crate::cli_platform::{
    CliInstallMethod, cli_platform_label, detect_install_method, is_dev_binary,
};
use crate::commands::disk_usage::{
    format_bytes_i64, format_disk_usage_report, format_host_disk_report, get_disk_usage_report,
    get_host_disk_report,
};
use crate::commands::{RestartArgs, cmd_restart};
use crate::constants::COCKPIT_EXPOSED;
use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use console::style;
use dialoguer::{Confirm, MultiSelect};
use opencode_cloud_core::config::load_config_or_default;
use opencode_cloud_core::docker::update::PREVIOUS_TAG;
use opencode_cloud_core::docker::update::tag_current_as_previous;
use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT, ImageState, ProgressReporter,
    build_image, container_exists, container_is_running, exec_command, exec_command_with_status,
    get_cli_version, get_image_version, get_registry_latest_version, has_previous_image,
    image_exists, pull_image, rollback_image, save_state, setup_and_start, stop_service,
};
use serde::Deserialize;
use std::process::Command;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UpdateTarget {
    Cli,
    Container,
    Opencode,
}

struct UpdateCandidate {
    target: UpdateTarget,
    label: &'static str,
    current: String,
    target_display: Option<String>,
    available: bool,
    selectable: bool,
    note: Option<String>,
}

struct CliTargetVersion {
    target: String,
    compatible: Option<String>,
}

impl UpdateCandidate {
    fn target_display(&self) -> &str {
        self.target_display.as_deref().unwrap_or("latest")
    }

    fn summary_line(&self) -> String {
        let label = format!("{:<10}", self.label);
        if self.available {
            format!(
                "{} {} -> {}",
                label,
                format_value(&self.current),
                format_value(self.target_display())
            )
        } else if !self.selectable {
            format!("{} {} (unavailable)", label, format_value(&self.current))
        } else {
            format!("{} {} (up to date)", label, format_value(&self.current))
        }
    }

    fn selection_label(&self) -> String {
        let label = format!("{:<10}", self.label);
        format!(
            "{} {} -> {}",
            label,
            format_value(&self.current),
            format_value(self.target_display())
        )
    }
}

fn format_value(value: &str) -> String {
    format!("{}", style(value).dim())
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
            if !args.rollback {
                return cmd_update_selector(args, maybe_host, quiet, verbose).await;
            }
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

async fn cmd_update_selector(
    args: &UpdateArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    let spinner = CommandSpinner::new_maybe("Checking for updates...", quiet);
    let cli_label = cli_platform_label();
    spinner.update(&format!("Checking {cli_label} version..."));
    let cli_candidate = build_cli_candidate();

    let (config, config_note) = load_update_config();
    let (docker_client, docker_note) =
        resolve_update_docker(config.is_some(), maybe_host, verbose).await;

    spinner.update("Checking container image...");
    let container_candidate = build_container_candidate(
        config.as_ref(),
        docker_client.as_ref(),
        config_note.as_deref(),
        docker_note.as_deref(),
    )
    .await;

    spinner.update("Checking opencode commit...");
    let (latest_opencode_commit, opencode_note) = match fetch_latest_opencode_commit().await {
        Ok(commit) => (Some(commit), None),
        Err(err) => (
            None,
            Some(format!("Failed to fetch latest opencode commit: {err}")),
        ),
    };
    let opencode_candidate = build_opencode_candidate(
        config.is_some(),
        docker_client.as_ref(),
        config_note.as_deref(),
        docker_note.as_deref(),
        latest_opencode_commit.as_deref(),
        opencode_note.as_deref(),
    )
    .await;

    spinner.success("Update check complete");

    let candidates = vec![cli_candidate, container_candidate, opencode_candidate];
    print_update_summary(&candidates, quiet);

    let selected_targets = select_update_targets(args, &candidates, quiet)?;
    if selected_targets.is_empty() {
        return Ok(());
    }

    if !confirm_update_selection(args)? {
        if !quiet {
            eprintln!("Update cancelled.");
        }
        return Ok(());
    }

    let selection = UpdateSelection::from_targets(&selected_targets);
    run_selected_updates(
        selection,
        config.as_ref(),
        docker_client.as_ref(),
        args,
        maybe_host,
        quiet,
        verbose,
    )
    .await
}

fn build_cli_candidate() -> UpdateCandidate {
    let current_cli = get_cli_version().to_string();
    let mut available = false;
    let mut selectable = true;
    let mut note = None;
    let mut target_display = None;
    let label = cli_platform_label();

    if is_dev_binary() {
        selectable = false;
        note = Some(
            "Dev build detected; self-update is disabled. Nice job running a dev build 😃"
                .to_string(),
        );
    } else if let Some(install_method) = detect_install_method() {
        let target_version = get_target_cli_version(&install_method);
        match target_version.as_ref() {
            Some(target) if target.target == current_cli => {
                available = false;
            }
            Some(target) => {
                available = true;
                target_display = Some(format!("v{}", target.target));
                if target.compatible.is_some() {
                    note = Some(
                        "Latest CLI requires a newer Rust toolchain. Run: rustup update, then re-run occ update cli."
                            .to_string(),
                    );
                }
            }
            None => {
                available = true;
                target_display = Some("latest (unknown)".to_string());
            }
        }
    } else {
        selectable = false;
        note = Some(
            "Unable to detect install method. Try: cargo install opencode-cloud or npm install -g opencode-cloud."
                .to_string(),
        );
    }

    UpdateCandidate {
        target: UpdateTarget::Cli,
        label,
        current: format!("v{current_cli}"),
        target_display,
        available,
        selectable,
        note,
    }
}

fn load_update_config() -> (Option<opencode_cloud_core::config::Config>, Option<String>) {
    match load_config_or_default() {
        Ok(config) => (Some(config), None),
        Err(err) => (None, Some(format!("Failed to load config: {err}"))),
    }
}

async fn resolve_update_docker(
    config_present: bool,
    maybe_host: Option<&str>,
    verbose: u8,
) -> (Option<DockerClient>, Option<String>) {
    if !config_present {
        return (None, None);
    }

    match crate::resolve_docker_client(maybe_host).await {
        Ok((client, host_name)) => {
            if verbose > 0 {
                let target = host_name.as_deref().unwrap_or("local");
                eprintln!(
                    "{} Connecting to Docker on {}...",
                    style("[info]").cyan(),
                    target
                );
            }
            match client.verify_connection().await {
                Ok(_) => (Some(client), None),
                Err(err) => (None, Some(format!("Docker connection error: {err}"))),
            }
        }
        Err(err) => (None, Some(format!("Docker connection error: {err}"))),
    }
}

async fn build_container_candidate(
    config: Option<&opencode_cloud_core::config::Config>,
    client: Option<&DockerClient>,
    config_note: Option<&str>,
    docker_note: Option<&str>,
) -> UpdateCandidate {
    let Some(config) = config else {
        return UpdateCandidate {
            target: UpdateTarget::Container,
            label: "Container",
            current: "unknown".to_string(),
            target_display: None,
            available: false,
            selectable: false,
            note: config_note.map(ToString::to_string),
        };
    };

    let Some(client) = client else {
        return UpdateCandidate {
            target: UpdateTarget::Container,
            label: "Container",
            current: "unknown".to_string(),
            target_display: None,
            available: false,
            selectable: false,
            note: docker_note.map(ToString::to_string),
        };
    };

    let image_name = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");
    let image_present = image_exists(client, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT)
        .await
        .unwrap_or(false);
    let current_version = if image_present {
        get_image_version(client, &image_name).await.ok().flatten()
    } else {
        None
    };
    let current_display = if !image_present {
        "not installed".to_string()
    } else if let Some(version) = current_version.as_deref() {
        if version == "dev" {
            "dev".to_string()
        } else {
            format!("v{version}")
        }
    } else {
        "unknown".to_string()
    };
    if current_version.as_deref() == Some("dev") {
        return UpdateCandidate {
            target: UpdateTarget::Container,
            label: "Container",
            current: current_display,
            target_display: None,
            available: false,
            selectable: false,
            note: Some(
                "Dev container detected; updates are disabled for dev images. Nice job running a dev build 👏"
                    .to_string(),
            ),
        };
    }

    let use_build = config.image_source == "build";
    let mut note = None;
    let maybe_registry_version = if use_build {
        None
    } else {
        match get_registry_latest_version(client).await {
            Ok(version) => version,
            Err(err) => {
                note = Some(format!("Failed to fetch registry version: {err}"));
                None
            }
        }
    };

    let target_display = if use_build {
        Some("build from source".to_string())
    } else if let Some(version) = maybe_registry_version.as_deref() {
        Some(format!("v{version}"))
    } else {
        Some("latest (unknown)".to_string())
    };

    let mut available = true;
    if !use_build
        && let (Some(current), Some(latest)) = (
            current_version.as_deref(),
            maybe_registry_version.as_deref(),
        )
        && current == latest
    {
        available = false;
    }

    UpdateCandidate {
        target: UpdateTarget::Container,
        label: "Container",
        current: current_display,
        target_display,
        available,
        selectable: true,
        note,
    }
}

async fn build_opencode_candidate(
    config_present: bool,
    client: Option<&DockerClient>,
    config_note: Option<&str>,
    docker_note: Option<&str>,
    latest_commit: Option<&str>,
    opencode_note: Option<&str>,
) -> UpdateCandidate {
    if !config_present {
        return UpdateCandidate {
            target: UpdateTarget::Opencode,
            label: "Opencode",
            current: "unknown".to_string(),
            target_display: latest_commit
                .map(|commit| commit.to_string())
                .or_else(|| Some("latest (unknown)".to_string())),
            available: false,
            selectable: false,
            note: config_note
                .map(ToString::to_string)
                .or_else(|| opencode_note.map(ToString::to_string)),
        };
    }

    let Some(client) = client else {
        return UpdateCandidate {
            target: UpdateTarget::Opencode,
            label: "Opencode",
            current: "unknown".to_string(),
            target_display: latest_commit
                .map(|commit| commit.to_string())
                .or_else(|| Some("latest (unknown)".to_string())),
            available: false,
            selectable: false,
            note: docker_note
                .map(ToString::to_string)
                .or_else(|| opencode_note.map(ToString::to_string)),
        };
    };

    let exists = container_exists(client, CONTAINER_NAME)
        .await
        .unwrap_or(false);
    if !exists {
        return UpdateCandidate {
            target: UpdateTarget::Opencode,
            label: "Opencode",
            current: "missing".to_string(),
            target_display: latest_commit
                .map(|commit| commit.to_string())
                .or_else(|| Some("latest (unknown)".to_string())),
            available: false,
            selectable: false,
            note: Some("Container not found; start or update the container first.".to_string()),
        };
    }

    let running = container_is_running(client, CONTAINER_NAME)
        .await
        .unwrap_or(false);
    let current_commit = if running {
        get_current_opencode_commit(client).await
    } else {
        None
    };
    let current_display = if running {
        current_commit
            .clone()
            .unwrap_or_else(|| "missing".to_string())
    } else {
        "unknown (stopped)".to_string()
    };

    let mut available = true;
    if let (Some(current), Some(latest)) = (current_commit.as_deref(), latest_commit)
        && current == latest
    {
        available = false;
    }

    UpdateCandidate {
        target: UpdateTarget::Opencode,
        label: "Opencode",
        current: current_display,
        target_display: latest_commit
            .map(|commit| commit.to_string())
            .or_else(|| Some("latest (unknown)".to_string())),
        available,
        selectable: true,
        note: opencode_note.map(ToString::to_string),
    }
}

fn print_update_summary(candidates: &[UpdateCandidate], quiet: bool) {
    if quiet {
        return;
    }

    eprintln!();
    eprintln!("{}", style("Update status").bold());
    eprintln!("{}", style("-------------").dim());
    for candidate in candidates {
        eprintln!("{}", candidate.summary_line());
        if let Some(note) = candidate.note.as_deref() {
            eprintln!("  {}", style(note).dim());
        }
    }
    eprintln!();
}

fn select_update_targets(
    args: &UpdateArgs,
    candidates: &[UpdateCandidate],
    quiet: bool,
) -> Result<Vec<UpdateTarget>> {
    let selectable_candidates: Vec<&UpdateCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.selectable && candidate.available)
        .collect();

    if selectable_candidates.is_empty() {
        if !quiet {
            eprintln!("Everything is already up to date.");
        }
        return Ok(Vec::new());
    }

    let selected_targets: Vec<UpdateTarget> = if args.yes {
        selectable_candidates
            .iter()
            .map(|candidate| candidate.target)
            .collect()
    } else {
        let labels: Vec<String> = selectable_candidates
            .iter()
            .map(|candidate| candidate.selection_label())
            .collect();
        let defaults = vec![true; labels.len()];
        let selections = MultiSelect::new()
            .with_prompt("Select updates to apply (Space to toggle, Enter to confirm)")
            .items(&labels)
            .defaults(&defaults)
            .interact()?;
        selections
            .into_iter()
            .map(|index| selectable_candidates[index].target)
            .collect()
    };

    if selected_targets.is_empty() && !quiet {
        eprintln!("No updates selected.");
    }

    Ok(selected_targets)
}

fn confirm_update_selection(args: &UpdateArgs) -> Result<bool> {
    if args.yes {
        return Ok(true);
    }

    let confirmed = Confirm::new()
        .with_prompt("Proceed with selected updates?")
        .default(true)
        .interact()?;
    Ok(confirmed)
}

struct UpdateSelection {
    cli: bool,
    container: bool,
    opencode: bool,
}

impl UpdateSelection {
    fn from_targets(targets: &[UpdateTarget]) -> Self {
        let mut selection = Self {
            cli: false,
            container: false,
            opencode: false,
        };
        for target in targets {
            match target {
                UpdateTarget::Cli => selection.cli = true,
                UpdateTarget::Container => selection.container = true,
                UpdateTarget::Opencode => selection.opencode = true,
            }
        }
        selection
    }
}

async fn run_selected_updates(
    selection: UpdateSelection,
    config: Option<&opencode_cloud_core::config::Config>,
    docker_client: Option<&DockerClient>,
    args: &UpdateArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    if selection.cli {
        let cli_args = UpdateCliArgs { yes: args.yes };
        cmd_update_cli(&cli_args, maybe_host, quiet, verbose).await?;
    }

    if selection.container {
        let Some(config) = config else {
            return Err(anyhow!("Failed to load config; cannot update container."));
        };
        let Some(client) = docker_client else {
            return Err(anyhow!("Docker is unavailable; cannot update container."));
        };
        handle_update(client, config, args.yes, quiet, verbose, None).await?;
    }

    if selection.opencode {
        let opencode_args = UpdateOpencodeArgs {
            branch: None,
            commit: None,
            yes: args.yes,
        };
        cmd_update_opencode(&opencode_args, maybe_host, quiet, verbose).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_cargo_info_versions;

    #[test]
    fn parse_cargo_info_versions_latest() {
        let output = "name: opencode-cloud\nversion: 10.4.0 (latest 11.0.0)\n";
        let parsed = parse_cargo_info_versions(output).expect("should parse version");
        assert_eq!(parsed.target, "11.0.0");
        assert_eq!(parsed.compatible.as_deref(), Some("10.4.0"));
    }

    #[test]
    fn parse_cargo_info_versions_plain() {
        let output = "name: opencode-cloud\nversion: 11.0.0\n";
        let parsed = parse_cargo_info_versions(output).expect("should parse version");
        assert_eq!(parsed.target, "11.0.0");
        assert!(parsed.compatible.is_none());
    }

    #[test]
    fn parse_cargo_info_versions_from_path() {
        let output = "name: opencode-cloud\nversion: 11.0.0 (from ./packages/cli-rust)\n";
        let parsed = parse_cargo_info_versions(output).expect("should parse version");
        assert_eq!(parsed.target, "11.0.0");
        assert!(parsed.compatible.is_none());
    }
}

async fn cmd_update_cli(
    args: &UpdateCliArgs,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    let cli_label = cli_platform_label();

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

    let current_version = get_cli_version();
    let maybe_target_version = get_target_cli_version(&install_method);
    if let Some(target_version) = maybe_target_version.as_ref()
        && target_version.target == current_version
    {
        if !quiet {
            let check = style("✓").green();
            eprintln!(
                "{} opencode-cloud {cli_label} is already up to date (version {}).",
                check,
                style(current_version).dim()
            );
        }
        return Ok(());
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will update the opencode-cloud {cli_label} and restart the service.",
            style("Warning:").yellow().bold()
        );
        eprintln!("Install:    {}", style(install_method.label()).dim());
        eprintln!("Current:    {}", style(current_version).dim());
        if let Some(target_version) = maybe_target_version.as_ref() {
            eprintln!(
                "Target:     {}",
                style(format!("v{}", target_version.target)).dim()
            );
        } else {
            eprintln!("Target:     {}", style("latest").dim());
        }
        if let Some(target_version) = maybe_target_version.as_ref()
            && target_version.compatible.is_some()
        {
            eprintln!();
            eprintln!(
                "{} Latest CLI requires a newer Rust toolchain.",
                style("Warning:").yellow().bold()
            );
            eprintln!("  Run: {}", style("rustup update").dim());
            eprintln!("  Then re-run: {}", style("occ update cli").dim());
        }
        eprintln!();
    }

    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Continue with opencode-cloud {cli_label} update?"))
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Update cancelled.");
            }
            return Ok(());
        }
    }

    let spinner =
        CommandSpinner::new_maybe(&format!("Updating opencode-cloud {cli_label}..."), quiet);
    let target_version = maybe_target_version
        .as_ref()
        .map(|info| info.target.as_str());
    install_method
        .run_update(target_version)
        .map_err(|e| anyhow!("{e}"))?;
    spinner.success(&format!("opencode-cloud {cli_label} updated"));

    let restart_args = RestartArgs {};
    cmd_restart(&restart_args, maybe_host, quiet, verbose).await?;

    if !quiet {
        eprintln!();
        eprintln!(
            "{} opencode-cloud {cli_label} updated successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
    }

    Ok(())
}

fn get_target_cli_version(install_method: &CliInstallMethod) -> Option<CliTargetVersion> {
    let (program, args) = match install_method {
        CliInstallMethod::Cargo => ("cargo", vec!["info", "opencode-cloud"]),
        CliInstallMethod::Npm => ("npm", vec!["view", "opencode-cloud", "version"]),
    };

    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match install_method {
        CliInstallMethod::Cargo => parse_cargo_info_versions(&stdout),
        CliInstallMethod::Npm => parse_npm_view_version(&stdout).map(|version| CliTargetVersion {
            target: version,
            compatible: None,
        }),
    }
}

fn parse_cargo_info_versions(output: &str) -> Option<CliTargetVersion> {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("version:"))?;
    let (_, value) = line.split_once(':')?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let first_token = value.split_whitespace().next()?;
    let compatible = first_token.trim_matches(|ch| ch == '(' || ch == ')');
    let latest = parse_latest_token(value);
    let target = latest.unwrap_or_else(|| compatible.to_string());
    let compatible = if target == compatible {
        None
    } else {
        Some(compatible.to_string())
    };

    Some(CliTargetVersion { target, compatible })
}

fn parse_latest_token(value: &str) -> Option<String> {
    let mut iter = value.split_whitespace();
    while let Some(token) = iter.next() {
        let cleaned = token.trim_matches(|ch| ch == '(' || ch == ')');
        if cleaned == "latest" {
            let next = iter.next()?;
            let version = next.trim_matches(|ch| ch == '(' || ch == ')');
            if version.is_empty() {
                return None;
            }
            return Some(version.to_string());
        }
    }
    None
}

fn parse_npm_view_version(output: &str) -> Option<String> {
    let value = output.lines().next()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
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

#[derive(Deserialize)]
struct GithubCommitResponse {
    sha: String,
}

async fn fetch_latest_opencode_commit() -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("opencode-cloud")
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {e}"))?;

    let response = client
        .get("https://api.github.com/repos/pRizz/opencode/commits/dev")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to request latest commit: {e}"))?;

    if !response.status().is_success() {
        return Err(anyhow!("GitHub API returned status {}", response.status()));
    }

    let commit: GithubCommitResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse commit response: {e}"))?;

    Ok(short_commit(&commit.sha))
}

async fn purge_unused_docker_resources(client: &DockerClient, quiet: bool) -> Result<Option<i64>> {
    let spinner = CommandSpinner::new_maybe("Pruning unused Docker resources...", quiet);
    let mut reclaimed = 0i64;
    let mut has_reclaimed = false;

    let container_prune = client
        .inner()
        .prune_containers::<String>(None)
        .await
        .map_err(|e| anyhow!("Failed to prune containers: {e}"))?;
    if let Some(value) = container_prune.space_reclaimed {
        reclaimed += value;
        has_reclaimed = true;
    }

    let image_prune = client
        .inner()
        .prune_images::<String>(None)
        .await
        .map_err(|e| anyhow!("Failed to prune images: {e}"))?;
    if let Some(value) = image_prune.space_reclaimed {
        reclaimed += value;
        has_reclaimed = true;
    }

    client
        .inner()
        .prune_networks::<String>(None)
        .await
        .map_err(|e| anyhow!("Failed to prune networks: {e}"))?;

    spinner.success("Docker resources pruned");
    Ok(has_reclaimed.then_some(reclaimed))
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
    let image_name = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");
    let maybe_current_image_version = get_image_version(client, &image_name).await.ok().flatten();
    if maybe_current_image_version.as_deref() == Some("dev") {
        if !quiet {
            eprintln!(
                "{} Dev container detected; updates are disabled for dev images.",
                style("Note:").yellow()
            );
            eprintln!(
                "{} Nice job running a dev build 🎉 Rebuild from source or switch to a prebuilt image.",
                style("Tip:").cyan()
            );
        }
        return Ok(());
    }
    let maybe_registry_version = if quiet || use_build {
        None
    } else {
        let spinner = CommandSpinner::new_maybe("Checking registry version...", quiet);
        match get_registry_latest_version(client).await {
            Ok(version) => {
                spinner.success("Registry version checked");
                version
            }
            Err(err) => {
                spinner.fail("Failed to check registry version");
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };

    if !quiet
        && let (Some(current), Some(latest)) = (
            maybe_current_image_version.as_deref(),
            maybe_registry_version.as_deref(),
        )
        && current == latest
    {
        let check = style("✓").green();
        eprintln!(
            "{} Container image is already up to date (version {}).",
            check,
            style(latest).dim()
        );
        return Ok(());
    }
    let maybe_usage_before = if quiet {
        None
    } else {
        match get_disk_usage_report(client).await {
            Ok(report) => Some(report),
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };
    let maybe_host_before = if quiet {
        None
    } else {
        match get_host_disk_report(client) {
            Ok(Some(report)) => Some(report),
            Ok(None) => {
                if client.is_remote() {
                    eprintln!(
                        "{} Host disk stats unavailable for remote Docker hosts.",
                        style("Note:").yellow()
                    );
                }
                None
            }
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };

    // Show warning about downtime
    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will briefly stop the service to apply the update.",
            style("Warning:").yellow().bold()
        );
        let current = maybe_current_image_version.as_deref().unwrap_or("unknown");
        eprintln!("Current:    {}", style(current).dim());
        if use_build {
            eprintln!("Target:     {}", style("build from source").dim());
        } else if let Some(version) = maybe_registry_version.as_deref() {
            eprintln!(
                "Target:     {}",
                style(format!("latest (registry, version {version})")).dim()
            );
        } else {
            eprintln!("Target:     {}", style("latest (registry)").dim());
        }
        eprintln!();
        if let Some(report) = maybe_usage_before {
            for line in format_disk_usage_report("before update", report, None) {
                eprintln!("{line}");
            }
            eprintln!();
        }
        if let Some(report) = maybe_host_before {
            for line in format_host_disk_report("before update", report, None) {
                eprintln!("{line}");
            }
            eprintln!();
        }
    }

    // Confirm with user unless --yes
    if !skip_confirm {
        if !quiet {
            eprintln!(
                "{} Unused images and containers will be purged to save space.",
                style("Notice:").yellow().bold()
            );
            eprintln!();
        }
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

    let mut prebuilt_pulled = false;
    let mut maybe_target_version = None;
    if !use_build {
        if verbose > 0 {
            eprintln!(
                "{} Checking for image updates...",
                style("[preflight]").cyan()
            );
        }
        tag_current_as_previous(client)
            .await
            .map_err(|e| anyhow!("Failed to backup current image: {e}"))?;

        let mut progress = if quiet {
            ProgressReporter::new()
        } else {
            ProgressReporter::with_context("Checking image")
        };

        let full_image = pull_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress)
            .await
            .map_err(|e| anyhow!("Failed to pull image: {e}"))?;
        prebuilt_pulled = true;
        maybe_target_version = get_image_version(client, &full_image).await.ok().flatten();

        let previous_image = format!("{IMAGE_NAME_GHCR}:{PREVIOUS_TAG}");
        let maybe_previous_version = get_image_version(client, &previous_image)
            .await
            .ok()
            .flatten();
        if maybe_previous_version.is_some() && maybe_target_version == maybe_previous_version {
            if !quiet {
                let check = style("✓").green();
                let version = maybe_target_version.as_deref().unwrap_or("unknown");
                eprintln!(
                    "{} Container image is already up to date (version {}).",
                    check,
                    style(version).dim()
                );
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

        let full_image = if prebuilt_pulled {
            image_name.clone()
        } else {
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

            pull_image(client, Some(IMAGE_TAG_DEFAULT), &mut progress)
                .await
                .map_err(|e| anyhow!("Failed to pull image: {e}"))?
        };

        // Determine registry and save provenance
        let registry = if full_image.starts_with("ghcr.io") {
            "ghcr.io"
        } else {
            "docker.io"
        };
        let version = maybe_target_version
            .as_deref()
            .unwrap_or_else(|| get_cli_version());
        save_state(&ImageState::prebuilt(version, registry)).ok();
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

    let maybe_usage_after_update = if quiet {
        None
    } else {
        match get_disk_usage_report(client).await {
            Ok(report) => Some(report),
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };
    let maybe_host_after_update = if quiet {
        None
    } else {
        match get_host_disk_report(client) {
            Ok(report) => report,
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };

    let maybe_reclaimed = purge_unused_docker_resources(client, quiet).await?;
    let maybe_usage_after_purge = if quiet {
        None
    } else {
        match get_disk_usage_report(client).await {
            Ok(report) => Some(report),
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };
    let maybe_host_after_purge = if quiet {
        None
    } else {
        match get_host_disk_report(client) {
            Ok(report) => report,
            Err(err) => {
                eprintln!("{} {err}", style("Warning:").yellow().bold());
                None
            }
        }
    };

    if !quiet {
        eprintln!();
        if let Some(report) = maybe_usage_after_update {
            for line in format_disk_usage_report("after update", report, maybe_usage_before) {
                eprintln!("{line}");
            }
            eprintln!();
        }
        if let Some(report) = maybe_host_after_update {
            for line in format_host_disk_report("after update", report, maybe_host_before) {
                eprintln!("{line}");
            }
            eprintln!();
        }
        if let Some(reclaimed) = maybe_reclaimed {
            eprintln!(
                "Docker purge reclaimed: {}",
                style(format_bytes_i64(reclaimed)).dim()
            );
        }
        if let Some(report) = maybe_usage_after_purge {
            for line in format_disk_usage_report("after purge", report, maybe_usage_before) {
                eprintln!("{line}");
            }
            eprintln!();
        }
        if let Some(report) = maybe_host_after_purge {
            for line in format_host_disk_report("after purge", report, maybe_host_before) {
                eprintln!("{line}");
            }
            eprintln!();
        }
    }

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
