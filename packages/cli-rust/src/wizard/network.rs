//! Network configuration prompts
//!
//! Handles port and hostname configuration.

use anyhow::{Result, anyhow};
use console::{Term, style};
use dialoguer::{Confirm, Input, Select};
use std::net::TcpListener;

/// Handle Ctrl+C by restoring cursor and returning error
fn handle_interrupt() -> anyhow::Error {
    let _ = Term::stdout().show_cursor();
    anyhow!("Setup cancelled")
}

/// Check if a port is available for binding
fn check_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find the next available port starting from the given port
fn find_next_available_port(start: u16) -> Option<u16> {
    (start..start.saturating_add(100)).find(|&p| check_port_available(p))
}

/// Validate port number
fn validate_port(input: &str) -> Result<u16, String> {
    let port: u16 = input
        .parse()
        .map_err(|_| "Invalid port number. Must be between 1 and 65535.".to_string())?;

    if port == 0 {
        return Err("Port 0 is reserved. Use a port between 1 and 65535.".to_string());
    }

    Ok(port)
}

/// Prompt for port number
///
/// Shows explanation and validates input.
/// Checks port availability and suggests alternatives if in use.
pub fn prompt_port(step: usize, total: usize, default_port: u16) -> Result<u16> {
    println!(
        "{} {}",
        style(format!("[{step}/{total}]")).dim(),
        style("Port Configuration").bold()
    );
    println!();
    println!("Port for the opencode web UI");
    println!();

    loop {
        let port_str: String = Input::new()
            .with_prompt(format!("Port (default: {default_port})"))
            .default(default_port.to_string())
            .validate_with(|input: &String| validate_port(input).map(|_| ()))
            .interact_text()
            .map_err(|_| handle_interrupt())?;

        let port = validate_port(&port_str).expect("validated above");

        // Warn about privileged ports
        if port < 1024 {
            println!(
                "{}",
                style("Note: Ports below 1024 may require elevated privileges").yellow()
            );
        }

        // Check port availability
        if !check_port_available(port) {
            println!("{}", style(format!("Port {port} is already in use")).red());

            if let Some(next_port) = find_next_available_port(port) {
                let use_next = Confirm::new()
                    .with_prompt(format!("Use port {next_port} instead?"))
                    .default(true)
                    .interact()
                    .map_err(|_| handle_interrupt())?;

                if use_next {
                    println!();
                    return Ok(next_port);
                }
            }
            println!();
            continue;
        }

        println!();
        return Ok(port);
    }
}

/// Prompt for hostname/bind address
///
/// Offers localhost vs 0.0.0.0 selection with explanations.
pub fn prompt_hostname(step: usize, total: usize, default_bind: &str) -> Result<String> {
    println!(
        "{} {}",
        style(format!("[{step}/{total}]")).dim(),
        style("Network Binding").bold()
    );
    println!();
    println!("Network binding address:");
    println!(
        "  {}  - Accessible only from this machine (recommended)",
        style("localhost").cyan()
    );
    println!(
        "  {}    - Accessible from network (requires firewall/auth)",
        style("0.0.0.0").cyan()
    );
    println!();

    let options = vec!["localhost (local only)", "0.0.0.0 (network accessible)"];

    let default_index = if default_bind == "0.0.0.0" { 1 } else { 0 };

    let selection = Select::new()
        .with_prompt("Select binding")
        .items(&options)
        .default(default_index)
        .interact()
        .map_err(|_| handle_interrupt())?;

    let bind = match selection {
        0 => "localhost".to_string(),
        1 => {
            println!();
            println!(
                "{}",
                style("Warning: Network exposure enabled. Ensure firewall rules and authentication are configured.")
                    .yellow()
            );
            "0.0.0.0".to_string()
        }
        _ => unreachable!(),
    };

    println!();
    Ok(bind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_port_valid() {
        assert!(validate_port("3000").is_ok());
        assert!(validate_port("80").is_ok());
        assert!(validate_port("65535").is_ok());
        assert!(validate_port("1").is_ok());
    }

    #[test]
    fn test_validate_port_invalid() {
        assert!(validate_port("0").is_err());
        assert!(validate_port("-1").is_err());
        assert!(validate_port("65536").is_err());
        assert!(validate_port("abc").is_err());
        assert!(validate_port("").is_err());
    }

    #[test]
    fn test_check_port_available_privileged() {
        // Port 1 is privileged and typically unavailable
        assert!(!check_port_available(1));
    }

    #[test]
    fn test_find_next_port_finds_available() {
        // Should find something in the dynamic port range
        let result = find_next_available_port(49152);
        assert!(result.is_some());
    }
}
