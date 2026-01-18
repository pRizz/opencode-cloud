//! opencode-cloud CLI - Manage your opencode cloud service
//!
//! This is the main entry point for the Rust CLI binary.

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use opencode_cloud_core::get_version;

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
    // Placeholder for future commands
    // Real commands will be added in later phases:
    // - Start
    // - Stop
    // - Status
    // - Config { subcommand }
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configure color output
    if cli.no_color {
        console::set_colors_enabled(false);
    }

    // If no subcommand provided, show help
    match cli.command {
        Some(_cmd) => {
            // Commands will be handled here in future phases
            unreachable!("No commands implemented yet")
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
        }
    }

    Ok(())
}
