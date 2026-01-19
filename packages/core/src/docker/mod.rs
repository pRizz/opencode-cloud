//! Docker operations module
//!
//! This module provides Docker container management functionality including:
//! - Docker client wrapper with connection handling
//! - Docker-specific error types
//! - Embedded Dockerfile for building the opencode image
//! - Progress reporting for build and pull operations
//! - Image build and pull operations

mod client;
mod dockerfile;
mod error;
pub mod image;
pub mod progress;

pub use client::DockerClient;
pub use dockerfile::{DOCKERFILE, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};
pub use error::DockerError;
pub use image::{build_image, image_exists};
pub use progress::ProgressReporter;
