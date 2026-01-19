//! CLI command implementations
//!
//! This module contains the implementations for service lifecycle commands.

mod install;
mod logs;
mod restart;
mod start;
mod status;
mod stop;
mod uninstall;

pub use install::{InstallArgs, cmd_install};
pub use logs::{LogsArgs, cmd_logs};
pub use restart::{RestartArgs, cmd_restart};
pub use start::{StartArgs, cmd_start};
pub use status::{StatusArgs, cmd_status};
pub use stop::{StopArgs, cmd_stop};
pub use uninstall::{UninstallArgs, cmd_uninstall};
