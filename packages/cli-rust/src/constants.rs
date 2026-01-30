//! Shared CLI constants.

/// Default random password length.
///
/// Override with `OPENCODE_PASSWORD_LENGTH` (valid range: 12-128).
pub const DEFAULT_PASSWORD_LENGTH: usize = 30;

const MIN_PASSWORD_LENGTH: usize = 12;
const MAX_PASSWORD_LENGTH: usize = 128;

/// Resolve the configured password length from the environment.
pub fn password_length() -> usize {
    std::env::var("OPENCODE_PASSWORD_LENGTH")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| (MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(value))
        .unwrap_or(DEFAULT_PASSWORD_LENGTH)
}
