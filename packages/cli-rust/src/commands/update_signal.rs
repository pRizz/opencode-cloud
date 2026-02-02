//! Host-side update command listener.
//!
//! Watches a bind-mounted command file and triggers `occ update opencode`.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use opencode_cloud_core::config::Config;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, MOUNT_STATE, ParsedMount};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tokio::fs;
use tokio::time;

use super::update::{UpdateOpencodeArgs, cmd_update_opencode};

const COMMAND_DIR_SUFFIX: &str = "opencode-cloud/commands";
const COMMAND_FILE_NAME: &str = "update-command.json";
const RESULT_FILE_NAME: &str = "update-command.result.json";
const COMMAND_POLL_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_WRITE_GRACE: Duration = Duration::from_secs(2);
const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct UpdateCommandRequest {
    command: String,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    commit: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct UpdateCommandResult {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    message: String,
    started_at: String,
    finished_at: String,
}

struct CommandPaths {
    command_file: PathBuf,
    result_file: PathBuf,
    container_path: String,
}

enum CommandLoad {
    Missing,
    Retry,
    Parsed(UpdateCommandRequest),
    Invalid(String),
}

pub async fn run_update_command_listener(
    client: &DockerClient,
    config: &Config,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    let maybe_paths = resolve_command_paths(config, quiet)?;
    if maybe_host.is_some() && !quiet {
        eprintln!("Update command listener disabled for remote hosts (local bind mount only).");
    }
    if maybe_paths.is_none() && !quiet {
        eprintln!(
            "Update command listener disabled (missing writable bind mount for {MOUNT_STATE})."
        );
    }

    if !quiet {
        eprintln!("Listening for update commands (Ctrl+C to stop)...");
        if let Some(paths) = &maybe_paths {
            eprintln!("  Command file: {}", paths.command_file.display());
            eprintln!("  Container path: {}", paths.container_path);
        }
        eprintln!();
    }

    let mut interval = time::interval(COMMAND_POLL_INTERVAL);
    let mut last_processed: Option<Instant> = None;
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                if !quiet {
                    eprintln!("Stopping update command listener.");
                }
                return Ok(());
            }
            _ = interval.tick() => {
                if !opencode_cloud_core::docker::container_is_running(client, CONTAINER_NAME)
                    .await
                    .unwrap_or(false)
                {
                    if !quiet {
                        eprintln!("Container stopped. Exiting update command listener.");
                    }
                    return Ok(());
                }
                let Some(paths) = &maybe_paths else {
                    continue;
                };
                if maybe_host.is_some() {
                    continue;
                }
                ensure_command_dir(paths).await?;
                if !can_process_command(last_processed) {
                    continue;
                }
                if let Some(result) = poll_command(paths, maybe_host, quiet, verbose).await? {
                    write_result(&paths.result_file, &result).await?;
                    last_processed = Some(Instant::now());
                }
            }
        }
    }
}

fn resolve_command_paths(config: &Config, quiet: bool) -> Result<Option<CommandPaths>> {
    let maybe_mount = find_state_mount(config);
    let Some(mount) = maybe_mount else {
        return Ok(None);
    };
    if mount.read_only {
        if !quiet {
            eprintln!("Update command listener disabled ({MOUNT_STATE} is mounted read-only).");
        }
        return Ok(None);
    }
    let command_dir = mount.host_path.join(COMMAND_DIR_SUFFIX);
    let command_file = command_dir.join(COMMAND_FILE_NAME);
    let result_file = command_dir.join(RESULT_FILE_NAME);
    let container_path = format!("{MOUNT_STATE}/{COMMAND_DIR_SUFFIX}/{COMMAND_FILE_NAME}");
    Ok(Some(CommandPaths {
        command_file,
        result_file,
        container_path,
    }))
}

