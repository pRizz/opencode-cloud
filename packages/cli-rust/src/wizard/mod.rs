//! Interactive setup wizard
//!
//! Guides users through first-time configuration with interactive prompts.

mod auth;
mod network;
mod prechecks;
mod summary;

pub use auth::create_container_user;
pub use prechecks::{verify_docker_available, verify_tty};

use anyhow::{Result, anyhow};
use console::{Term, style};
use dialoguer::Confirm;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, container_is_running};
use opencode_cloud_core::{Config, config::default_mounts};

use auth::prompt_auth;
use network::{prompt_hostname, prompt_port};
use summary::display_summary;

/// Wizard state holding collected configuration values
#[derive(Debug, Clone)]
pub struct WizardState {
    /// Username for authentication
    pub auth_username: Option<String>,
    /// Password for authentication
    pub auth_password: Option<String>,
    /// Port for the web UI
    pub port: u16,
    /// Bind address (localhost or 0.0.0.0)
    pub bind: String,
    /// Image source preference: "prebuilt" or "build"
    pub image_source: String,
    /// Default bind mounts for persistence
    pub mounts: Vec<String>,
}

impl WizardState {
    /// Apply wizard state to a Config struct
    pub fn apply_to_config(&self, config: &mut Config) {
        if let Some(ref username) = self.auth_username {
            config.auth_username = Some(username.clone());
        }
        if let Some(ref password) = self.auth_password {
            config.auth_password = Some(password.clone());
        }
        config.opencode_web_port = self.port;
        config.bind = self.bind.clone();
        config.image_source = self.image_source.clone();
        config.mounts = self.mounts.clone();
    }
}

/// Handle Ctrl+C during wizard by restoring cursor and returning error
fn handle_interrupt() -> anyhow::Error {
    // Restore cursor in case it was hidden
    let _ = Term::stdout().show_cursor();
    anyhow!("Setup cancelled")
}

/// Prompt user to choose image source
fn prompt_image_source(step: usize, total: usize) -> Result<String> {
    println!(
        "{}",
        style(format!("Step {step}/{total}: Image Source"))
            .cyan()
            .bold()
    );
    println!();
    println!("How would you like to get the Docker image?");
    println!();
    println!("  {} Pull prebuilt image (~2 minutes)", style("[1]").bold());
    println!("      Download from GitHub Container Registry");
    println!("      Fast, verified builds published automatically");
    println!();
    println!(
        "  {} Build from source (30-60 minutes)",
        style("[2]").bold()
    );
    println!("      Compile everything locally");
    println!("      Full transparency, customizable Dockerfile");
    println!();
    println!(
        "{}",
        style("Build history: https://github.com/pRizz/opencode-cloud/actions").dim()
    );
    println!();

    let options = vec!["Pull prebuilt image (recommended)", "Build from source"];

    let selection = dialoguer::Select::new()
        .with_prompt("Select image source")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|_| handle_interrupt())?;

    println!();

    Ok(if selection == 0 { "prebuilt" } else { "build" }.to_string())
}

fn display_mounts_info(step: usize, total: usize, mounts: &[String]) -> Result<()> {
    println!(
        "{}",
        style(format!("Step {step}/{total}: Data Persistence"))
            .cyan()
            .bold()
    );
    println!();
    if mounts.is_empty() {
        println!(
            "{}",
            style("No default host mounts are available.").yellow()
        );
        println!();
        return Ok(());
    }

    println!("Persist opencode data on your host using these mounts:");
    println!();
    for mount in mounts {
        println!("  {}", style(mount).cyan());
    }
    println!();
    println!(
        "{}",
        style("You can change these later with `occ mount add/remove` or by editing the config.")
            .dim()
    );
    println!();
    Ok(())
}

fn prompt_mounts(step: usize, total: usize, mounts: &[String]) -> Result<Vec<String>> {
    display_mounts_info(step, total, mounts)?;
    if mounts.is_empty() {
        return Ok(Vec::new());
    }

    let confirmed = Confirm::new()
        .with_prompt("Use these host mounts for persistence?")
        .default(true)
        .interact()
        .map_err(|_| handle_interrupt())?;
    println!();

    if confirmed {
        Ok(mounts.to_vec())
    } else {
        Ok(Vec::new())
    }
}

