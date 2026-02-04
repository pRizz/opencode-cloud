//! Docker image build and pull operations
//!
//! This module provides functionality to build Docker images from the embedded
//! Dockerfile and pull images from registries with progress feedback.

use super::progress::ProgressReporter;
use super::{
    DOCKERFILE, DockerClient, DockerError, IMAGE_NAME_DOCKERHUB, IMAGE_NAME_GHCR, IMAGE_TAG_DEFAULT,
};
use bollard::moby::buildkit::v1::StatusResponse as BuildkitStatusResponse;
use bollard::models::BuildInfoAux;
use bollard::query_parameters::{
    BuildImageOptions, BuilderVersion, CreateImageOptions, ListImagesOptionsBuilder,
    RemoveImageOptionsBuilder,
};
use bytes::Bytes;
use flate2::Compression;
use flate2::write::GzEncoder;
use futures_util::StreamExt;
use http_body_util::{Either, Full};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Builder as TarBuilder;
use tracing::{debug, warn};

/// Default number of recent build log lines to capture for error context
const DEFAULT_BUILD_LOG_BUFFER_SIZE: usize = 20;

/// Default number of error lines to capture separately
const DEFAULT_ERROR_LOG_BUFFER_SIZE: usize = 10;

/// Read a log buffer size from env with bounds
fn read_log_buffer_size(var_name: &str, default: usize) -> usize {
    let Ok(value) = env::var(var_name) else {
        return default;
    };
    let Ok(parsed) = value.trim().parse::<usize>() else {
        return default;
    };
    parsed.clamp(5, 500)
}

/// Check if a line looks like an error message
fn is_error_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("error")
        || lower.contains("failed")
        || lower.contains("cannot")
        || lower.contains("unable to")
        || lower.contains("not found")
        || lower.contains("permission denied")
}

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

/// Remove all images whose tags, digests, or labels match the provided name fragment
///
/// Returns the number of images removed.
pub async fn remove_images_by_name(
    client: &DockerClient,
    name_fragment: &str,
    force: bool,
) -> Result<usize, DockerError> {
    debug!("Removing Docker images matching '{name_fragment}'");

    let images = list_docker_images(client).await?;

    let image_ids = collect_image_ids(&images, name_fragment);
    remove_image_ids(client, image_ids, force).await
}

/// List all local Docker images (including intermediate layers).
async fn list_docker_images(
    client: &DockerClient,
) -> Result<Vec<bollard::models::ImageSummary>, DockerError> {
    let list_options = ListImagesOptionsBuilder::new().all(true).build();
    client
        .inner()
        .list_images(Some(list_options))
        .await
        .map_err(|e| DockerError::Image(format!("Failed to list images: {e}")))
}

const LABEL_TITLE: &str = "org.opencontainers.image.title";
const LABEL_SOURCE: &str = "org.opencontainers.image.source";
const LABEL_URL: &str = "org.opencontainers.image.url";

const LABEL_TITLE_VALUE: &str = "opencode-cloud";
const LABEL_SOURCE_VALUE: &str = "https://github.com/pRizz/opencode-cloud";
const LABEL_URL_VALUE: &str = "https://github.com/pRizz/opencode-cloud";

/// Collect image IDs that contain the provided name fragment or match opencode labels.
fn collect_image_ids(
    images: &[bollard::models::ImageSummary],
    name_fragment: &str,
) -> HashSet<String> {
    let mut image_ids = HashSet::new();
    for image in images {
        if image_matches_fragment_or_labels(image, name_fragment) {
            image_ids.insert(image.id.clone());
        }
    }
    image_ids
}

fn image_matches_fragment_or_labels(
    image: &bollard::models::ImageSummary,
    name_fragment: &str,
) -> bool {
    let tag_match = image
        .repo_tags
        .iter()
        .any(|tag| tag != "<none>:<none>" && tag.contains(name_fragment));
    let digest_match = image
        .repo_digests
        .iter()
        .any(|digest| digest.contains(name_fragment));
    let label_match = image_labels_match(&image.labels);

    tag_match || digest_match || label_match
}

