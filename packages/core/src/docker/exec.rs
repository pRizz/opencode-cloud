//! Container exec wrapper for running commands in containers
//!
//! This module provides functions to execute commands inside running Docker
//! containers, with support for capturing output and providing stdin input.
//! Used for user management operations like useradd, chpasswd, etc.

use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use super::profile::remap_container_name;
use super::{DockerClient, DockerError};

/// Execute a command in a running container and capture output
///
/// Creates an exec instance, runs the command, and collects stdout/stderr.
/// Returns the combined output as a String.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `cmd` - Command and arguments to execute
///
/// # Example
/// ```ignore
/// let output = exec_command(&client, "opencode-cloud-sandbox", vec!["whoami"]).await?;
/// ```
pub async fn exec_command(
    client: &DockerClient,
    container: &str,
    cmd: Vec<&str>,
) -> Result<String, DockerError> {
    let container = remap_container_name(container);
    let exec_config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        user: Some("root".to_string()),
        ..Default::default()
    };

    let exec = client
        .inner()
        .create_exec(&container, exec_config)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to create exec: {e}")))?;

    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    let mut output = String::new();

    match client
        .inner()
        .start_exec(&exec.id, Some(start_config))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to start exec: {e}")))?
    {
        StartExecResults::Attached {
            output: mut stream, ..
        } => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(log_output) => {
                        output.push_str(&log_output.to_string());
                    }
                    Err(e) => {
                        return Err(DockerError::Container(format!(
                            "Error reading exec output: {e}"
                        )));
                    }
                }
            }
        }
        StartExecResults::Detached => {
            return Err(DockerError::Container(
                "Exec unexpectedly detached".to_string(),
            ));
        }
    }

    Ok(output)
}

/// Execute a command and capture output plus exit code
///
/// Returns a tuple of (output, exit_code). Exit code is -1 if not available.
pub async fn exec_command_with_status(
    client: &DockerClient,
    container: &str,
    cmd: Vec<&str>,
) -> Result<(String, i64), DockerError> {
    let container = remap_container_name(container);
    let exec_config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        user: Some("root".to_string()),
        ..Default::default()
    };

    let exec = client
        .inner()
        .create_exec(&container, exec_config)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to create exec: {e}")))?;

    let exec_id = exec.id.clone();
    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    let mut output = String::new();

    match client
        .inner()
        .start_exec(&exec.id, Some(start_config))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to start exec: {e}")))?
    {
        StartExecResults::Attached {
            output: mut stream, ..
        } => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(log_output) => {
                        output.push_str(&log_output.to_string());
                    }
                    Err(e) => {
                        return Err(DockerError::Container(format!(
                            "Error reading exec output: {e}"
                        )));
                    }
                }
            }
        }
        StartExecResults::Detached => {
            return Err(DockerError::Container(
                "Exec unexpectedly detached".to_string(),
            ));
        }
    }

    let inspect = client
        .inner()
        .inspect_exec(&exec_id)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to inspect exec: {e}")))?;

    let exit_code = inspect.exit_code.unwrap_or(-1);

    Ok((output, exit_code))
}

