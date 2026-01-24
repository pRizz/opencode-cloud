//! Docker container lifecycle management
//!
//! This module provides functions to create, start, stop, and remove
//! Docker containers for the opencode-cloud service.

use super::dockerfile::{IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};
use super::volume::{
    MOUNT_CONFIG, MOUNT_PROJECTS, MOUNT_SESSION, VOLUME_CONFIG, VOLUME_PROJECTS, VOLUME_SESSION,
};
use super::{DockerClient, DockerError};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::service::{HostConfig, Mount, MountTypeEnum, PortBinding, PortMap};
use std::collections::HashMap;
use tracing::debug;

/// Default container name
pub const CONTAINER_NAME: &str = "opencode-cloud";

/// Default port for opencode web UI
pub const OPENCODE_WEB_PORT: u16 = 3000;

/// Create the opencode container with volume mounts
///
/// Does not start the container - use start_container after creation.
/// Returns the container ID on success.
///
/// # Arguments
/// * `client` - Docker client
/// * `name` - Container name (defaults to CONTAINER_NAME)
/// * `image` - Image to use (defaults to IMAGE_NAME_GHCR:IMAGE_TAG_DEFAULT)
/// * `opencode_web_port` - Port to bind on host for opencode web UI (defaults to OPENCODE_WEB_PORT)
/// * `env_vars` - Additional environment variables (optional)
/// * `bind_address` - IP address to bind on host (defaults to "127.0.0.1")
/// * `cockpit_port` - Port to bind on host for Cockpit (defaults to 9090)
/// * `cockpit_enabled` - Whether to enable Cockpit port mapping (defaults to true)
#[allow(clippy::too_many_arguments)]
pub async fn create_container(
    client: &DockerClient,
    name: Option<&str>,
    image: Option<&str>,
    opencode_web_port: Option<u16>,
    env_vars: Option<Vec<String>>,
    bind_address: Option<&str>,
    cockpit_port: Option<u16>,
    cockpit_enabled: Option<bool>,
) -> Result<String, DockerError> {
    let container_name = name.unwrap_or(CONTAINER_NAME);
    let default_image = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");
    let image_name = image.unwrap_or(&default_image);
    let port = opencode_web_port.unwrap_or(OPENCODE_WEB_PORT);
    let cockpit_port_val = cockpit_port.unwrap_or(9090);
    let cockpit_enabled_val = cockpit_enabled.unwrap_or(true);

    debug!(
        "Creating container {} from image {} with port {} and cockpit_port {} (enabled: {})",
        container_name, image_name, port, cockpit_port_val, cockpit_enabled_val
    );

    // Check if container already exists
    if container_exists(client, container_name).await? {
        return Err(DockerError::Container(format!(
            "Container '{container_name}' already exists. Remove it first with 'occ stop --remove' or use a different name."
        )));
    }

    // Check if image exists
    let image_parts: Vec<&str> = image_name.split(':').collect();
    let (image_repo, image_tag) = if image_parts.len() == 2 {
        (image_parts[0], image_parts[1])
    } else {
        (image_name, "latest")
    };

    if !super::image::image_exists(client, image_repo, image_tag).await? {
        return Err(DockerError::Container(format!(
            "Image '{image_name}' not found. Run 'occ pull' first to download the image."
        )));
    }

    // Create volume mounts
    let mounts = vec![
        Mount {
            target: Some(MOUNT_SESSION.to_string()),
            source: Some(VOLUME_SESSION.to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
        Mount {
            target: Some(MOUNT_PROJECTS.to_string()),
            source: Some(VOLUME_PROJECTS.to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
        Mount {
            target: Some(MOUNT_CONFIG.to_string()),
            source: Some(VOLUME_CONFIG.to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        },
    ];

    // Create port bindings (default to localhost for security)
    let bind_addr = bind_address.unwrap_or("127.0.0.1");
    let mut port_bindings: PortMap = HashMap::new();

    // opencode web port
    port_bindings.insert(
        "3000/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some(bind_addr.to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    // Cockpit port (if enabled)
    // Container always listens on 9090, map to host's configured port
    if cockpit_enabled_val {
        port_bindings.insert(
            "9090/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some(bind_addr.to_string()),
                host_port: Some(cockpit_port_val.to_string()),
            }]),
        );
    }

    // Create exposed ports map
    let mut exposed_ports = HashMap::new();
    exposed_ports.insert("3000/tcp".to_string(), HashMap::new());
    if cockpit_enabled_val {
        exposed_ports.insert("9090/tcp".to_string(), HashMap::new());
    }

    // Create host config
    // When Cockpit is enabled, add systemd-specific settings (requires Linux host)
    // When Cockpit is disabled, use simpler tini-based config (works everywhere)
    let host_config = if cockpit_enabled_val {
        HostConfig {
            mounts: Some(mounts),
            port_bindings: Some(port_bindings),
            auto_remove: Some(false),
            // CAP_SYS_ADMIN required for systemd cgroup access
            cap_add: Some(vec!["SYS_ADMIN".to_string()]),
            // tmpfs for /run, /run/lock, and /tmp (required for systemd)
            tmpfs: Some(HashMap::from([
                ("/run".to_string(), "exec".to_string()),
                ("/run/lock".to_string(), String::new()),
                ("/tmp".to_string(), String::new()),
            ])),
            // cgroup mount (read-write for systemd)
            binds: Some(vec!["/sys/fs/cgroup:/sys/fs/cgroup:rw".to_string()]),
            // Use HOST cgroup namespace for systemd compatibility across Linux distros:
            // - cgroups v2 (Amazon Linux 2023, Fedora 31+, Ubuntu 21.10+, Debian 11+): required
            // - cgroups v1 (CentOS 7, Ubuntu 18.04, Debian 10): works fine
            // - Docker Desktop (macOS/Windows VM): works fine
            // Note: PRIVATE mode is more isolated but causes systemd to exit(255) on cgroups v2.
            // Since we already use privileged mode, HOST namespace is acceptable.
            cgroupns_mode: Some(bollard::models::HostConfigCgroupnsModeEnum::HOST),
            // Privileged mode required for systemd to manage cgroups and system services
            privileged: Some(true),
            ..Default::default()
        }
    } else {
        // Simple config for tini mode (works on macOS and Linux)
        HostConfig {
            mounts: Some(mounts),
            port_bindings: Some(port_bindings),
            auto_remove: Some(false),
            ..Default::default()
        }
    };

    // Build environment variables
    // Add USE_SYSTEMD=1 when Cockpit is enabled to tell entrypoint to use systemd
    let final_env = if cockpit_enabled_val {
        let mut env = env_vars.unwrap_or_default();
        env.push("USE_SYSTEMD=1".to_string());
        Some(env)
    } else {
        env_vars
    };

    // Create container config
    let config = Config {
        image: Some(image_name.to_string()),
        hostname: Some(CONTAINER_NAME.to_string()),
        working_dir: Some("/workspace".to_string()),
        exposed_ports: Some(exposed_ports),
        env: final_env,
        host_config: Some(host_config),
        ..Default::default()
    };

    // Create container
    let options = CreateContainerOptions {
        name: container_name,
        platform: None,
    };

    let response = client
        .inner()
        .create_container(Some(options), config)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("port is already allocated") || msg.contains("address already in use") {
                DockerError::Container(format!(
                    "Port {port} is already in use. Stop the service using that port or use a different port with --port."
                ))
            } else {
                DockerError::Container(format!("Failed to create container: {e}"))
            }
        })?;

    debug!("Container created with ID: {}", response.id);
    Ok(response.id)
}

/// Start an existing container
pub async fn start_container(client: &DockerClient, name: &str) -> Result<(), DockerError> {
    debug!("Starting container: {}", name);

    client
        .inner()
        .start_container(name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to start container {name}: {e}")))?;

    debug!("Container {} started", name);
    Ok(())
}

/// Stop a running container with graceful shutdown
///
/// # Arguments
/// * `client` - Docker client
/// * `name` - Container name
/// * `timeout_secs` - Seconds to wait before force kill (default: 10)
pub async fn stop_container(
    client: &DockerClient,
    name: &str,
    timeout_secs: Option<i64>,
) -> Result<(), DockerError> {
    let timeout = timeout_secs.unwrap_or(10);
    debug!("Stopping container {} with {}s timeout", name, timeout);

    let options = StopContainerOptions { t: timeout };

    client
        .inner()
        .stop_container(name, Some(options))
        .await
        .map_err(|e| {
            let msg = e.to_string();
            // "container already stopped" is not an error
            if msg.contains("is not running") || msg.contains("304") {
                debug!("Container {} was already stopped", name);
                return DockerError::Container(format!("Container '{name}' is not running"));
            }
            DockerError::Container(format!("Failed to stop container {name}: {e}"))
        })?;

    debug!("Container {} stopped", name);
    Ok(())
}

/// Remove a container
///
/// # Arguments
/// * `client` - Docker client
/// * `name` - Container name
/// * `force` - Remove even if running
pub async fn remove_container(
    client: &DockerClient,
    name: &str,
    force: bool,
) -> Result<(), DockerError> {
    debug!("Removing container {} (force={})", name, force);

    let options = RemoveContainerOptions {
        force,
        v: false, // Don't remove volumes
        link: false,
    };

    client
        .inner()
        .remove_container(name, Some(options))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to remove container {name}: {e}")))?;

    debug!("Container {} removed", name);
    Ok(())
}

/// Check if container exists
pub async fn container_exists(client: &DockerClient, name: &str) -> Result<bool, DockerError> {
    debug!("Checking if container exists: {}", name);

    match client.inner().inspect_container(name, None).await {
        Ok(_) => Ok(true),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(false),
        Err(e) => Err(DockerError::Container(format!(
            "Failed to inspect container {name}: {e}"
        ))),
    }
}

/// Check if container is running
pub async fn container_is_running(client: &DockerClient, name: &str) -> Result<bool, DockerError> {
    debug!("Checking if container is running: {}", name);

    match client.inner().inspect_container(name, None).await {
        Ok(info) => {
            let running = info.state.and_then(|s| s.running).unwrap_or(false);
            Ok(running)
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(false),
        Err(e) => Err(DockerError::Container(format!(
            "Failed to inspect container {name}: {e}"
        ))),
    }
}

/// Get container state (running, stopped, etc.)
pub async fn container_state(client: &DockerClient, name: &str) -> Result<String, DockerError> {
    debug!("Getting container state: {}", name);

    match client.inner().inspect_container(name, None).await {
        Ok(info) => {
            let state = info
                .state
                .and_then(|s| s.status)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Ok(state)
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Err(DockerError::Container(format!(
            "Container '{name}' not found"
        ))),
        Err(e) => Err(DockerError::Container(format!(
            "Failed to inspect container {name}: {e}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_constants_are_correct() {
        assert_eq!(CONTAINER_NAME, "opencode-cloud");
        assert_eq!(OPENCODE_WEB_PORT, 3000);
    }

    #[test]
    fn default_image_format() {
        let expected = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");
        assert_eq!(expected, "ghcr.io/prizz/opencode-cloud-sandbox:latest");
    }
}
