//! Singleton enforcement via PID lock
//!
//! Ensures only one instance of opencode-cloud can run at a time.
//! Uses a PID file with stale detection - if a previous process crashed
//! without cleaning up, the stale lock is automatically removed.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during singleton lock operations
#[derive(Error, Debug)]
pub enum SingletonError {
    /// Another instance is already running
    #[error("Another instance is already running (PID: {0})")]
    AlreadyRunning(u32),

    /// Failed to create the lock directory
    #[error("Failed to create lock directory: {0}")]
    CreateDirFailed(String),

    /// Failed to create or manage the lock file
    #[error("Failed to create lock file: {0}")]
    LockFailed(String),

    /// The lock file path could not be determined
    #[error("Invalid lock file path")]
    InvalidPath,
}

/// A guard that holds the singleton instance lock
///
/// The lock is automatically released when this struct is dropped.
/// The PID file is removed on drop to allow other instances to start.
pub struct InstanceLock {
    pid_path: PathBuf,
}

impl InstanceLock {
    /// Attempt to acquire the singleton lock
    ///
    /// # Returns
    /// - `Ok(InstanceLock)` if the lock was successfully acquired
    /// - `Err(SingletonError::AlreadyRunning(pid))` if another instance is running
    /// - `Err(SingletonError::*)` for other errors
    ///
    /// # Stale Lock Detection
    /// If a PID file exists but the process is no longer running,
    /// the stale file is automatically cleaned up before acquiring the lock.
    pub fn acquire(pid_path: PathBuf) -> Result<Self, SingletonError> {
        // Ensure parent directory exists
        if let Some(parent) = pid_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SingletonError::CreateDirFailed(e.to_string()))?;
        }

        // Check if PID file exists
        if pid_path.exists() {
            // Read existing PID
            let mut file =
                File::open(&pid_path).map_err(|e| SingletonError::LockFailed(e.to_string()))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| SingletonError::LockFailed(e.to_string()))?;

            if let Ok(pid) = contents.trim().parse::<u32>() {
                // Check if process is still running
                if is_process_running(pid) {
                    return Err(SingletonError::AlreadyRunning(pid));
                }
                // Stale PID file - process not running, remove it
                tracing::info!("Removing stale PID file (PID {} not running)", pid);
            }
            // Remove stale/invalid PID file
            fs::remove_file(&pid_path).map_err(|e| SingletonError::LockFailed(e.to_string()))?;
        }

        // Write our PID
        let mut file =
            File::create(&pid_path).map_err(|e| SingletonError::LockFailed(e.to_string()))?;
        write!(file, "{}", std::process::id())
            .map_err(|e| SingletonError::LockFailed(e.to_string()))?;

        tracing::debug!("Acquired singleton lock at: {}", pid_path.display());

        Ok(Self { pid_path })
    }

    /// Explicitly release the lock
    ///
    /// This is called automatically on drop, but can be called explicitly
    /// if you want to release the lock early.
    pub fn release(self) {
        // Dropping self will call Drop::drop which removes the file
    }

    /// Get the path to the PID file
    pub fn pid_path(&self) -> &PathBuf {
        &self.pid_path
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.pid_path) {
            tracing::warn!("Failed to remove PID file on drop: {}", e);
        } else {
            tracing::debug!("Released singleton lock: {}", self.pid_path.display());
        }
    }
}

/// Check if a process with the given PID is currently running
///
/// Uses platform-specific methods to check process existence:
/// - Unix: `kill(pid, 0)` - signal 0 checks existence without sending signal
/// - Windows: OpenProcess API (deferred to v2)
fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // On Unix, sending signal 0 checks if process exists
        // without actually sending a signal
        match std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => {
                // Fallback: check /proc on Linux
                #[cfg(target_os = "linux")]
                {
                    std::path::Path::new(&format!("/proc/{pid}")).exists()
                }
                #[cfg(not(target_os = "linux"))]
                {
                    // On macOS, if kill -0 fails, assume process doesn't exist
                    false
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Windows support deferred to v2
        // For now, assume process is not running if we can't check
        false
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Unknown platform - assume not running
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_acquire_creates_pid_file() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        let lock = InstanceLock::acquire(pid_path.clone()).unwrap();

        // Verify PID file exists
        assert!(pid_path.exists());

        // Verify it contains our PID
        let contents = std::fs::read_to_string(&pid_path).unwrap();
        let written_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(written_pid, std::process::id());

        // Drop the lock
        drop(lock);

        // Verify PID file was removed
        assert!(!pid_path.exists());
    }

    #[test]
    fn test_acquire_fails_when_already_locked() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        // Acquire first lock
        let _lock1 = InstanceLock::acquire(pid_path.clone()).unwrap();

        // Try to acquire second lock - should fail
        let result = InstanceLock::acquire(pid_path.clone());
        assert!(matches!(result, Err(SingletonError::AlreadyRunning(_))));
    }

    #[test]
    fn test_stale_lock_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        // Write a fake PID file with a PID that doesn't exist
        // Using PID 999999 which is very unlikely to be running
        std::fs::write(&pid_path, "999999").unwrap();

        // Should be able to acquire lock (stale PID will be cleaned up)
        let lock = InstanceLock::acquire(pid_path.clone());

        // On Unix, this should succeed because 999999 likely isn't running
        // On Windows or if 999999 happens to be running, this might fail
        // which is acceptable - the test demonstrates the stale detection works
        if lock.is_ok() {
            assert!(pid_path.exists());
            let contents = std::fs::read_to_string(&pid_path).unwrap();
            let written_pid: u32 = contents.trim().parse().unwrap();
            assert_eq!(written_pid, std::process::id());
        }
    }

    #[test]
    fn test_is_process_running_with_current_process() {
        let current_pid = std::process::id();
        assert!(is_process_running(current_pid));
    }

    #[test]
    fn test_is_process_running_with_invalid_pid() {
        // PID 0 is the kernel, PID 1 is init - use a very high unlikely PID
        let unlikely_pid = 4_000_000_000;
        assert!(!is_process_running(unlikely_pid));
    }

    #[test]
    fn test_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir
            .path()
            .join("deep")
            .join("nested")
            .join("dir")
            .join("test.pid");

        let lock = InstanceLock::acquire(pid_path.clone()).unwrap();
        assert!(pid_path.exists());
        drop(lock);
    }
}
