//! Container-mode user management.

use crate::commands::container::{exec_command, exec_command_with_status, exec_command_with_stdin};
use crate::commands::user::{
    UserAddArgs, UserArgs, UserCommands, UserDisableArgs, UserEnableArgs, UserListArgs,
    UserPasswdArgs, UserRemoveArgs,
};
use crate::passwords::{generate_random_password, print_generated_password};
use anyhow::{Result, anyhow, bail};
use comfy_table::{Cell, Color, Table};
use console::style;
use dialoguer::{Confirm, Input, Password};
use opencode_cloud_core::docker::{MOUNT_USERS, UserInfo};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const USERS_STORE_DIR: &str = MOUNT_USERS;
const PROTECTED_USER: &str = "opencode";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistedUserRecord {
    username: String,
    password_hash: String,
    locked: bool,
}

pub async fn cmd_user_container(args: &UserArgs, quiet: bool, verbose: u8) -> Result<()> {
    ensure_root()?;

    match &args.command {
        UserCommands::Add(add_args) => cmd_user_add_container(add_args, quiet, verbose).await,
        UserCommands::Remove(remove_args) => {
            cmd_user_remove_container(remove_args, quiet, verbose).await
        }
        UserCommands::List(list_args) => cmd_user_list_container(list_args, quiet, verbose).await,
        UserCommands::Passwd(passwd_args) => {
            cmd_user_passwd_container(passwd_args, quiet, verbose).await
        }
        UserCommands::Enable(enable_args) => {
            cmd_user_enable_container(enable_args, quiet, verbose).await
        }
        UserCommands::Disable(disable_args) => {
            cmd_user_disable_container(disable_args, quiet, verbose).await
        }
    }
}

/// Add a new user to the container (local exec).
async fn cmd_user_add_container(args: &UserAddArgs, quiet: bool, _verbose: u8) -> Result<()> {
    if args.print_password_only && !args.generate {
        bail!("--print-password-only requires --generate");
    }

    let username = if let Some(ref name) = args.username {
        validate_username(name).map_err(|e| anyhow!("{e}"))?;
        name.clone()
    } else {
        Input::new()
            .with_prompt("Username")
            .default("opencode".to_string())
            .validate_with(|input: &String| validate_username(input))
            .interact_text()?
    };

    if user_exists(&username).await? {
        bail!("User '{username}' already exists in the container");
    }

    let mut generated = args.generate;
    let password =
        if args.generate {
            generate_random_password()
        } else {
            if !quiet {
                println!();
                println!(
                    "{}",
                    style("Set a password for the new container user.").dim()
                );
                println!(
                    "{}",
                    style("This will be used for opencode web login.").dim()
                );
                println!(
                "{}",
                style("Authentication is handled by the system via PAM - we don't store passwords.")
                    .dim()
            );
                println!(
                    "  {} Use {} to auto-generate a secure password.",
                    style("Tip:").cyan(),
                    style("--generate (-g)").bold()
                );
                println!(
                    "  {} Press Enter to auto-generate and display a secure password.",
                    style("Tip:").cyan()
                );
            }
            loop {
                let pwd = Password::new()
                    .with_prompt("Password")
                    .allow_empty_password(true)
                    .interact()?;
                if pwd.is_empty() {
                    generated = true;
                    break generate_random_password();
                }
                let confirm = Password::new().with_prompt("Confirm password").interact()?;
                if pwd != confirm {
                    eprintln!("{}", style("Passwords do not match").red());
                    continue;
                }
                break pwd;
            }
        };

    create_user(&username).await?;
    set_user_password(&username, &password).await?;
    persist_user(&username).await?;

    if args.print_password_only {
        println!("{password}");
        return Ok(());
    }

    if !quiet {
        println!(
            "{} User '{}' created successfully",
            style("Success:").green().bold(),
            username
        );
        if generated {
            print_generated_password(
                &password,
                "Save this password securely - it won't be shown again.",
            );
        }
    }

    Ok(())
}

