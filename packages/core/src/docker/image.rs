//! Docker image build and pull operations
//!
//! This module provides functionality to build Docker images from the embedded
//! Dockerfile and pull images from registries with progress feedback.

use super::progress::ProgressReporter;
use super::{DOCKERFILE, DockerClient, DockerError, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT};
use bollard::image::BuildImageOptions;
use bollard::models::BuildInfoAux;
use bytes::Bytes;
use flate2::Compression;
use flate2::write::GzEncoder;
use futures_util::StreamExt;
use tar::Builder as TarBuilder;
use tracing::debug;

/// Check if an image exists locally
pub async fn image_exists(
    client: &DockerClient,
    image: &str,
    tag: &str,
) -> Result<bool, DockerError> {
    let full_name = format!("{image}:{tag}");
    debug!("Checking if image exists: {}", full_name);

    match client.inner().inspect_image(&full_name).await {
        Ok(_) => Ok(true),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(false),
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Build the opencode image from embedded Dockerfile
///
/// Shows real-time build progress with streaming output.
/// Returns the full image:tag string on success.
pub async fn build_image(
    client: &DockerClient,
    tag: Option<&str>,
    progress: &mut ProgressReporter,
) -> Result<String, DockerError> {
    let tag = tag.unwrap_or(IMAGE_TAG_DEFAULT);
    let full_name = format!("{IMAGE_NAME_GHCR}:{tag}");
    debug!("Building image: {}", full_name);

    // Create tar archive containing Dockerfile
    let context = create_build_context()
        .map_err(|e| DockerError::Build(format!("Failed to create build context: {e}")))?;

    // Set up build options
    let options = BuildImageOptions {
        t: full_name.clone(),
        dockerfile: "Dockerfile".to_string(),
        rm: true,
        ..Default::default()
    };

    // Create build body from context
    let body = Bytes::from(context);

    // Start build with streaming output
    let mut stream = client.inner().build_image(options, None, Some(body));

    // Add main build spinner
    progress.add_spinner("build", "Building image...");

    let mut maybe_image_id = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                // Handle stream output (build log messages)
                if let Some(stream_msg) = info.stream {
                    let msg = stream_msg.trim();
                    if !msg.is_empty() {
                        progress.update_spinner("build", msg);

                        // Capture step information for better progress
                        if msg.starts_with("Step ") {
                            debug!("Build step: {}", msg);
                        }
                    }
                }

                // Handle error messages
                if let Some(error_msg) = info.error {
                    progress.abandon_all(&error_msg);
                    return Err(DockerError::Build(error_msg));
                }

                // Capture the image ID from aux field
                if let Some(aux) = info.aux {
                    match aux {
                        BuildInfoAux::Default(image_id) => {
                            if let Some(id) = image_id.id {
                                maybe_image_id = Some(id);
                            }
                        }
                        BuildInfoAux::BuildKit(_) => {
                            // BuildKit responses are handled via stream messages
                        }
                    }
                }
            }
            Err(e) => {
                progress.abandon_all("Build failed");
                return Err(DockerError::Build(format!("Build failed: {e}")));
            }
        }
    }

    let image_id = maybe_image_id.unwrap_or_else(|| "unknown".to_string());
    let finish_msg = format!("Build complete: {image_id}");
    progress.finish("build", &finish_msg);

    Ok(full_name)
}

/// Create a gzipped tar archive containing the Dockerfile
fn create_build_context() -> Result<Vec<u8>, std::io::Error> {
    let mut archive_buffer = Vec::new();

    {
        let encoder = GzEncoder::new(&mut archive_buffer, Compression::default());
        let mut tar = TarBuilder::new(encoder);

        // Add Dockerfile to archive
        let dockerfile_bytes = DOCKERFILE.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_path("Dockerfile")?;
        header.set_size(dockerfile_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();

        tar.append(&header, dockerfile_bytes)?;
        tar.finish()?;

        // Finish gzip encoding
        let encoder = tar.into_inner()?;
        encoder.finish()?;
    }

    Ok(archive_buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_build_context_succeeds() {
        let context = create_build_context().expect("should create context");
        assert!(!context.is_empty(), "context should not be empty");

        // Verify it's gzip-compressed (gzip magic bytes)
        assert_eq!(context[0], 0x1f, "should be gzip compressed");
        assert_eq!(context[1], 0x8b, "should be gzip compressed");
    }

    #[test]
    fn default_tag_is_latest() {
        assert_eq!(IMAGE_TAG_DEFAULT, "latest");
    }
}
