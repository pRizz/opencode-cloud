//! opencode-cloud CLI - Manage your opencode cloud service
//!
//! This module contains the shared CLI implementation used by all binaries.

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use opencode_cloud_core::{Config, InstanceLock, SingletonError, config, get_version, load_config};

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
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key to set
        key: String,
        /// Value to set
        value: String,
    },
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

pub fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Configure color output
    if cli.no_color {
        console::set_colors_enabled(false);
    }

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
            eprintln!("  {}", e);
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

    match cli.command {
        Some(Commands::Start(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_start(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Stop(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_stop(&args, cli.quiet))
        }
        Some(Commands::Restart(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_restart(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Status(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_status(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Logs(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_logs(&args, cli.quiet))
        }
        Some(Commands::Install(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_install(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Uninstall(args)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::cmd_uninstall(&args, cli.quiet, cli.verbose))
        }
        Some(Commands::Config(cmd)) => handle_config(cmd, &config),
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
            eprintln!("  {}", msg);
            eprintln!();
            if let Some(data_dir) = config::paths::get_data_dir() {
                eprintln!("  {} Check permissions for:", style("Tip:").cyan());
                eprintln!("       {}", style(data_dir.display()).yellow());
            }
        }
        SingletonError::LockFailed(msg) => {
            eprintln!("{} Failed to acquire lock", style("Error:").red().bold());
            eprintln!();
            eprintln!("  {}", msg);
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

fn handle_config(cmd: ConfigCommands, config: &Config) -> Result<()> {
    match cmd {
        ConfigCommands::Show => {
            // Display current config as formatted JSON
            let json = serde_json::to_string_pretty(config)
                .map_err(|err| anyhow::anyhow!("Config serialization failed: {err}"))?;
            println!("{}", json);
            Ok(())
        }
        ConfigCommands::Set { key, value } => {
            // Placeholder - not yet implemented
            eprintln!(
                "{} config set is not yet implemented",
                style("Note:").yellow()
            );
            eprintln!();
            eprintln!(
                "  Attempted to set: {} = {}",
                style(&key).cyan(),
                style(&value).green()
            );
            eprintln!();
            eprintln!("  For now, edit the config file directly at:");
            if let Some(path) = config::paths::get_config_path() {
                eprintln!("  {}", style(path.display()).yellow());
            }
            Ok(())
        }
    }
}
