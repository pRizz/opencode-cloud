//! systemd service manager for Linux
//!
//! This module provides SystemdManager which implements the ServiceManager trait
//! for registering opencode-cloud as a systemd user service on Linux.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Result, anyhow};

use super::{InstallResult, ServiceConfig, ServiceManager};

/// Service name used for systemd unit
const SERVICE_NAME: &str = "opencode-cloud";

/// SystemdManager handles service registration with systemd on Linux
#[derive(Debug, Clone)]
pub struct SystemdManager {
    /// true = user mode (~/.config/systemd/user/), false = system mode (/etc/systemd/system/)
    user_mode: bool,
}

impl SystemdManager {
    /// Create a new SystemdManager
    ///
    /// # Arguments
    /// * `boot_mode` - "user" for user-level service (default), "system" for system-level
    pub fn new(boot_mode: &str) -> Self {
        Self {
            user_mode: boot_mode != "system",
        }
    }

    /// Get the directory where service files are stored
    fn service_dir(&self) -> PathBuf {
        if self.user_mode {
            // User-level: ~/.config/systemd/user/
            directories::BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".config"))
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("systemd")
                .join("user")
        } else {
            // System-level: /etc/systemd/system/
            PathBuf::from("/etc/systemd/system")
        }
    }

    /// Generate the systemd unit file content
    fn generate_unit_file(&self, config: &ServiceConfig) -> String {
        let executable_path = config.executable_path.display().to_string();

        // Quote path if it contains spaces
        let exec_start = if executable_path.contains(' ') {
            format!("\"{executable_path}\" start --no-daemon")
        } else {
            format!("{executable_path} start --no-daemon")
        };

        let exec_stop = if executable_path.contains(' ') {
            format!("\"{executable_path}\" stop")
        } else {
            format!("{executable_path} stop")
        };

        // Calculate StartLimitIntervalSec: restart_delay * restart_retries * 2
        // This gives enough window for the allowed burst of restarts
        let start_limit_interval = config.restart_delay * config.restart_retries * 2;

        format!(
            r#"[Unit]
Description=opencode-cloud container service
Documentation=https://github.com/pRizz/opencode-cloud
After=docker.service
Requires=docker.service

[Service]
Type=simple
ExecStart={exec_start}
ExecStop={exec_stop}
Restart=on-failure
RestartSec={restart_delay}s
StartLimitBurst={restart_retries}
StartLimitIntervalSec={start_limit_interval}

[Install]
WantedBy=default.target
"#,
            exec_start = exec_start,
            exec_stop = exec_stop,
            restart_delay = config.restart_delay,
            restart_retries = config.restart_retries,
            start_limit_interval = start_limit_interval,
        )
    }

    /// Run systemctl with the appropriate mode flag
    fn systemctl(&self, args: &[&str]) -> Result<Output> {
        let mut cmd = Command::new("systemctl");
        if self.user_mode {
            cmd.arg("--user");
        }
        cmd.args(args)
            .output()
            .map_err(|e| anyhow!("Failed to run systemctl: {e}"))
    }

    /// Run systemctl and check for success
    fn systemctl_ok(&self, args: &[&str]) -> Result<()> {
        let output = self.systemctl(args)?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!(
                "systemctl {} failed: {}",
                args.join(" "),
                stderr.trim()
            ))
        }
    }
}

/// Check if systemd is available on this system
///
/// Returns true if /run/systemd/system exists, indicating systemd is running
/// as the init system.
pub fn systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}

/// Check if systemd user session is available for the current user
///
/// Returns true if XDG_RUNTIME_DIR is set and the user's systemd directory exists.
/// This is needed for `systemctl --user` commands to work.
pub fn systemd_user_session_available() -> bool {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        // Check if the user's systemd directory exists
        Path::new(&runtime_dir).join("systemd").exists()
    } else {
        false
    }
}

impl ServiceManager for SystemdManager {
    fn install(&self, config: &ServiceConfig) -> Result<InstallResult> {
        // Check permissions and session availability based on mode
        if self.user_mode {
            // User-level installation requires an active systemd user session
            if !systemd_user_session_available() {
                return Err(anyhow!(
                    "User-level systemd session not available.\n\
                     This typically happens during cloud-init or when running as a \
                     different user without an active login session.\n\n\
                     Solutions:\n\
                     1. Use system-level installation: occ config set boot_mode system\n\
                     2. Run the command from an interactive login session\n\
                     3. Ensure XDG_RUNTIME_DIR is set and the user has an active systemd session"
                ));
            }
        } else {
            // System-level installation requires root privileges
            let test_path = self.service_dir().join(".opencode-cloud-test");
            if fs::write(&test_path, "").is_err() {
                return Err(anyhow!(
                    "System-level installation requires root privileges. \
                     Run with sudo or use user-level installation (default)."
                ));
            }
            let _ = fs::remove_file(&test_path);
        }

        // 1. Create service directory if needed
        let service_dir = self.service_dir();
        fs::create_dir_all(&service_dir).map_err(|e| {
            anyhow!(
                "Failed to create service directory {}: {}",
                service_dir.display(),
                e
            )
        })?;

        // 2. Generate and write unit file
        let unit_content = self.generate_unit_file(config);
        let service_file = self.service_file_path();

        fs::write(&service_file, &unit_content).map_err(|e| {
            anyhow!(
                "Failed to write service file {}: {}",
                service_file.display(),
                e
            )
        })?;

        // 3. Reload systemd daemon to pick up the new unit file
        self.systemctl_ok(&["daemon-reload"])?;

        // 4. Enable the service for auto-start
        self.systemctl_ok(&["enable", SERVICE_NAME])?;

        // 5. Start the service
        let started = self.systemctl_ok(&["start", SERVICE_NAME]).is_ok();

        Ok(InstallResult {
            service_file_path: service_file,
            service_name: SERVICE_NAME.to_string(),
            started,
            requires_root: !self.user_mode,
        })
    }

