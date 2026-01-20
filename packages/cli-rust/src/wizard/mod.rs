//! Interactive setup wizard
//!
//! Guides users through first-time configuration with interactive prompts.

mod auth;
mod network;
mod prechecks;
mod summary;

pub use prechecks::{verify_docker_available, verify_tty};

use anyhow::{Result, anyhow};
use console::{Term, style};
use dialoguer::Confirm;
use opencode_cloud_core::Config;

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
    }
}

/// Handle Ctrl+C during wizard by restoring cursor and returning error
fn handle_interrupt() -> anyhow::Error {
    // Restore cursor in case it was hidden
    let _ = Term::stdout().show_cursor();
    anyhow!("Setup cancelled")
}

/// Run the interactive setup wizard
///
/// Guides the user through configuration, collecting values and returning
/// a complete Config. Does NOT save - the caller is responsible for saving.
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

    println!();
    println!("{}", style("opencode-cloud Setup Wizard").cyan().bold());
    println!("{}", style("=".repeat(30)).dim());
    println!();

    // 2. If existing config with auth, show current summary and ask to reconfigure
    if let Some(config) = existing_config {
        if config.has_required_auth() {
            println!("{}", style("Current configuration:").bold());
            println!(
                "  Username: {}",
                config.auth_username.as_deref().unwrap_or("-")
            );
            println!("  Password: ********");
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
    let total_steps = if quick { 1 } else { 3 };

    let (username, password) = prompt_auth(1, total_steps)?;

    let (port, bind) = if quick {
        (3000, "localhost".to_string())
    } else {
        let port = prompt_port(2, total_steps, 3000)?;
        let bind = prompt_hostname(3, total_steps, "localhost")?;
        (port, bind)
    };

    let state = WizardState {
        auth_username: Some(username),
        auth_password: Some(password),
        port,
        bind,
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

    // 7. Build and return config
    let mut config = existing_config.cloned().unwrap_or_default();
    state.apply_to_config(&mut config);
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
        };

        let mut config = Config::default();
        state.apply_to_config(&mut config);

        assert_eq!(config.auth_username, Some("testuser".to_string()));
        assert_eq!(config.auth_password, Some("testpass".to_string()));
        assert_eq!(config.opencode_web_port, 8080);
        assert_eq!(config.bind, "0.0.0.0");
    }

    #[test]
    fn test_wizard_state_preserves_other_config_fields() {
        let state = WizardState {
            auth_username: Some("admin".to_string()),
            auth_password: Some("secret".to_string()),
            port: 3000,
            bind: "localhost".to_string(),
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
    }
}
