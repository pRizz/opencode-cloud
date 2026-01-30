//! Password helpers for the CLI.

use console::style;
use rand::Rng;
use rand::distr::Alphanumeric;

/// Default random password length.
///
/// Override with `OPENCODE_PASSWORD_LENGTH` (valid range: 12-128).
const DEFAULT_PASSWORD_LENGTH: usize = 30;

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

/// Generate a secure random password.
pub fn generate_random_password() -> String {
    // ThreadRng is a CSPRNG seeded from the OS.
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(password_length())
        .map(char::from)
        .collect()
}

/// Print a generated password with a standard warning.
pub fn print_generated_password(password: &str, message: &str) {
    println!();
    println!("  Password: {}", style(password).cyan());
    println!();
    print_password_notice(message);
}

/// Print the standard password warning message.
pub fn print_password_notice(message: &str) {
    println!("{}", style(message).yellow());
    println!(
        "{}",
        style("Password generated using a cryptographically secure random number generator.").dim()
    );
}
