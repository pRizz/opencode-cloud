//! Host management subcommand implementations
//!
//! Provides `occ host` subcommands for managing remote Docker hosts.

mod add;
mod default;
mod edit;
mod list;
mod remove;
mod show;
mod test;

use anyhow::Result;
use clap::{Args, Subcommand};

pub use add::cmd_host_add;
pub use default::cmd_host_default;
pub use edit::cmd_host_edit;
pub use list::cmd_host_list;
pub use remove::cmd_host_remove;
pub use show::cmd_host_show;
pub use test::cmd_host_test;

/// Host management command arguments
#[derive(Args)]
pub struct HostArgs {
    #[command(subcommand)]
    pub command: HostCommands,
}

/// Host management subcommands
#[derive(Subcommand)]
pub enum HostCommands {
    /// Add a new remote host
    Add(add::HostAddArgs),
    /// Remove a remote host
    Remove(remove::HostRemoveArgs),
    /// List all configured hosts
    List(list::HostListArgs),
    /// Show details for a host
    Show(show::HostShowArgs),
    /// Edit host configuration
    Edit(edit::HostEditArgs),
    /// Test connection to a host
    Test(test::HostTestArgs),
    /// Set or show the default host
    Default(default::HostDefaultArgs),
}

/// Handle host command
///
/// Routes to the appropriate handler based on the subcommand.
pub async fn cmd_host(args: &HostArgs, quiet: bool, verbose: u8) -> Result<()> {
    match &args.command {
        HostCommands::Add(add_args) => cmd_host_add(add_args, quiet, verbose).await,
        HostCommands::Remove(remove_args) => cmd_host_remove(remove_args, quiet, verbose).await,
        HostCommands::List(list_args) => cmd_host_list(list_args, quiet, verbose).await,
        HostCommands::Show(show_args) => cmd_host_show(show_args, quiet, verbose).await,
        HostCommands::Edit(edit_args) => cmd_host_edit(edit_args, quiet, verbose).await,
        HostCommands::Test(test_args) => cmd_host_test(test_args, quiet, verbose).await,
        HostCommands::Default(default_args) => cmd_host_default(default_args, quiet, verbose).await,
    }
}
