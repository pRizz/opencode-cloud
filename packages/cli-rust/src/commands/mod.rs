//! CLI command implementations
//!
//! This module contains the implementations for service lifecycle commands.

mod cleanup;
mod cockpit;
mod config;
pub(crate) mod container;
mod disk_usage;
mod host;
mod install;
mod logs;
mod mount;
mod reset;
mod restart;
pub(crate) mod runtime_shared;
mod service;
mod setup;
mod start;
mod status;
mod stop;
mod uninstall;
mod update;
mod update_signal;
mod user;

pub use cockpit::{CockpitArgs, cmd_cockpit};
pub use config::{ConfigArgs, cmd_config};
pub use host::{HostArgs, cmd_host};
pub use install::{InstallArgs, cmd_install};
pub use logs::{LogsArgs, cmd_logs};
pub use mount::{MountArgs, cmd_mount};
pub use reset::{ResetArgs, cmd_reset};
pub use restart::{RestartArgs, cmd_restart};
pub use setup::{SetupArgs, cmd_setup};
pub use start::{StartArgs, cmd_start};
pub use status::{StatusArgs, cmd_status};
pub use stop::{StopArgs, cmd_stop};
pub use uninstall::{UninstallArgs, cmd_uninstall};
pub use update::{UpdateArgs, UpdateCommand, UpdateOpencodeArgs, cmd_update};
pub use user::{UserArgs, cmd_user};
