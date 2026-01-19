//! Docker operations module
//!
//! This module provides Docker container management functionality including:
//! - Docker client wrapper with connection handling
//! - Docker-specific error types
//! - Embedded Dockerfile for building the opencode image
//! - Progress reporting for build and pull operations
//! - Image build and pull operations
//! - Volume management for persistent storage
//! - Container lifecycle (create, start, stop, remove)

mod client;
pub mod container;
mod dockerfile;
mod error;
pub mod image;
pub mod progress;
pub mod volume;

// Core types
pub use client::DockerClient;
pub use error::DockerError;
pub use progress::ProgressReporter;

// Dockerfile constants
pub use dockerfile::{DOCKERFILE, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};

// Image operations
pub use image::{build_image, image_exists, pull_image};

// Volume management
pub use volume::{
    MOUNT_CONFIG, MOUNT_PROJECTS, MOUNT_SESSION, VOLUME_CONFIG, VOLUME_NAMES, VOLUME_PROJECTS,
    VOLUME_SESSION, ensure_volumes_exist, remove_all_volumes, remove_volume, volume_exists,
};

// Container lifecycle
pub use container::{
    CONTAINER_NAME, OPENCODE_WEB_PORT, container_exists, container_is_running, container_state,
    create_container, remove_container, start_container, stop_container,
};

/// Full setup: ensure volumes exist, create container if needed, start it
///
/// This is the primary entry point for starting the opencode service.
/// Returns the container ID on success.
///
/// # Arguments
/// * `client` - Docker client
/// * `opencode_web_port` - Port to bind on host for opencode web UI (defaults to OPENCODE_WEB_PORT)
/// * `env_vars` - Additional environment variables (optional)
pub async fn setup_and_start(
    client: &DockerClient,
    opencode_web_port: Option<u16>,
    env_vars: Option<Vec<String>>,
) -> Result<String, DockerError> {
    // Ensure volumes exist first
    volume::ensure_volumes_exist(client).await?;

    // Check if container already exists
    let container_id = if container::container_exists(client, container::CONTAINER_NAME).await? {
        // Get existing container ID
        let info = client
            .inner()
            .inspect_container(container::CONTAINER_NAME, None)
            .await
            .map_err(|e| {
                DockerError::Container(format!("Failed to inspect existing container: {}", e))
            })?;
        info.id
            .unwrap_or_else(|| container::CONTAINER_NAME.to_string())
    } else {
        // Create new container
        container::create_container(client, None, None, opencode_web_port, env_vars).await?
    };

    // Start if not running
    if !container::container_is_running(client, container::CONTAINER_NAME).await? {
        container::start_container(client, container::CONTAINER_NAME).await?;
    }

    Ok(container_id)
}

/// Stop and optionally remove the opencode container
///
/// # Arguments
/// * `client` - Docker client
/// * `remove` - Also remove the container after stopping
pub async fn stop_service(client: &DockerClient, remove: bool) -> Result<(), DockerError> {
    let name = container::CONTAINER_NAME;

    // Check if container exists
    if !container::container_exists(client, name).await? {
        return Err(DockerError::Container(format!(
            "Container '{}' does not exist",
            name
        )));
    }

    // Stop if running
    if container::container_is_running(client, name).await? {
        container::stop_container(client, name, None).await?;
    }

    // Remove if requested
    if remove {
        container::remove_container(client, name, false).await?;
    }

    Ok(())
}