fn image_labels_match(labels: &HashMap<String, String>) -> bool {
    labels
        .get(LABEL_SOURCE)
        .is_some_and(|value| value == LABEL_SOURCE_VALUE)
        || labels
            .get(LABEL_URL)
            .is_some_and(|value| value == LABEL_URL_VALUE)
        || labels
            .get(LABEL_TITLE)
            .is_some_and(|value| value == LABEL_TITLE_VALUE)
}

/// Remove image IDs, returning the number removed.
async fn remove_image_ids(
    client: &DockerClient,
    image_ids: HashSet<String>,
    force: bool,
) -> Result<usize, DockerError> {
    if image_ids.is_empty() {
        return Ok(0);
    }

    let remove_options = RemoveImageOptionsBuilder::new().force(force).build();
    let mut removed = 0usize;
    for image_id in image_ids {
        let result = client
            .inner()
            .remove_image(&image_id, Some(remove_options.clone()), None)
            .await;
        match result {
            Ok(_) => removed += 1,
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                debug!("Docker image already removed: {}", image_id);
            }
            Err(err) => {
                return Err(DockerError::Image(format!(
                    "Failed to remove image {image_id}: {err}"
                )));
            }
        }
    }

    Ok(removed)
}

/// Build the opencode image from embedded Dockerfile
///
/// Shows real-time build progress with streaming output.
/// Returns the full image:tag string on success.
///
/// # Arguments
/// * `client` - Docker client
/// * `tag` - Image tag (defaults to IMAGE_TAG_DEFAULT)
/// * `progress` - Progress reporter for build feedback
/// * `no_cache` - If true, build without using Docker layer cache
pub async fn build_image(
    client: &DockerClient,
    tag: Option<&str>,
    progress: &mut ProgressReporter,
    no_cache: bool,
    build_args: Option<HashMap<String, String>>,
) -> Result<String, DockerError> {
    let tag = tag.unwrap_or(IMAGE_TAG_DEFAULT);
    let full_name = format!("{IMAGE_NAME_GHCR}:{tag}");
    debug!("Building image: {} (no_cache: {})", full_name, no_cache);

    // Create tar archive containing Dockerfile
    let context = create_build_context()
        .map_err(|e| DockerError::Build(format!("Failed to create build context: {e}")))?;

    // Set up build options
    // Explicitly use BuildKit builder to support cache mounts (--mount=type=cache)
    // BuildKit requires a unique session ID for each build
    let session_id = format!(
        "opencode-cloud-build-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let build_args = build_args.unwrap_or_default();
    let options = BuildImageOptions {
        t: Some(full_name.clone()),
        dockerfile: "Dockerfile".to_string(),
        version: BuilderVersion::BuilderBuildKit,
        session: Some(session_id),
        rm: true,
        nocache: no_cache,
        buildargs: Some(build_args),
        platform: String::new(),
        target: String::new(),
        ..Default::default()
    };

    // Create build body from context
    let body: Either<Full<Bytes>, _> = Either::Left(Full::new(Bytes::from(context)));

    // Start build with streaming output
    let mut stream = client.inner().build_image(options, None, Some(body));

    // Add main build spinner (context prefix like "Building image" is set by caller)
    progress.add_spinner("build", "Initializing...");

    let mut maybe_image_id = None;
    let mut log_state = BuildLogState::new();

    while let Some(result) = stream.next().await {
        let Ok(info) = result else {
            return Err(handle_stream_error(
                "Build failed",
                result.expect_err("checked error").to_string(),
                &log_state,
                progress,
            ));
        };

        handle_stream_message(&info, progress, &mut log_state);

        if let Some(error_detail) = &info.error_detail
            && let Some(error_msg) = &error_detail.message
        {
            progress.abandon_all(error_msg);
            let context = format_build_error_with_context(
                error_msg,
                &log_state.recent_logs,
                &log_state.error_logs,
                &log_state.recent_buildkit_logs,
            );
            return Err(DockerError::Build(context));
        }

        if let Some(aux) = info.aux {
            match aux {
                BuildInfoAux::Default(image_id) => {
                    if let Some(id) = image_id.id {
                        maybe_image_id = Some(id);
                    }
                }
                BuildInfoAux::BuildKit(status) => {
                    handle_buildkit_status(&status, progress, &mut log_state);
                }
            }
        }
    }

    let image_id = maybe_image_id.unwrap_or_else(|| "unknown".to_string());
    let finish_msg = format!("Build complete: {image_id}");
    progress.finish("build", &finish_msg);

    Ok(full_name)
}

struct BuildLogState {
    recent_logs: VecDeque<String>,
    error_logs: VecDeque<String>,
    recent_buildkit_logs: VecDeque<String>,
    build_log_buffer_size: usize,
    error_log_buffer_size: usize,
    last_buildkit_vertex: Option<String>,
    last_buildkit_vertex_id: Option<String>,
    export_vertex_id: Option<String>,
    export_vertex_name: Option<String>,
    buildkit_logs_by_vertex_id: HashMap<String, String>,
    vertex_name_by_vertex_id: HashMap<String, String>,
}

impl BuildLogState {
    fn new() -> Self {
        let build_log_buffer_size = read_log_buffer_size(
            "OPENCODE_DOCKER_BUILD_LOG_TAIL",
            DEFAULT_BUILD_LOG_BUFFER_SIZE,
        );
        let error_log_buffer_size = read_log_buffer_size(
            "OPENCODE_DOCKER_BUILD_ERROR_TAIL",
            DEFAULT_ERROR_LOG_BUFFER_SIZE,
        );
        Self {
            recent_logs: VecDeque::with_capacity(build_log_buffer_size),
            error_logs: VecDeque::with_capacity(error_log_buffer_size),
            recent_buildkit_logs: VecDeque::with_capacity(build_log_buffer_size),
            build_log_buffer_size,
            error_log_buffer_size,
            last_buildkit_vertex: None,
            last_buildkit_vertex_id: None,
            export_vertex_id: None,
            export_vertex_name: None,
            buildkit_logs_by_vertex_id: HashMap::new(),
            vertex_name_by_vertex_id: HashMap::new(),
        }
    }
}

fn handle_stream_message(
    info: &bollard::models::BuildInfo,
    progress: &mut ProgressReporter,
    state: &mut BuildLogState,
) {
    let Some(stream_msg) = info.stream.as_deref() else {
        return;
    };
    let msg = stream_msg.trim();
    if msg.is_empty() {
        return;
    }

    if progress.is_plain_output() {
        eprint!("{stream_msg}");
    } else {
        let has_runtime_vertex = state
            .last_buildkit_vertex
            .as_deref()
            .is_some_and(|name| name.starts_with("[runtime "));
        let is_internal_msg = msg.contains("[internal]");
        if !(has_runtime_vertex && is_internal_msg) {
            progress.update_spinner("build", stream_msg);
        }
    }

    if state.recent_logs.len() >= state.build_log_buffer_size {
        state.recent_logs.pop_front();
    }
    state.recent_logs.push_back(msg.to_string());

    if is_error_line(msg) {
        if state.error_logs.len() >= state.error_log_buffer_size {
            state.error_logs.pop_front();
        }
        state.error_logs.push_back(msg.to_string());
    }

    if msg.starts_with("Step ") {
        debug!("Build step: {}", msg);
    }
}

fn handle_buildkit_status(
    status: &BuildkitStatusResponse,
    progress: &mut ProgressReporter,
    state: &mut BuildLogState,
) {
    let latest_logs = append_buildkit_logs(&mut state.buildkit_logs_by_vertex_id, status);
    update_buildkit_vertex_names(&mut state.vertex_name_by_vertex_id, status);
    update_export_vertex_from_logs(
        &latest_logs,
        &state.vertex_name_by_vertex_id,
        &mut state.export_vertex_id,
        &mut state.export_vertex_name,
    );
    let (vertex_id, vertex_name) = match select_latest_buildkit_vertex(
        status,
        &state.vertex_name_by_vertex_id,
        state.export_vertex_id.as_deref(),
        state.export_vertex_name.as_deref(),
    ) {
        Some((vertex_id, vertex_name)) => (vertex_id, vertex_name),
        None => {
            let Some(log_entry) = latest_logs.last() else {
                return;
            };
            let name = state
                .vertex_name_by_vertex_id
                .get(&log_entry.vertex_id)
                .cloned()
                .or_else(|| state.last_buildkit_vertex.clone())
                .unwrap_or_else(|| format_vertex_fallback_label(&log_entry.vertex_id));
            (log_entry.vertex_id.clone(), name)
        }
    };
    record_buildkit_logs(state, &latest_logs, &vertex_id, &vertex_name);
    state.last_buildkit_vertex_id = Some(vertex_id.clone());
    if state.last_buildkit_vertex.as_deref() != Some(&vertex_name) {
        state.last_buildkit_vertex = Some(vertex_name.clone());
    }

    let message = if progress.is_plain_output() {
        vertex_name
    } else if let Some(log_entry) = latest_logs
        .iter()
        .rev()
        .find(|entry| entry.vertex_id == vertex_id)
    {
        format!("{vertex_name} · {}", log_entry.message)
    } else {
        vertex_name
    };
    progress.update_spinner("build", &message);

    if progress.is_plain_output() {
        for log_entry in latest_logs {
            eprintln!("[{}] {}", log_entry.vertex_id, log_entry.message);
        }
        return;
    }

    let (Some(current_id), Some(current_name)) = (
        state.last_buildkit_vertex_id.as_ref(),
        state.last_buildkit_vertex.as_ref(),
    ) else {
        return;
    };

    let name = state
        .vertex_name_by_vertex_id
        .get(current_id)
        .unwrap_or(current_name);
    // Keep non-verbose output on the spinner line only.
    let _ = name;
}

fn handle_stream_error(
    prefix: &str,
    error_str: String,
    state: &BuildLogState,
    progress: &mut ProgressReporter,
) -> DockerError {
    progress.abandon_all(prefix);

    let buildkit_hint = if error_str.contains("mount")
        || error_str.contains("--mount")
        || state
            .recent_logs
            .iter()
            .any(|log| log.contains("--mount") && log.contains("cache"))
    {
        "\n\nNote: This Dockerfile uses BuildKit cache mounts (--mount=type=cache).\n\
         The build is configured to use BuildKit, but the Docker daemon may not support it.\n\
         Ensure BuildKit is enabled in Docker Desktop settings and the daemon is restarted."
    } else {
        ""
    };

    let context = format!(
        "{}{}",
        format_build_error_with_context(
            &error_str,
            &state.recent_logs,
            &state.error_logs,
            &state.recent_buildkit_logs,
        ),
        buildkit_hint
    );
    DockerError::Build(context)
}

fn update_buildkit_vertex_names(
    vertex_name_by_vertex_id: &mut HashMap<String, String>,
    status: &BuildkitStatusResponse,
) {
    for vertex in &status.vertexes {
        if vertex.name.is_empty() {
            continue;
        }
        vertex_name_by_vertex_id
            .entry(vertex.digest.clone())
            .or_insert_with(|| vertex.name.clone());
    }
}

fn select_latest_buildkit_vertex(
    status: &BuildkitStatusResponse,
    vertex_name_by_vertex_id: &HashMap<String, String>,
    export_vertex_id: Option<&str>,
    export_vertex_name: Option<&str>,
) -> Option<(String, String)> {
    if let Some(export_vertex_id) = export_vertex_id {
        let name = export_vertex_name
            .map(str::to_string)
            .or_else(|| vertex_name_by_vertex_id.get(export_vertex_id).cloned())
            .unwrap_or_else(|| format_vertex_fallback_label(export_vertex_id));
        return Some((export_vertex_id.to_string(), name));
    }

    let mut best_runtime: Option<(u32, String, String)> = None;
    let mut fallback: Option<(String, String)> = None;

    for vertex in &status.vertexes {
        let name = if vertex.name.is_empty() {
            vertex_name_by_vertex_id.get(&vertex.digest).cloned()
        } else {
            Some(vertex.name.clone())
        };

        let Some(name) = name else {
            continue;
        };

        if fallback.is_none() && !name.starts_with("[internal]") {
            fallback = Some((vertex.digest.clone(), name.clone()));
        }

        if let Some(step) = parse_runtime_step(&name) {
            match &best_runtime {
                Some((best_step, _, _)) if *best_step >= step => {}
                _ => {
                    best_runtime = Some((step, vertex.digest.clone(), name.clone()));
                }
            }
        }
    }

    if let Some((_, digest, name)) = best_runtime {
        Some((digest, name))
    } else {
        fallback.or_else(|| {
            status.vertexes.iter().find_map(|vertex| {
                let name = if vertex.name.is_empty() {
                    vertex_name_by_vertex_id.get(&vertex.digest).cloned()
                } else {
                    Some(vertex.name.clone())
                };
                name.map(|resolved| (vertex.digest.clone(), resolved))
            })
        })
    }
}

fn parse_runtime_step(name: &str) -> Option<u32> {
    let prefix = "[runtime ";
    let start = name.find(prefix)? + prefix.len();
    let rest = &name[start..];
    let end = rest.find('/')?;
    rest[..end].trim().parse::<u32>().ok()
}

fn format_vertex_fallback_label(vertex_id: &str) -> String {
    let short = vertex_id
        .strip_prefix("sha256:")
        .unwrap_or(vertex_id)
        .chars()
        .take(12)
        .collect::<String>();
    format!("vertex {short}")
}

fn update_export_vertex_from_logs(
    latest_logs: &[BuildkitLogEntry],
    vertex_name_by_vertex_id: &HashMap<String, String>,
    export_vertex_id: &mut Option<String>,
    export_vertex_name: &mut Option<String>,
) {
    if let Some(entry) = latest_logs
        .iter()
        .rev()
        .find(|log| log.message.trim_start().starts_with("exporting to image"))
    {
        *export_vertex_id = Some(entry.vertex_id.clone());
        if let Some(name) = vertex_name_by_vertex_id.get(&entry.vertex_id) {
            *export_vertex_name = Some(name.clone());
        }
    }
}

fn record_buildkit_logs(
    state: &mut BuildLogState,
    latest_logs: &[BuildkitLogEntry],
    current_vertex_id: &str,
    current_vertex_name: &str,
) {
    for log_entry in latest_logs {
        let name = state
            .vertex_name_by_vertex_id
            .get(&log_entry.vertex_id)
            .cloned()
            .or_else(|| {
                if log_entry.vertex_id == current_vertex_id {
                    Some(current_vertex_name.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| format_vertex_fallback_label(&log_entry.vertex_id));

        let message = log_entry.message.replace('\r', "").trim_end().to_string();
        if message.is_empty() {
            continue;
        }

        if state.recent_buildkit_logs.len() >= state.build_log_buffer_size {
            state.recent_buildkit_logs.pop_front();
        }
        state
            .recent_buildkit_logs
            .push_back(format!("[{name}] {message}"));
    }
}

#[derive(Debug, Clone)]
struct BuildkitLogEntry {
    vertex_id: String,
    message: String,
}

fn append_buildkit_logs(
    logs: &mut HashMap<String, String>,
    status: &BuildkitStatusResponse,
) -> Vec<BuildkitLogEntry> {
    let mut latest: Vec<BuildkitLogEntry> = Vec::new();

    for log in &status.logs {
        let vertex_id = log.vertex.clone();
        let message = String::from_utf8_lossy(&log.msg).to_string();
        let entry = logs.entry(vertex_id.clone()).or_default();
        entry.push_str(&message);
        latest.push(BuildkitLogEntry { vertex_id, message });
    }

    latest
}

/// Pull the opencode image from registry with automatic fallback
///
/// Tries GHCR first, falls back to Docker Hub on failure.
/// Returns the full image:tag string on success.
pub async fn pull_image(
    client: &DockerClient,
    tag: Option<&str>,
    progress: &mut ProgressReporter,
) -> Result<String, DockerError> {
    let tag = tag.unwrap_or(IMAGE_TAG_DEFAULT);

    // Try GHCR first
    debug!("Attempting to pull from GHCR: {}:{}", IMAGE_NAME_GHCR, tag);
    let ghcr_err = match pull_from_registry(client, IMAGE_NAME_GHCR, tag, progress).await {
        Ok(()) => {
            let full_name = format!("{IMAGE_NAME_GHCR}:{tag}");
            return Ok(full_name);
        }
        Err(e) => e,
    };

    warn!(
        "GHCR pull failed: {}. Trying Docker Hub fallback...",
        ghcr_err
    );

    // Try Docker Hub as fallback
    debug!(
        "Attempting to pull from Docker Hub: {}:{}",
        IMAGE_NAME_DOCKERHUB, tag
    );
    match pull_from_registry(client, IMAGE_NAME_DOCKERHUB, tag, progress).await {
        Ok(()) => {
            let full_name = format!("{IMAGE_NAME_DOCKERHUB}:{tag}");
            Ok(full_name)
        }
        Err(dockerhub_err) => Err(DockerError::Pull(format!(
            "Failed to pull from both registries. GHCR: {ghcr_err}. Docker Hub: {dockerhub_err}"
        ))),
    }
}

/// Maximum number of retry attempts for pull operations
const MAX_PULL_RETRIES: usize = 3;

/// Pull from a specific registry with retry logic
async fn pull_from_registry(
    client: &DockerClient,
    image: &str,
    tag: &str,
    progress: &mut ProgressReporter,
) -> Result<(), DockerError> {
    let full_name = format!("{image}:{tag}");

    // Manual retry loop since async closures can't capture mutable references
    let mut last_error = None;
    for attempt in 1..=MAX_PULL_RETRIES {
        debug!(
            "Pull attempt {}/{} for {}",
            attempt, MAX_PULL_RETRIES, full_name
        );

        match do_pull(client, image, tag, progress).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!("Pull attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < MAX_PULL_RETRIES {
                    // Exponential backoff: 1s, 2s, 4s
                    let delay_ms = 1000 * (1 << (attempt - 1));
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        DockerError::Pull(format!(
            "Pull failed for {full_name} after {MAX_PULL_RETRIES} attempts"
        ))
    }))
}

/// Perform the actual pull operation
async fn do_pull(
    client: &DockerClient,
    image: &str,
    tag: &str,
    progress: &mut ProgressReporter,
) -> Result<(), DockerError> {
    let full_name = format!("{image}:{tag}");

    let options = CreateImageOptions {
        from_image: Some(image.to_string()),
        tag: Some(tag.to_string()),
        platform: String::new(),
        ..Default::default()
    };

    let mut stream = client.inner().create_image(Some(options), None, None);

    // Add main spinner for overall progress
    progress.add_spinner("pull", &format!("Pulling {full_name}..."));

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                // Handle errors from the stream
                if let Some(error_detail) = &info.error_detail
                    && let Some(error_msg) = &error_detail.message
                {
                    progress.abandon_all(error_msg);
                    return Err(DockerError::Pull(error_msg.to_string()));
                }

                // Handle layer progress
                if let Some(layer_id) = &info.id {
                    let status = info.status.as_deref().unwrap_or("");

                    match status {
                        "Already exists" => {
                            progress.finish(layer_id, "Already exists");
                        }
                        "Pull complete" => {
                            progress.finish(layer_id, "Pull complete");
                        }
                        "Downloading" | "Extracting" => {
                            if let Some(progress_detail) = &info.progress_detail {
                                let current = progress_detail.current.unwrap_or(0) as u64;
                                let total = progress_detail.total.unwrap_or(0) as u64;

                                if total > 0 {
                                    progress.update_layer(layer_id, current, total, status);
                                }
                            }
                        }
                        _ => {
                            // Other statuses (Waiting, Verifying, etc.)
                            progress.update_spinner(layer_id, status);
                        }
                    }
                } else if let Some(status) = &info.status {
                    // Overall status messages (no layer id)
                    progress.update_spinner("pull", status);
                }
            }
            Err(e) => {
                progress.abandon_all("Pull failed");
                return Err(DockerError::Pull(format!("Pull failed: {e}")));
            }
        }
    }

    progress.finish("pull", &format!("Pull complete: {full_name}"));
    Ok(())
}

