//! occ host add - Add a new remote host

use anyhow::{Result, bail};
use clap::Args;
use console::style;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use opencode_cloud_core::{
    HostConfig, HostError, detect_distro, get_docker_install_commands, host_exists_in_ssh_config,
    install_docker, load_hosts, query_ssh_config, save_hosts, test_connection,
    verify_docker_installed, write_ssh_config_entry,
};

/// Arguments for host add command
#[derive(Args)]
pub struct HostAddArgs {
    /// Name to identify this host (e.g., "prod-1", "staging")
    pub name: String,

    /// SSH hostname or IP address
    pub hostname: String,

    /// SSH username (default: from SSH config or current user)
    #[arg(short, long)]
    pub user: Option<String>,

    /// SSH port (default: from SSH config or 22)
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

    /// Don't prompt to add host to SSH config
    #[arg(long)]
    pub no_ssh_config: bool,
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

    // Query SSH config for this hostname to auto-fill settings
    let ssh_config_match = query_ssh_config(&args.hostname).unwrap_or_default();

    if !quiet && ssh_config_match.has_settings() {
        println!(
            "{} Found in ~/.ssh/config: {}",
            style("SSH Config:").cyan(),
            ssh_config_match.display_settings()
        );
    }

    // Build host config, preferring explicit args > SSH config > defaults
    let mut config = HostConfig::new(&args.hostname);

    // User: explicit arg > SSH config > current user (HostConfig default)
    let effective_user = args.user.clone().or_else(|| ssh_config_match.user.clone());
    if let Some(user) = &effective_user {
        config = config.with_user(user);
    }

    // Port: explicit arg > SSH config > default (22)
    let effective_port = args.port.or(ssh_config_match.port);
    if let Some(port) = effective_port {
        config = config.with_port(port);
    }

    // Identity file: explicit arg > SSH config
    let effective_identity = args
        .identity_file
        .clone()
        .or_else(|| ssh_config_match.identity_file.clone());
    if let Some(key) = &effective_identity {
        config = config.with_identity_file(key);
    }

    // Jump host: explicit arg > SSH config
    let effective_jump = args
        .jump_host
        .clone()
        .or_else(|| ssh_config_match.proxy_jump.clone());
    if let Some(jump) = &effective_jump {
        config = config.with_jump_host(jump);
    }

    // Groups and description (no SSH config equivalent)
    for group in &args.group {
        config = config.with_group(group);
    }
    if let Some(desc) = &args.description {
        config = config.with_description(desc);
    }

    // Track if user provided custom settings that aren't in SSH config
    let has_custom_settings = args.user.is_some()
        || args.identity_file.is_some()
        || args.port.is_some()
        || args.jump_host.is_some();

