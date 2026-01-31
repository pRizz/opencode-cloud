//! opencode-cloud CLI - Manage your opencode cloud service
//!
//! This module contains the shared CLI implementation used by all binaries.

mod commands;
mod constants;
mod output;
mod passwords;
pub mod wizard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use opencode_cloud_core::{
    DockerClient, InstanceLock, SingletonError, config, get_version, load_config, load_hosts,
    save_config,
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
    #[arg(long, global = true)]
    host: Option<String>,
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
    /// Update to the latest version or rollback
    Update(commands::UpdateArgs),
    /// Open Cockpit web console
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

/// Resolve which Docker client to use based on --host flag and default_host config
///
/// Returns (DockerClient, Option<host_name>) where host_name is Some for remote connections.
///
/// Resolution order:
/// 1. --host flag (explicit)
/// 2. default_host from hosts.json
/// 3. Local Docker (no host_name)
pub async fn resolve_docker_client(
    maybe_host: Option<&str>,
) -> anyhow::Result<(DockerClient, Option<String>)> {
    let hosts = load_hosts().unwrap_or_default();

    // Determine target host
    let target_host = maybe_host
        .map(String::from)
        .or_else(|| hosts.default_host.clone());

    match target_host {
        Some(name) if name != "local" && !name.is_empty() => {
            // Remote host requested
            let host_config = hosts.get_host(&name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Host '{name}' not found. Run 'occ host list' to see available hosts."
                )
            })?;

            let client = DockerClient::connect_remote(host_config, &name).await?;
            Ok((client, Some(name)))
        }
        _ => {
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

    // Load config (creates default if missing)
    let config_path = config::paths::get_config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

    let config = match load_config() {
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

    // Store host flag for command handlers
    let target_host = cli.host.clone();

    // Check if wizard needed (missing auth and not running setup/config/user command)
    let needs_wizard = !config.has_required_auth()
        && !matches!(
            cli.command,
            Some(Commands::Setup(_)) | Some(Commands::Config(_)) | Some(Commands::User(_))
        );

    if needs_wizard {
        eprintln!(
            "{} First-time setup required. Running wizard...",
            style("Note:").cyan()
        );
        eprintln!();
        let rt = tokio::runtime::Runtime::new()?;
        let new_config = rt.block_on(wizard::run_wizard(Some(&config)))?;
        save_config(&new_config)?;
        eprintln!();
        eprintln!(
            "{} Setup complete! Run your command again, or use 'occ start' to begin.",
            style("Success:").green().bold()
        );
        return Ok(());
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
            rt.block_on(commands::cmd_mount(&args, cli.quiet, cli.verbose))
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
            // No command - show a welcome message and hint to use --help
            if !cli.quiet {
                println!(
                    "{} {}",
                    style("opencode-cloud").cyan().bold(),
                    style(get_version()).dim()
                );
                println!();
                println!("Run {} for available commands.", style("--help").green());
            }
            Ok(())
        }
    }
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
