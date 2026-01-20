//! Network configuration prompts
//!
//! Handles port and hostname configuration.

use anyhow::Result;

/// Prompt for port number
///
/// Shows explanation and validates input.
pub fn prompt_port(_step: usize, _total: usize, _default_port: u16) -> Result<u16> {
    // Stub - will be implemented in Task 2
    todo!("prompt_port will be implemented in Task 2")
}

/// Prompt for hostname/bind address
///
/// Offers localhost vs 0.0.0.0 selection with explanations.
pub fn prompt_hostname(_step: usize, _total: usize, _default_bind: &str) -> Result<String> {
    // Stub - will be implemented in Task 2
    todo!("prompt_hostname will be implemented in Task 2")
}
