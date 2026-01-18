//! opencode-cloud-core - Core library for opencode-cloud
//!
//! This library provides the shared functionality for both the Rust CLI
//! and Node.js bindings via NAPI-RS.

pub mod version;

// Re-export version functions for Rust consumers
pub use version::{get_version, get_version_long};

// NAPI bindings for Node.js consumers (only when napi feature is enabled)
#[cfg(feature = "napi")]
mod napi_bindings {
    use super::*;
    use napi_derive::napi;

    /// Get the version string for Node.js consumers
    #[napi]
    pub fn get_version_js() -> String {
        get_version()
    }

    /// Get the long version string with build info for Node.js consumers
    #[napi]
    pub fn get_version_long_js() -> String {
        get_version_long()
    }
}

#[cfg(feature = "napi")]
pub use napi_bindings::*;
