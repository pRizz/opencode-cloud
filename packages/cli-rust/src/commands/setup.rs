//! Setup command implementation
//!
//! Runs the interactive setup wizard.

use anyhow::Result;
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, container_is_running};
use opencode_cloud_core::{Config, load_config_or_default, save_config};

use crate::commands::iotp::{IOTP_FALLBACK_COMMAND, IotpState, fetch_iotp_snapshot};
use crate::commands::{cmd_start, cmd_stop};
use crate::constants::COCKPIT_EXPOSED;
use crate::wizard::run_wizard;

/// Arguments for the setup command
#[derive(Args)]
pub struct SetupArgs {
    /// Skip wizard if configuration already exists
    #[arg(long, short)]
    pub yes: bool,

    /// Non-interactive bootstrap for automated environments
    #[arg(long, alias = "non-interactive", alias = "headless")]
    pub bootstrap: bool,

    /// Run setup for a remote host instead of local Docker
    #[arg(long, conflicts_with = "local")]
    pub remote_host: Option<String>,

    /// Force local Docker (ignores default_host)
    #[arg(long, conflicts_with = "remote_host")]
    pub local: bool,
}

/// Run the setup command
pub async fn cmd_setup(args: &SetupArgs, quiet: bool) -> Result<()> {
    // Load existing config (or create default)
    let existing_config = load_config_or_default().ok();
    let target_host = crate::resolve_target_host(args.remote_host.as_deref(), args.local);

    if args.bootstrap {
        return run_bootstrap_setup(existing_config, target_host.as_deref(), quiet).await;
    }

    // Handle --yes flag for non-interactive mode
    if args.yes {
        let config_exists =
            opencode_cloud_core::config::paths::get_config_path().is_some_and(|path| path.exists());
        if config_exists && existing_config.is_some() {
            if !quiet {
                println!("{}", style("Configuration already set").green());
            }
            return Ok(());
        }

        anyhow::bail!(
            "Non-interactive mode requires an existing configuration.\n\n\
            Use:\n  \
            occ setup\n\n\
            Or for automated environments:\n  \
            occ setup --bootstrap"
        );
    }

    // Run the wizard
    let new_config = run_wizard(existing_config.as_ref()).await?;

    // Save the config
    save_config(&new_config)?;

    if quiet {
        return Ok(());
    }

    println!();
    println!(
        "{} Configuration saved successfully!",
        style("Success:").green().bold()
    );
    println!();

    // Check if container is already running
    let (client, host_name) = crate::resolve_docker_client(target_host.as_deref()).await?;
    let is_running = container_is_running(&client, CONTAINER_NAME)
        .await
        .unwrap_or(false);

    // Determine if restart-relevant config changed
    let config_changed = existing_config
        .as_ref()
        .is_some_and(|old| requires_restart(old, &new_config));

    // Choose appropriate prompt based on state
    let (prompt, action) = match (is_running, config_changed) {
        (true, true) => (
            "Config changed. Restart opencode-cloud to apply?",
            Action::Restart,
        ),
        (true, false) => {
            // Running but no restart-relevant changes - just show status
            show_running_status(&new_config, host_name.as_deref());
            return Ok(());
        }
        (false, _) => ("Start opencode-cloud now?", Action::Start),
    };

    let confirmed = Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact()
        .unwrap_or(false);

    if !confirmed {
        return Ok(());
    }

    println!();

    // Stop first if restarting (use longer timeout for graceful shutdown)
    if action == Action::Restart {
        let stop_args = crate::commands::StopArgs {
            timeout: 60,
            remove: false,
        };
        cmd_stop(&stop_args, target_host.as_deref(), quiet).await?;
        println!();
    }

    // Start the service
    let start_args = crate::commands::StartArgs {
        port: Some(new_config.opencode_web_port),
        ..Default::default()
    };
    cmd_start(&start_args, target_host.as_deref(), quiet, 0).await?;
    maybe_print_iotp_info(&client, host_name.as_deref(), &new_config).await;

    Ok(())
}

async fn run_bootstrap_setup(
    existing_config: Option<Config>,
    target_host: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let new_config = build_bootstrap_config(existing_config.clone());
    save_config(&new_config)?;

    if quiet {
        return start_or_restart_after_setup(
            existing_config.as_ref(),
            &new_config,
            target_host,
            quiet,
            true,
        )
        .await;
    }

    println!();
    println!(
        "{} Bootstrap configuration saved successfully!",
        style("Success:").green().bold()
    );
    println!(
        "{}",
        style("Unauthenticated network access is enabled.").yellow()
    );
    println!();

    start_or_restart_after_setup(
        existing_config.as_ref(),
        &new_config,
        target_host,
        quiet,
        true,
    )
    .await
}

