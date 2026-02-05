//! Container-mode update command implementation.

use crate::commands::UpdateArgs;
use crate::commands::UpdateCommand;
use crate::commands::UpdateOpencodeArgs;
use crate::commands::container::{exec_command, exec_command_with_status, systemd_available};
use crate::commands::update::{build_opencode_update_script, short_commit};
use crate::output::CommandSpinner;
use anyhow::{Result, anyhow};
use console::style;
use dialoguer::Confirm;
use std::fs;

pub async fn cmd_update_container(args: &UpdateArgs, quiet: bool, _verbose: u8) -> Result<()> {
    if args.rollback {
        return Err(anyhow!(
            "Rollback is not supported in container runtime.\n\
Use host runtime instead: occ --runtime host update --rollback"
        ));
    }

    match args.command {
        Some(UpdateCommand::Opencode(ref op_args)) => {
            cmd_update_opencode_container(op_args, quiet).await
        }
        _ => Err(anyhow!(
            "Only `occ update opencode` is supported in container runtime.\n\
To use host runtime: occ --runtime host update <target>"
        )),
    }
}

async fn cmd_update_opencode_container(args: &UpdateOpencodeArgs, quiet: bool) -> Result<()> {
    if !systemd_available() {
        return Err(anyhow!(
            "Opencode update requires systemd in container runtime.\n\
Run the update from the host instead: occ --runtime host update opencode"
        ));
    }

    let target_ref = args
        .commit
        .clone()
        .or_else(|| args.branch.clone())
        .unwrap_or_else(|| "dev".to_string());
    let checkout_cmd = if args.commit.is_some() {
        "git checkout \"$OPENCODE_REF\"".to_string()
    } else {
        "git checkout -B \"$OPENCODE_REF\" \"origin/$OPENCODE_REF\"".to_string()
    };

    let current_version = get_current_opencode_version().await;
    let current_commit = get_current_opencode_commit();
    let next_commit = if let Some(commit) = args.commit.as_deref() {
        Some(short_commit(commit))
    } else {
        resolve_remote_commit_local(&target_ref).await
    };

    if current_commit.is_some() && current_commit == next_commit {
        if !quiet {
            let check = style("✓").green();
            eprintln!(
                "{} Opencode is already up to date (hash: {}).",
                check,
                style(current_commit.unwrap_or_else(|| "unknown".to_string())).dim()
            );
        }
        return Ok(());
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "{} This will stop the opencode service, update from {target_ref}, rebuild, and restart.",
            style("Warning:").yellow().bold()
        );
        eprintln!(
            "Current:    version={}, hash={}",
            style(current_version.unwrap_or_else(|| "unknown".to_string())).dim(),
            style(current_commit.unwrap_or_else(|| "unknown".to_string())).dim()
        );
        let next_hash = next_commit.as_deref().unwrap_or("unknown");
        eprintln!("Next hash:  {}", style(next_hash).dim());
        eprintln!();
    }

    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt("Continue with opencode update?")
            .default(true)
            .interact()?;

        if !confirmed {
            if !quiet {
                eprintln!("Update cancelled.");
            }
            return Ok(());
        }
    }

    let spinner = CommandSpinner::new_maybe("Updating opencode...", quiet);

    stop_opencode_systemd(quiet).await?;

    let update_script = build_opencode_update_script(&target_ref, &checkout_cmd);
    let (update_output, update_status) =
        exec_command_with_status("bash", &["-lc", &update_script]).await?;
    if !quiet && !update_output.trim().is_empty() {
        eprintln!(
            "{} Update output:\n{}",
            style("[info]").cyan(),
            update_output.trim()
        );
    }
    if update_status != 0 {
        return Err(anyhow!(
            "Opencode update failed (exit {update_status}).\n{update_output}"
        ));
    }

    if let Some(expected) = next_commit.as_deref() {
        let updated_commit = get_current_opencode_commit();
        if updated_commit.as_deref() != Some(expected) {
            let found = updated_commit.unwrap_or_else(|| "unknown".to_string());
            return Err(anyhow!(
                "Opencode update did not apply (expected {expected}, found {found}).\n{update_output}"
            ));
        }
    }

    spinner.update("Restarting opencode service...");
    restart_opencode_systemd().await?;

    spinner.success("Opencode updated");

    if !quiet {
        eprintln!();
        eprintln!(
            "{} Opencode updated successfully!",
            style("Success:").green().bold()
        );
        eprintln!();
    }

    Ok(())
}

async fn stop_opencode_systemd(quiet: bool) -> Result<()> {
    let (output, status) = exec_command_with_status(
        "systemctl",
        &["stop", "opencode.service", "opencode-broker.service"],
    )
    .await?;
    if status != 0 && !quiet {
        eprintln!(
            "{} Failed to stop opencode via systemd (exit {status}). Continuing update.",
            style("Warning:").yellow()
        );
        if !output.trim().is_empty() {
            eprintln!("{}", style(output.trim()).dim());
        }
    }
    Ok(())
}

async fn restart_opencode_systemd() -> Result<()> {
    let (output, status) = exec_command_with_status(
        "systemctl",
        &["restart", "opencode.service", "opencode-broker.service"],
    )
    .await?;
    if status != 0 {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Err(anyhow!(
                "Failed to restart opencode via systemd (exit {status})."
            ));
        }
        return Err(anyhow!(
            "Failed to restart opencode via systemd (exit {status}).\n{trimmed}"
        ));
    }
    Ok(())
}

async fn get_current_opencode_version() -> Option<String> {
    let output = exec_command("/opt/opencode/bin/opencode", &["--version"])
        .await
        .ok()?;
    let version = output.lines().next()?.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn get_current_opencode_commit() -> Option<String> {
    let contents = fs::read_to_string("/opt/opencode/COMMIT").ok()?;
    let commit = contents.lines().next()?.trim();
    if commit.is_empty() {
        None
    } else {
        Some(short_commit(commit))
    }
}

async fn resolve_remote_commit_local(target_ref: &str) -> Option<String> {
    let (output, status) = exec_command_with_status(
        "git",
        &[
            "ls-remote",
            "https://github.com/pRizz/opencode.git",
            target_ref,
        ],
    )
    .await
    .ok()?;
    if status != 0 {
        return None;
    }
    let full = output.split_whitespace().next()?;
    Some(short_commit(full))
}
