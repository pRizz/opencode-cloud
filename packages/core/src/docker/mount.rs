//! Bind mount parsing and validation for container configuration.
//!
//! This module provides functionality to:
//! - Parse mount strings in Docker format (`/host:/container[:ro|rw]`)
//! - Validate mount paths (existence, type, permissions)
//! - Convert parsed mounts to Bollard's Mount type for Docker API
//! - Warn about potentially dangerous container mount points

use bollard::service::{Mount, MountTypeEnum};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during mount parsing and validation.
#[derive(Debug, Error)]
pub enum MountError {
    /// Mount path is relative, but must be absolute.
    #[error("Mount paths must be absolute. Use: /full/path/to/dir (got: {0})")]
    RelativePath(String),

    /// Mount string format is invalid.
    #[error("Invalid mount format. Expected: /host/path:/container/path[:ro] (got: {0})")]
    InvalidFormat(String),

    /// Path does not exist or cannot be accessed.
    #[error("Path not found: {0} ({1})")]
    PathNotFound(String, String),

    /// Path exists but is not a directory.
    #[error("Path is not a directory: {0}")]
    NotADirectory(String),

    /// Permission denied accessing path.
    #[error("Cannot access path (permission denied): {0}")]
    PermissionDenied(String),
}

/// A parsed bind mount specification.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMount {
    /// Host path to mount (absolute).
    pub host_path: PathBuf,

    /// Container path where the host path is mounted.
    pub container_path: String,

    /// Whether the mount is read-only.
    pub read_only: bool,
}

impl ParsedMount {
    /// Parse a mount string in Docker format.
    ///
    /// Format: `/host/path:/container/path[:ro|rw]`
    ///
    /// # Arguments
    /// * `mount_str` - The mount specification string.
    ///
    /// # Returns
    /// * `Ok(ParsedMount)` - Successfully parsed mount.
    /// * `Err(MountError)` - Parse error.
    ///
    /// # Examples
    /// ```
    /// use opencode_cloud_core::docker::ParsedMount;
    ///
    /// // Read-write mount (default)
    /// let mount = ParsedMount::parse("/home/user/data:/home/opencoder/workspace/data").unwrap();
    /// assert_eq!(mount.host_path.to_str().unwrap(), "/home/user/data");
    /// assert_eq!(mount.container_path, "/home/opencoder/workspace/data");
    /// assert!(!mount.read_only);
    ///
    /// // Read-only mount
    /// let mount = ParsedMount::parse("/home/user/config:/etc/app:ro").unwrap();
    /// assert!(mount.read_only);
    /// ```
    pub fn parse(mount_str: &str) -> Result<Self, MountError> {
        let parts: Vec<&str> = mount_str.split(':').collect();

        match parts.len() {
            2 => {
                // /host:/container (default rw)
                let host_path = PathBuf::from(parts[0]);
                if !host_path.is_absolute() {
                    return Err(MountError::RelativePath(parts[0].to_string()));
                }
                Ok(Self {
                    host_path,
                    container_path: parts[1].to_string(),
                    read_only: false,
                })
            }
            3 => {
                // /host:/container:ro or /host:/container:rw
                let host_path = PathBuf::from(parts[0]);
                if !host_path.is_absolute() {
                    return Err(MountError::RelativePath(parts[0].to_string()));
                }
                let read_only = match parts[2].to_lowercase().as_str() {
                    "ro" => true,
                    "rw" => false,
                    _ => return Err(MountError::InvalidFormat(mount_str.to_string())),
                };
                Ok(Self {
                    host_path,
                    container_path: parts[1].to_string(),
                    read_only,
                })
            }
            _ => Err(MountError::InvalidFormat(mount_str.to_string())),
        }
    }

    /// Convert to a Bollard Mount for the Docker API.
    ///
    /// Returns a bind mount with the parsed host and container paths.
    pub fn to_bollard_mount(&self) -> Mount {
        Mount {
            target: Some(self.container_path.clone()),
            source: Some(self.host_path.to_string_lossy().to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(self.read_only),
            ..Default::default()
        }
    }
}

/// Validate that a mount host path exists and is accessible.
///
/// Checks:
/// 1. Path is absolute.
/// 2. Path exists (via canonicalize, which also resolves symlinks).
/// 3. Path is a directory.
///
/// # Arguments
/// * `path` - The path to validate.
///
/// # Returns
/// * `Ok(PathBuf)` - The canonical (resolved) path.
/// * `Err(MountError)` - Validation error.
pub fn validate_mount_path(path: &std::path::Path) -> Result<PathBuf, MountError> {
    // Check absolute
    if !path.is_absolute() {
        return Err(MountError::RelativePath(path.display().to_string()));
    }

    // Canonicalize (resolves symlinks, checks existence)
    let canonical = std::fs::canonicalize(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            MountError::PermissionDenied(path.display().to_string())
        } else {
            MountError::PathNotFound(path.display().to_string(), e.to_string())
        }
    })?;

    // Check it's a directory
    let metadata = std::fs::metadata(&canonical).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            MountError::PermissionDenied(path.display().to_string())
        } else {
            MountError::PathNotFound(path.display().to_string(), e.to_string())
        }
    })?;

    if !metadata.is_dir() {
        return Err(MountError::NotADirectory(path.display().to_string()));
    }

    Ok(canonical)
}

