//! macOS launchd service manager implementation
//!
//! Manages launchd user agents and system daemons for service registration.
//! Uses plist serialization for property list generation and launchctl for
//! service lifecycle management.

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, anyhow};
use serde::Serialize;

use super::{InstallResult, ServiceConfig, ServiceManager};

/// Service label used for launchd registration
const SERVICE_LABEL: &str = "com.opencode-cloud.service";

/// Plist structure for launchd service definition
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LaunchdPlist {
    label: String,
    program_arguments: Vec<String>,
    run_at_load: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<KeepAliveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    throttle_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    standard_out_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    standard_error_path: Option<String>,
}

/// KeepAlive configuration for restart behavior
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct KeepAliveConfig {
    /// Restart only on non-zero exit (false = restart on crash)
    #[serde(skip_serializing_if = "Option::is_none")]
    successful_exit: Option<bool>,
    /// Restart on signal-based crash (SIGSEGV, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    crashed: Option<bool>,
}

/// macOS launchd service manager
///
/// Manages launchd user agents (~/Library/LaunchAgents/) or system daemons
/// (/Library/LaunchDaemons/) depending on the boot mode.
pub struct LaunchdManager {
    /// true = user mode (~/Library/LaunchAgents/), false = system (/Library/LaunchDaemons/)
    user_mode: bool,
}

impl LaunchdManager {
    /// Create a new LaunchdManager
    ///
    /// # Arguments
    /// * `boot_mode` - "user" for user agents, "system" for system daemons
    pub fn new(boot_mode: &str) -> Self {
        Self {
            user_mode: boot_mode != "system",
        }
    }

    /// Get the directory where service files are stored
    fn service_dir(&self) -> PathBuf {
        if self.user_mode {
            directories::BaseDirs::new()
                .expect("Could not determine base directories")
                .home_dir()
                .join("Library/LaunchAgents")
        } else {
            PathBuf::from("/Library/LaunchDaemons")
        }
    }

    /// Get the service label
    fn label(&self) -> &str {
        SERVICE_LABEL
    }

    /// Get the log path for a given stream (stdout or stderr)
    fn log_path(&self, stream: &str) -> PathBuf {
        directories::BaseDirs::new()
            .expect("Could not determine base directories")
            .home_dir()
            .join(format!("Library/Logs/opencode-cloud.{stream}.log"))
    }

    /// Generate plist content from service configuration
    fn generate_plist(&self, config: &ServiceConfig) -> LaunchdPlist {
        LaunchdPlist {
            label: self.label().to_string(),
            program_arguments: vec![
                config.executable_path.display().to_string(),
                "start".to_string(),
                "--no-daemon".to_string(),
            ],
            run_at_load: true,
            keep_alive: Some(KeepAliveConfig {
                // Only restart on non-zero exit (crash)
                successful_exit: Some(false),
                // Restart on signal-based crash
                crashed: Some(true),
            }),
            throttle_interval: Some(config.restart_delay),
            standard_out_path: Some(self.log_path("stdout").display().to_string()),
            standard_error_path: Some(self.log_path("stderr").display().to_string()),
        }
    }
}

/// Get the current user's UID
fn get_user_id() -> Result<u32> {
    let output = Command::new("id").arg("-u").output()?;
    let uid_str = String::from_utf8_lossy(&output.stdout);
    uid_str
        .trim()
        .parse()
        .map_err(|e| anyhow!("Failed to parse UID: {}", e))
}

impl ServiceManager for LaunchdManager {
    fn install(&self, config: &ServiceConfig) -> Result<InstallResult> {
        // Check permissions for system-level install
        if !self.user_mode {
            let uid = get_user_id()?;
            if uid != 0 {
                return Err(anyhow!(
                    "System-level installation requires root. Run with sudo."
                ));
            }
        }

        // Create service directory if needed
        let service_dir = self.service_dir();
        if !service_dir.exists() {
            fs::create_dir_all(&service_dir)?;
        }

        let plist_path = self.service_file_path();

        // If service is already loaded, bootout first
        if plist_path.exists() {
            // Ignore errors during bootout - service might not be running
            let _ = self.bootout();
        }

        // Generate and write plist
        let plist = self.generate_plist(config);
        let file = File::create(&plist_path)?;
        plist::to_writer_xml(file, &plist)?;

        // Bootstrap the service
        self.bootstrap(&plist_path)?;

        Ok(InstallResult {
            service_file_path: plist_path,
            service_name: self.label().to_string(),
            started: true,
            requires_root: !self.user_mode,
        })
    }

    fn uninstall(&self) -> Result<()> {
        let plist_path = self.service_file_path();

        // Bootout service if running (ignore errors for idempotency)
        let _ = self.bootout();

        // Remove plist file if it exists
        if plist_path.exists() {
            fs::remove_file(&plist_path)?;
        }

        Ok(())
    }

    fn is_installed(&self) -> Result<bool> {
        Ok(self.service_file_path().exists())
    }

    fn service_file_path(&self) -> PathBuf {
        self.service_dir().join(format!("{}.plist", self.label()))
    }

