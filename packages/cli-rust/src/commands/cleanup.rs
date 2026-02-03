//! Filesystem cleanup helpers for destructive commands.

use anyhow::{Context, Result, bail};
use opencode_cloud_core::config::{Config, get_config_path, load_config_or_default};
use opencode_cloud_core::docker::ParsedMount;
use opencode_cloud_core::load_hosts;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub struct MountCollection {
    pub mounts: Vec<ParsedMount>,
    pub skipped: Vec<String>,
}

pub struct MountCleanupResult {
    pub cleaned: Vec<PathBuf>,
    pub purged: Vec<PathBuf>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

impl MountCleanupResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

pub fn resolve_target_host_name(maybe_host: Option<&str>) -> Option<String> {
    if let Some(name) = maybe_host {
        return Some(name.to_string());
    }

    let hosts = load_hosts().unwrap_or_default();
    hosts.default_host.clone()
}

pub fn is_remote_host(maybe_host: Option<&str>) -> bool {
    matches!(
        resolve_target_host_name(maybe_host),
        Some(name) if !name.is_empty() && name != "local"
    )
}

pub fn load_config_for_mounts(include_defaults_if_missing: bool) -> Result<(Config, bool)> {
    let config_path =
        get_config_path().ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;
    if config_path.exists() {
        Ok((load_config_or_default()?, true))
    } else {
        let mut config = Config::default();
        if !include_defaults_if_missing {
            config.mounts = Vec::new();
        }
        Ok((config, false))
    }
}

pub fn collect_config_mounts(config: &Config) -> MountCollection {
    let mut mounts = Vec::new();
    let mut skipped = Vec::new();

    for mount_str in &config.mounts {
        match ParsedMount::parse(mount_str) {
            Ok(parsed) => mounts.push(parsed),
            Err(_) => skipped.push(mount_str.clone()),
        }
    }

    MountCollection { mounts, skipped }
}

pub fn cleanup_mounts(mounts: &[ParsedMount], purge: bool) -> MountCleanupResult {
    let mut result = MountCleanupResult {
        cleaned: Vec::new(),
        purged: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    for mount in mounts {
        let host_path = mount.host_path.as_path();
        let host_display = host_path.display().to_string();
        match cleanup_single_mount(host_path, purge) {
            Ok(CleanupOutcome::Skipped(reason)) => {
                result.skipped.push(format!("{host_display}: {reason}"));
            }
            Ok(CleanupOutcome::Cleaned(path)) => {
                result.cleaned.push(path);
            }
            Ok(CleanupOutcome::Purged(path)) => {
                result.purged.push(path);
            }
            Err(error) => {
                result.errors.push(format!("{host_display}: {error}"));
            }
        }
    }

    result
}

pub fn remove_mounts_from_config(config: &mut Config, hosts: &[String]) -> usize {
    if hosts.is_empty() {
        return 0;
    }

    let mut removed = 0;
    config.mounts.retain(|mount_str| {
        let parsed = match ParsedMount::parse(mount_str) {
            Ok(parsed) => parsed,
            Err(_) => return true,
        };

        let host_str = parsed.host_path.to_string_lossy().to_string();
        if hosts.iter().any(|host| host == &host_str) {
            removed += 1;
            false
        } else {
            true
        }
    });

    removed
}

enum CleanupOutcome {
    Cleaned(PathBuf),
    Purged(PathBuf),
    Skipped(String),
}

fn cleanup_single_mount(path: &Path, purge: bool) -> Result<CleanupOutcome> {
    if !path.is_absolute() {
        bail!("Mount path is not absolute");
    }

    if !path.exists() {
        if purge {
            return Ok(CleanupOutcome::Skipped("path does not exist".to_string()));
        }
        ensure_dir_exists(path)?;
    }

    let canonical = fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve path for cleanup: {}", path.display()))?;

    validate_safe_path(&canonical)?;

    if purge {
        purge_dir(&canonical)?;
        remove_symlink_if_needed(path, &canonical)?;
        return Ok(CleanupOutcome::Purged(canonical));
    }

    ensure_dir_exists(&canonical)?;
    clean_dir_contents(&canonical)?;
    Ok(CleanupOutcome::Cleaned(canonical))
}

fn ensure_dir_exists(path: &Path) -> Result<()> {
    if path.exists() {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read mount path metadata: {}", path.display()))?;
        if !metadata.is_dir() {
            bail!("Mount path is not a directory");
        }
        return Ok(());
    }

    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create mount directory: {}", path.display()))?;
    Ok(())
}

fn remove_symlink_if_needed(original: &Path, canonical: &Path) -> Result<()> {
    if original == canonical {
        return Ok(());
    }

    let metadata = match fs::symlink_metadata(original) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };

    if metadata.file_type().is_symlink() {
        fs::remove_file(original)
            .with_context(|| format!("Failed to remove symlink: {}", original.display()))?;
    }

    Ok(())
}

pub(crate) fn clean_dir_contents(path: &Path) -> Result<()> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)
            .with_context(|| format!("Failed to read entry metadata: {}", entry_path.display()))?;

        if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&entry_path)
                .with_context(|| format!("Failed to remove directory: {}", entry_path.display()))?;
        } else {
            fs::remove_file(&entry_path)
                .with_context(|| format!("Failed to remove file: {}", entry_path.display()))?;
        }
    }

    Ok(())
}

