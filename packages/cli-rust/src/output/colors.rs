//! Color utilities for CLI output
//!
//! Provides consistent color styling for service states and log levels.

use console::{Style, StyledObject};

/// Style a service state string with appropriate colors
///
/// - "running" -> green bold
/// - "stopped", "exited" -> red
/// - "starting", "restarting" -> yellow
/// - other -> dim
pub fn state_style(state: &str) -> StyledObject<String> {
    let lowercase = state.to_lowercase();
    let style = match lowercase.as_str() {
        "running" => Style::new().green().bold(),
        "stopped" | "exited" => Style::new().red(),
        "starting" | "restarting" | "created" => Style::new().yellow(),
        _ => Style::new().dim(),
    };
    style.apply_to(state.to_string())
}

/// Style a log line based on detected log level
///
/// - Contains "ERROR" or "error" -> red
/// - Contains "WARN" or "warn" -> yellow
/// - Contains "INFO" or "info" -> cyan
/// - Contains "DEBUG" or "debug" -> dim
/// - else -> unstyled
pub fn log_level_style(line: &str) -> StyledObject<&str> {
    let style = if line.contains("ERROR") || line.contains("error") {
        Style::new().red()
    } else if line.contains("WARN") || line.contains("warn") {
        Style::new().yellow()
    } else if line.contains("INFO") || line.contains("info") {
        Style::new().cyan()
    } else if line.contains("DEBUG") || line.contains("debug") {
        Style::new().dim()
    } else {
        Style::new()
    };
    style.apply_to(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_style_running_is_green() {
        let styled = state_style("running");
        assert_eq!(styled.to_string(), "running");
    }

    #[test]
    fn state_style_stopped_is_red() {
        let styled = state_style("stopped");
        assert_eq!(styled.to_string(), "stopped");
    }

    #[test]
    fn state_style_exited_is_red() {
        let styled = state_style("exited");
        assert_eq!(styled.to_string(), "exited");
    }

    #[test]
    fn state_style_starting_is_yellow() {
        let styled = state_style("starting");
        assert_eq!(styled.to_string(), "starting");
    }

    #[test]
    fn state_style_unknown_is_dim() {
        let styled = state_style("unknown");
        assert_eq!(styled.to_string(), "unknown");
    }

    #[test]
    fn state_style_case_insensitive() {
        let styled = state_style("RUNNING");
        assert_eq!(styled.to_string(), "RUNNING");
    }

    #[test]
    fn log_level_error_is_red() {
        let styled = log_level_style("2024-01-01 ERROR: something failed");
        assert!(styled.to_string().contains("ERROR"));
    }

    #[test]
    fn log_level_warn_is_yellow() {
        let styled = log_level_style("2024-01-01 WARN: something concerning");
        assert!(styled.to_string().contains("WARN"));
    }

    #[test]
    fn log_level_info_is_cyan() {
        let styled = log_level_style("2024-01-01 INFO: started");
        assert!(styled.to_string().contains("INFO"));
    }

    #[test]
    fn log_level_debug_is_dim() {
        let styled = log_level_style("2024-01-01 DEBUG: internal state");
        assert!(styled.to_string().contains("DEBUG"));
    }

    #[test]
    fn log_level_none_unstyled() {
        let styled = log_level_style("plain log line");
        assert_eq!(styled.to_string(), "plain log line");
    }
}
