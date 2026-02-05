//! Container-mode logs command implementation.

use crate::commands::LogsArgs;
use crate::commands::container::systemd_available;
use crate::commands::logs::emit_log_line;
use anyhow::{Result, anyhow};
use console::style;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub async fn cmd_logs_container(args: &LogsArgs, quiet: bool) -> Result<()> {
    if !systemd_available() {
        return Err(anyhow!(
            "Logs unavailable in container runtime without systemd.\n\
Use host runtime instead: occ --runtime host logs"
        ));
    }

    let service = if args.broker {
        "opencode-broker"
    } else {
        "opencode"
    };
    let cmd = build_journalctl_command(args, service)?;

    if !quiet && !args.no_follow {
        let label = if args.broker {
            "Following broker logs (Ctrl+C to exit)..."
        } else {
            "Following logs (Ctrl+C to exit)..."
        };
        eprintln!("{}", style(label).dim());
        eprintln!();
    }

    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("Failed to start journalctl: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to capture journalctl output"))?;

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line?;
        emit_log_line(&line, args, None, quiet);
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("journalctl exited with status {}", status));
    }

    Ok(())
}

fn build_journalctl_command(args: &LogsArgs, service: &str) -> Result<Vec<String>> {
    let mut cmd = vec![
        "journalctl".to_string(),
        "--no-pager".to_string(),
        "-u".to_string(),
        service.to_string(),
    ];

    if args.timestamps {
        cmd.push("-o".to_string());
        cmd.push("short-iso".to_string());
        cmd.push("--no-hostname".to_string());
    } else {
        cmd.push("-o".to_string());
        cmd.push("cat".to_string());
    }

    let lines = args.lines.trim();
    if lines.eq_ignore_ascii_case("all") {
        // no -n flag
    } else if !lines.is_empty() && lines.chars().all(|c| c.is_ascii_digit()) {
        cmd.push("-n".to_string());
        cmd.push(lines.to_string());
    } else {
        return Err(anyhow!("Invalid value for --lines. Use a number or 'all'."));
    }

    if !args.no_follow {
        cmd.push("-f".to_string());
    }

    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> LogsArgs {
        LogsArgs {
            lines: "50".to_string(),
            no_follow: false,
            timestamps: false,
            grep: None,
            broker: false,
        }
    }

    #[test]
    fn journalctl_command_default() {
        let args = base_args();
        let cmd = build_journalctl_command(&args, "opencode").unwrap();
        assert!(cmd.contains(&"-u".to_string()));
        assert!(cmd.contains(&"opencode".to_string()));
        assert!(cmd.contains(&"-n".to_string()));
        assert!(cmd.contains(&"50".to_string()));
        assert!(cmd.contains(&"-f".to_string()));
    }

    #[test]
    fn journalctl_command_all_lines() {
        let mut args = base_args();
        args.lines = "all".to_string();
        let cmd = build_journalctl_command(&args, "opencode").unwrap();
        assert!(!cmd.contains(&"-n".to_string()));
    }

    #[test]
    fn journalctl_command_no_follow() {
        let mut args = base_args();
        args.no_follow = true;
        let cmd = build_journalctl_command(&args, "opencode").unwrap();
        assert!(!cmd.contains(&"-f".to_string()));
    }
}
