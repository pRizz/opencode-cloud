//! Docker volume management
//!
//! This module provides functions to create and manage Docker volumes
//! for persistent storage across container restarts.

use super::{DockerClient, DockerError};
use bollard::models::VolumeCreateRequest;
use bollard::query_parameters::RemoveVolumeOptions;
use std::collections::HashMap;
use tracing::debug;

/// Volume name for opencode data
pub const VOLUME_SESSION: &str = "opencode-data";

/// Volume name for opencode state
pub const VOLUME_STATE: &str = "opencode-state";

/// Volume name for opencode cache
pub const VOLUME_CACHE: &str = "opencode-cache";

/// Volume name for project files
pub const VOLUME_PROJECTS: &str = "opencode-workspace";

/// Volume name for opencode configuration
pub const VOLUME_CONFIG: &str = "opencode-config";

/// Volume name for persisted user records
pub const VOLUME_USERS: &str = "opencode-users";

/// All volume names as array for iteration
pub const VOLUME_NAMES: [&str; 6] = [
    VOLUME_SESSION,
    VOLUME_STATE,
    VOLUME_CACHE,
    VOLUME_PROJECTS,
    VOLUME_CONFIG,
    VOLUME_USERS,
];

/// Mount point for opencode data inside container
pub const MOUNT_SESSION: &str = "/home/opencode/.local/share/opencode";

/// Mount point for opencode state inside container
pub const MOUNT_STATE: &str = "/home/opencode/.local/state/opencode";

/// Mount point for opencode cache inside container
pub const MOUNT_CACHE: &str = "/home/opencode/.cache/opencode";

/// Mount point for project files inside container
pub const MOUNT_PROJECTS: &str = "/home/opencode/workspace";

/// Mount point for configuration inside container
pub const MOUNT_CONFIG: &str = "/home/opencode/.config/opencode";

/// Mount point for persisted user records inside container
pub const MOUNT_USERS: &str = "/var/lib/opencode-users";

/// Ensure all required volumes exist
///
/// Creates volumes if they don't exist. This operation is idempotent -
/// calling it multiple times has no additional effect.
pub async fn ensure_volumes_exist(client: &DockerClient) -> Result<(), DockerError> {
    debug!("Ensuring all required volumes exist");

    for volume_name in VOLUME_NAMES {
        ensure_volume_exists(client, volume_name).await?;
    }

    debug!("All volumes verified/created");
    Ok(())
}

/// Ensure a specific volume exists
async fn ensure_volume_exists(client: &DockerClient, name: &str) -> Result<(), DockerError> {
    debug!("Checking volume: {}", name);

    // Create volume request with default local driver (bollard v0.20+ uses VolumeCreateRequest)
    let options = VolumeCreateRequest {
        name: Some(name.to_string()),
        driver: Some("local".to_string()),
        driver_opts: Some(HashMap::new()),
        labels: Some(HashMap::from([(
            "managed-by".to_string(),
            "opencode-cloud".to_string(),
        )])),
        cluster_volume_spec: None,
    };

    // create_volume is idempotent - returns existing volume if it exists
    client
        .inner()
        .create_volume(options)
        .await
        .map_err(|e| DockerError::Volume(format!("Failed to create volume {name}: {e}")))?;

    debug!("Volume {} ready", name);
    Ok(())
}

/// Check if a specific volume exists
pub async fn volume_exists(client: &DockerClient, name: &str) -> Result<bool, DockerError> {
    debug!("Checking if volume exists: {}", name);

    match client.inner().inspect_volume(name).await {
        Ok(_) => Ok(true),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(false),
        Err(e) => Err(DockerError::Volume(format!(
            "Failed to inspect volume {name}: {e}"
        ))),
    }
}

/// Remove a volume
///
/// Returns error if volume is in use by a container.
/// Use force_remove_volume for cleanup during uninstall.
pub async fn remove_volume(client: &DockerClient, name: &str) -> Result<(), DockerError> {
    debug!("Removing volume: {}", name);

    client
        .inner()
        .remove_volume(name, None::<RemoveVolumeOptions>)
        .await
        .map_err(|e| DockerError::Volume(format!("Failed to remove volume {name}: {e}")))?;

    debug!("Volume {} removed", name);
    Ok(())
}

/// Remove all opencode-cloud volumes
///
/// Used during uninstall. Fails if any volume is in use.
pub async fn remove_all_volumes(client: &DockerClient) -> Result<(), DockerError> {
    debug!("Removing all opencode-cloud volumes");

    for volume_name in VOLUME_NAMES {
        // Check if volume exists before trying to remove
        if volume_exists(client, volume_name).await? {
            remove_volume(client, volume_name).await?;
        }
    }

    debug!("All volumes removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_constants_are_correct() {
        assert_eq!(VOLUME_SESSION, "opencode-data");
        assert_eq!(VOLUME_STATE, "opencode-state");
        assert_eq!(VOLUME_CACHE, "opencode-cache");
        assert_eq!(VOLUME_PROJECTS, "opencode-workspace");
        assert_eq!(VOLUME_CONFIG, "opencode-config");
        assert_eq!(VOLUME_USERS, "opencode-users");
    }

    #[test]
    fn volume_names_array_has_all_volumes() {
        assert_eq!(VOLUME_NAMES.len(), 6);
        assert!(VOLUME_NAMES.contains(&VOLUME_SESSION));
        assert!(VOLUME_NAMES.contains(&VOLUME_STATE));
        assert!(VOLUME_NAMES.contains(&VOLUME_CACHE));
        assert!(VOLUME_NAMES.contains(&VOLUME_PROJECTS));
        assert!(VOLUME_NAMES.contains(&VOLUME_CONFIG));
        assert!(VOLUME_NAMES.contains(&VOLUME_USERS));
    }

    #[test]
    fn mount_points_are_correct() {
        assert_eq!(MOUNT_SESSION, "/home/opencode/.local/share/opencode");
        assert_eq!(MOUNT_STATE, "/home/opencode/.local/state/opencode");
        assert_eq!(MOUNT_CACHE, "/home/opencode/.cache/opencode");
        assert_eq!(MOUNT_PROJECTS, "/home/opencode/workspace");
        assert_eq!(MOUNT_CONFIG, "/home/opencode/.config/opencode");
        assert_eq!(MOUNT_USERS, "/var/lib/opencode-users");
    }
}