/// Execute a command with stdin input and capture output
///
/// Creates an exec instance with stdin attached, writes the provided data to
/// stdin, then collects stdout/stderr. Used for commands like `chpasswd` that
/// read passwords from stdin (never from command arguments for security).
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `cmd` - Command and arguments to execute
/// * `stdin_data` - Data to write to the command's stdin
///
/// # Security Note
/// This function is specifically designed for secure password handling.
/// The password is written to stdin and never appears in process arguments
/// or command logs.
///
/// # Example
/// ```ignore
/// // Set password via chpasswd (secure, non-interactive)
/// exec_command_with_stdin(
///     &client,
///     "opencode-cloud-sandbox",
///     vec!["chpasswd"],
///     "username:password\n"
/// ).await?;
/// ```
pub async fn exec_command_with_stdin(
    client: &DockerClient,
    container: &str,
    cmd: Vec<&str>,
    stdin_data: &str,
) -> Result<String, DockerError> {
    let container = remap_container_name(container);
    let exec_config = CreateExecOptions {
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        user: Some("root".to_string()),
        ..Default::default()
    };

    let exec = client
        .inner()
        .create_exec(&container, exec_config)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to create exec: {e}")))?;

    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    let mut output = String::new();

    match client
        .inner()
        .start_exec(&exec.id, Some(start_config))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to start exec: {e}")))?
    {
        StartExecResults::Attached {
            output: mut stream,
            input: mut input_sink,
        } => {
            // Write stdin data using AsyncWrite
            input_sink
                .write_all(stdin_data.as_bytes())
                .await
                .map_err(|e| DockerError::Container(format!("Failed to write to stdin: {e}")))?;

            // Close stdin to signal EOF
            input_sink
                .shutdown()
                .await
                .map_err(|e| DockerError::Container(format!("Failed to close stdin: {e}")))?;

            // Collect output
            while let Some(result) = stream.next().await {
                match result {
                    Ok(log_output) => {
                        output.push_str(&log_output.to_string());
                    }
                    Err(e) => {
                        return Err(DockerError::Container(format!(
                            "Error reading exec output: {e}"
                        )));
                    }
                }
            }
        }
        StartExecResults::Detached => {
            return Err(DockerError::Container(
                "Exec unexpectedly detached".to_string(),
            ));
        }
    }

    Ok(output)
}

/// Execute a command and return its exit code
///
/// Runs a command in the container and returns the exit code instead of output.
/// Useful for checking if a command succeeded (exit code 0) or failed.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `cmd` - Command and arguments to execute
///
/// # Example
/// ```ignore
/// // Check if user exists (id -u returns 0 if user exists)
/// let exit_code = exec_command_exit_code(&client, "opencode-cloud", vec!["id", "-u", "admin"]).await?;
/// let user_exists = exit_code == 0;
/// ```
pub async fn exec_command_exit_code(
    client: &DockerClient,
    container: &str,
    cmd: Vec<&str>,
) -> Result<i64, DockerError> {
    let container = remap_container_name(container);
    let exec_config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        user: Some("root".to_string()),
        ..Default::default()
    };

    let exec = client
        .inner()
        .create_exec(&container, exec_config)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to create exec: {e}")))?;

    let exec_id = exec.id.clone();

    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    // Run the command
    match client
        .inner()
        .start_exec(&exec.id, Some(start_config))
        .await
        .map_err(|e| DockerError::Container(format!("Failed to start exec: {e}")))?
    {
        StartExecResults::Attached { mut output, .. } => {
            // Drain the output stream (we don't care about the content)
            while output.next().await.is_some() {}
        }
        StartExecResults::Detached => {
            return Err(DockerError::Container(
                "Exec unexpectedly detached".to_string(),
            ));
        }
    }

    // Inspect the exec to get exit code
    let inspect = client
        .inner()
        .inspect_exec(&exec_id)
        .await
        .map_err(|e| DockerError::Container(format!("Failed to inspect exec: {e}")))?;

    // Exit code is None if process is still running, which shouldn't happen
    let exit_code = inspect.exit_code.unwrap_or(-1);

    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    // Note: These tests verify compilation and module structure.
    // Actual Docker exec tests require a running container and are
    // covered by integration tests.

    #[test]
    fn test_command_patterns() {
        // Verify the command patterns used in user management
        let useradd_cmd = ["useradd", "-m", "-s", "/bin/bash", "testuser"];
        assert_eq!(useradd_cmd.len(), 5);
        assert_eq!(useradd_cmd[0], "useradd");

        let id_cmd = ["id", "-u", "testuser"];
        assert_eq!(id_cmd.len(), 3);

        let chpasswd_cmd = ["chpasswd"];
        assert_eq!(chpasswd_cmd.len(), 1);
    }
}
