//! Container-mode command implementations and helpers.

pub mod logs;
pub mod status;
pub mod update;
pub mod users;

pub use logs::cmd_logs_container;
pub use status::cmd_status_container;
pub use update::cmd_update_container;
pub use users::cmd_user_container;

use anyhow::{Result, anyhow};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub(crate) fn systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}

pub(crate) async fn exec_command_with_status(cmd: &str, args: &[&str]) -> Result<(String, i32)> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow!("Failed to run {cmd}: {e}"))?;

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    let status = output.status.code().unwrap_or(-1);
    Ok((combined, status))
}

pub(crate) async fn exec_command(cmd: &str, args: &[&str]) -> Result<String> {
    let (output, status) = exec_command_with_status(cmd, args).await?;
    if status != 0 {
        let joined = args.join(" ");
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Command failed: {cmd} {joined} (exit {status})"));
        }
        return Err(anyhow!(
            "Command failed: {cmd} {joined} (exit {status})\n{trimmed}"
        ));
    }
    Ok(output)
}

pub(crate) async fn exec_command_with_stdin(
    cmd: &str,
    args: &[&str],
    stdin_data: &str,
) -> Result<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to run {cmd}: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .map_err(|e| anyhow!("Failed to write stdin for {cmd}: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to read output for {cmd}: {e}"))?;

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        let trimmed = combined.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Command failed: {cmd} (exit {status})"));
        }
        return Err(anyhow!("Command failed: {cmd} (exit {status})\n{trimmed}"));
    }

    Ok(combined)
}
