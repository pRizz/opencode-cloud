use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CliInstallMethod {
    Cargo,
    Npm,
}

impl CliInstallMethod {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            CliInstallMethod::Cargo => "cargo install",
            CliInstallMethod::Npm => "npm install -g",
        }
    }

    pub(crate) fn run_update(&self, target: Option<&str>) -> Result<(), String> {
        let mut args: Vec<String> = match self {
            CliInstallMethod::Cargo => vec!["install".to_string(), "opencode-cloud".to_string()],
            CliInstallMethod::Npm => vec![
                "install".to_string(),
                "-g".to_string(),
                "opencode-cloud".to_string(),
            ],
        };

        if let Some(version) = target {
            match self {
                CliInstallMethod::Cargo => {
                    args.push("--version".to_string());
                    args.push(version.to_string());
                }
                CliInstallMethod::Npm => {
                    let _ = args.pop();
                    args.push(format!("opencode-cloud@{version}"));
                }
            }
        }

        let program = match self {
            CliInstallMethod::Cargo => "cargo",
            CliInstallMethod::Npm => "npm",
        };

        let status = Command::new(program)
            .args(&args)
            .status()
            .map_err(|e| format!("Failed to execute {program}: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("{program} update failed with status {status}"))
        }
    }
}

pub(crate) fn detect_install_method() -> Option<CliInstallMethod> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_str = exe_path.to_string_lossy();

    if exe_str.contains("node_modules") || exe_str.contains("@opencode-cloud") {
        return Some(CliInstallMethod::Npm);
    }

    if exe_str.contains(".cargo") || exe_str.contains("cargo/bin") {
        return Some(CliInstallMethod::Cargo);
    }

    None
}

pub(crate) fn is_dev_binary() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let exe_str = exe_path.to_string_lossy();

    exe_str.contains("/target/debug/")
}

pub(crate) fn cli_platform_label() -> &'static str {
    if is_dev_binary() {
        return "Rust CLI";
    }

    match detect_install_method() {
        Some(CliInstallMethod::Cargo) => "Rust CLI",
        Some(CliInstallMethod::Npm) => "Node.js CLI",
        None => "CLI",
    }
}
