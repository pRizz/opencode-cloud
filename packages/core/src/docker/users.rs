//! Container user management operations
//!
//! This module provides functions to manage Linux system users inside
//! the running Docker container. opencode authenticates against PAM,
//! so opencode-cloud must manage system users in the container.
//!
//! Security note: Passwords are never passed as command arguments.
//! Instead, we use `chpasswd` which reads from stdin.

use super::exec::{exec_command, exec_command_exit_code, exec_command_with_stdin};
use super::volume::MOUNT_USERS;
use super::{DockerClient, DockerError};
use serde::{Deserialize, Serialize};

/// User persistence store directory inside the container.
///
/// Format: one JSON file per user with strict permissions (root-owned, 0700 dir, 0600 files).
/// Stored on a managed Docker volume mounted at this path.
const USERS_STORE_DIR: &str = MOUNT_USERS;
const PROTECTED_SYSTEM_USER: &str = "opencoder";
const HIDDEN_BUILTIN_USERS: [&str; 2] = [PROTECTED_SYSTEM_USER, "ubuntu"];

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistedUserRecord {
    username: String,
    password_hash: String,
    locked: bool,
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
/// create_user(&client, "opencode-cloud-sandbox", "admin").await?;
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
                "User '{username}' already exists"
            )));
        }
        return Err(DockerError::Container(format!(
            "Failed to create user '{username}': useradd returned exit code {exit_code}"
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
/// set_user_password(&client, "opencode-cloud-sandbox", "admin", "secret123").await?;
/// ```
pub async fn set_user_password(
    client: &DockerClient,
    container: &str,
    username: &str,
    password: &str,
) -> Result<(), DockerError> {
    let cmd = vec!["chpasswd"];
    let stdin_data = format!("{username}:{password}\n");

    exec_command_with_stdin(client, container, cmd, &stdin_data).await?;

    Ok(())
}

/// Set a user's password hash directly (no plaintext required)
///
/// Uses `usermod -p` with a precomputed shadow hash.
async fn set_user_password_hash(
    client: &DockerClient,
    container: &str,
    username: &str,
    password_hash: &str,
) -> Result<(), DockerError> {
    if password_hash.is_empty() {
        return Ok(());
    }

    let cmd = vec!["usermod", "-p", password_hash, username];
    let exit_code = exec_command_exit_code(client, container, cmd).await?;

    if exit_code != 0 {
        return Err(DockerError::Container(format!(
            "Failed to set password hash for '{username}': usermod returned exit code {exit_code}"
        )));
    }

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
            "Failed to lock user '{username}': passwd returned exit code {exit_code}"
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
            "Failed to unlock user '{username}': passwd returned exit code {exit_code}"
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
                "User '{username}' does not exist"
            )));
        }
        return Err(DockerError::Container(format!(
            "Failed to delete user '{username}': userdel returned exit code {exit_code}"
        )));
    }

    Ok(())
}

/// List managed users in the container.
///
/// This is intentionally a records-backed view, not a raw `/etc/passwd` dump.
/// We only show users that were explicitly managed/persisted by opencode-cloud.
///
/// # Arguments
/// * `client` - Docker client
/// * `container` - Container name or ID
pub async fn list_users(
    client: &DockerClient,
    container: &str,
) -> Result<Vec<UserInfo>, DockerError> {
    let records = read_user_records(client, container).await?;
    let mut users = Vec::new();

    for username in managed_usernames_from_records(&records) {
        if let Some(user) = read_user_info(client, container, &username).await? {
            users.push(user);
        }
    }

    users.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(users)
}

/// Persist a user's credentials and lock state to the managed volume.
///
/// Stores the shadow hash (not plaintext) and lock status in a JSON record.
pub async fn persist_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    if is_builtin_system_user(username) {
        return Err(DockerError::Container(format!(
            "User '{username}' is built-in and cannot be persisted"
        )));
    }

    ensure_users_store_dir(client, container).await?;

    let shadow_hash = get_user_shadow_hash(client, container, username).await?;
    let locked = is_user_locked(client, container, username).await?;

    let record = PersistedUserRecord {
        username: username.to_string(),
        password_hash: shadow_hash,
        locked,
    };

    write_user_record(client, container, &record).await?;
    Ok(())
}

