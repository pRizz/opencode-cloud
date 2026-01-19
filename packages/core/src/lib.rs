//! opencode-cloud-core - Core library for opencode-cloud
//!
//! This library provides the shared functionality for both the Rust CLI
//! and Node.js bindings via NAPI-RS.

pub mod config;
pub mod docker;
pub mod platform;
pub mod singleton;
pub mod version;

// Re-export version functions for Rust consumers
pub use version::{get_version, get_version_long};

// Re-export config types and functions
pub use config::{Config, load_config, save_config};

// Re-export singleton types
pub use singleton::{InstanceLock, SingletonError};

// Re-export docker types
pub use docker::{CONTAINER_NAME, DockerClient, DockerError, OPENCODE_WEB_PORT};

// Re-export platform types
pub use platform::{
    InstallResult, ServiceConfig, ServiceManager, get_service_manager,
    is_service_registration_supported,
};

// NAPI bindings for Node.js consumers (only when napi feature is enabled)
#[cfg(feature = "napi")]
use napi_derive::napi;

/// Get the version string for Node.js consumers
#[cfg(feature = "napi")]
#[napi]
pub fn get_version_js() -> String {
    get_version()
}

/// Get the long version string with build info for Node.js consumers
#[cfg(feature = "napi")]
#[napi]
pub fn get_version_long_js() -> String {
    get_version_long()
}