/// Run the interactive setup wizard
///
/// Guides the user through configuration, collecting values and returning
/// a complete Config. Does NOT save - the caller is responsible for saving.
///
/// Creates PAM-based users in the container if it's running.
/// Migrates old auth_username/auth_password to new users array.
///
/// # Arguments
/// * `existing_config` - Optional existing config to show current values
///
/// # Returns
/// * `Ok(Config)` - Completed configuration ready to save
/// * `Err` - User cancelled or prechecks failed
pub async fn run_wizard(existing_config: Option<&Config>) -> Result<Config> {
    // 1. Prechecks
    verify_tty()?;
    verify_docker_available().await?;

    // Connect to Docker for container operations
    let client = DockerClient::new()?;
    let is_container_running = container_is_running(&client, CONTAINER_NAME)
        .await
        .unwrap_or(false);

    println!();
    println!("{}", style("opencode-cloud Setup Wizard").cyan().bold());
    println!("{}", style("=".repeat(30)).dim());
    println!();

    // 2. If existing config with users configured, show current summary and ask to reconfigure
    if let Some(config) = existing_config {
        let has_users = !config.users.is_empty();
        let has_old_auth = config.has_required_auth();

        if has_users || has_old_auth {
            println!("{}", style("Current configuration:").bold());
            if has_users {
                println!("  Users:    {}", config.users.join(", "));
            } else if has_old_auth {
                println!(
                    "  Username: {} (legacy)",
                    config.auth_username.as_deref().unwrap_or("-")
                );
                println!("  Password: ********");
            }
            println!("  Port:     {}", config.opencode_web_port);
            println!("  Binding:  {}", config.bind);
            println!();

            let reconfigure = Confirm::new()
                .with_prompt("Reconfigure?")
                .default(false)
                .interact()
                .map_err(|_| handle_interrupt())?;

            if !reconfigure {
                return Err(anyhow!("Setup cancelled"));
            }
            println!();
        }
    }

    // 3. Quick setup offer
    let quick = Confirm::new()
        .with_prompt("Use defaults for everything except credentials?")
        .default(false)
        .interact()
        .map_err(|_| handle_interrupt())?;

    println!();

    // 4. Collect values
    let total_steps = if quick { 3 } else { 5 };

    let (username, password) = prompt_auth(1, total_steps)?;
    let image_source = prompt_image_source(2, total_steps)?;

    let (port, bind) = if quick {
        (3000, "localhost".to_string())
    } else {
        let port = prompt_port(3, total_steps, 3000)?;
        let bind = prompt_hostname(4, total_steps, "localhost")?;
        (port, bind)
    };

    let default_mounts = default_mounts();
    let mounts = if quick {
        display_mounts_info(3, total_steps, &default_mounts)?;
        default_mounts
    } else {
        prompt_mounts(5, total_steps, &default_mounts)?
    };

    let state = WizardState {
        auth_username: Some(username.clone()),
        auth_password: Some(password.clone()),
        port,
        bind,
        image_source,
        mounts,
    };

    // 5. Summary
    println!();
    display_summary(&state);
    println!();

    // 6. Confirm save
    let save = Confirm::new()
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()
        .map_err(|_| handle_interrupt())?;

    if !save {
        return Err(anyhow!("Setup cancelled"));
    }

    // 7. Create user in container if running
    if is_container_running {
        println!();
        println!("{}", style("Creating user in container...").cyan());
        auth::create_container_user(&client, &username, &password).await?;
    } else {
        println!();
        println!(
            "{}",
            style("Note: User will be created when container starts.").dim()
        );
    }

    // 8. Build and return config
    let mut config = existing_config.cloned().unwrap_or_default();
    state.apply_to_config(&mut config);

    // Update config.users array (PAM-based auth tracking)
    if !config.users.contains(&username) {
        config.users.push(username);
    }

    // Migrate old auth_username/auth_password if present
    if let Some(ref old_username) = config.auth_username {
        if !old_username.is_empty() && !config.users.contains(old_username) {
            println!(
                "{}",
                style(format!(
                    "Migrating existing user '{old_username}' to PAM-based authentication..."
                ))
                .dim()
            );
            config.users.push(old_username.clone());
        }
    }

    // Clear legacy auth fields (keep them empty for schema compatibility)
    config.auth_username = Some(String::new());
    config.auth_password = Some(String::new());

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_state_apply_to_config() {
        let state = WizardState {
            auth_username: Some("testuser".to_string()),
            auth_password: Some("testpass".to_string()),
            port: 8080,
            bind: "0.0.0.0".to_string(),
            image_source: "prebuilt".to_string(),
            mounts: default_mounts(),
        };

        let mut config = Config::default();
        state.apply_to_config(&mut config);

        assert_eq!(config.auth_username, Some("testuser".to_string()));
        assert_eq!(config.auth_password, Some("testpass".to_string()));
        assert_eq!(config.opencode_web_port, 8080);
        assert_eq!(config.bind, "0.0.0.0");
        assert_eq!(config.image_source, "prebuilt");
    }

    #[test]
    fn test_wizard_state_preserves_other_config_fields() {
        let state = WizardState {
            auth_username: Some("admin".to_string()),
            auth_password: Some("secret".to_string()),
            port: 3000,
            bind: "localhost".to_string(),
            image_source: "build".to_string(),
            mounts: default_mounts(),
        };

        let mut config = Config {
            auto_restart: false,
            restart_retries: 10,
            ..Config::default()
        };
        state.apply_to_config(&mut config);

        // Should preserve existing fields
        assert!(!config.auto_restart);
        assert_eq!(config.restart_retries, 10);

        // Should update wizard fields
        assert_eq!(config.auth_username, Some("admin".to_string()));
        assert_eq!(config.image_source, "build");
    }
}
