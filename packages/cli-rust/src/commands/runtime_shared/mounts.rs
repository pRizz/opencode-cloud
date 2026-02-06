//! Shared mount parsing, normalization, and comparison helpers.

use anyhow::{Result, anyhow};
use console::style;
use opencode_cloud_core::Config;
use opencode_cloud_core::docker::{
    ContainerBindMount, ParsedMount, check_container_path_warning, validate_mount_path,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMountTargetResolution {
    pub container_path: String,
    pub kept_mount: ParsedMount,
    pub removed_mounts: Vec<ParsedMount>,
}

/// Collect and validate bind mounts from config and optional CLI flags.
///
/// If multiple mounts target the same container path, the last one wins.
pub fn collect_bind_mounts(
    config: &Config,
    cli_mounts: &[String],
    no_mounts: bool,
    quiet: bool,
) -> Result<Vec<ParsedMount>> {
    let mut all_mounts = Vec::new();

    if !no_mounts {
        for mount_str in &config.mounts {
            let parsed = ParsedMount::parse(mount_str)
                .map_err(|e| anyhow!("Invalid config mount '{mount_str}': {e}"))?;
            all_mounts.push(parsed);
        }
    }

    for mount_str in cli_mounts {
        let parsed = ParsedMount::parse(mount_str)
            .map_err(|e| anyhow!("Invalid mount '{mount_str}': {e}"))?;
        all_mounts.push(parsed);
    }

    let (all_mounts, duplicate_resolutions) = normalize_mount_targets(all_mounts);
    if !quiet {
        for resolution in duplicate_resolutions {
            let kept = format_mount_display(&resolution.kept_mount);
            eprintln!(
                "{}",
                style(format!(
                    "Warning: multiple mounts target '{}'; using last entry {} and ignoring {} earlier entr{}.",
                    resolution.container_path,
                    kept,
                    resolution.removed_mounts.len(),
                    if resolution.removed_mounts.len() == 1 {
                        "y"
                    } else {
                        "ies"
                    }
                ))
                .yellow()
            );
        }
    }

    for parsed in &all_mounts {
        if let Err(e) = validate_mount_path(&parsed.host_path) {
            return Err(anyhow!(
                "Mount path validation failed for '{}':\n  {}\n\nDid the directory move? Run: occ mount remove {}",
                parsed.host_path.display(),
                e,
                parsed.host_path.display()
            ));
        }

        if !quiet && let Some(warning) = check_container_path_warning(&parsed.container_path) {
            eprintln!("{}", style(&warning).yellow());
        }
    }

    Ok(all_mounts)
}

pub fn normalize_mount_targets(
    mounts: Vec<ParsedMount>,
) -> (Vec<ParsedMount>, Vec<DuplicateMountTargetResolution>) {
    let mut last_idx_by_target = HashMap::new();
    let mut target_order = Vec::new();

    for (idx, mount) in mounts.iter().enumerate() {
        if !last_idx_by_target.contains_key(&mount.container_path) {
            target_order.push(mount.container_path.clone());
        }
        last_idx_by_target.insert(mount.container_path.clone(), idx);
    }

    let mut normalized = Vec::new();
    let mut removed_by_target: HashMap<String, Vec<ParsedMount>> = HashMap::new();
    let mut kept_by_target: HashMap<String, ParsedMount> = HashMap::new();

    for (idx, mount) in mounts.into_iter().enumerate() {
        if last_idx_by_target.get(&mount.container_path) == Some(&idx) {
            kept_by_target.insert(mount.container_path.clone(), mount.clone());
            normalized.push(mount);
        } else {
            removed_by_target
                .entry(mount.container_path.clone())
                .or_default()
                .push(mount);
        }
    }

    let mut duplicate_resolutions = Vec::new();
    for target in target_order {
        let Some(removed_mounts) = removed_by_target.remove(&target) else {
            continue;
        };
        if removed_mounts.is_empty() {
            continue;
        }
        let Some(kept_mount) = kept_by_target.get(&target) else {
            continue;
        };
        duplicate_resolutions.push(DuplicateMountTargetResolution {
            container_path: target,
            kept_mount: kept_mount.clone(),
            removed_mounts,
        });
    }

    (normalized, duplicate_resolutions)
}

/// Check if two host paths match, accounting for macOS path translation.
///
/// Docker on macOS translates paths: /tmp -> /private/tmp -> /host_mnt/private/tmp
pub fn host_paths_match(container_path: &str, configured_path: &str) -> bool {
    if container_path == configured_path {
        return true;
    }

    if let Some(stripped) = container_path.strip_prefix("/host_mnt") {
        if stripped == configured_path {
            return true;
        }
        if let Some(private_stripped) = stripped.strip_prefix("/private")
            && private_stripped == configured_path
        {
            return true;
        }
    }

    if let Some(private_path) = configured_path.strip_prefix("/private")
        && container_path.ends_with(private_path)
    {
        return true;
    }

    false
}

pub fn mount_has_match(conf: &ParsedMount, current: &[ContainerBindMount]) -> bool {
    let conf_host = conf.host_path.to_string_lossy();

    current.iter().any(|cur| {
        cur.target == conf.container_path
            && cur.read_only == conf.read_only
            && host_paths_match(&cur.source, &conf_host)
    })
}

/// Compare container bind mounts with configured mounts (ignoring order).
pub fn mounts_equal(current: &[ContainerBindMount], configured: &[ParsedMount]) -> bool {
    current.len() == configured.len()
        && configured.iter().all(|conf| mount_has_match(conf, current))
}

fn format_mount_display(mount: &ParsedMount) -> String {
    let mode = if mount.read_only { "ro" } else { "rw" };
    format!(
        "{} -> {} ({mode})",
        mount.host_path.display(),
        mount.container_path
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_container_mount(source: &str, target: &str, read_only: bool) -> ContainerBindMount {
        ContainerBindMount {
            source: source.to_string(),
            target: target.to_string(),
            read_only,
        }
    }

    fn make_parsed_mount(host: &str, container: &str, read_only: bool) -> ParsedMount {
        ParsedMount {
            host_path: PathBuf::from(host),
            container_path: container.to_string(),
            read_only,
        }
    }

    #[test]
    fn host_paths_match_direct_match() {
        assert!(host_paths_match("/tmp", "/tmp"));
        assert!(host_paths_match("/home/user/data", "/home/user/data"));
    }

    #[test]
    fn host_paths_match_no_match() {
        assert!(!host_paths_match("/tmp", "/var"));
        assert!(!host_paths_match("/foo", "/bar"));
    }

    #[test]
    fn host_paths_match_host_mnt_prefix() {
        assert!(host_paths_match("/host_mnt/tmp", "/tmp"));
        assert!(host_paths_match("/host_mnt/home/user", "/home/user"));
    }

    #[test]
    fn host_paths_match_host_mnt_private_prefix() {
        assert!(host_paths_match("/host_mnt/private/tmp", "/tmp"));
        assert!(host_paths_match("/host_mnt/private/var", "/var"));
    }

    #[test]
    fn host_paths_match_private_prefix_in_config() {
        assert!(host_paths_match("/host_mnt/private/tmp", "/private/tmp"));
    }

    #[test]
    fn host_paths_match_no_false_positives() {
        assert!(!host_paths_match("/host_mnt/tmpdir", "/tmp"));
        assert!(!host_paths_match("/tmp2", "/tmp"));
    }

    #[test]
    fn mounts_equal_empty_lists() {
        let current: Vec<ContainerBindMount> = vec![];
        let configured: Vec<ParsedMount> = vec![];
        assert!(mounts_equal(&current, &configured));
    }

    #[test]
    fn mounts_equal_single_match() {
        let current = vec![make_container_mount("/tmp", "/mnt/tmp", false)];
        let configured = vec![make_parsed_mount("/tmp", "/mnt/tmp", false)];
        assert!(mounts_equal(&current, &configured));
    }

    #[test]
    fn mounts_equal_multiple_match() {
        let current = vec![
            make_container_mount("/host_mnt/private/tmp", "/mnt/tmp", false),
            make_container_mount("/home/user", "/mnt/home", true),
        ];
        let configured = vec![
            make_parsed_mount("/tmp", "/mnt/tmp", false),
            make_parsed_mount("/home/user", "/mnt/home", true),
        ];
        assert!(mounts_equal(&current, &configured));
    }

    #[test]
    fn mounts_equal_different_order() {
        let current = vec![
            make_container_mount("/home/user", "/mnt/home", true),
            make_container_mount("/tmp", "/mnt/tmp", false),
        ];
        let configured = vec![
            make_parsed_mount("/tmp", "/mnt/tmp", false),
            make_parsed_mount("/home/user", "/mnt/home", true),
        ];
        assert!(mounts_equal(&current, &configured));
    }

    #[test]
    fn mounts_equal_length_mismatch() {
        let current = vec![make_container_mount("/tmp", "/mnt/tmp", false)];
        let configured = vec![
            make_parsed_mount("/tmp", "/mnt/tmp", false),
            make_parsed_mount("/var", "/mnt/var", false),
        ];
        assert!(!mounts_equal(&current, &configured));
    }

    #[test]
    fn mounts_equal_content_mismatch() {
        let current = vec![make_container_mount("/tmp", "/mnt/tmp", false)];
        let configured = vec![make_parsed_mount("/var", "/mnt/var", false)];
        assert!(!mounts_equal(&current, &configured));
    }

    #[test]
    fn normalize_mount_targets_keeps_last_entry_per_target() {
        let mounts = vec![
            make_parsed_mount("/a", "/workspace", false),
            make_parsed_mount("/b", "/data", false),
            make_parsed_mount("/c", "/workspace", true),
        ];

        let (normalized, resolutions) = normalize_mount_targets(mounts);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0], make_parsed_mount("/b", "/data", false));
        assert_eq!(normalized[1], make_parsed_mount("/c", "/workspace", true));

        assert_eq!(resolutions.len(), 1);
        assert_eq!(resolutions[0].container_path, "/workspace");
        assert_eq!(
            resolutions[0].kept_mount,
            make_parsed_mount("/c", "/workspace", true)
        );
        assert_eq!(resolutions[0].removed_mounts.len(), 1);
        assert_eq!(
            resolutions[0].removed_mounts[0],
            make_parsed_mount("/a", "/workspace", false)
        );
    }
}
