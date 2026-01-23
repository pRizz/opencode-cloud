//! occ host test - Test connection to a host

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use opencode_cloud_core::{load_hosts, test_connection};
use std::time::Duration;

/// Arguments for host test command
#[derive(Args)]
pub struct HostTestArgs {
    /// Name of the host to test
    pub name: String,
}

pub async fn cmd_host_test(args: &HostTestArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let hosts = load_hosts()?;

    let config = hosts
        .get_host(&args.name)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", args.name))?;

    if quiet {
        // Quiet mode: exit 0 on success, 1 on failure
        match test_connection(config).await {
            Ok(_) => return Ok(()),
            Err(_) => std::process::exit(1),
        }
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message(format!(
        "Testing connection to {} ({}@{})...",
        style(&args.name).cyan(),
        config.user,
        config.hostname
    ));
    spinner.enable_steady_tick(Duration::from_millis(100));

    match test_connection(config).await {
        Ok(docker_version) => {
            spinner.finish_with_message(format!(
                "{} Connection successful",
                style("✓").green().bold()
            ));
            println!();
            println!("  {:<15} {}", style("Host:").dim(), args.name);
            println!(
                "  {:<15} {}@{}",
                style("SSH:").dim(),
                config.user,
                config.hostname
            );
            println!("  {:<15} {}", style("Docker:").dim(), docker_version);
            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message(format!("{} Connection failed", style("✗").red().bold()));
            println!();
            println!("  {}", e);
            println!();

            // Provide troubleshooting hints
            println!("{}", style("Troubleshooting:").yellow());
            println!(
                "  1. Verify SSH access: ssh {}@{}",
                config.user, config.hostname
            );
            println!(
                "  2. Check Docker is running: ssh {}@{} docker info",
                config.user, config.hostname
            );
            if config.identity_file.is_some() {
                println!(
                    "  3. Ensure key is loaded: ssh-add {}",
                    config.identity_file.as_ref().unwrap()
                );
            } else {
                println!("  3. Ensure key is loaded: ssh-add");
            }

            bail!("Connection test failed");
        }
    }
}
