//! Configuration validation with actionable error messages
//!
//! Validates the configuration and provides exact commands to fix issues.

use super::schema::{Config, validate_bind_address};
use console::style;

/// A configuration validation error with an actionable fix command
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The config field that has an error
    pub field: String,
    /// Description of what's wrong
    pub message: String,
    /// Exact occ command to fix the issue
    pub fix_command: String,
}

/// A configuration validation warning (non-fatal)
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// The config field with a potential issue
    pub field: String,
    /// Description of the warning
    pub message: String,
    /// Suggested occ command to address the warning
    pub fix_command: String,
}

/// Validate configuration and return warnings or first error
///
/// Returns Ok(warnings) if validation passes (possibly with non-fatal warnings).
/// Returns Err(error) on the first fatal validation error encountered.
///
/// Validation is performed in order, stopping at the first error.
pub fn validate_config(config: &Config) -> Result<Vec<ValidationWarning>, ValidationError> {
    let mut warnings = Vec::new();

    // Port validation
    if config.opencode_web_port < 1024 {
        return Err(ValidationError {
            field: "opencode_web_port".to_string(),
            message: "Port must be >= 1024 (non-privileged)".to_string(),
            fix_command: "occ config set opencode_web_port 3000".to_string(),
        });
    }
    // Note: No need to check > 65535 - u16 type enforces this limit

    // Bind address validation
    if let Err(msg) = validate_bind_address(&config.bind_address) {
        return Err(ValidationError {
            field: "bind_address".to_string(),
            message: msg,
            fix_command: "occ config set bind_address 127.0.0.1".to_string(),
        });
    }

    // Boot mode validation
    if config.boot_mode != "user" && config.boot_mode != "system" {
        return Err(ValidationError {
            field: "boot_mode".to_string(),
            message: "boot_mode must be 'user' or 'system'".to_string(),
            fix_command: "occ config set boot_mode user".to_string(),
        });
    }

    // Rate limit validation
    if config.rate_limit_attempts == 0 {
        return Err(ValidationError {
            field: "rate_limit_attempts".to_string(),
            message: "rate_limit_attempts must be > 0".to_string(),
            fix_command: "occ config set rate_limit_attempts 5".to_string(),
        });
    }

    if config.rate_limit_window_seconds == 0 {
        return Err(ValidationError {
            field: "rate_limit_window_seconds".to_string(),
            message: "rate_limit_window_seconds must be > 0".to_string(),
            fix_command: "occ config set rate_limit_window_seconds 60".to_string(),
        });
    }

    // Warnings (non-fatal)

    // Network exposure without auth
    if config.is_network_exposed()
        && config.users.is_empty()
        && !config.allow_unauthenticated_network
    {
        warnings.push(ValidationWarning {
            field: "bind_address".to_string(),
            message: "Network exposed without authentication".to_string(),
            fix_command: "occ user add".to_string(),
        });
    }

    // Legacy auth fields present
    if let Some(ref username) = config.auth_username {
        if !username.is_empty() {
            warnings.push(ValidationWarning {
                field: "auth_username".to_string(),
                message: "Legacy auth fields present; consider using 'occ user add' instead"
                    .to_string(),
                fix_command: "occ config set auth_username ''".to_string(),
            });
        }
    }

    if let Some(ref password) = config.auth_password {
        if !password.is_empty() {
            warnings.push(ValidationWarning {
                field: "auth_password".to_string(),
                message: "Legacy auth fields present; consider using 'occ user add' instead"
                    .to_string(),
                fix_command: "occ config set auth_password ''".to_string(),
            });
        }
    }

    Ok(warnings)
}

/// Display a validation error with styled formatting
pub fn display_validation_error(error: &ValidationError) {
    eprintln!();
    eprintln!("{}", style("Error: Configuration error").red().bold());
    eprintln!();
    eprintln!("  {}  {}", style("Field:").dim(), error.field);
    eprintln!("  {}  {}", style("Problem:").dim(), error.message);
    eprintln!();
    eprintln!("{}:", style("To fix, run").dim());
    eprintln!("  {}", style(&error.fix_command).cyan());
    eprintln!();
}

/// Display a validation warning with styled formatting
pub fn display_validation_warning(warning: &ValidationWarning) {
    eprintln!();
    eprintln!(
        "{}",
        style("Warning: Configuration warning").yellow().bold()
    );
    eprintln!();
    eprintln!("  {}  {}", style("Field:").dim(), warning.field);
    eprintln!("  {}  {}", style("Issue:").dim(), warning.message);
    eprintln!();
    eprintln!("{}:", style("To address, run").dim());
    eprintln!("  {}", style(&warning.fix_command).cyan());
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config_passes() {
        let config = Config::default();
        let result = validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_port_too_low() {
        let config = Config {
            opencode_web_port: 80,
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field, "opencode_web_port");
        assert!(err.message.contains("1024"));
    }

    // Note: No test for port > 65535 - u16 type enforces this limit at compile time

    #[test]
    fn test_invalid_bind_address() {
        let config = Config {
            bind_address: "not-an-ip".to_string(),
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field, "bind_address");
    }

    #[test]
    fn test_invalid_boot_mode() {
        let config = Config {
            boot_mode: "invalid".to_string(),
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field, "boot_mode");
    }

    #[test]
    fn test_rate_limit_attempts_zero() {
        let config = Config {
            rate_limit_attempts: 0,
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field, "rate_limit_attempts");
    }

    #[test]
    fn test_rate_limit_window_zero() {
        let config = Config {
            rate_limit_window_seconds: 0,
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field, "rate_limit_window_seconds");
    }

    #[test]
    fn test_network_exposed_without_auth_warning() {
        let config = Config {
            bind_address: "0.0.0.0".to_string(),
            users: Vec::new(),
            allow_unauthenticated_network: false,
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(!warnings.is_empty());
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("Network exposed"))
        );
    }

    #[test]
    fn test_legacy_auth_username_warning() {
        let config = Config {
            auth_username: Some("admin".to_string()),
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.field == "auth_username"));
    }

    #[test]
    fn test_legacy_auth_password_warning() {
        let config = Config {
            auth_password: Some("secret".to_string()),
            ..Config::default()
        };
        let result = validate_config(&config);
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.field == "auth_password"));
    }
}