pub(crate) fn purge_dir(path: &Path) -> Result<()> {
    fs::remove_dir_all(path)
        .with_context(|| format!("Failed to remove directory: {}", path.display()))?;
    Ok(())
}

pub(crate) fn validate_safe_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("Path is not absolute");
    }

    if is_root_path(path) {
        bail!("Refusing to operate on filesystem root");
    }

    if let Some(home) = dirs::home_dir() {
        let home_canonical = home.canonicalize().unwrap_or(home);
        if path == home_canonical {
            bail!("Refusing to operate on home directory");
        }
    }

    Ok(())
}

fn is_root_path(path: &Path) -> bool {
    let mut has_normal = false;
    for component in path.components() {
        if matches!(component, Component::Normal(_)) {
            has_normal = true;
            break;
        }
    }

    !has_normal
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn clean_dir_contents_removes_children() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("file.txt");
        let nested_dir = dir.path().join("nested");
        let nested_file = nested_dir.join("nested.txt");

        fs::write(&file_path, "data").expect("write file");
        fs::create_dir_all(&nested_dir).expect("create dir");
        fs::write(&nested_file, "data").expect("write nested file");

        clean_dir_contents(dir.path()).expect("clean");

        let entries: Vec<_> = fs::read_dir(dir.path()).expect("read dir").collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn purge_dir_removes_directory() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("purge");
        fs::create_dir_all(&target).expect("create dir");
        fs::write(target.join("file.txt"), "data").expect("write file");

        purge_dir(&target).expect("purge");
        assert!(!target.exists());
    }

    #[test]
    fn validate_safe_path_rejects_root() {
        #[cfg(target_family = "unix")]
        {
            assert!(validate_safe_path(Path::new("/")).is_err());
        }
    }

    #[test]
    fn validate_safe_path_rejects_home() {
        if let Some(home) = dirs::home_dir() {
            let canonical = home.canonicalize().unwrap_or(home);
            assert!(validate_safe_path(&canonical).is_err());
        }
    }

    #[test]
    fn collect_config_mounts_skips_invalid() {
        let dir = tempdir().expect("tempdir");
        let mut config = Config::default();
        let mount = format!("{}:/data", dir.path().display());
        config.mounts = vec![mount.clone(), "invalid".to_string()];

        let collection = collect_config_mounts(&config);
        assert_eq!(collection.mounts.len(), 1);
        assert_eq!(collection.skipped.len(), 1);
    }
}