    // Test connection unless --no-verify
    let mut verification_succeeded = false;
    if !args.no_verify {
        if !quiet {
            // Show the effective SSH command being used
            println!(
                "{} {}",
                style("SSH Command:").cyan(),
                style(format_ssh_command(&config)).dim()
            );

            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .expect("valid template"),
            );
            spinner.set_message(format!(
                "Testing connection to {}@{}...",
                config.user, args.hostname
            ));
            spinner.enable_steady_tick(std::time::Duration::from_millis(100));

            match test_connection(&config).await {
                Ok(docker_version) => {
                    spinner.finish_with_message(format!(
                        "{} Connected (Docker {})",
                        style("✓").green(),
                        docker_version
                    ));
                    verification_succeeded = true;
                }
                Err(HostError::RemoteDockerUnavailable(_)) => {
                    spinner.finish_with_message(format!(
                        "{} Docker not installed",
                        style("!").yellow()
                    ));
                    eprintln!();

                    // Offer to install Docker
                    if let Some(installed) =
                        offer_docker_installation(&config, &args.hostname, quiet)?
                    {
                        if installed {
                            verification_succeeded = true;
                        }
                    } else {
                        bail!("Docker is required on the remote host");
                    }
                }
                Err(e) => {
                    spinner.finish_with_message(format!("{} Connection failed", style("✗").red()));
                    eprintln!();
                    eprintln!("  {}", e);
                    eprintln!();

                    // Provide helpful tips based on the error
                    print_connection_failure_tips(
                        &config,
                        &args.hostname,
                        args.user.is_none(),
                        args.identity_file.is_none(),
                    );

                    bail!("Connection verification failed");
                }
            }
        } else {
            // Quiet mode - just test, fail silently
            test_connection(&config).await?;
            verification_succeeded = true;
        }
    }

    // Add host to config
    let is_overwrite = hosts.has_host(&args.name);
    hosts.add_host(&args.name, config.clone());

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

        // Offer to add to SSH config if:
        // 1. Verification succeeded
        // 2. User provided custom settings (user, identity, port, jump)
        // 3. Host alias doesn't already exist in SSH config
        // 4. User hasn't disabled this with --no-ssh-config
        if verification_succeeded
            && has_custom_settings
            && !args.no_ssh_config
            && !host_exists_in_ssh_config(&args.name)
        {
            println!();
            let should_add = Confirm::new()
                .with_prompt(format!(
                    "Add '{}' to ~/.ssh/config for easier SSH access?",
                    args.name
                ))
                .default(true)
                .interact()?;

            if should_add {
                match write_ssh_config_entry(
                    &args.name,
                    &args.hostname,
                    args.user.as_deref(),
                    args.port,
                    args.identity_file.as_deref(),
                    args.jump_host.as_deref(),
                ) {
                    Ok(path) => {
                        println!(
                            "  {} Added to {}",
                            style("SSH Config:").green(),
                            path.display()
                        );
                        println!(
                            "  {} You can now use: {}",
                            style("Tip:").dim(),
                            style(format!("ssh {}", args.name)).yellow()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to update SSH config: {}",
                            style("Warning:").yellow(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Offer to install Docker on a remote host
///
/// Returns:
/// - `Ok(Some(true))` - Docker was installed successfully
/// - `Ok(Some(false))` - User declined or installation failed
/// - `Ok(None)` - User declined installation
fn offer_docker_installation(
    config: &HostConfig,
    hostname: &str,
    quiet: bool,
) -> Result<Option<bool>> {
    if quiet {
        return Ok(None);
    }

    println!(
        "  {} Docker is not installed on {}",
        style("Detected:").yellow(),
        style(hostname).cyan()
    );
    println!();

    // Detect the Linux distribution
    let distro = match detect_distro(config) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "  {} Could not detect Linux distribution: {}",
                style("Error:").red(),
                e
            );
            return Ok(None);
        }
    };

    println!(
        "  {} {} ({})",
        style("Distribution:").dim(),
        distro.pretty_name,
        distro.family
    );
    println!();

    // Get the commands that would be run
    let commands = match get_docker_install_commands(&distro) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} {}", style("Error:").red(), e);
            println!();
            println!(
                "  {} Install Docker manually, then re-run this command.",
                style("Tip:").dim()
            );
            return Ok(None);
        }
    };

    // Show what will be done
    println!(
        "  {} The following commands will be run:",
        style("Installation:").cyan()
    );
    for cmd in &commands {
        println!("    {}", style(cmd).dim());
    }
    println!();

    // Ask for confirmation
    let should_install = Confirm::new()
        .with_prompt("Install Docker on the remote host?")
        .default(true)
        .interact()?;

    if !should_install {
        println!();
        println!(
            "  {} You can install Docker manually, then run:",
            style("Tip:").dim()
        );
        println!(
            "       {}",
            style(format!("occ host add {} {}", hostname, hostname)).yellow()
        );
        return Ok(None);
    }

    println!();

    // Create a spinner for installation
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Installing Docker...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    // Run installation with output streaming
    match install_docker(config, &distro, |line| {
        // Update spinner message with latest output
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            spinner.set_message(format!("Installing: {}", truncate_str(trimmed, 50)));
        }
    }) {
        Ok(()) => {
            spinner.finish_with_message(format!("{} Docker installed", style("✓").green()));
        }
        Err(e) => {
            spinner.finish_with_message(format!("{} Installation failed: {}", style("✗").red(), e));
            return Ok(Some(false));
        }
    }

    println!();
    println!(
        "  {} Group membership changes require a new SSH session.",
        style("Note:").yellow()
    );

    // Verify Docker is working (may need sudo if group not yet active)
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Verifying Docker installation...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    match verify_docker_installed(config) {
        Ok(version) => {
            spinner.finish_with_message(format!(
                "{} Docker {} verified",
                style("✓").green(),
                version
            ));
            Ok(Some(true))
        }
        Err(e) => {
            spinner.finish_with_message(format!("{} Verification: {}", style("!").yellow(), e));
            println!();
            println!(
                "  {} Docker was installed but verification failed.",
                style("Note:").yellow()
            );
            println!(
                "       This is often because the user needs to reconnect for group membership."
            );
            println!(
                "       Try: {}",
                style("ssh <host> docker --version").yellow()
            );
            // Still count as success since Docker was installed
            Ok(Some(true))
        }
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format the effective SSH command for display
///
/// Shows the user exactly what SSH command will be used to connect,
/// including all options derived from their input and SSH config.
fn format_ssh_command(config: &HostConfig) -> String {
    let mut parts = vec!["ssh".to_string()];

    // Port (if non-default)
    if let Some(port) = config.port {
        if port != 22 {
            parts.push(format!("-p {}", port));
        }
    }

    // Identity file
    if let Some(key) = &config.identity_file {
        parts.push(format!("-i {}", key));
    }

    // Jump host
    if let Some(jump) = &config.jump_host {
        parts.push(format!("-J {}", jump));
    }

    // Target: user@hostname
    parts.push(format!("{}@{}", config.user, config.hostname));

    parts.join(" ")
}

/// Print helpful tips when connection verification fails
fn print_connection_failure_tips(
    config: &HostConfig,
    hostname: &str,
    no_user_specified: bool,
    no_identity_specified: bool,
) {
    println!("{}", style("Troubleshooting tips:").yellow());

    let mut tip_num = 1;

    // If no user was specified, suggest common cloud usernames
    if no_user_specified {
        println!(
            "  {} Cloud instances often use specific usernames:",
            style(format!("{}.", tip_num)).dim()
        );
        println!("     • AWS EC2: {}", style("--user ubuntu").yellow());
        println!(
            "     • AWS EC2 (Amazon Linux): {}",
            style("--user ec2-user").yellow()
        );
        println!(
            "     • GCP: {}",
            style("--user <your-gcp-username>").yellow()
        );
        println!("     • Azure: {}", style("--user azureuser").yellow());
        println!("     • DigitalOcean: {}", style("--user root").yellow());
        println!();
        tip_num += 1;
    }

    // If no identity file was specified, suggest available keys
    if no_identity_specified {
        let keys = find_ssh_keys();
        if !keys.is_empty() {
            println!(
                "  {} Try specifying an identity file:",
                style(format!("{}.", tip_num)).dim()
            );
            for key in keys.iter().take(5) {
                // Show up to 5 keys
                println!(
                    "     {}",
                    style(format!("--identity-file {}", key)).yellow()
                );
            }
            if keys.len() > 5 {
                println!(
                    "     {} ({} more keys in ~/.ssh/)",
                    style("...").dim(),
                    keys.len() - 5
                );
            }
            println!();
            tip_num += 1;
        }
    }

    // Suggest verifying SSH access manually
    let ssh_cmd = if let Some(key) = &config.identity_file {
        format!("ssh -i {} {}@{}", key, config.user, hostname)
    } else {
        format!("ssh {}@{}", config.user, hostname)
    };
    println!(
        "  {} Verify SSH access manually: {}",
        style(format!("{}.", tip_num)).dim(),
        style(&ssh_cmd).yellow()
    );
    tip_num += 1;

    // Suggest checking Docker
    println!(
        "  {} Ensure Docker is running on the remote host",
        style(format!("{}.", tip_num)).dim()
    );
    tip_num += 1;

    // Suggest --no-verify
    println!(
        "  {} Use {} to add the host without verification",
        style(format!("{}.", tip_num)).dim(),
        style("--no-verify").yellow()
    );
}

/// Find SSH private keys in ~/.ssh/
fn find_ssh_keys() -> Vec<String> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let ssh_dir = home.join(".ssh");
    if !ssh_dir.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(&ssh_dir) else {
        return Vec::new();
    };

    let mut keys = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // Skip public keys, known_hosts, config, and other non-key files
        if name.ends_with(".pub")
            || name == "known_hosts"
            || name == "known_hosts.old"
            || name == "config"
            || name == "authorized_keys"
            || name.starts_with(".")
        {
            continue;
        }

        // Check if it looks like a private key (common patterns)
        let is_likely_key = name.starts_with("id_")
            || name.ends_with(".pem")
            || name.ends_with("_rsa")
            || name.ends_with("_ed25519")
            || name.ends_with("_ecdsa")
            || name.ends_with("_dsa")
            || name.contains("key");

        // Also check file contents for "PRIVATE KEY" header if name doesn't match patterns
        let is_key = if is_likely_key {
            true
        } else {
            // Read first line to check for private key header
            std::fs::read_to_string(&path)
                .map(|content| content.contains("PRIVATE KEY"))
                .unwrap_or(false)
        };

        if is_key {
            keys.push(path.display().to_string());
        }
    }

    // Sort for consistent output
    keys.sort();
    keys
}
