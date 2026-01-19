//! Docker operations module
//!
//! This module provides Docker container management functionality including:
//! - Docker client wrapper with connection handling
//! - Docker-specific error types
//! - Embedded Dockerfile for building the opencode image
//! - Progress reporting for build and pull operations
//! - Image build and pull operations
//! - Volume management for persistent storage

mod client;
mod dockerfile;
mod error;
pub mod image;
pub mod progress;
pub mod volume;

pub use client::DockerClient;
pub use dockerfile::{DOCKERFILE, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};
pub use error::DockerError;
pub use image::{build_image, image_exists, pull_image};
pub use progress::ProgressReporter;
pub use volume::{
    MOUNT_CONFIG, MOUNT_PROJECTS, MOUNT_SESSION, VOLUME_CONFIG, VOLUME_NAMES, VOLUME_PROJECTS,
    VOLUME_SESSION, ensure_volumes_exist, remove_all_volumes, remove_volume, volume_exists,
};
