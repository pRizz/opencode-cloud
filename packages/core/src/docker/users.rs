//! Container user management operations
//!
//! This module provides functions to manage Linux system users inside
//! the running Docker container. opencode authenticates against PAM,
//! so opencode-cloud must manage system users in the container.
//!
//! Security note: Passwords are never passed as command arguments.
//! Instead, we use `chpasswd` which reads from stdin.

use super::exec::{exec_command, exec_command_exit_code, exec_command_with_stdin};
use super::{DockerClient, DockerError};

/// Information about a container user
#[derive(Debug, Clone, PartialEq)]
pub struct UserInfo {
    /// Username
    pub username: String,
    /// User ID (uid)
    pub uid: u32,
    /// Home directory path
    pub home: String,
    /// Login shell
    pub shell: String,
    /// Whether the account is locked
    pub locked: bool,
}

/// Create a new user in the container
///
/// Creates a user with a home directory and /bin/bash shell.
/// Returns an error if the user already exists.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to create
///
/// # Example
/// ```ignore
/// create_user(&client, "opencode-cloud", "admin").await?;
/// ```
pub async fn create_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["useradd", "-m", "-s", "/bin/bash", username];

    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    if exit_code != 0 {
        // Check if user already exists
        if user_exists(client, container, username).await? {
            return Err(DockerError::Container(format!(
                "User '{}' already exists",
                username
            )));
        }
        return Err(DockerError::Container(format!(
            "Failed to create user '{}': useradd returned exit code {}",
            username, exit_code
        )));
    }

    Ok(())
}

/// Set or change a user's password
///
/// Uses chpasswd with stdin for secure password setting.
/// The password never appears in command arguments or process list.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to set password for
/// * `password` - New password (will be written to stdin)
///
/// # Security
/// The password is written directly to chpasswd's stdin, never appearing
/// in command arguments, environment variables, or process listings.
///
/// # Example
/// ```ignore
/// set_user_password(&client, "opencode-cloud", "admin", "secret123").await?;
/// ```
pub async fn set_user_password(
    client: &DockerClient,
    container: &str,
    username: &str,
    password: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["chpasswd"];
    let stdin_data = format!("{}:{}\n", username, password);

    exec_command_with_stdin(client, container, cmd, &stdin_data).await?;

    Ok(())
}

/// Check if a user exists in the container
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to check
///
/// # Returns
/// `true` if the user exists, `false` otherwise
pub async fn user_exists(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<bool, DockerError> {
    let cmd = vec!["id", "-u", username];
    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    Ok(exit_code == 0)
}

/// Lock a user account (disable password authentication)
///
/// Uses `passwd -l` to lock the account. The user will not be able
/// to log in using password authentication.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to lock
pub async fn lock_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["passwd", "-l", username];
    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    if exit_code != 0 {
        return Err(DockerError::Container(format!(
            "Failed to lock user '{}': passwd returned exit code {}",
            username, exit_code
        )));
    }

    Ok(())
}

/// Unlock a user account (re-enable password authentication)
///
/// Uses `passwd -u` to unlock the account.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to unlock
pub async fn unlock_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["passwd", "-u", username];
    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    if exit_code != 0 {
        return Err(DockerError::Container(format!(
            "Failed to unlock user '{}': passwd returned exit code {}",
            username, exit_code
        )));
    }

    Ok(())
}

/// Delete a user from the container
///
/// Uses `userdel -r` to remove the user and their home directory.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
/// * `username` - Username to delete
pub async fn delete_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["userdel", "-r", username];
    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    if exit_code != 0 {
        // Check if user doesn't exist
        if !user_exists(client, container, username).await? {
            return Err(DockerError::Container(format!(
                "User '{}' does not exist",
                username
            )));
        }
        return Err(DockerError::Container(format!(
            "Failed to delete user '{}': userdel returned exit code {}",
            username, exit_code
        )));
    }

    Ok(())
}

/// List users in the container with home directories
///
/// Returns users that have home directories under /home/.
/// Excludes system users.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
pub async fn list_users(
    client: &DockerClient,
    container: &str,
) -> Result<Vec<UserInfo>, DockerError> {
    // Get all users with home directories in /home
    let cmd = vec!["sh", "-c", "getent passwd | grep '/home/'"];
    let output = exec_command(client, container, cmd).await?;

    let mut users = Vec::new();

    for line in output.lines() {
        if let Some(info) = parse_passwd_line(line) {
            // Check if user is locked
            let locked = is_user_locked(client, container, &info.username).await?;

            users.push(UserInfo {
                username: info.username,
                uid: info.uid,
                home: info.home,
                shell: info.shell,
                locked,
            });
        }
    }

    Ok(users)
}

/// Check if a user account is locked
///
/// Uses `passwd -S` to get account status.
/// Returns true if the status starts with "L" (locked).
async fn is_user_locked(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<bool, DockerError> {
    let cmd = vec!["passwd", "-S", username];
    let output = exec_command(client, container, cmd).await?;

    // passwd -S output format: "username L/P/NP ... "
    // L = locked, P = password set, NP = no password
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() >= 2 {
        return Ok(parts[1] == "L");
    }

    Ok(false)
}

/// Parsed user info from /etc/passwd line (intermediate struct)
struct ParsedUser {
    username: String,
    uid: u32,
    home: String,
    shell: String,
}

/// Parse a line from /etc/passwd
///
/// Format: username:x:uid:gid:gecos:home:shell
fn parse_passwd_line(line: &str) -> Option<ParsedUser> {
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 7 {
        return None;
    }

    let uid = fields[2].parse::<u32>().ok()?;

    Some(ParsedUser {
        username: fields[0].to_string(),
        uid,
        home: fields[5].to_string(),
        shell: fields[6].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_passwd_line_valid() {
        let line = "admin:x:1001:1001:Admin User:/home/admin:/bin/bash";
        let parsed = parse_passwd_line(line).unwrap();
        assert_eq!(parsed.username, "admin");
        assert_eq!(parsed.uid, 1001);
        assert_eq!(parsed.home, "/home/admin");
        assert_eq!(parsed.shell, "/bin/bash");
    }

    #[test]
    fn test_parse_passwd_line_minimal() {
        let line = "user:x:1000:1000::/home/user:/bin/sh";
        let parsed = parse_passwd_line(line).unwrap();
        assert_eq!(parsed.username, "user");
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.home, "/home/user");
        assert_eq!(parsed.shell, "/bin/sh");
    }

    #[test]
    fn test_parse_passwd_line_invalid() {
        assert!(parse_passwd_line("invalid").is_none());
        assert!(parse_passwd_line("too:few:fields").is_none());
        assert!(parse_passwd_line("user:x:not_a_number:1000::/home/user:/bin/bash").is_none());
    }

    #[test]
    fn test_user_info_struct() {
        let info = UserInfo {
            username: "admin".to_string(),
            uid: 1001,
            home: "/home/admin".to_string(),
            shell: "/bin/bash".to_string(),
            locked: false,
        };
        assert_eq!(info.username, "admin");
        assert!(!info.locked);
    }

    #[test]
    fn test_user_info_equality() {
        let info1 = UserInfo {
            username: "admin".to_string(),
            uid: 1001,
            home: "/home/admin".to_string(),
            shell: "/bin/bash".to_string(),
            locked: false,
        };
        let info2 = info1.clone();
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_user_info_debug() {
        let info = UserInfo {
            username: "test".to_string(),
            uid: 1000,
            home: "/home/test".to_string(),
            shell: "/bin/bash".to_string(),
            locked: true,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("test"));
        assert!(debug.contains("1000"));
        assert!(debug.contains("locked: true"));
    }
}
