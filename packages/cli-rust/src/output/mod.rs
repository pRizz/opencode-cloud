//! Output utilities for CLI commands
//!
//! This module provides terminal output helpers including spinners
//! with elapsed time display for long-running operations, color
//! utilities for consistent state and log level styling, centralized
//! error formatting for Docker errors, and URL formatting helpers
//! for consistent URL display.

pub mod colors;
pub mod errors;
pub mod spinner;
pub mod urls;

pub use colors::{log_level_style, state_style};
pub use errors::{format_docker_error, format_docker_error_anyhow, show_docker_error};
pub use spinner::CommandSpinner;
pub use urls::{
    format_cockpit_url, format_service_url, localhost_display_addr, normalize_bind_addr,
    resolve_remote_addr,
};
