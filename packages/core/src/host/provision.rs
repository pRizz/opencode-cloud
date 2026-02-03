//! Remote host provisioning
//!
//! Functions to detect Linux distribution and install Docker on remote hosts.

use std::process::{Command, Stdio};

use super::error::HostError;
use super::schema::HostConfig;

/// Linux distribution family
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistroFamily {
    /// Debian, Ubuntu, and derivatives (apt-based)
    Debian,
    /// RHEL, CentOS, Fedora, Amazon Linux (dnf/yum-based)
    RedHat,
    /// Alpine Linux (apk-based)
    Alpine,
    /// Arch Linux (pacman-based)
    Arch,
    /// SUSE/openSUSE (zypper-based)
    Suse,
    /// Unknown distribution
    Unknown(String),
}

impl std::fmt::Display for DistroFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistroFamily::Debian => write!(f, "Debian/Ubuntu"),
            DistroFamily::RedHat => write!(f, "RHEL/Amazon Linux"),
            DistroFamily::Alpine => write!(f, "Alpine"),
            DistroFamily::Arch => write!(f, "Arch"),
            DistroFamily::Suse => write!(f, "SUSE"),
            DistroFamily::Unknown(id) => write!(f, "Unknown ({id})"),
        }
    }
}

/// Detected distribution information
#[derive(Debug, Clone)]
pub struct DistroInfo {
    /// Distribution family (Debian, RedHat, etc.)
    pub family: DistroFamily,
    /// Distribution ID (e.g., "ubuntu", "amzn", "debian")
    pub id: String,
    /// Pretty name (e.g., "Ubuntu 24.04 LTS")
    pub pretty_name: String,
    /// Version ID (e.g., "24.04", "2023")
    pub version_id: Option<String>,
}

/// Detect the Linux distribution on a remote host
///
/// Runs `cat /etc/os-release` via SSH to parse distribution info.
pub fn detect_distro(host: &HostConfig) -> Result<DistroInfo, HostError> {
    let output = run_ssh_command(host, "cat /etc/os-release")?;

    parse_os_release(&output)
}

/// Parse /etc/os-release content into DistroInfo
fn parse_os_release(content: &str) -> Result<DistroInfo, HostError> {
    let mut id = String::new();
    let mut id_like = String::new();
    let mut pretty_name = String::new();
    let mut version_id = None;

    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"');
            match key {
                "ID" => id = value.to_lowercase(),
                "ID_LIKE" => id_like = value.to_lowercase(),
                "PRETTY_NAME" => pretty_name = value.to_string(),
                "VERSION_ID" => version_id = Some(value.to_string()),
                _ => {}
            }
        }
    }

    if id.is_empty() {
        return Err(HostError::ConnectionFailed(
            "Could not detect Linux distribution".to_string(),
        ));
    }

    // Determine distribution family
    let family = match id.as_str() {
        "ubuntu" | "debian" | "linuxmint" | "pop" | "elementary" | "raspbian" => {
            DistroFamily::Debian
        }
        "amzn" | "rhel" | "centos" | "fedora" | "rocky" | "almalinux" | "ol" => {
            DistroFamily::RedHat
        }
        "alpine" => DistroFamily::Alpine,
        "arch" | "manjaro" | "endeavouros" => DistroFamily::Arch,
        "opensuse" | "sles" | "opensuse-leap" | "opensuse-tumbleweed" => DistroFamily::Suse,
        _ => {
            // Check ID_LIKE for derivatives
            if id_like.contains("debian") || id_like.contains("ubuntu") {
                DistroFamily::Debian
            } else if id_like.contains("rhel")
                || id_like.contains("fedora")
                || id_like.contains("centos")
            {
                DistroFamily::RedHat
            } else if id_like.contains("arch") {
                DistroFamily::Arch
            } else if id_like.contains("suse") {
                DistroFamily::Suse
            } else {
                DistroFamily::Unknown(id.clone())
            }
        }
    };

    Ok(DistroInfo {
        family,
        id,
        pretty_name,
        version_id,
    })
}

/// Install Docker on a remote host
///
/// Returns a vector of commands that will be executed (for user review).
pub fn get_docker_install_commands(distro: &DistroInfo) -> Result<Vec<&'static str>, HostError> {
    match &distro.family {
        DistroFamily::Debian => Ok(vec![
            // Update package index
            "sudo apt-get update",
            // Install prerequisites
            "sudo apt-get install -y ca-certificates curl gnupg",
            // Add Docker's official GPG key
            "sudo install -m 0755 -d /etc/apt/keyrings",
            "curl -fsSL https://download.docker.com/linux/$(. /etc/os-release && echo \"$ID\")/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg",
            "sudo chmod a+r /etc/apt/keyrings/docker.gpg",
            // Set up the repository
            "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$(. /etc/os-release && echo \"$ID\") $(. /etc/os-release && echo \"$VERSION_CODENAME\") stable\" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null",
            // Install Docker
            "sudo apt-get update",
            "sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin",
            // Start Docker
            "sudo systemctl enable docker",
            "sudo systemctl start docker",
            // Add current user to docker group
            "sudo usermod -aG docker $USER",
        ]),

        DistroFamily::RedHat => {
            // Amazon Linux 2023 uses dnf, Amazon Linux 2 uses yum
            // We'll use a command that works for both
            Ok(vec![
                // Install Docker (Amazon Linux uses amazon-linux-extras or dnf)
                "sudo yum install -y docker || sudo dnf install -y docker",
                // Start Docker
                "sudo systemctl enable docker",
                "sudo systemctl start docker",
                // Add current user to docker group
                "sudo usermod -aG docker $USER",
            ])
        }

        DistroFamily::Alpine => Ok(vec![
            "sudo apk add docker docker-cli-compose",
            "sudo rc-update add docker boot",
            "sudo service docker start",
            "sudo addgroup $USER docker",
        ]),

        DistroFamily::Arch => Ok(vec![
            "sudo pacman -Sy --noconfirm docker docker-compose",
            "sudo systemctl enable docker",
            "sudo systemctl start docker",
            "sudo usermod -aG docker $USER",
        ]),

        DistroFamily::Suse => Ok(vec![
            "sudo zypper install -y docker docker-compose",
            "sudo systemctl enable docker",
            "sudo systemctl start docker",
            "sudo usermod -aG docker $USER",
        ]),

        DistroFamily::Unknown(id) => Err(HostError::ConnectionFailed(format!(
            "Unsupported Linux distribution: {id}. Please install Docker manually."
        ))),
    }
}

