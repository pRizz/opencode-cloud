//! Version information for opencode-cloud

/// Get the current version string
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the long version string with build information
///
/// Returns version plus build metadata when available (git commit, build date).
/// Falls back gracefully if build info is not available.
pub fn get_version_long() -> String {
    let version = get_version();

    // Build info is set via environment variables during build
    // These may be set by CI or build scripts
    let git_hash = option_env!("OCC_GIT_HASH").unwrap_or("unknown");
    let build_date = option_env!("OCC_BUILD_DATE").unwrap_or("unknown");

    format!("{version} (git: {git_hash}, built: {build_date})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_returns_valid_semver() {
        let version = get_version();
        assert!(!version.is_empty());
        // Basic semver format check
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[test]
    fn test_get_version_long_contains_version() {
        let long = get_version_long();
        let short = get_version();
        assert!(
            long.contains(&short),
            "Long version should contain short version"
        );
    }
}