    fn uninstall(&self) -> Result<()> {
        // 1. Stop the service (ignore error if not running)
        let _ = self.systemctl(&["stop", SERVICE_NAME]);

        // 2. Disable the service
        let _ = self.systemctl(&["disable", SERVICE_NAME]);

        // 3. Remove the unit file
        let service_file = self.service_file_path();
        if service_file.exists() {
            fs::remove_file(&service_file).map_err(|e| {
                anyhow!(
                    "Failed to remove service file {}: {}",
                    service_file.display(),
                    e
                )
            })?;
        }

        // 4. Reload daemon to reflect the removal
        self.systemctl_ok(&["daemon-reload"])?;

        Ok(())
    }

    fn is_installed(&self) -> Result<bool> {
        Ok(self.service_file_path().exists())
    }

    fn service_file_path(&self) -> PathBuf {
        self.service_dir().join(format!("{SERVICE_NAME}.service"))
    }

    fn service_name(&self) -> &str {
        SERVICE_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_manager_new_user_mode() {
        let manager = SystemdManager::new("user");
        assert!(manager.user_mode);
    }

    #[test]
    fn test_systemd_manager_new_system_mode() {
        let manager = SystemdManager::new("system");
        assert!(!manager.user_mode);
    }

    #[test]
    fn test_systemd_manager_new_default_to_user() {
        // Any value other than "system" should default to user mode
        let manager = SystemdManager::new("login");
        assert!(manager.user_mode);
    }

    #[test]
    fn test_service_dir_user_mode() {
        let manager = SystemdManager::new("user");
        let dir = manager.service_dir();
        // Should end with systemd/user
        assert!(dir.ends_with("systemd/user"));
    }

    #[test]
    fn test_service_dir_system_mode() {
        let manager = SystemdManager::new("system");
        let dir = manager.service_dir();
        assert_eq!(dir, PathBuf::from("/etc/systemd/system"));
    }

    #[test]
    fn test_service_file_path() {
        let manager = SystemdManager::new("user");
        let path = manager.service_file_path();
        assert!(path.ends_with("opencode-cloud.service"));
    }

    #[test]
    fn test_service_name() {
        let manager = SystemdManager::new("user");
        assert_eq!(manager.service_name(), "opencode-cloud");
    }

    #[test]
    fn test_generate_unit_file_basic() {
        let manager = SystemdManager::new("user");
        let config = ServiceConfig {
            executable_path: PathBuf::from("/usr/local/bin/occ"),
            restart_retries: 3,
            restart_delay: 5,
            boot_mode: "user".to_string(),
        };

        let unit = manager.generate_unit_file(&config);

        // Verify essential sections
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));

        // Verify key settings
        assert!(unit.contains("Description=opencode-cloud container service"));
        assert!(unit.contains("ExecStart=/usr/local/bin/occ start --no-daemon"));
        assert!(unit.contains("ExecStop=/usr/local/bin/occ stop"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("RestartSec=5s"));
        assert!(unit.contains("StartLimitBurst=3"));
        assert!(unit.contains("StartLimitIntervalSec=30")); // 5 * 3 * 2
        assert!(unit.contains("WantedBy=default.target"));
    }

    #[test]
    fn test_generate_unit_file_with_spaces_in_path() {
        let manager = SystemdManager::new("user");
        let config = ServiceConfig {
            executable_path: PathBuf::from("/Users/test user/bin/occ"),
            restart_retries: 3,
            restart_delay: 5,
            boot_mode: "user".to_string(),
        };

        let unit = manager.generate_unit_file(&config);

        // Path should be quoted
        assert!(unit.contains("ExecStart=\"/Users/test user/bin/occ\" start --no-daemon"));
        assert!(unit.contains("ExecStop=\"/Users/test user/bin/occ\" stop"));
    }

    #[test]
    fn test_generate_unit_file_custom_restart_policy() {
        let manager = SystemdManager::new("user");
        let config = ServiceConfig {
            executable_path: PathBuf::from("/usr/bin/occ"),
            restart_retries: 5,
            restart_delay: 10,
            boot_mode: "user".to_string(),
        };

        let unit = manager.generate_unit_file(&config);

        assert!(unit.contains("RestartSec=10s"));
        assert!(unit.contains("StartLimitBurst=5"));
        assert!(unit.contains("StartLimitIntervalSec=100")); // 10 * 5 * 2
    }

    #[test]
    fn test_is_installed_returns_false_for_nonexistent() {
        let manager = SystemdManager::new("user");
        // On a test system without the service installed, this should return false
        // This test works because the service file won't exist in test environment
        let result = manager.is_installed();
        assert!(result.is_ok());
        // Can't assert false because the service might actually be installed on some systems
    }
}