/// Execute Docker installation on remote host
///
/// Runs the installation commands via SSH and captures output.
pub fn install_docker(
    host: &HostConfig,
    distro: &DistroInfo,
    on_output: impl Fn(&str),
) -> Result<(), HostError> {
    let commands = get_docker_install_commands(distro)?;

    // Combine all commands with && to fail fast
    let combined = commands.join(" && ");

    on_output(&format!("Installing Docker on {} host...", distro.family));

    // Run the installation
    run_ssh_command_with_output(host, &combined, on_output)?;

    Ok(())
}

/// Run a command on remote host via SSH and return output
fn run_ssh_command(host: &HostConfig, command: &str) -> Result<String, HostError> {
    let mut cmd = build_ssh_command(host);
    cmd.arg(command);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HostError::SshSpawn("SSH not found. Install OpenSSH client.".to_string())
        } else {
            HostError::SshSpawn(e.to_string())
        }
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(HostError::ConnectionFailed(stderr.to_string()))
    }
}

/// Run a command on remote host via SSH with streaming output
fn run_ssh_command_with_output(
    host: &HostConfig,
    command: &str,
    on_output: impl Fn(&str),
) -> Result<(), HostError> {
    use std::io::{BufRead, BufReader};

    let mut cmd = build_ssh_command(host);

    // Request a pseudo-terminal for interactive commands (like sudo)
    cmd.arg("-t").arg("-t");
    cmd.arg(command);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HostError::SshSpawn("SSH not found. Install OpenSSH client.".to_string())
        } else {
            HostError::SshSpawn(e.to_string())
        }
    })?;

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            on_output(&line);
        }
    }

    let status = child
        .wait()
        .map_err(|e| HostError::SshSpawn(e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(HostError::ConnectionFailed(
            "Docker installation failed".to_string(),
        ))
    }
}

/// Build base SSH command with host config
fn build_ssh_command(host: &HostConfig) -> Command {
    let mut cmd = Command::new("ssh");

    // Standard options
    cmd.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=30")
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new");

    // Host-specific options (port, identity, jump, user@host)
    cmd.args(host.ssh_args());

    cmd
}

/// Verify Docker is working after installation
///
/// Note: Due to group membership changes, this may fail until the user
/// reconnects. We run with sudo as a fallback.
pub fn verify_docker_installed(host: &HostConfig) -> Result<String, HostError> {
    // Try without sudo first (if group membership is active)
    let output = run_ssh_command(
        host,
        "docker version --format '{{.Server.Version}}' 2>/dev/null || sudo docker version --format '{{.Server.Version}}'",
    );

    match output {
        Ok(version) => Ok(version.trim().to_string()),
        Err(_) => Err(HostError::RemoteDockerUnavailable(
            "Docker installed but not accessible. You may need to reconnect for group membership to take effect.".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_os_release_ubuntu() {
        let content = r#"
PRETTY_NAME="Ubuntu 24.04 LTS"
NAME="Ubuntu"
VERSION_ID="24.04"
VERSION="24.04 LTS (Noble Numbat)"
VERSION_CODENAME=noble
ID=ubuntu
ID_LIKE=debian
"#;
        let info = parse_os_release(content).unwrap();
        assert_eq!(info.family, DistroFamily::Debian);
        assert_eq!(info.id, "ubuntu");
        assert_eq!(info.version_id, Some("24.04".to_string()));
    }

    #[test]
    fn test_parse_os_release_amazon_linux() {
        let content = r#"
NAME="Amazon Linux"
VERSION="2023"
ID="amzn"
ID_LIKE="fedora"
VERSION_ID="2023"
PRETTY_NAME="Amazon Linux 2023"
"#;
        let info = parse_os_release(content).unwrap();
        assert_eq!(info.family, DistroFamily::RedHat);
        assert_eq!(info.id, "amzn");
    }

    #[test]
    fn test_parse_os_release_debian() {
        let content = r#"
PRETTY_NAME="Debian GNU/Linux 12 (bookworm)"
NAME="Debian GNU/Linux"
VERSION_ID="12"
VERSION="12 (bookworm)"
ID=debian
"#;
        let info = parse_os_release(content).unwrap();
        assert_eq!(info.family, DistroFamily::Debian);
        assert_eq!(info.id, "debian");
    }

    #[test]
    fn test_get_docker_install_commands() {
        let debian_info = DistroInfo {
            family: DistroFamily::Debian,
            id: "ubuntu".to_string(),
            pretty_name: "Ubuntu 24.04".to_string(),
            version_id: Some("24.04".to_string()),
        };
        let commands = get_docker_install_commands(&debian_info).unwrap();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.contains("docker")));

        let redhat_info = DistroInfo {
            family: DistroFamily::RedHat,
            id: "amzn".to_string(),
            pretty_name: "Amazon Linux 2023".to_string(),
            version_id: Some("2023".to_string()),
        };
        let commands = get_docker_install_commands(&redhat_info).unwrap();
        assert!(!commands.is_empty());
    }
}
