//! Output utilities for CLI commands
//!
//! This module provides terminal output helpers including spinners
//! with elapsed time display for long-running operations, and color
//! utilities for consistent state and log level styling.

pub mod colors;
pub mod spinner;

pub use colors::{log_level_style, state_style};
pub use spinner::CommandSpinner;
