//! Docker volume management
//!
//! This module provides functions to create and manage Docker volumes
//! for persistent storage across container restarts.

use super::{DockerClient, DockerError};
use bollard::volume::CreateVolumeOptions;
use std::collections::HashMap;
use tracing::debug;

/// Volume name for opencode data
pub const VOLUME_SESSION: &str = "opencode-data";

/// Volume name for project files
pub const VOLUME_PROJECTS: &str = "opencode-workspace";

/// Volume name for opencode configuration
pub const VOLUME_CONFIG: &str = "opencode-config";

/// All volume names as array for iteration
pub const VOLUME_NAMES: [&str; 3] = [VOLUME_SESSION, VOLUME_PROJECTS, VOLUME_CONFIG];

/// Mount point for opencode data inside container
pub const MOUNT_SESSION: &str = "/home/opencode/.local/share";

/// Mount point for opencode app data inside container (XDG_DATA_HOME)
pub const MOUNT_APP_DATA: &str = "/home/opencode/.local/share/opencode-cloud";

/// Mount point for project files inside container
pub const MOUNT_PROJECTS: &str = "/home/opencode/workspace";

/// Mount point for configuration inside container
pub const MOUNT_CONFIG: &str = "/home/opencode/.config";

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

    // Create volume options with default local driver
    let options = CreateVolumeOptions {
        name,
        driver: "local",
        driver_opts: HashMap::new(),
        labels: HashMap::from([("managed-by", "opencode-cloud")]),
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
        .remove_volume(name, None)
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
        assert_eq!(VOLUME_PROJECTS, "opencode-workspace");
        assert_eq!(VOLUME_CONFIG, "opencode-config");
    }

    #[test]
    fn volume_names_array_has_all_volumes() {
        assert_eq!(VOLUME_NAMES.len(), 3);
        assert!(VOLUME_NAMES.contains(&VOLUME_SESSION));
        assert!(VOLUME_NAMES.contains(&VOLUME_PROJECTS));
        assert!(VOLUME_NAMES.contains(&VOLUME_CONFIG));
    }

    #[test]
    fn mount_points_are_correct() {
        assert_eq!(MOUNT_SESSION, "/home/opencode/.local/share");
        assert_eq!(MOUNT_APP_DATA, "/home/opencode/.local/share/opencode-cloud");
        assert_eq!(MOUNT_PROJECTS, "/home/opencode/workspace");
        assert_eq!(MOUNT_CONFIG, "/home/opencode/.config");
    }
}
