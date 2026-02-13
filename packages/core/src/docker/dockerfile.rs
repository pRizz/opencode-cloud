//! Embedded Dockerfile content
//!
//! This module contains the Dockerfile for building the opencode-cloud-sandbox
//! container image, embedded at compile time for distribution with the CLI.
//!
//! Note: The image is named "opencode-cloud-sandbox" (not "opencode-cloud") to
//! clearly indicate this is the sandboxed container environment that the
//! opencode-cloud CLI deploys, not the CLI tool itself.

/// The Dockerfile for building the opencode-cloud-sandbox container image
pub const DOCKERFILE: &str = include_str!("Dockerfile");

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
//   prizz/opencode-cloud-sandbox:latest          - Docker Hub (registry omitted)
//   ghcr.io/prizz/opencode-cloud-sandbox:latest  - GitHub Container Registry
//   gcr.io/my-project/myapp:v1.0                 - Google Container Registry
//
// We publish to both Docker Hub (primary runtime tag) and GHCR (fallback pull source)
// for maximum accessibility.
// =============================================================================

/// Docker image name for Docker Hub (primary runtime image label).
///
/// Format: `{dockerhub-username}/{image-name}` (registry prefix omitted for Docker Hub)
pub const IMAGE_NAME_DOCKERHUB: &str = "prizz/opencode-cloud-sandbox";

/// Docker image name for GitHub Container Registry (fallback pull source).
///
/// Format: `ghcr.io/{github-username}/{image-name}`
pub const IMAGE_NAME_GHCR: &str = "ghcr.io/prizz/opencode-cloud-sandbox";

/// Canonical local image repository used for runtime operations.
pub const IMAGE_NAME_PRIMARY: &str = IMAGE_NAME_DOCKERHUB;

/// Default image tag
pub const IMAGE_TAG_DEFAULT: &str = "latest";
