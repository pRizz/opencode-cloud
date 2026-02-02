//! Setup command implementation
//!
//! Runs the interactive setup wizard.

use anyhow::Result;
use clap::Args;
use console::style;
use dialoguer::Confirm;
use opencode_cloud_core::docker::{CONTAINER_NAME, container_is_running};
use opencode_cloud_core::{Config, load_config_or_default, save_config};

use crate::commands::{cmd_start, cmd_stop};
use crate::constants::COCKPIT_EXPOSED;
use crate::wizard::run_wizard;

/// Arguments for the setup command
#[derive(Args)]
pub struct SetupArgs {
    /// Skip wizard if auth credentials are already configured
    #[arg(long, short)]
    pub yes: bool,

    /// Non-interactive bootstrap for automated environments
    #[arg(long, alias = "non-interactive", alias = "headless")]
    pub bootstrap: bool,

    /// Run setup for a remote host instead of local Docker
    #[arg(long)]
    pub host: Option<String>,
}

/// Run the setup command
pub async fn cmd_setup(args: &SetupArgs, quiet: bool) -> Result<()> {
    // Load existing config (or create default)
    let existing_config = load_config_or_default().ok();

    if args.bootstrap {
        return run_bootstrap_setup(existing_config, args.host.as_deref(), quiet).await;
    }

    // Handle --yes flag for non-interactive mode
    if args.yes {
        if let Some(ref config) = existing_config {
            if config.has_required_auth() {
                if !quiet {
                    println!("{}", style("Configuration already set").green());
                }
                return Ok(());
            }
        }

        anyhow::bail!(
            "Non-interactive mode requires auth credentials to be pre-set.\n\n\
            Use:\n  \
            occ config set username <user>\n  \
            occ config set password"
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
    let (client, host_name) = crate::resolve_docker_client(args.host.as_deref()).await?;
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
        cmd_stop(&stop_args, args.host.as_deref(), quiet).await?;
        println!();
    }

    // Start the service
    let start_args = crate::commands::StartArgs {
        port: Some(new_config.opencode_web_port),
        open: false,
        no_daemon: false,
        pull_sandbox_image: false,
        cached_rebuild_sandbox_image: false,
        full_rebuild_sandbox_image: false,
        ignore_version: false,
        no_update_check: false,
        mounts: Vec::new(),
        no_mounts: false,
    };
    cmd_start(&start_args, args.host.as_deref(), quiet, 0).await?;

    Ok(())
}

async fn run_bootstrap_setup(
    existing_config: Option<Config>,
    host: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let new_config = build_bootstrap_config(existing_config.clone());
    save_config(&new_config)?;

    if quiet {
        return start_or_restart_after_setup(
            existing_config.as_ref(),
            &new_config,
            host,
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

    start_or_restart_after_setup(existing_config.as_ref(), &new_config, host, quiet, true).await
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
    host: Option<&str>,
    quiet: bool,
    non_interactive: bool,
) -> Result<()> {
    let (client, host_name) = crate::resolve_docker_client(host).await?;
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
        cmd_stop(&stop_args, host, quiet || non_interactive).await?;
    }

    let start_args = crate::commands::StartArgs {
        port: Some(new_config.opencode_web_port),
        open: false,
        no_daemon: false,
        pull_sandbox_image: false,
        cached_rebuild_sandbox_image: false,
        full_rebuild_sandbox_image: false,
        ignore_version: false,
        no_update_check: false,
        mounts: Vec::new(),
        no_mounts: false,
    };
    cmd_start(&start_args, host, quiet || non_interactive, 0).await?;
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
