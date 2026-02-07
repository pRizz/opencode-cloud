//! Docker image update and rollback operations
//!
//! This module provides functionality to update the opencode image to the latest
//! version and rollback to a previous version if needed.

use super::image::{image_exists, pull_image};
use super::profile::active_resource_names;
use super::progress::ProgressReporter;
use super::{DockerClient, DockerError, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};
use bollard::query_parameters::TagImageOptions;
use tracing::debug;

/// Tag for the previous image version (used for rollback)
pub const PREVIOUS_TAG: &str = "previous";

/// Result of an update operation
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateResult {
    /// Update completed successfully
    Success,
    /// Already on the latest version
    AlreadyLatest,
}

/// Tag the current image as "previous" for rollback support
///
/// This allows users to rollback to the version they had before updating.
/// If the current image doesn't exist, this is silently skipped.
///
/// # Arguments
/// * `client` - Docker client
pub async fn tag_current_as_previous(client: &DockerClient) -> Result<(), DockerError> {
    let names = active_resource_names();
    let current_image = format!("{IMAGE_NAME_GHCR}:{}", names.image_tag);
    let previous_image = format!("{IMAGE_NAME_GHCR}:{}", names.previous_image_tag);

    debug!(
        "Tagging current image {} as {}",
        current_image, previous_image
    );

    // Check if current image exists
    if !image_exists(client, IMAGE_NAME_GHCR, &names.image_tag).await? {
        debug!("Current image not found, skipping backup tag");
        return Ok(());
    }

    // Tag current as previous
    let options = TagImageOptions {
        repo: Some(IMAGE_NAME_GHCR.to_string()),
        tag: Some(names.previous_image_tag),
    };

    client
        .inner()
        .tag_image(&current_image, Some(options))
        .await
        .map_err(|e| {
            DockerError::Container(format!("Failed to tag current image as previous: {e}"))
        })?;

    debug!("Successfully tagged current image as previous");
    Ok(())
}

/// Check if a previous image exists for rollback
///
/// Returns true if a rollback is possible, false otherwise.
///
/// # Arguments
/// * `client` - Docker client
pub async fn has_previous_image(client: &DockerClient) -> Result<bool, DockerError> {
    let names = active_resource_names();
    image_exists(client, IMAGE_NAME_GHCR, &names.previous_image_tag).await
}

/// Update the opencode image to the latest version
///
/// This operation:
/// 1. Tags the current image as "previous" for rollback
/// 2. Pulls the latest image from the registry
///
/// Returns UpdateResult indicating success or if already on latest.
///
/// # Arguments
/// * `client` - Docker client
/// * `progress` - Progress reporter for user feedback
pub async fn update_image(
    client: &DockerClient,
    progress: &mut ProgressReporter,
) -> Result<UpdateResult, DockerError> {
    // Step 1: Tag current image as previous for rollback
    progress.add_spinner("backup", "Backing up current image");
    tag_current_as_previous(client).await?;
    progress.finish("backup", "Current image backed up");

    // Step 2: Pull latest image
    progress.add_spinner("pull", "Pulling latest image");
    pull_image(client, Some(IMAGE_TAG_DEFAULT), progress).await?;
    progress.finish("pull", "Latest image pulled");

    Ok(UpdateResult::Success)
}

/// Rollback to the previous image version
///
/// This re-tags the "previous" image as "latest", effectively reverting
/// to the version that was active before the last update.
///
/// Returns an error if no previous image exists.
///
/// # Arguments
/// * `client` - Docker client
pub async fn rollback_image(client: &DockerClient) -> Result<(), DockerError> {
    // Check if previous image exists
    if !has_previous_image(client).await? {
        return Err(DockerError::Container(
            "No previous image available for rollback. Update at least once before using rollback."
                .to_string(),
        ));
    }

    let names = active_resource_names();
    let previous_image = format!("{IMAGE_NAME_GHCR}:{}", names.previous_image_tag);
    let current_image = format!("{IMAGE_NAME_GHCR}:{}", names.image_tag);

    debug!("Rolling back from {} to {}", current_image, previous_image);

    // Re-tag previous as latest
    let options = TagImageOptions {
        repo: Some(IMAGE_NAME_GHCR.to_string()),
        tag: Some(names.image_tag),
    };

    client
        .inner()
        .tag_image(&previous_image, Some(options))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to rollback image: {e}")))?;

    debug!("Successfully rolled back to previous image");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_tag_constant() {
        assert_eq!(PREVIOUS_TAG, "previous");
    }

    #[test]
    fn update_result_variants() {
        assert_eq!(UpdateResult::Success, UpdateResult::Success);
        assert_eq!(UpdateResult::AlreadyLatest, UpdateResult::AlreadyLatest);
        assert_ne!(UpdateResult::Success, UpdateResult::AlreadyLatest);
    }
}
