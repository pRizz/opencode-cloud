//! opencode-cloud CLI - Manage your opencode cloud service
//!
//! This module contains the shared CLI implementation used by all binaries.

mod commands;
mod constants;
mod output;
mod passwords;
pub mod wizard;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::{
    DockerClient, InstanceLock, SingletonError, config, get_version, load_config_or_default,
    load_hosts, save_config,
};

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
    /// Update to the latest version or rollback
    Update(commands::UpdateArgs),
    /// Open Cockpit web console
    #[command(hide = true)]
    Cockpit(commands::CockpitArgs),
    /// Manage remote hosts
    Host(commands::HostArgs),
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

pub fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Configure color output
    if cli.no_color {
        console::set_colors_enabled(false);
    }

    eprintln!(
        "{} This tool is still a work in progress and is rapidly evolving. Expect frequent updates and breaking changes. Follow updates at https://github.com/pRizz/opencode-cloud. Stability will be announced at some point. Use with caution.",
        style("Warning:").yellow().bold()
    );
    eprintln!();

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