fn find_state_mount(config: &Config) -> Option<ParsedMount> {
    config.mounts.iter().find_map(|mount_str| {
        ParsedMount::parse(mount_str)
            .ok()
            .filter(|parsed| parsed.container_path == MOUNT_STATE)
    })
}

async fn ensure_command_dir(paths: &CommandPaths) -> Result<()> {
    if let Some(parent) = paths.command_file.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create command dir: {}", parent.display()))?;
    }
    Ok(())
}

async fn poll_command(
    paths: &CommandPaths,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<Option<UpdateCommandResult>> {
    match load_command(&paths.command_file).await? {
        CommandLoad::Missing | CommandLoad::Retry => Ok(None),
        CommandLoad::Invalid(error) => {
            let now = Utc::now();
            let result = build_result(None, "error", error, now, now);
            fs::remove_file(&paths.command_file).await.ok();
            Ok(Some(result))
        }
        CommandLoad::Parsed(request) => {
            let started_at = Utc::now();
            let request_id = request.request_id.clone();
            let result = match handle_command(request, maybe_host, quiet, verbose).await {
                Ok(message) => build_result(request_id, "success", message, started_at, Utc::now()),
                Err(error) => build_result(
                    request_id,
                    "error",
                    error.to_string(),
                    started_at,
                    Utc::now(),
                ),
            };
            fs::remove_file(&paths.command_file).await.ok();
            Ok(Some(result))
        }
    }
}

async fn load_command(command_file: &Path) -> Result<CommandLoad> {
    let metadata = match fs::metadata(command_file).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CommandLoad::Missing);
        }
        Err(error) => {
            return Err(anyhow!(
                "Failed to read update command file metadata: {error}"
            ));
        }
    };

    let contents = fs::read_to_string(command_file).await.with_context(|| {
        format!(
            "Failed to read update command file: {}",
            command_file.display()
        )
    })?;

    match serde_json::from_str::<UpdateCommandRequest>(&contents) {
        Ok(request) => Ok(CommandLoad::Parsed(request)),
        Err(error) => {
            if is_recently_modified(&metadata, COMMAND_WRITE_GRACE) {
                return Ok(CommandLoad::Retry);
            }
            Ok(CommandLoad::Invalid(format!(
                "Invalid update command JSON: {error}"
            )))
        }
    }
}

async fn handle_command(
    request: UpdateCommandRequest,
    maybe_host: Option<&str>,
    quiet: bool,
    verbose: u8,
) -> Result<String> {
    if request.command != "update_opencode" {
        return Err(anyhow!(
            "Unsupported command '{}'. Expected: update_opencode",
            request.command
        ));
    }
    if request.branch.is_some() && request.commit.is_some() {
        return Err(anyhow!("Specify only one of branch or commit."));
    }

    let args = UpdateOpencodeArgs {
        branch: request.branch.clone(),
        commit: request.commit.clone(),
        yes: true,
    };
    cmd_update_opencode(&args, maybe_host, quiet, verbose).await?;
    Ok("Update completed".to_string())
}

fn build_result(
    request_id: Option<String>,
    status: &str,
    message: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
) -> UpdateCommandResult {
    UpdateCommandResult {
        status: status.to_string(),
        request_id,
        message,
        started_at: started_at.to_rfc3339(),
        finished_at: finished_at.to_rfc3339(),
    }
}

fn is_recently_modified(metadata: &std::fs::Metadata, threshold: Duration) -> bool {
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };
    elapsed < threshold
}

fn can_process_command(last_processed: Option<Instant>) -> bool {
    let Some(last_processed) = last_processed else {
        return true;
    };
    last_processed.elapsed() >= MIN_REQUEST_INTERVAL
}

async fn write_result(path: &Path, result: &UpdateCommandResult) -> Result<()> {
    let payload = serde_json::to_string_pretty(result)
        .context("Failed to serialize update command result")?;
    fs::write(path, payload)
        .await
        .with_context(|| format!("Failed to write update command result: {}", path.display()))?;
    Ok(())
}
