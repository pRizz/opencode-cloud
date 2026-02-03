//! Docker image version detection
//!
//! Reads version information from Docker image labels.

use super::registry::fetch_registry_version;
use super::{DockerClient, DockerError, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};

/// Version label key in Docker image
pub const VERSION_LABEL: &str = "org.opencode-cloud.version";

/// Get version from image label
///
/// Returns None if image doesn't exist or has no version label.
/// Version label is set during automated builds; local builds have "dev".
pub async fn get_image_version(
    client: &DockerClient,
    image_name: &str,
) -> Result<Option<String>, DockerError> {
    let inspect = match client.inner().inspect_image(image_name).await {
        Ok(info) => info,
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            return Ok(None);
        }
        Err(e) => {
            return Err(DockerError::Connection(format!(
                "Failed to inspect image: {e}"
            )));
        }
    };

    // Extract version from labels
    let version = inspect
        .config
        .and_then(|c| c.labels)
        .and_then(|labels| labels.get(VERSION_LABEL).cloned());

    Ok(version)
}

pub async fn get_registry_latest_version(
    client: &DockerClient,
) -> Result<Option<String>, DockerError> {
    match fetch_ghcr_registry_version(client).await {
        Ok(version) => Ok(version),
        Err(ghcr_err) => fetch_dockerhub_registry_version(client).await.map_err(|dockerhub_err| {
            DockerError::Connection(format!(
                "Failed to fetch registry version. GHCR: {ghcr_err}. Docker Hub: {dockerhub_err}"
            ))
        }),
    }
}

async fn fetch_ghcr_registry_version(client: &DockerClient) -> Result<Option<String>, DockerError> {
    let repo = IMAGE_NAME_GHCR
        .strip_prefix("ghcr.io/")
        .unwrap_or(IMAGE_NAME_GHCR);
    let reference = format!("{IMAGE_NAME_GHCR}:{IMAGE_TAG_DEFAULT}");
    let digest = fetch_registry_digest(client, &reference).await;
    fetch_registry_version(
        "https://ghcr.io",
        &format!("https://ghcr.io/token?scope=repository:{repo}:pull"),
        repo,
        IMAGE_TAG_DEFAULT,
        digest.as_deref(),
        VERSION_LABEL,
    )
    .await
}

async fn fetch_dockerhub_registry_version(
    client: &DockerClient,
) -> Result<Option<String>, DockerError> {
    let repo = IMAGE_NAME_DOCKERHUB;
    let reference = format!("{IMAGE_NAME_DOCKERHUB}:{IMAGE_TAG_DEFAULT}");
    let digest = fetch_registry_digest(client, &reference).await;
    fetch_registry_version(
        "https://registry-1.docker.io",
        &format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{repo}:pull"
        ),
        repo,
        IMAGE_TAG_DEFAULT,
        digest.as_deref(),
        VERSION_LABEL,
    )
    .await
}

async fn fetch_registry_digest(client: &DockerClient, reference: &str) -> Option<String> {
    client
        .inner()
        .inspect_registry_image(reference, None)
        .await
        .ok()
        .and_then(|info| info.descriptor.digest)
}

/// CLI version from Cargo.toml
pub fn get_cli_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Compare versions and determine if they match
///
/// Returns true if versions are compatible (same or dev build).
/// Returns false if versions differ and user should be prompted.
pub fn versions_compatible(cli_version: &str, image_version: Option<&str>) -> bool {
    match image_version {
        None => true,        // No version label = local build, assume compatible
        Some("dev") => true, // Dev build, assume compatible
        Some(img_ver) => cli_version == img_ver,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versions_compatible_none() {
        assert!(versions_compatible("1.0.8", None));
    }

    #[test]
    fn test_versions_compatible_dev() {
        assert!(versions_compatible("1.0.8", Some("dev")));
    }

    #[test]
    fn test_versions_compatible_same() {
        assert!(versions_compatible("1.0.8", Some("1.0.8")));
    }

    #[test]
    fn test_versions_compatible_different() {
        assert!(!versions_compatible("1.0.8", Some("1.0.7")));
    }

    #[test]
    fn test_get_cli_version_format() {
        let version = get_cli_version();
        // Should be semver format
        assert!(version.contains('.'));
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_version_label_constant() {
        assert_eq!(VERSION_LABEL, "org.opencode-cloud.version");
    }
}
