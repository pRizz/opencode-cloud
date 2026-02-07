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
//! - Container exec for running commands inside containers
//! - User management operations (create, delete, lock/unlock users)
//! - Image update and rollback operations

mod client;
pub mod container;
mod dockerfile;
mod error;
pub mod exec;
mod health;
pub mod image;
pub mod mount;
pub mod profile;
pub mod progress;
mod registry;
pub mod state;
pub mod update;
pub mod users;
mod version;
pub mod volume;

// Core types
pub use client::{DockerClient, DockerEndpoint};
pub use error::DockerError;
pub use progress::ProgressReporter;

// Health check operations
pub use health::{
    ExtendedHealthResponse, HealthError, HealthResponse, check_health, check_health_extended,
};

// Dockerfile constants
pub use dockerfile::{DOCKERFILE, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};

// Image operations
pub use image::{build_image, image_exists, pull_image, remove_images_by_name};
pub use profile::{
    DockerResourceNames, INSTANCE_LABEL_KEY, SANDBOX_INSTANCE_ENV, active_resource_names,
    env_instance_id, remap_container_name, remap_image_tag, resource_names_for_instance,
};

// Update operations
pub use update::{UpdateResult, has_previous_image, rollback_image, update_image};

// Version detection
pub use version::{
    VERSION_LABEL, get_cli_version, get_image_version, get_registry_latest_version,
    versions_compatible,
};

// Container exec operations
pub use exec::{
    exec_command, exec_command_exit_code, exec_command_with_status, exec_command_with_stdin,
};

// User management operations
pub use users::{
    UserInfo, create_user, delete_user, list_users, lock_user, persist_user, remove_persisted_user,
    restore_persisted_users, set_user_password, unlock_user, user_exists,
};

// Volume management
pub use volume::{
    MOUNT_CACHE, MOUNT_CONFIG, MOUNT_PROJECTS, MOUNT_SESSION, MOUNT_STATE, MOUNT_USERS,
    VOLUME_CACHE, VOLUME_CONFIG, VOLUME_NAMES, VOLUME_PROJECTS, VOLUME_SESSION, VOLUME_STATE,
    VOLUME_USERS, ensure_volumes_exist, remove_all_volumes, remove_volume, volume_exists,
};

/// Determine whether the Docker host supports systemd-in-container.
///
/// Returns true only for Linux hosts that are not Docker Desktop and not rootless.
pub async fn docker_supports_systemd(client: &DockerClient) -> Result<bool, DockerError> {
    let info = client.inner().info().await.map_err(DockerError::from)?;

    let os_type = info.os_type.unwrap_or_default();
    if os_type.to_lowercase() != "linux" {
        return Ok(false);
    }

    let operating_system = match info.operating_system {
        Some(value) => value,
        None => return Ok(false),
    };
    if operating_system.to_lowercase().contains("docker desktop") {
        return Ok(false);
    }

    let security_options = match info.security_options {
        Some(options) => options,
        None => return Ok(false),
    };
    let is_rootless = security_options
        .iter()
        .any(|opt| opt.to_lowercase().contains("name=rootless"));
    if is_rootless {
        return Ok(false);
    }

    Ok(true)
}

// Bind mount parsing and validation
pub use mount::{MountError, ParsedMount, check_container_path_warning, validate_mount_path};

// Container lifecycle
pub use container::{
    CONTAINER_NAME, ContainerBindMount, ContainerPorts, OPENCODE_WEB_PORT, container_exists,
    container_is_running, container_state, create_container, get_container_bind_mounts,
    get_container_ports, remove_container, start_container, stop_container,
};

// Image state tracking
pub use state::{ImageState, clear_state, get_state_path, load_state, save_state};

/// Full setup: ensure volumes exist, create container if needed, start it
///
/// This is the primary entry point for starting the opencode service.
/// Returns the container ID on success.
///
/// # Arguments
/// * `client` - Docker client
/// * `opencode_web_port` - Port to bind on host for opencode web UI (defaults to OPENCODE_WEB_PORT)
/// * `env_vars` - Additional environment variables (optional)
/// * `bind_address` - IP address to bind on host (defaults to "127.0.0.1")
/// * `cockpit_port` - Port to bind on host for Cockpit (defaults to 9090)
/// * `cockpit_enabled` - Whether to enable Cockpit port mapping (defaults to false)
/// * `systemd_enabled` - Whether to use systemd as init (defaults to false)
/// * `bind_mounts` - User-defined bind mounts from config and CLI flags (optional)
#[allow(clippy::too_many_arguments)]
pub async fn setup_and_start(
    client: &DockerClient,
    opencode_web_port: Option<u16>,
    env_vars: Option<Vec<String>>,
    bind_address: Option<&str>,
    cockpit_port: Option<u16>,
    cockpit_enabled: Option<bool>,
    systemd_enabled: Option<bool>,
    bind_mounts: Option<Vec<mount::ParsedMount>>,
) -> Result<String, DockerError> {
    let names = active_resource_names();

    // Ensure volumes exist first
    volume::ensure_volumes_exist(client).await?;

    // Check if container already exists
    let container_id = if container::container_exists(client, &names.container_name).await? {
        // Get existing container ID
        let info = client
            .inner()
            .inspect_container(&names.container_name, None)
            .await
            .map_err(|e| {
                DockerError::Container(format!("Failed to inspect existing container: {e}"))
            })?;
        info.id.unwrap_or_else(|| names.container_name.to_string())
    } else {
        // Create new container
        container::create_container(
            client,
            None,
            None,
            opencode_web_port,
            env_vars,
            bind_address,
            cockpit_port,
            cockpit_enabled,
            systemd_enabled,
            bind_mounts,
        )
        .await?
    };

    // Start if not running
    if !container::container_is_running(client, &names.container_name).await? {
        container::start_container(client, &names.container_name).await?;
    }

    // Restore persisted users after the container is running
    users::restore_persisted_users(client, &names.container_name).await?;

    Ok(container_id)
}

/// Default graceful shutdown timeout in seconds
pub const DEFAULT_STOP_TIMEOUT_SECS: i64 = 30;

/// Stop and optionally remove the opencode container
///
/// # Arguments
/// * `client` - Docker client
/// * `remove` - Also remove the container after stopping
/// * `timeout_secs` - Graceful shutdown timeout (default: 30 seconds)
pub async fn stop_service(
    client: &DockerClient,
    remove: bool,
    timeout_secs: Option<i64>,
) -> Result<(), DockerError> {
    let names = active_resource_names();
    let name = names.container_name.as_str();
    let timeout = timeout_secs.unwrap_or(DEFAULT_STOP_TIMEOUT_SECS);

    // Check if container exists
    if !container::container_exists(client, name).await? {
        return Err(DockerError::Container(format!(
            "Container '{name}' does not exist"
        )));
    }

    // Stop if running
    if container::container_is_running(client, name).await? {
        container::stop_container(client, name, Some(timeout)).await?;
    }

    // Remove if requested
    if remove {
        container::remove_container(client, name, false).await?;
    }

    Ok(())
}