fn build_bootstrap_config(existing_config: Option<Config>) -> Config {
    let mut config = existing_config.unwrap_or_default();
    config.bind = "0.0.0.0".to_string();
    config.bind_address = "0.0.0.0".to_string();
    config.cockpit_enabled = false;
    config.allow_unauthenticated_network = true;
    config
}

async fn start_or_restart_after_setup(
    existing_config: Option<&Config>,
    new_config: &Config,
    target_host: Option<&str>,
    quiet: bool,
    non_interactive: bool,
) -> Result<()> {
    let (client, host_name) = crate::resolve_docker_client(target_host).await?;
    let is_running = container_is_running(&client, CONTAINER_NAME)
        .await
        .unwrap_or(false);

    let config_changed = existing_config.is_some_and(|old| requires_restart(old, new_config));

    if is_running && !config_changed {
        if !quiet {
            show_running_status(new_config, host_name.as_deref());
        }
        return Ok(());
    }

    if is_running && config_changed {
        let stop_args = crate::commands::StopArgs {
            timeout: 60,
            remove: false,
        };
        cmd_stop(&stop_args, target_host, quiet || non_interactive).await?;
    }

    let start_args = crate::commands::StartArgs {
        port: Some(new_config.opencode_web_port),
        ..Default::default()
    };
    cmd_start(&start_args, target_host, quiet || non_interactive, 0).await?;
    if !quiet {
        maybe_print_iotp_info(&client, host_name.as_deref(), new_config).await;
    }
    Ok(())
}

#[derive(PartialEq)]
enum Action {
    Start,
    Restart,
}

/// Check if config changes require a container restart
fn requires_restart(old: &Config, new: &Config) -> bool {
    if old.opencode_web_port != new.opencode_web_port || old.bind != new.bind {
        return true;
    }
    if COCKPIT_EXPOSED
        && (old.cockpit_port != new.cockpit_port || old.cockpit_enabled != new.cockpit_enabled)
    {
        return true;
    }
    false
}

/// Show status when service is running and no restart needed
fn show_running_status(config: &Config, host: Option<&str>) {
    let msg = crate::format_host_message(host, "Service is already running");
    println!("{}", style(msg).dim());
    println!();

    let bind_addr = if config.bind == "0.0.0.0" || config.bind == "::" {
        "127.0.0.1"
    } else {
        &config.bind
    };

    println!(
        "URL: {}",
        style(format!("http://{}:{}", bind_addr, config.opencode_web_port)).cyan()
    );
}

async fn maybe_print_iotp_info(client: &DockerClient, host: Option<&str>, config: &Config) {
    if !config.users.is_empty() {
        return;
    }

    println!();
    let headline = crate::format_host_message(host, "First-time onboarding");
    println!("{}", style(headline).cyan().bold());

    let snapshot = fetch_iotp_snapshot(client).await;
    if matches!(snapshot.state, IotpState::ActiveUnused)
        && let Some(iotp) = snapshot.otp.as_deref()
    {
        println!(
            "Initial One-Time Password (IOTP): {}",
            style(iotp).green().bold()
        );
        println!(
            "Enter this in the web login first-time setup panel, then enroll a passkey for {}.",
            style("opencoder").cyan()
        );
        return;
    }

    for (idx, line) in build_iotp_fallback_message(
        config.allow_unauthenticated_network,
        snapshot.detail.as_deref(),
    )
    .lines()
    .enumerate()
    {
        if idx == 0 {
            println!("{}", style(line).yellow());
        } else {
            println!("{line}");
        }
    }
}

fn build_iotp_fallback_message(
    allow_unauthenticated_network: bool,
    reason_detail: Option<&str>,
) -> String {
    let mut message = format!(
        "Could not retrieve the Initial One-Time Password (IOTP) from bootstrap state.\n\
Fetch logs with:\n  occ logs\n\
Try extracting IOTP with:\n  {IOTP_FALLBACK_COMMAND}"
    );

    if let Some(reason) = reason_detail {
        message.push_str(&format!("\nReason: {reason}"));
    }

    if allow_unauthenticated_network {
        message.push_str(
            "\nNote: allow_unauthenticated_network=true may skip IOTP generation by design.",
        );
    }

    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_iotp_fallback_message_default() {
        let message = build_iotp_fallback_message(false, None);
        assert!(message.contains("Could not retrieve"));
        assert!(message.contains(IOTP_FALLBACK_COMMAND));
        assert!(!message.contains("allow_unauthenticated_network"));
    }

    #[test]
    fn test_build_iotp_fallback_message_with_unauth_hint() {
        let message = build_iotp_fallback_message(true, None);
        assert!(message.contains(IOTP_FALLBACK_COMMAND));
        assert!(message.contains("allow_unauthenticated_network=true"));
    }

    #[test]
    fn test_build_iotp_fallback_message_includes_reason() {
        let message = build_iotp_fallback_message(false, Some("bootstrap helper unavailable"));
        assert!(message.contains("Reason: bootstrap helper unavailable"));
    }
}