/// System paths that should typically not be mounted over.
const SYSTEM_PATHS: &[&str] = &["/etc", "/usr", "/bin", "/sbin", "/lib", "/var"];

/// Check if mounting to a container path might be dangerous.
///
/// Returns a warning message if the container path is a system path,
/// or `None` if the path appears safe.
///
/// # Arguments
/// * `container_path` - The path inside the container.
///
/// # Returns
/// * `Some(String)` - Warning message about the system path.
/// * `None` - Path appears safe.
pub fn check_container_path_warning(container_path: &str) -> Option<String> {
    for system_path in SYSTEM_PATHS {
        if container_path == *system_path || container_path.starts_with(&format!("{system_path}/"))
        {
            return Some(format!(
                "Warning: mounting to '{container_path}' may affect container system files"
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_mount_rw() {
        let mount = ParsedMount::parse("/a:/b").unwrap();
        assert_eq!(mount.host_path, PathBuf::from("/a"));
        assert_eq!(mount.container_path, "/b");
        assert!(!mount.read_only);
    }

    #[test]
    fn parse_valid_mount_ro() {
        let mount = ParsedMount::parse("/a:/b:ro").unwrap();
        assert_eq!(mount.host_path, PathBuf::from("/a"));
        assert_eq!(mount.container_path, "/b");
        assert!(mount.read_only);
    }

    #[test]
    fn parse_valid_mount_explicit_rw() {
        let mount = ParsedMount::parse("/a:/b:rw").unwrap();
        assert_eq!(mount.host_path, PathBuf::from("/a"));
        assert_eq!(mount.container_path, "/b");
        assert!(!mount.read_only);
    }

    #[test]
    fn parse_valid_mount_ro_uppercase() {
        let mount = ParsedMount::parse("/a:/b:RO").unwrap();
        assert!(mount.read_only);
    }

    #[test]
    fn parse_invalid_format_single_part() {
        let result = ParsedMount::parse("invalid");
        assert!(matches!(result, Err(MountError::InvalidFormat(_))));
    }

    #[test]
    fn parse_invalid_format_too_many_parts() {
        let result = ParsedMount::parse("/a:/b:ro:extra");
        assert!(matches!(result, Err(MountError::InvalidFormat(_))));
    }

    #[test]
    fn parse_invalid_format_bad_mode() {
        let result = ParsedMount::parse("/a:/b:invalid");
        assert!(matches!(result, Err(MountError::InvalidFormat(_))));
    }

    #[test]
    fn parse_relative_path_rejected() {
        let result = ParsedMount::parse("./rel:/b");
        assert!(matches!(result, Err(MountError::RelativePath(_))));
    }

    #[test]
    fn parse_relative_path_no_dot_rejected() {
        let result = ParsedMount::parse("relative/path:/b");
        assert!(matches!(result, Err(MountError::RelativePath(_))));
    }

    #[test]
    fn system_path_warning_etc() {
        let warning = check_container_path_warning("/etc");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("/etc"));
    }

    #[test]
    fn system_path_warning_etc_subdir() {
        let warning = check_container_path_warning("/etc/passwd");
        assert!(warning.is_some());
    }

    #[test]
    fn system_path_warning_usr() {
        let warning = check_container_path_warning("/usr");
        assert!(warning.is_some());
    }

    #[test]
    fn system_path_warning_usr_local() {
        let warning = check_container_path_warning("/usr/local");
        assert!(warning.is_some());
    }

    #[test]
    fn non_system_path_no_warning() {
        let warning = check_container_path_warning("/home/opencoder/workspace/data");
        assert!(warning.is_none());
    }

    #[test]
    fn non_system_path_home_no_warning() {
        let warning = check_container_path_warning("/home/user/data");
        assert!(warning.is_none());
    }

    #[test]
    fn to_bollard_mount_structure() {
        let mount = ParsedMount {
            host_path: PathBuf::from("/host/path"),
            container_path: "/container/path".to_string(),
            read_only: true,
        };
        let bollard_mount = mount.to_bollard_mount();
        assert_eq!(bollard_mount.target, Some("/container/path".to_string()));
        assert_eq!(bollard_mount.source, Some("/host/path".to_string()));
        assert_eq!(bollard_mount.typ, Some(MountTypeEnum::BIND));
        assert_eq!(bollard_mount.read_only, Some(true));
    }

    #[test]
    fn validate_mount_path_relative_rejected() {
        let result = validate_mount_path(std::path::Path::new("./relative"));
        assert!(matches!(result, Err(MountError::RelativePath(_))));
    }

    #[test]
    fn validate_mount_path_nonexistent() {
        let result = validate_mount_path(std::path::Path::new("/nonexistent/path/xyz123"));
        assert!(matches!(result, Err(MountError::PathNotFound(_, _))));
    }

    #[test]
    fn validate_mount_path_existing_directory() {
        // Use /tmp which should exist on any Unix system
        let result = validate_mount_path(std::path::Path::new("/tmp"));
        assert!(result.is_ok());
    }
}
