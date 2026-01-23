//! occ host add - Add a new remote host

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use opencode_cloud_core::{
    HostConfig, load_hosts, save_hosts, test_connection,
};

/// Arguments for host add command
#[derive(Args)]
pub struct HostAddArgs {
    /// Name to identify this host (e.g., "prod-1", "staging")
    pub name: String,

    /// SSH hostname or IP address
    pub hostname: String,

    /// SSH username (default: current user)
    #[arg(short, long)]
    pub user: Option<String>,

    /// SSH port (default: 22)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Path to SSH identity file (private key)
    #[arg(short, long)]
    pub identity_file: Option<String>,

    /// Jump host for ProxyJump (user@host:port format)
    #[arg(short = 'J', long)]
    pub jump_host: Option<String>,

    /// Group/tag for organization (can be specified multiple times)
    #[arg(short, long)]
    pub group: Vec<String>,

    /// Description for this host
    #[arg(short, long)]
    pub description: Option<String>,

    /// Skip connection verification
    #[arg(long)]
    pub no_verify: bool,

    /// Overwrite if host already exists
    #[arg(long)]
    pub force: bool,
}

pub async fn cmd_host_add(args: &HostAddArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Load existing hosts
    let mut hosts = load_hosts()?;

    // Check if host already exists
    if hosts.has_host(&args.name) && !args.force {
        bail!(
            "Host '{}' already exists. Use --force to overwrite, or choose a different name.",
            args.name
        );
    }

    // Build host config
    let mut config = HostConfig::new(&args.hostname);

    if let Some(user) = &args.user {
        config = config.with_user(user);
    }
    if let Some(port) = args.port {
        config = config.with_port(port);
    }
    if let Some(key) = &args.identity_file {
        config = config.with_identity_file(key);
    }
    if let Some(jump) = &args.jump_host {
        config = config.with_jump_host(jump);
    }
    for group in &args.group {
        config = config.with_group(group);
    }
    if let Some(desc) = &args.description {
        config = config.with_description(desc);
    }

    // Test connection unless --no-verify
    if !args.no_verify {
        if !quiet {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .expect("valid template"),
            );
            spinner.set_message(format!("Testing connection to {}...", args.hostname));
            spinner.enable_steady_tick(std::time::Duration::from_millis(100));

            match test_connection(&config).await {
                Ok(docker_version) => {
                    spinner.finish_with_message(format!(
                        "{} Connected (Docker {})",
                        style("✓").green(),
                        docker_version
                    ));
                }
                Err(e) => {
                    spinner.finish_with_message(format!("{} Connection failed", style("✗").red()));
                    eprintln!();
                    eprintln!("  {}", e);
                    eprintln!();
                    eprintln!(
                        "  {} Use {} to add the host anyway.",
                        style("Tip:").cyan(),
                        style("--no-verify").yellow()
                    );
                    bail!("Connection verification failed");
                }
            }
        } else {
            // Quiet mode - just test, fail silently
            test_connection(&config).await?;
        }
    }

    // Add host to config
    let is_overwrite = hosts.has_host(&args.name);
    hosts.add_host(&args.name, config);

    // Save
    save_hosts(&hosts)?;

    if !quiet {
        if is_overwrite {
            println!(
                "{} Host '{}' updated ({}).",
                style("Updated:").yellow(),
                style(&args.name).cyan(),
                args.hostname
            );
        } else {
            println!(
                "{} Host '{}' added ({}).",
                style("Added:").green(),
                style(&args.name).cyan(),
                args.hostname
            );
        }

        if args.no_verify {
            println!(
                "  {} Connection not verified. Run {} to test.",
                style("Note:").dim(),
                style(format!("occ host test {}", args.name)).yellow()
            );
        }
    }

    Ok(())
}