/// Remove a persisted user record from the managed volume.
pub async fn remove_persisted_user(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<(), DockerError> {
    let record_path = user_record_path(username);
    let cmd_string = format!("rm -f {record_path}");
    let cmd = vec!["sh", "-c", cmd_string.as_str()];
    exec_command(client, container, cmd).await?;
    Ok(())
}

/// Restore users from the persisted store into the container.
///
/// Returns the list of usernames restored or updated.
pub async fn restore_persisted_users(
    client: &DockerClient,
    container: &str,
) -> Result<Vec<String>, DockerError> {
    let records = read_user_records(client, container).await?;
    if records.is_empty() {
        let users = list_non_builtin_home_usernames(client, container).await?;
        let mut persisted = Vec::new();
        for username in users {
            persist_user(client, container, &username).await?;
            persisted.push(username);
        }
        return Ok(persisted);
    }

    let mut restored = Vec::new();

    for record in records {
        if !user_exists(client, container, &record.username).await? {
            create_user(client, container, &record.username).await?;
        }

        set_user_password_hash(client, container, &record.username, &record.password_hash).await?;

        if record.locked {
            lock_user(client, container, &record.username).await?;
        } else {
            unlock_user(client, container, &record.username).await?;
        }

        restored.push(record.username);
    }

    Ok(restored)
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

async fn ensure_users_store_dir(client: &DockerClient, container: &str) -> Result<(), DockerError> {
    let cmd_string = format!("install -d -m 700 {USERS_STORE_DIR}");
    let cmd = vec!["sh", "-c", cmd_string.as_str()];
    exec_command(client, container, cmd).await?;
    Ok(())
}

async fn get_user_shadow_hash(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<String, DockerError> {
    let output = exec_command(client, container, vec!["getent", "shadow", username]).await?;
    let line = output.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return Err(DockerError::Container(format!(
            "Failed to read shadow entry for '{username}'"
        )));
    }

    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 2 {
        return Err(DockerError::Container(format!(
            "Invalid shadow entry for '{username}'"
        )));
    }

    Ok(fields[1].to_string())
}

async fn read_user_info(
    client: &DockerClient,
    container: &str,
    username: &str,
) -> Result<Option<UserInfo>, DockerError> {
    // Skip stale records cleanly when the system user no longer exists.
    if !user_exists(client, container, username).await? {
        return Ok(None);
    }

    let output = exec_command(client, container, vec!["getent", "passwd", username]).await?;
    let line = output.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return Ok(None);
    }

    let info = parse_passwd_line(line).ok_or_else(|| {
        DockerError::Container(format!("Failed to parse passwd entry for '{username}'"))
    })?;
    let locked = is_user_locked(client, container, &info.username).await?;

    Ok(Some(UserInfo {
        username: info.username,
        uid: info.uid,
        home: info.home,
        shell: info.shell,
        locked,
    }))
}

async fn list_non_builtin_home_usernames(
    client: &DockerClient,
    container: &str,
) -> Result<Vec<String>, DockerError> {
    let cmd = vec!["sh", "-c", "getent passwd | grep '/home/'"];
    let output = exec_command(client, container, cmd).await?;
    let mut users = Vec::new();

    for line in output.lines() {
        let Some(info) = parse_passwd_line(line) else {
            continue;
        };
        if is_builtin_system_user(&info.username) {
            continue;
        }
        users.push(info.username);
    }

    Ok(users)
}

async fn read_user_records(
    client: &DockerClient,
    container: &str,
) -> Result<Vec<PersistedUserRecord>, DockerError> {
    let list_command =
        format!("if [ -d {USERS_STORE_DIR} ]; then ls -1 {USERS_STORE_DIR}/*.json 2>/dev/null; fi");
    let list_cmd = vec!["sh", "-c", list_command.as_str()];
    let output = exec_command(client, container, list_cmd).await?;
    let mut records = Vec::new();

    for path in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let contents = exec_command(client, container, vec!["cat", path]).await?;
        let record: PersistedUserRecord = serde_json::from_str(&contents).map_err(|e| {
            DockerError::Container(format!("Failed to parse user record {path}: {e}"))
        })?;
        if is_builtin_system_user(&record.username) {
            continue;
        }
        records.push(record);
    }

    Ok(records)
}

fn user_record_path(username: &str) -> String {
    format!("{USERS_STORE_DIR}/{username}.json")
}

fn is_builtin_system_user(username: &str) -> bool {
    HIDDEN_BUILTIN_USERS.contains(&username)
}

fn managed_usernames_from_records(records: &[PersistedUserRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| record.username.clone())
        .filter(|username| !is_builtin_system_user(username))
        .collect()
}

async fn write_user_record(
    client: &DockerClient,
    container: &str,
    record: &PersistedUserRecord,
) -> Result<(), DockerError> {
    let payload =
        serde_json::to_string_pretty(record).map_err(|e| DockerError::Container(e.to_string()))?;
    let record_path = user_record_path(&record.username);
    let write_command =
        format!("install -d -m 700 {USERS_STORE_DIR} && umask 077 && cat > {record_path}");
    let cmd = vec!["sh", "-c", write_command.as_str()];
    exec_command_with_stdin(client, container, cmd, &payload).await?;
    Ok(())
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
    fn test_protected_system_user_constant() {
        assert_eq!(PROTECTED_SYSTEM_USER, "opencoder");
    }

    #[test]
    fn test_is_builtin_system_user() {
        assert!(is_builtin_system_user("opencoder"));
        assert!(is_builtin_system_user("ubuntu"));
        assert!(!is_builtin_system_user("admin"));
    }

    #[test]
    fn test_managed_usernames_from_records_filters_builtin_users() {
        let records = vec![
            PersistedUserRecord {
                username: "ubuntu".to_string(),
                password_hash: "x".to_string(),
                locked: true,
            },
            PersistedUserRecord {
                username: "admin".to_string(),
                password_hash: "y".to_string(),
                locked: false,
            },
            PersistedUserRecord {
                username: "opencoder".to_string(),
                password_hash: "z".to_string(),
                locked: true,
            },
        ];
        let usernames = managed_usernames_from_records(&records);
        assert_eq!(usernames, vec!["admin".to_string()]);
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
        let debug = format!("{info:?}");
        assert!(debug.contains("test"));
        assert!(debug.contains("1000"));
        assert!(debug.contains("locked: true"));
    }
}