async fn cmd_user_remove_container(args: &UserRemoveArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let username = &args.username;

    if username == PROTECTED_USER {
        bail!(
            "Cannot remove '{PROTECTED_USER}' - this is a protected system user required for the container to function.\n\n\
            To manage authentication users, use:\n  \
            occ user add <username>\n  \
            occ user remove <username>"
        );
    }

    if !user_exists(username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    let users = list_users().await?;
    let is_last_user = users.len() == 1;
    if is_last_user && !args.force {
        bail!(
            "Cannot remove last user. Add another user first or use --force.\n\n\
            To add a new user:\n  \
            occ user add <username>\n\n\
            To force removal:\n  \
            occ user remove {username} --force"
        );
    }

    if !args.force {
        let confirm = Confirm::new()
            .with_prompt(format!("Remove user '{username}'?"))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirm {
            if !quiet {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    delete_user(username).await?;
    remove_persisted_user(username)?;

    if !quiet {
        println!(
            "{} User '{}' removed successfully",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}

async fn cmd_user_list_container(_args: &UserListArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let users = list_users().await?;

    if users.is_empty() {
        if !quiet {
            println!("No users configured.");
        }
        return Ok(());
    }

    if quiet {
        for user in &users {
            println!("{}", user.username);
        }
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Username", "Status", "UID", "Home", "Shell"]);

    for user in &users {
        let status_cell = if user.locked {
            Cell::new("disabled").fg(Color::Yellow)
        } else {
            Cell::new("enabled").fg(Color::Green)
        };

        table.add_row(vec![
            Cell::new(&user.username),
            status_cell,
            Cell::new(user.uid.to_string()),
            Cell::new(&user.home),
            Cell::new(&user.shell),
        ]);
    }

    println!("{table}");
    Ok(())
}

async fn cmd_user_passwd_container(args: &UserPasswdArgs, quiet: bool, _verbose: u8) -> Result<()> {
    if args.print_password_only && !args.generate {
        bail!("--print-password-only requires --generate");
    }

    let username = &args.username;

    if !user_exists(username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    let password = if args.generate {
        generate_random_password()
    } else {
        if !quiet {
            println!(
                "  {} Use {} to auto-generate a secure password.",
                style("Tip:").cyan(),
                style("--generate (-g)").bold()
            );
        }
        Password::new()
            .with_prompt("New password")
            .with_confirmation("Confirm new password", "Passwords do not match")
            .interact()?
    };

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    set_user_password(username, &password).await?;
    persist_user(username).await?;

    if args.print_password_only {
        println!("{password}");
        return Ok(());
    }

    if !quiet {
        println!(
            "{} Password changed for user '{}'",
            style("Success:").green().bold(),
            username
        );
        if args.generate {
            print_generated_password(
                &password,
                "Save this password securely - it won't be shown again.",
            );
        }
    }

    Ok(())
}

async fn cmd_user_enable_container(args: &UserEnableArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let username = &args.username;

    if !user_exists(username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    unlock_user(username).await?;
    persist_user(username).await?;

    if !quiet {
        println!(
            "{} User '{}' enabled",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}

async fn cmd_user_disable_container(
    args: &UserDisableArgs,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let username = &args.username;

    if !user_exists(username).await? {
        bail!("User '{username}' does not exist in the container");
    }

    lock_user(username).await?;
    persist_user(username).await?;

    if !quiet {
        println!(
            "{} User '{}' disabled",
            style("Success:").green().bold(),
            username
        );
    }

    Ok(())
}

fn ensure_root() -> Result<()> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| anyhow!("Failed to check user id: {e}"))?;
    let uid = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .unwrap_or(1);
    if uid != 0 {
        bail!(
            "User management in container runtime requires root.\n\
Try:\n  sudo occ user <subcommand>\n\
Or from the host:\n  docker exec -u root -it opencode-cloud-sandbox occ user <subcommand>"
        );
    }
    Ok(())
}

fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if username.len() < 3 {
        return Err("Username must be at least 3 characters".to_string());
    }
    if username.len() > 32 {
        return Err("Username must be at most 32 characters".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username must contain only letters, numbers, and underscores".to_string());
    }
    Ok(())
}

async fn create_user(username: &str) -> Result<()> {
    let cmd = ["useradd", "-m", "-s", "/bin/bash", username];
    let (_output, status) = exec_command_with_status(cmd[0], &cmd[1..]).await?;
    if status != 0 {
        if user_exists(username).await? {
            return Err(anyhow!("User '{username}' already exists"));
        }
        return Err(anyhow!(
            "Failed to create user '{username}' (exit {status})"
        ));
    }
    Ok(())
}

async fn delete_user(username: &str) -> Result<()> {
    let cmd = ["userdel", "-r", username];
    let (output, status) = exec_command_with_status(cmd[0], &cmd[1..]).await?;
    if status != 0 {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Err(anyhow!(
                "Failed to remove user '{username}' (exit {status})"
            ));
        }
        return Err(anyhow!(
            "Failed to remove user '{username}' (exit {status})\n{trimmed}"
        ));
    }
    Ok(())
}

async fn set_user_password(username: &str, password: &str) -> Result<()> {
    let stdin_data = format!("{username}:{password}\n");
    exec_command_with_stdin("chpasswd", &[], &stdin_data).await?;
    Ok(())
}

async fn lock_user(username: &str) -> Result<()> {
    let (_output, status) = exec_command_with_status("passwd", &["-l", username]).await?;
    if status != 0 {
        return Err(anyhow!("Failed to lock user '{username}' (exit {status})"));
    }
    Ok(())
}

async fn unlock_user(username: &str) -> Result<()> {
    let (_output, status) = exec_command_with_status("passwd", &["-u", username]).await?;
    if status != 0 {
        return Err(anyhow!(
            "Failed to unlock user '{username}' (exit {status})"
        ));
    }
    Ok(())
}

async fn user_exists(username: &str) -> Result<bool> {
    let (_output, status) = exec_command_with_status("id", &["-u", username]).await?;
    Ok(status == 0)
}

async fn list_users() -> Result<Vec<UserInfo>> {
    let (output, _status) =
        exec_command_with_status("sh", &["-c", "getent passwd | grep '/home/'"]).await?;

    let mut users = Vec::new();
    for line in output.lines() {
        if let Some(info) = parse_passwd_line(line) {
            let locked = is_user_locked(&info.username).await?;
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

async fn persist_user(username: &str) -> Result<()> {
    ensure_users_store_dir()?;
    let shadow_hash = get_user_shadow_hash(username).await?;
    let locked = is_user_locked(username).await?;

    let record = PersistedUserRecord {
        username: username.to_string(),
        password_hash: shadow_hash,
        locked,
    };

    write_user_record(&record)?;
    Ok(())
}

fn remove_persisted_user(username: &str) -> Result<()> {
    let record_path = user_record_path(username);
    if Path::new(&record_path).exists() {
        fs::remove_file(record_path)?;
    }
    Ok(())
}

async fn is_user_locked(username: &str) -> Result<bool> {
    let output = exec_command("passwd", &["-S", username]).await?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() >= 2 {
        return Ok(parts[1] == "L");
    }
    Ok(false)
}

fn ensure_users_store_dir() -> Result<()> {
    fs::create_dir_all(USERS_STORE_DIR)?;
    fs::set_permissions(USERS_STORE_DIR, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

async fn get_user_shadow_hash(username: &str) -> Result<String> {
    let output = exec_command("getent", &["shadow", username]).await?;
    let line = output.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return Err(anyhow!("Failed to read shadow entry for '{username}'"));
    }
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 2 {
        return Err(anyhow!("Invalid shadow entry for '{username}'"));
    }
    Ok(fields[1].to_string())
}

fn user_record_path(username: &str) -> String {
    format!("{USERS_STORE_DIR}/{username}.json")
}

fn write_user_record(record: &PersistedUserRecord) -> Result<()> {
    let payload = serde_json::to_string_pretty(record)?;
    ensure_users_store_dir()?;
    let record_path = user_record_path(&record.username);
    fs::write(&record_path, payload)?;
    fs::set_permissions(&record_path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Parsed user info from /etc/passwd line (intermediate struct).
struct ParsedUser {
    username: String,
    uid: u32,
    home: String,
    shell: String,
}

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
    fn parse_passwd_line_valid() {
        let line = "admin:x:1001:1001:Admin User:/home/admin:/bin/bash";
        let parsed = parse_passwd_line(line).unwrap();
        assert_eq!(parsed.username, "admin");
        assert_eq!(parsed.uid, 1001);
        assert_eq!(parsed.home, "/home/admin");
        assert_eq!(parsed.shell, "/bin/bash");
    }

    #[test]
    fn parse_passwd_line_invalid() {
        assert!(parse_passwd_line("invalid").is_none());
        assert!(parse_passwd_line("too:few:fields").is_none());
        assert!(parse_passwd_line("user:x:not_a_number:1000::/home/user:/bin/bash").is_none());
    }
}
