//! Mount add subcommand

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use opencode_cloud_core::config::{load_config_or_default, save_config};
use opencode_cloud_core::docker::{ParsedMount, check_container_path_warning, validate_mount_path};

#[derive(Args)]
pub struct MountAddArgs {
    /// Mount specification: /host/path:/container/path[:ro]
    pub mount_spec: String,

    /// Skip path validation (useful for paths that will exist later)
    #[arg(long)]
    pub no_validate: bool,

    /// Force add even if warning about system paths
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum MountUpsertOutcome {
    Added,
    AlreadyConfigured,
    Replaced { replaced_mounts: Vec<ParsedMount> },
}

fn upsert_mount_by_target(
    existing_mounts: &[String],
    mount_spec: &str,
    parsed: &ParsedMount,
) -> (Vec<String>, MountUpsertOutcome) {
    let mut retained = Vec::with_capacity(existing_mounts.len() + 1);
    let mut same_target = Vec::new();

    for existing in existing_mounts {
        match ParsedMount::parse(existing) {
            Ok(existing_parsed) if existing_parsed.container_path == parsed.container_path => {
                same_target.push(existing_parsed);
            }
            _ => retained.push(existing.clone()),
        }
    }

    let exact_match_count = same_target
        .iter()
        .filter(|mount| mount.host_path == parsed.host_path && mount.read_only == parsed.read_only)
        .count();

    if exact_match_count == 1 && same_target.len() == 1 {
        return (
            existing_mounts.to_vec(),
            MountUpsertOutcome::AlreadyConfigured,
        );
    }

    retained.push(mount_spec.to_string());
    if same_target.is_empty() {
        (retained, MountUpsertOutcome::Added)
    } else {
        (
            retained,
            MountUpsertOutcome::Replaced {
                replaced_mounts: same_target,
            },
        )
    }
}

fn mount_to_spec(mount: &ParsedMount) -> String {
    let mode = if mount.read_only { ":ro" } else { "" };
    format!(
        "{}:{}{}",
        mount.host_path.display(),
        mount.container_path,
        mode
    )
}

pub async fn cmd_mount_add(args: &MountAddArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Parse the mount spec
    let parsed = ParsedMount::parse(&args.mount_spec)?;

    // Validate host path unless --no-validate
    if !args.no_validate {
        validate_mount_path(&parsed.host_path)?;
    }

    // Check for system path warning
    if let Some(warning) = check_container_path_warning(&parsed.container_path) {
        if !args.force {
            eprintln!("{}", style(&warning).yellow());
            eprintln!();
            eprintln!("Use {} to add anyway.", style("--force").cyan());
            bail!("Mount target is a system path. Use --force to override.");
        }
        if !quiet {
            eprintln!("{}", style(&warning).yellow());
        }
    }

    // Load config and add mount
    let mut config = load_config_or_default()?;
    let host_str = parsed.host_path.to_string_lossy().to_string();
    let (updated_mounts, outcome) =
        upsert_mount_by_target(&config.mounts, &args.mount_spec, &parsed);
    config.mounts = updated_mounts;
    save_config(&config)?;

    if quiet {
        return Ok(());
    }

    match outcome {
        MountUpsertOutcome::AlreadyConfigured => {
            println!(
                "Mount already configured: {} -> {}",
                style(&host_str).cyan(),
                style(&parsed.container_path).cyan()
            );
            return Ok(());
        }
        MountUpsertOutcome::Added => {
            let mode = if parsed.read_only { "ro" } else { "rw" };
            println!(
                "Added mount: {} -> {} ({mode})",
                style(&host_str).cyan(),
                style(&parsed.container_path).cyan(),
            );
        }
        MountUpsertOutcome::Replaced { replaced_mounts } => {
            let mode = if parsed.read_only { "ro" } else { "rw" };
            println!(
                "Replaced mount target {} with {} -> {} ({mode})",
                style(&parsed.container_path).cyan(),
                style(&host_str).cyan(),
                style(&parsed.container_path).cyan(),
            );
            println!();
            println!("Previous mount(s) for this target:");
            for previous in &replaced_mounts {
                let previous_mode = if previous.read_only { "ro" } else { "rw" };
                println!(
                    "  - {} -> {} ({previous_mode})",
                    previous.host_path.display(),
                    previous.container_path
                );
            }
            println!();
            println!("If this replacement was not intended:");
            println!(
                "  1) Remove the new mount: {}",
                style(format!("occ mount remove {host_str}")).cyan()
            );
            if let Some(previous) = replaced_mounts.last() {
                println!(
                    "  2) Re-add a previous mount: {}",
                    style(format!("occ mount add {}", mount_to_spec(previous))).cyan()
                );
            } else {
                println!(
                    "  2) Re-add your previous mount with: {}",
                    style("occ mount add /host/path:/container/path[:ro]").cyan()
                );
            }
        }
    }

    println!();
    println!(
        "{}",
        style("Note: Run `occ restart` to apply mount changes. If mounts changed, you will be prompted to recreate the container.").dim()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_mount_by_target_exact_same_is_noop() {
        let existing = vec![
            "/host/a:/home/opencoder/workspace".to_string(),
            "/host/b:/home/opencoder/.cache/opencode".to_string(),
        ];
        let parsed = ParsedMount::parse("/host/a:/home/opencoder/workspace").unwrap();

        let (updated, outcome) =
            upsert_mount_by_target(&existing, "/host/a:/home/opencoder/workspace", &parsed);

        assert_eq!(updated, existing);
        assert_eq!(outcome, MountUpsertOutcome::AlreadyConfigured);
    }

    #[test]
    fn upsert_mount_by_target_replaces_same_target_with_new_host() {
        let existing = vec![
            "/host/old:/home/opencoder/workspace".to_string(),
            "/host/cache:/home/opencoder/.cache/opencode".to_string(),
        ];
        let parsed = ParsedMount::parse("/host/new:/home/opencoder/workspace").unwrap();

        let (updated, outcome) =
            upsert_mount_by_target(&existing, "/host/new:/home/opencoder/workspace", &parsed);

        assert_eq!(
            updated,
            vec![
                "/host/cache:/home/opencoder/.cache/opencode".to_string(),
                "/host/new:/home/opencoder/workspace".to_string(),
            ]
        );
        assert_eq!(
            outcome,
            MountUpsertOutcome::Replaced {
                replaced_mounts: vec![
                    ParsedMount::parse("/host/old:/home/opencoder/workspace").unwrap(),
                ],
            }
        );
    }

    #[test]
    fn upsert_mount_by_target_replaces_multiple_stale_targets() {
        let existing = vec![
            "/host/old1:/home/opencoder/workspace".to_string(),
            "/host/cache:/home/opencoder/.cache/opencode".to_string(),
            "/host/old2:/home/opencoder/workspace:ro".to_string(),
        ];
        let parsed = ParsedMount::parse("/host/new:/home/opencoder/workspace").unwrap();

        let (updated, outcome) =
            upsert_mount_by_target(&existing, "/host/new:/home/opencoder/workspace", &parsed);

        assert_eq!(
            updated,
            vec![
                "/host/cache:/home/opencoder/.cache/opencode".to_string(),
                "/host/new:/home/opencoder/workspace".to_string(),
            ]
        );
        assert_eq!(
            outcome,
            MountUpsertOutcome::Replaced {
                replaced_mounts: vec![
                    ParsedMount::parse("/host/old1:/home/opencoder/workspace").unwrap(),
                    ParsedMount::parse("/host/old2:/home/opencoder/workspace:ro").unwrap(),
                ],
            }
        );
    }
}
