//! Output utilities for CLI commands
//!
//! This module provides terminal output helpers including spinners
//! with elapsed time display for long-running operations.

pub mod spinner;

pub use spinner::CommandSpinner;
