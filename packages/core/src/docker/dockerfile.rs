//! Embedded Dockerfile content
//!
//! This module contains the Dockerfile for building the opencode-cloud-sandbox
//! container image, embedded at compile time for distribution with the CLI.
//!
//! Note: The image is named "opencode-cloud-sandbox" (not "opencode-cloud") to
//! clearly indicate this is the sandboxed container environment that the
//! opencode-cloud CLI deploys, not the CLI tool itself.

use std::collections::HashMap;

/// The Dockerfile for building the opencode-cloud-sandbox container image
pub const DOCKERFILE: &str = include_str!("Dockerfile");

/// Build arg name for the opencode commit used in the Dockerfile.
pub const OPENCODE_COMMIT_BUILD_ARG: &str = "OPENCODE_COMMIT";

/// Default opencode commit pinned in the Dockerfile.
pub const OPENCODE_COMMIT_DEFAULT: &str = "dac099a4892689d11abedb0fcc1098b50e0958c8";

/// Build args for overriding the opencode commit in the Dockerfile.
pub fn build_args_for_opencode_commit(
    maybe_commit: Option<&str>,
) -> Option<HashMap<String, String>> {
    let commit = maybe_commit?;
    let mut args = HashMap::new();
    args.insert(OPENCODE_COMMIT_BUILD_ARG.to_string(), commit.to_string());
    Some(args)
}

// =============================================================================
// Docker Image Naming
// =============================================================================
//
// Docker images follow the naming convention: [registry/]namespace/image[:tag]
//
// - Registry: The server hosting the image (e.g., ghcr.io, gcr.io, docker.io)
//   When omitted, Docker Hub (docker.io) is assumed.
// - Namespace: Usually the username or organization (e.g., prizz)
// - Image: The image name (e.g., opencode-cloud-sandbox)
// - Tag: Version identifier (e.g., latest, v1.0.0). Defaults to "latest" if omitted.
//
// Examples:
//   ghcr.io/prizz/opencode-cloud-sandbox:latest  - GitHub Container Registry
//   prizz/opencode-cloud-sandbox:latest          - Docker Hub (registry omitted)
//   gcr.io/my-project/myapp:v1.0                 - Google Container Registry
//
// We publish to both GHCR (primary) and Docker Hub for maximum accessibility.
// =============================================================================

/// Docker image name for GitHub Container Registry (primary registry)
///
/// Format: `ghcr.io/{github-username}/{image-name}`
pub const IMAGE_NAME_GHCR: &str = "ghcr.io/prizz/opencode-cloud-sandbox";

/// Docker image name for Docker Hub (secondary registry)
///
/// Format: `{dockerhub-username}/{image-name}` (registry prefix omitted for Docker Hub)
pub const IMAGE_NAME_DOCKERHUB: &str = "prizz/opencode-cloud-sandbox";

/// Default image tag
pub const IMAGE_TAG_DEFAULT: &str = "latest";
