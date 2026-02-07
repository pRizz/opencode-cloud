//! opencode-cloud CLI - Manage your opencode cloud service
//!
//! This module contains the shared CLI implementation used by all binaries.

mod cli_platform;
mod commands;
mod constants;
mod output;
mod passwords;
mod sandbox_profile;
pub mod wizard;

use crate::commands::runtime_shared::drift::{
    RuntimeAssetDrift, detect_runtime_asset_drift, stale_container_warning_lines,
};
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::{
    DockerClient, InstanceLock, SingletonError, config, get_version, load_config_or_default,
    load_hosts, save_config,
};
use std::path::Path;

/// Manage your opencode cloud service
#[derive(Parser)]
#[command(name = "opencode-cloud")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Manage your opencode cloud service", long_about = None)]
#[command(after_help = get_banner())]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Increase verbosity level
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Target remote host (overrides default_host)
    #[arg(long, global = true, conflicts_with = "local")]
    remote_host: Option<String>,

    /// Force local Docker (ignores default_host)
    #[arg(long, global = true, conflicts_with = "remote_host")]
    local: bool,

    /// Runtime mode (auto-detect container vs host)
    #[arg(long, global = true, value_enum)]
    runtime: Option<RuntimeChoice>,

    /// Optional sandbox instance profile for worktree-isolated resources
    #[arg(long, global = true, value_name = "NAME|auto")]
    sandbox_instance: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the opencode service
    Start(commands::StartArgs),
    /// Stop the opencode service
    Stop(commands::StopArgs),
    /// Restart the opencode service
    Restart(commands::RestartArgs),
    /// Show service status
    Status(commands::StatusArgs),
    /// View service logs
    Logs(commands::LogsArgs),
    /// Register service to start on boot/login
    Install(commands::InstallArgs),
    /// Remove service registration
    Uninstall(commands::UninstallArgs),
    /// Manage configuration
    Config(commands::ConfigArgs),
    /// Run interactive setup wizard
    Setup(commands::SetupArgs),
    /// Manage container users
    User(commands::UserArgs),
    /// Manage bind mounts
    Mount(commands::MountArgs),
    /// Reset containers, mounts, and host data
    Reset(commands::ResetArgs),
    /// Update to the latest version or rollback (interactive when no subcommand is provided)
    Update(commands::UpdateArgs),
    /// Open Cockpit web console
    #[command(hide = true)]
    Cockpit(commands::CockpitArgs),
    /// Manage remote hosts
    Host(commands::HostArgs),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum RuntimeChoice {
    Auto,
    Host,
    Container,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeMode {
    Host,
    Container,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    None,
    Status,
    Other,
}

/// Get the ASCII banner for help display
fn get_banner() -> &'static str {
    r#"
  ___  _ __   ___ _ __   ___ ___   __| | ___
 / _ \| '_ \ / _ \ '_ \ / __/ _ \ / _` |/ _ \
| (_) | |_) |  __/ | | | (_| (_) | (_| |  __/
 \___/| .__/ \___|_| |_|\___\___/ \__,_|\___|
      |_|                            cloud
"#
}

/// Resolve the target host name based on flags and hosts.json
///
/// Resolution order:
/// 1. --local (force local Docker)
/// 2. --remote-host flag (explicit)
/// 3. default_host from hosts.json
/// 4. Local Docker (no host_name)
pub fn resolve_target_host(remote_host: Option<&str>, force_local: bool) -> Option<String> {
    if force_local {
        return None;
    }

    if let Some(name) = remote_host {
        return Some(name.to_string());
    }

    let hosts = load_hosts().unwrap_or_default();
    hosts.default_host.clone()
}

/// Resolve which Docker client to use based on an explicit target host name
///
/// Returns (DockerClient, Option<host_name>) where host_name is Some for remote connections.
pub async fn resolve_docker_client(
    maybe_host: Option<&str>,
) -> anyhow::Result<(DockerClient, Option<String>)> {
    let hosts = load_hosts().unwrap_or_default();

    // Determine target host
    let target_host = maybe_host.map(String::from);

    match target_host {
        Some(name) => {
            // Remote host requested
            let host_config = hosts.get_host(&name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Host '{name}' not found. Run 'occ host list' to see available hosts."
                )
            })?;

            let client = DockerClient::connect_remote(host_config, &name).await?;
            Ok((client, Some(name)))
        }
        None => {
            // Local Docker
            let client = DockerClient::new()?;
            Ok((client, None))
        }
    }
}

/// Format a message with optional host prefix
///
/// For remote hosts: "[prod-1] Starting container..."
/// For local: "Starting container..."
pub fn format_host_message(host_name: Option<&str>, message: &str) -> String {
    match host_name {
        Some(name) => format!("[{}] {}", style(name).cyan(), message),
        None => message.to_string(),
    }
}

fn container_runtime_from_markers(is_container: bool, is_opencode_image: bool) -> bool {
    is_container && is_opencode_image
}

fn detect_container_runtime() -> bool {
    let is_container =
        Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists();
    let is_opencode_image = Path::new("/etc/opencode-cloud-version").exists()
        || Path::new("/opt/opencode/COMMIT").exists();
    container_runtime_from_markers(is_container, is_opencode_image)
}

fn runtime_choice_from_env() -> Option<RuntimeChoice> {
    let value = std::env::var("OPENCODE_RUNTIME").ok()?;
    match value.to_lowercase().as_str() {
        "auto" => Some(RuntimeChoice::Auto),
        "host" => Some(RuntimeChoice::Host),
        "container" => Some(RuntimeChoice::Container),
        _ => None,
    }
}

fn resolve_runtime(choice: RuntimeChoice) -> (RuntimeMode, bool) {
    let auto_container = detect_container_runtime();
    resolve_runtime_with_autodetect(choice, auto_container)
}

fn resolve_runtime_with_autodetect(
    choice: RuntimeChoice,
    auto_container: bool,
) -> (RuntimeMode, bool) {
    match choice {
        RuntimeChoice::Host => (RuntimeMode::Host, false),
        RuntimeChoice::Container => (RuntimeMode::Container, false),
        RuntimeChoice::Auto => {
            let mode = if auto_container {
                RuntimeMode::Container
            } else {
                RuntimeMode::Host
            };
            (mode, auto_container)
        }
    }
}

fn container_mode_unsupported_error() -> anyhow::Error {
    anyhow!(
        "Command not supported in container runtime.\n\
Supported commands:\n  occ status\n  occ logs\n  occ user\n  occ update opencode\n\
To force host runtime:\n  occ --runtime host <command>\n  OPENCODE_RUNTIME=host occ <command>"
    )
}

fn command_kind(command: Option<&Commands>) -> CommandKind {
    match command {
        None => CommandKind::None,
        Some(Commands::Status(_)) => CommandKind::Status,
        Some(_) => CommandKind::Other,
    }
}

fn should_run_runtime_asset_preflight(
    kind: CommandKind,
    target_host: Option<&str>,
    quiet: bool,
) -> bool {
    if quiet || target_host.is_some() {
        return false;
    }
    matches!(kind, CommandKind::Other)
}

fn run_container_mode(cli: &Cli) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    match cli.command {
        Some(Commands::Status(ref args)) => rt.block_on(commands::container::cmd_status_container(
            args,
            cli.quiet,
            cli.verbose,
        )),
        Some(Commands::Logs(ref args)) => {
            rt.block_on(commands::container::cmd_logs_container(args, cli.quiet))
        }
        Some(Commands::User(ref args)) => rt.block_on(commands::container::cmd_user_container(
            args,
            cli.quiet,
            cli.verbose,
        )),
        Some(Commands::Update(ref args)) => rt.block_on(commands::container::cmd_update_container(
            args,
            cli.quiet,
            cli.verbose,
        )),
        Some(_) => Err(container_mode_unsupported_error()),
        None => {
            let status_args = commands::StatusArgs {};
            rt.block_on(commands::container::cmd_status_container(
                &status_args,
                cli.quiet,
                cli.verbose,
            ))
        }
    }
}

pub fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Configure color output
    if cli.no_color {
        console::set_colors_enabled(false);
    }

    let sandbox_profile =
        sandbox_profile::resolve_sandbox_profile(cli.sandbox_instance.as_deref())?;
    sandbox_profile::apply_active_profile_env(&sandbox_profile);
    if cli.verbose > 0
        && let Some(instance) = sandbox_profile.instance_id.as_deref()
    {
        eprintln!(
            "{} Using sandbox instance profile: {}",
            style("[info]").cyan(),
            style(instance).cyan()
        );
    }

    eprintln!(
        "{} This tool is still a work in progress and is rapidly evolving. Expect frequent updates and breaking changes. Follow updates at https://github.com/pRizz/opencode-cloud. Stability will be announced at some point. Use with caution.",
        style("Warning:").yellow().bold()
    );
    eprintln!();

    let runtime_choice = cli
        .runtime
        .or_else(runtime_choice_from_env)
        .unwrap_or(RuntimeChoice::Auto);
    let (runtime_mode, auto_container) = resolve_runtime(runtime_choice);

    if runtime_mode == RuntimeMode::Container {
        if cli.remote_host.is_some() || cli.local {
            return Err(anyhow!(
                "Remote and local Docker flags are not supported in container runtime.\n\
Use host mode instead:\n  occ --runtime host <command>"
            ));
        }

        if auto_container && runtime_choice == RuntimeChoice::Auto && !cli.quiet {
            eprintln!(
                "{} Detected opencode container; using container runtime. Override with {} or {}.",
                style("Info:").cyan(),
                style("--runtime host").green(),
                style("OPENCODE_RUNTIME=host").green()
            );
            eprintln!();
        }

        return run_container_mode(&cli);
    }

    let config_path = config::paths::get_config_path()
        .ok_or_else(|| anyhow!("Could not determine config path"))?;
    let config_exists = config_path.exists();

    let skip_wizard = matches!(
        cli.command,
        Some(Commands::Setup(ref args)) if args.bootstrap || args.yes
    );

    if !config_exists && !skip_wizard {
        eprintln!(
            "{} First-time setup required. Running wizard...",
            style("Note:").cyan()
        );
        eprintln!();
        let rt = tokio::runtime::Runtime::new()?;
        let new_config = rt.block_on(wizard::run_wizard(None))?;
        save_config(&new_config)?;
        eprintln!();
        eprintln!(
            "{} Setup complete! Run your command again, or use 'occ start' to begin.",
            style("Success:").green().bold()
        );
        return Ok(());
    }

    // Load config
    let config = match load_config_or_default() {
        Ok(config) => {
            // If config was just created, inform the user
            if cli.verbose > 0 {
                eprintln!(
                    "{} Config loaded from: {}",
                    style("[info]").cyan(),
                    config_path.display()
                );
            }
            config
        }
        Err(e) => {
            // Display rich error for invalid config
            eprintln!("{} Configuration error", style("Error:").red().bold());
            eprintln!();
            eprintln!("  {e}");
            eprintln!();
            eprintln!("  Config file: {}", style(config_path.display()).yellow());
            eprintln!();
            eprintln!(
                "  {} Check the config file for syntax errors or unknown fields.",
                style("Tip:").cyan()
            );
            eprintln!(
                "  {} See schemas/config.example.jsonc for valid configuration.",
                style("Tip:").cyan()
            );
            std::process::exit(1);
        }
    };

    // Show verbose info if requested
    if cli.verbose > 0 {
        let data_dir = config::paths::get_data_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        eprintln!(
            "{} Config: {}",
            style("[info]").cyan(),
            config_path.display()
        );
        eprintln!("{} Data: {}", style("[info]").cyan(), data_dir);
    }

    // Store target host for command handlers
    let target_host = resolve_target_host(cli.remote_host.as_deref(), cli.local);
    let dispatch_kind = command_kind(cli.command.as_ref());

    if should_run_runtime_asset_preflight(dispatch_kind, target_host.as_deref(), cli.quiet) {
        match tokio::runtime::Runtime::new() {
            Ok(rt) => {
                if let Err(err) = rt.block_on(maybe_print_runtime_asset_preflight(
                    target_host.as_deref(),
                    cli.verbose,
                )) && cli.verbose > 0
                {
                    eprintln!(
                        "{} Runtime drift preflight failed: {err}",
                        style("[warn]").yellow()
                    );
                }
            }
            Err(err) => {
                if cli.verbose > 0 {
                    eprintln!(
                        "{} Failed to initialize runtime drift preflight: {err}",
                        style("[warn]").yellow()
                    );
                }
            }
        }
    }

    match cli.command {
        Some(Commands::Start(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_start(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Stop(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_stop(&args, target_host.as_deref(), cli.quiet))
        }
        Some(Commands::Restart(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_restart(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Status(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_status(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Logs(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_logs(&args, target_host.as_deref(), cli.quiet))
        }
        Some(Commands::Install(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_install(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Uninstall(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_uninstall(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Config(cmd)) => commands::cmd_config(cmd, &config, cli.quiet),
        Some(Commands::Setup(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_setup(&args, cli.quiet))
        }
        Some(Commands::User(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_user(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Mount(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_mount(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Reset(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_reset(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Update(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_update(
                &args,
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
        Some(Commands::Cockpit(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_cockpit(
                &args,
                target_host.as_deref(),
                cli.quiet,
            ))
        }
        Some(Commands::Host(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_host(&args, cli.quiet, cli.verbose))
        }
        None => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(handle_no_command(
                target_host.as_deref(),
                cli.quiet,
                cli.verbose,
            ))
        }
    }
}

async fn handle_no_command(target_host: Option<&str>, quiet: bool, verbose: u8) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let (client, host_name) = resolve_docker_client(target_host).await?;
    client
        .verify_connection()
        .await
        .map_err(|e| anyhow!("Docker connection error: {e}"))?;

    let running = opencode_cloud_core::docker::container_is_running(
        &client,
        opencode_cloud_core::docker::CONTAINER_NAME,
    )
    .await
    .map_err(|e| anyhow!("Docker error: {e}"))?;

    if running {
        let status_args = commands::StatusArgs {};
        return commands::cmd_status(&status_args, host_name.as_deref(), quiet, verbose).await;
    }

    eprintln!("{} Service is not running.", style("Note:").yellow());

    let confirmed = Confirm::new()
        .with_prompt("Start the service now?")
        .default(true)
        .interact()?;

    if confirmed {
        let start_args = commands::StartArgs {
            port: None,
            open: false,
            no_daemon: false,
            pull_sandbox_image: false,
            cached_rebuild_sandbox_image: false,
            full_rebuild_sandbox_image: false,
            local_opencode_submodule: false,
            ignore_version: false,
            no_update_check: false,
            mounts: Vec::new(),
            no_mounts: false,
        };
        commands::cmd_start(&start_args, host_name.as_deref(), quiet, verbose).await?;
        let status_args = commands::StatusArgs {};
        return commands::cmd_status(&status_args, host_name.as_deref(), quiet, verbose).await;
    }

    print_help_hint();
    Ok(())
}

async fn maybe_print_runtime_asset_preflight(target_host: Option<&str>, verbose: u8) -> Result<()> {
    let (client, host_name) = resolve_docker_client(target_host).await?;
    if host_name.is_some() {
        return Ok(());
    }

    let report = detect_runtime_asset_drift(&client).await;
    print_runtime_asset_preflight_warning(&report, verbose);
    Ok(())
}

fn print_runtime_asset_preflight_warning(report: &RuntimeAssetDrift, verbose: u8) {
    if !report.drift_detected {
        return;
    }

    eprintln!(
        "{} {}",
        style("Warning:").yellow().bold(),
        style("Local container drift detected.").yellow()
    );
    for line in render_runtime_asset_preflight_lines(report, verbose) {
        eprintln!("  {line}");
    }
    eprintln!();
}

fn render_runtime_asset_preflight_lines(report: &RuntimeAssetDrift, verbose: u8) -> Vec<String> {
    let mut lines = stale_container_warning_lines(report);
    if verbose > 0 {
        for detail in &report.diagnostics {
            lines.push(format!("diagnostic: {detail}"));
        }
    }
    lines
}

fn print_help_hint() {
    println!(
        "{} {}",
        style("opencode-cloud").cyan().bold(),
        style(get_version()).dim()
    );
    println!();
    println!("Run {} for available commands.", style("--help").green());
}

/// Acquire the singleton lock for service management commands
///
/// This should be called before any command that manages the service
/// (start, stop, restart, status, etc.) to ensure only one instance runs.
/// Config commands don't need the lock as they're read-only or file-based.
#[allow(dead_code)]
fn acquire_singleton_lock() -> Result<InstanceLock, SingletonError> {
    let pid_path = config::paths::get_data_dir()
        .ok_or(SingletonError::InvalidPath)?
        .join("opencode-cloud.pid");

    InstanceLock::acquire(pid_path)
}

/// Display a rich error message when another instance is already running
#[allow(dead_code)]
fn display_singleton_error(err: &SingletonError) {
    match err {
        SingletonError::AlreadyRunning(pid) => {
            eprintln!(
                "{} Another instance is already running",
                style("Error:").red().bold()
            );
            eprintln!();
            eprintln!("  Process ID: {}", style(pid).yellow());
            eprintln!();
            eprintln!(
                "  {} Stop the existing instance first:",
                style("Tip:").cyan()
            );
            eprintln!("       {} stop", style("opencode-cloud").green());
            eprintln!();
            eprintln!(
                "  {} If the process is stuck, kill it manually:",
                style("Tip:").cyan()
            );
            eprintln!("       {} {}", style("kill").green(), pid);
        }
        SingletonError::CreateDirFailed(msg) => {
            eprintln!(
                "{} Failed to create data directory",
                style("Error:").red().bold()
            );
            eprintln!();
            eprintln!("  {msg}");
            eprintln!();
            if let Some(data_dir) = config::paths::get_data_dir() {
                eprintln!("  {} Check permissions for:", style("Tip:").cyan());
                eprintln!("       {}", style(data_dir.display()).yellow());
            }
        }
        SingletonError::LockFailed(msg) => {
            eprintln!("{} Failed to acquire lock", style("Error:").red().bold());
            eprintln!();
            eprintln!("  {msg}");
        }
        SingletonError::InvalidPath => {
            eprintln!(
                "{} Could not determine lock file path",
                style("Error:").red().bold()
            );
            eprintln!();
            eprintln!(
                "  {} Ensure XDG_DATA_HOME or HOME is set.",
                style("Tip:").cyan()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_marker_logic_requires_both_markers() {
        assert!(container_runtime_from_markers(true, true));
        assert!(!container_runtime_from_markers(true, false));
        assert!(!container_runtime_from_markers(false, true));
        assert!(!container_runtime_from_markers(false, false));
    }

    #[test]
    fn runtime_precedence_respects_explicit_choice() {
        let (mode, auto) = resolve_runtime_with_autodetect(RuntimeChoice::Host, true);
        assert_eq!(mode, RuntimeMode::Host);
        assert!(!auto);

        let (mode, auto) = resolve_runtime_with_autodetect(RuntimeChoice::Container, false);
        assert_eq!(mode, RuntimeMode::Container);
        assert!(!auto);
    }

    #[test]
    fn runtime_auto_uses_detection() {
        let (mode, auto) = resolve_runtime_with_autodetect(RuntimeChoice::Auto, true);
        assert_eq!(mode, RuntimeMode::Container);
        assert!(auto);

        let (mode, auto) = resolve_runtime_with_autodetect(RuntimeChoice::Auto, false);
        assert_eq!(mode, RuntimeMode::Host);
        assert!(!auto);
    }

    #[test]
    fn command_kind_maps_none_status_and_other() {
        assert_eq!(command_kind(None), CommandKind::None);

        let status = Commands::Status(commands::StatusArgs {});
        assert_eq!(command_kind(Some(&status)), CommandKind::Status);

        let start = Commands::Start(commands::StartArgs::default());
        assert_eq!(command_kind(Some(&start)), CommandKind::Other);
    }

    #[test]
    fn should_run_runtime_asset_preflight_gating() {
        assert!(should_run_runtime_asset_preflight(
            CommandKind::Other,
            None,
            false
        ));
        assert!(!should_run_runtime_asset_preflight(
            CommandKind::Status,
            None,
            false
        ));
        assert!(!should_run_runtime_asset_preflight(
            CommandKind::None,
            None,
            false
        ));
        assert!(!should_run_runtime_asset_preflight(
            CommandKind::Other,
            Some("prod-host"),
            false
        ));
        assert!(!should_run_runtime_asset_preflight(
            CommandKind::Other,
            None,
            true
        ));
    }

    #[test]
    fn render_runtime_asset_preflight_lines_include_rebuild_suggestions() {
        let report = RuntimeAssetDrift {
            drift_detected: true,
            mismatched_assets: vec!["bootstrap helper".to_string()],
            diagnostics: vec![],
        };
        let lines = render_runtime_asset_preflight_lines(&report, 0);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("--cached-rebuild-sandbox-image"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("--full-rebuild-sandbox-image"))
        );
    }

    #[test]
    fn render_runtime_asset_preflight_lines_appends_diagnostics_in_verbose() {
        let report = RuntimeAssetDrift {
            drift_detected: true,
            mismatched_assets: vec!["entrypoint".to_string()],
            diagnostics: vec!["entrypoint: exit status 1".to_string()],
        };
        let lines = render_runtime_asset_preflight_lines(&report, 1);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("diagnostic: entrypoint"))
        );
    }
}
