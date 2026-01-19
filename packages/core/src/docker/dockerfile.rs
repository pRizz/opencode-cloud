//! Embedded Dockerfile content
//!
//! This module contains the Dockerfile for building the opencode-cloud container image,
//! embedded at compile time for distribution with the CLI.

/// The Dockerfile for building the opencode-cloud container image
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
// - Image: The image name (e.g., opencode-cloud)
// - Tag: Version identifier (e.g., latest, v1.0.0). Defaults to "latest" if omitted.
//
// Examples:
//   ghcr.io/prizz/opencode-cloud:latest  - GitHub Container Registry
//   prizz/opencode-cloud:latest          - Docker Hub (registry omitted)
//   gcr.io/my-project/myapp:v1.0         - Google Container Registry
//
// We use GHCR as the primary registry since it integrates well with GitHub
// Actions for CI/CD publishing.
// =============================================================================

/// Docker image name for GitHub Container Registry (primary registry)
///
/// Format: `ghcr.io/{github-username}/{image-name}`
pub const IMAGE_NAME_GHCR: &str = "ghcr.io/prizz/opencode-cloud";

/// Docker image name for Docker Hub (fallback registry)
///
/// Format: `{dockerhub-username}/{image-name}` (registry prefix omitted for Docker Hub)
pub const IMAGE_NAME_DOCKERHUB: &str = "prizz/opencode-cloud";

/// Default image tag
pub const IMAGE_TAG_DEFAULT: &str = "latest";