/// Format a build error with recent log context for actionable debugging
fn format_build_error_with_context(
    error: &str,
    recent_logs: &VecDeque<String>,
    error_logs: &VecDeque<String>,
    recent_buildkit_logs: &VecDeque<String>,
) -> String {
    let mut message = String::new();

    // Add main error message
    message.push_str(error);

    // Add captured error lines if they differ from recent logs
    // (these are error-like lines that may have scrolled off)
    if !error_logs.is_empty() {
        // Check if error_logs contains lines not in recent_logs
        let recent_set: std::collections::HashSet<_> = recent_logs.iter().collect();
        let unique_errors: Vec<_> = error_logs
            .iter()
            .filter(|line| !recent_set.contains(line))
            .collect();

        if !unique_errors.is_empty() {
            message.push_str("\n\nPotential errors detected during build:");
            for line in unique_errors {
                message.push_str("\n  ");
                message.push_str(line);
            }
        }
    }

    // Add recent BuildKit log context if available
    if !recent_buildkit_logs.is_empty() {
        message.push_str("\n\nRecent BuildKit output:");
        for line in recent_buildkit_logs {
            message.push_str("\n  ");
            message.push_str(line);
        }
    }

    // Add recent log context if available
    if !recent_logs.is_empty() {
        message.push_str("\n\nRecent build output:");
        for line in recent_logs {
            message.push_str("\n  ");
            message.push_str(line);
        }
    } else if recent_buildkit_logs.is_empty() {
        message.push_str("\n\nNo build output was received from the Docker daemon.");
        message.push_str("\nThis usually means the build failed before any logs were streamed.");
    }

    // Add actionable suggestions based on common error patterns
    let error_lower = error.to_lowercase();
    if error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("timeout")
    {
        message.push_str("\n\nSuggestion: Check your network connection and Docker's ability to reach the internet.");
    } else if error_lower.contains("disk")
        || error_lower.contains("space")
        || error_lower.contains("no space")
    {
        message.push_str("\n\nSuggestion: Free up disk space with 'docker system prune' or check available storage.");
    } else if error_lower.contains("permission") || error_lower.contains("denied") {
        message.push_str("\n\nSuggestion: Check Docker permissions. You may need to add your user to the 'docker' group.");
    }

    message
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
    use bollard::models::ImageSummary;
    use std::collections::HashMap;

    fn make_image_summary(
        id: &str,
        tags: Vec<&str>,
        digests: Vec<&str>,
        labels: HashMap<String, String>,
    ) -> ImageSummary {
        ImageSummary {
            id: id.to_string(),
            parent_id: String::new(),
            repo_tags: tags.into_iter().map(|tag| tag.to_string()).collect(),
            repo_digests: digests
                .into_iter()
                .map(|digest| digest.to_string())
                .collect(),
            created: 0,
            size: 0,
            shared_size: -1,
            labels,
            containers: 0,
            manifests: None,
            descriptor: None,
        }
    }

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

    #[test]
    fn format_build_error_includes_recent_logs() {
        let mut logs = VecDeque::new();
        logs.push_back("Step 1/5 : FROM ubuntu:24.04".to_string());
        logs.push_back("Step 2/5 : RUN apt-get update".to_string());
        logs.push_back("E: Unable to fetch some archives".to_string());
        let error_logs = VecDeque::new();
        let buildkit_logs = VecDeque::new();

        let result = format_build_error_with_context(
            "Build failed: exit code 1",
            &logs,
            &error_logs,
            &buildkit_logs,
        );

        assert!(result.contains("Build failed: exit code 1"));
        assert!(result.contains("Recent build output:"));
        assert!(result.contains("Step 1/5"));
        assert!(result.contains("Unable to fetch"));
    }

    #[test]
    fn format_build_error_handles_empty_logs() {
        let logs = VecDeque::new();
        let error_logs = VecDeque::new();
        let buildkit_logs = VecDeque::new();
        let result =
            format_build_error_with_context("Stream error", &logs, &error_logs, &buildkit_logs);

        assert!(result.contains("Stream error"));
        assert!(!result.contains("Recent build output:"));
    }

    #[test]
    fn format_build_error_adds_network_suggestion() {
        let logs = VecDeque::new();
        let error_logs = VecDeque::new();
        let buildkit_logs = VecDeque::new();
        let result = format_build_error_with_context(
            "connection timeout",
            &logs,
            &error_logs,
            &buildkit_logs,
        );

        assert!(result.contains("Check your network connection"));
    }

    #[test]
    fn format_build_error_adds_disk_suggestion() {
        let logs = VecDeque::new();
        let error_logs = VecDeque::new();
        let buildkit_logs = VecDeque::new();
        let result = format_build_error_with_context(
            "no space left on device",
            &logs,
            &error_logs,
            &buildkit_logs,
        );

        assert!(result.contains("Free up disk space"));
    }

    #[test]
    fn format_build_error_shows_error_lines_separately() {
        let mut recent_logs = VecDeque::new();
        recent_logs.push_back("Compiling foo v1.0".to_string());
        recent_logs.push_back("Successfully installed bar".to_string());

        let mut error_logs = VecDeque::new();
        error_logs.push_back("error: failed to compile dust".to_string());
        error_logs.push_back("error: failed to compile glow".to_string());

        let buildkit_logs = VecDeque::new();
        let result = format_build_error_with_context(
            "Build failed",
            &recent_logs,
            &error_logs,
            &buildkit_logs,
        );

        assert!(result.contains("Potential errors detected during build:"));
        assert!(result.contains("failed to compile dust"));
        assert!(result.contains("failed to compile glow"));
    }

    #[test]
    fn is_error_line_detects_errors() {
        assert!(is_error_line("error: something failed"));
        assert!(is_error_line("Error: build failed"));
        assert!(is_error_line("Failed to install package"));
        assert!(is_error_line("cannot find module"));
        assert!(is_error_line("Unable to locate package"));
        assert!(!is_error_line("Compiling foo v1.0"));
        assert!(!is_error_line("Successfully installed"));
    }

    #[test]
    fn collect_image_ids_matches_labels() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_SOURCE.to_string(), LABEL_SOURCE_VALUE.to_string());

        let images = vec![
            make_image_summary("sha256:opencode", vec![], vec![], labels),
            make_image_summary(
                "sha256:other",
                vec!["busybox:latest"],
                vec![],
                HashMap::new(),
            ),
        ];

        let ids = collect_image_ids(&images, "opencode-cloud-sandbox");
        assert!(ids.contains("sha256:opencode"));
        assert!(!ids.contains("sha256:other"));
    }
}