    fn service_name(&self) -> &str {
        self.label()
    }
}

impl LaunchdManager {
    /// Bootstrap the service using modern launchctl syntax
    fn bootstrap(&self, plist_path: &PathBuf) -> Result<()> {
        let output = if self.user_mode {
            let uid = get_user_id()?;
            let domain = format!("gui/{uid}");
            Command::new("launchctl")
                .args(["bootstrap", &domain, &plist_path.display().to_string()])
                .output()?
        } else {
            Command::new("launchctl")
                .args(["bootstrap", "system", &plist_path.display().to_string()])
                .output()?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Handle "already loaded" error gracefully - service is running
            if stderr.contains("already loaded") || stderr.contains("service already loaded") {
                return Ok(());
            }
            return Err(anyhow!("Failed to bootstrap service: {}", stderr.trim()));
        }

        Ok(())
    }

    /// Bootout the service using modern launchctl syntax
    fn bootout(&self) -> Result<()> {
        let output = if self.user_mode {
            let uid = get_user_id()?;
            let service_target = format!("gui/{uid}/{}", self.label());
            Command::new("launchctl")
                .args(["bootout", &service_target])
                .output()?
        } else {
            let service_target = format!("system/{}", self.label());
            Command::new("launchctl")
                .args(["bootout", &service_target])
                .output()?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Handle "not found" error gracefully - service wasn't running
            if stderr.contains("not find") || stderr.contains("No such") {
                return Ok(());
            }
            return Err(anyhow!("Failed to bootout service: {}", stderr.trim()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launchd_manager_user_mode() {
        let manager = LaunchdManager::new("user");
        assert!(manager.user_mode);
        assert_eq!(manager.label(), SERVICE_LABEL);
    }

    #[test]
    fn test_launchd_manager_system_mode() {
        let manager = LaunchdManager::new("system");
        assert!(!manager.user_mode);
    }

    #[test]
    fn test_service_dir_user_mode() {
        let manager = LaunchdManager::new("user");
        let service_dir = manager.service_dir();
        assert!(service_dir
            .to_string_lossy()
            .contains("Library/LaunchAgents"));
    }

    #[test]
    fn test_service_dir_system_mode() {
        let manager = LaunchdManager::new("system");
        let service_dir = manager.service_dir();
        assert_eq!(service_dir, PathBuf::from("/Library/LaunchDaemons"));
    }

    #[test]
    fn test_service_file_path() {
        let manager = LaunchdManager::new("user");
        let path = manager.service_file_path();
        assert!(path.to_string_lossy().ends_with(".plist"));
        assert!(path
            .to_string_lossy()
            .contains("com.opencode-cloud.service"));
    }

    #[test]
    fn test_log_path() {
        let manager = LaunchdManager::new("user");
        let stdout_path = manager.log_path("stdout");
        let stderr_path = manager.log_path("stderr");

        assert!(stdout_path.to_string_lossy().contains("Library/Logs"));
        assert!(stdout_path
            .to_string_lossy()
            .contains("opencode-cloud.stdout.log"));
        assert!(stderr_path
            .to_string_lossy()
            .contains("opencode-cloud.stderr.log"));
    }

    #[test]
    fn test_generate_plist() {
        let manager = LaunchdManager::new("user");
        let config = ServiceConfig {
            executable_path: PathBuf::from("/usr/local/bin/occ"),
            restart_retries: 3,
            restart_delay: 5,
            boot_mode: "user".to_string(),
        };

        let plist = manager.generate_plist(&config);

        assert_eq!(plist.label, SERVICE_LABEL);
        assert_eq!(plist.program_arguments.len(), 3);
        assert_eq!(plist.program_arguments[0], "/usr/local/bin/occ");
        assert_eq!(plist.program_arguments[1], "start");
        assert_eq!(plist.program_arguments[2], "--no-daemon");
        assert!(plist.run_at_load);
        assert!(plist.keep_alive.is_some());
        assert_eq!(plist.throttle_interval, Some(5));
    }

    #[test]
    fn test_plist_serialization() {
        let manager = LaunchdManager::new("user");
        let config = ServiceConfig {
            executable_path: PathBuf::from("/usr/local/bin/occ"),
            restart_retries: 3,
            restart_delay: 5,
            boot_mode: "user".to_string(),
        };

        let plist = manager.generate_plist(&config);

        // Serialize to XML string to verify format
        let mut buffer = Vec::new();
        plist::to_writer_xml(&mut buffer, &plist).expect("Failed to serialize plist");
        let xml = String::from_utf8(buffer).expect("Invalid UTF-8");

        // Verify key elements are present
        assert!(xml.contains("<key>Label</key>"));
        assert!(xml.contains("<string>com.opencode-cloud.service</string>"));
        assert!(xml.contains("<key>ProgramArguments</key>"));
        assert!(xml.contains("<key>RunAtLoad</key>"));
        assert!(xml.contains("<true/>"));
        assert!(xml.contains("<key>KeepAlive</key>"));
        assert!(xml.contains("<key>ThrottleInterval</key>"));
    }
}
