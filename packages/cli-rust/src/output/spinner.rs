//! Command spinner with elapsed time display
//!
//! Provides visual feedback during long-running CLI operations with
//! animated spinner and elapsed time indicator.

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// A spinner for command operations with elapsed time display
///
/// The spinner shows an animated indicator with a message and elapsed time.
/// It respects quiet mode by becoming a no-op when quiet is enabled.
///
/// # Example
///
/// ```ignore
/// let spinner = CommandSpinner::new("Starting service...");
/// // ... do work ...
/// spinner.success("Service started");
/// ```
pub struct CommandSpinner {
    bar: Option<ProgressBar>,
}

impl CommandSpinner {
    /// Create a new spinner with the given message
    ///
    /// The spinner starts ticking immediately at 100ms intervals.
    /// Shows: `spinner message (elapsed)`
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} ({elapsed})")
                .expect("invalid spinner template")
                .tick_chars("\u{28CB}\u{2819}\u{2839}\u{2838}\u{283C}\u{2834}\u{2826}\u{2827}\u{2807}\u{280F}"),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));
        Self { bar: Some(bar) }
    }

    /// Create a spinner that respects quiet mode
    ///
    /// If `quiet` is true, returns a no-op spinner that doesn't output anything.
    pub fn new_maybe(message: &str, quiet: bool) -> Self {
        if quiet {
            Self { bar: None }
        } else {
            Self::new(message)
        }
    }

    /// Update the spinner message
    pub fn update(&self, message: &str) {
        if let Some(ref bar) = self.bar {
            bar.set_message(message.to_string());
        }
    }

    /// Finish the spinner with a success message (green checkmark)
    pub fn success(self, message: &str) {
        if let Some(bar) = self.bar {
            bar.finish_with_message(format!(
                "{} {}",
                console::style("\u{2713}").green(),
                message
            ));
        }
    }

    /// Finish the spinner with a failure message (red X)
    pub fn fail(self, message: &str) {
        if let Some(bar) = self.bar {
            bar.finish_with_message(format!("{} {}", console::style("\u{2717}").red(), message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_new_does_not_panic() {
        // Just ensure creation doesn't panic
        let spinner = CommandSpinner::new("test");
        spinner.success("done");
    }

    #[test]
    fn spinner_quiet_mode_is_noop() {
        let spinner = CommandSpinner::new_maybe("test", true);
        assert!(spinner.bar.is_none());
        // Should not panic
        spinner.update("updated");
    }

    #[test]
    fn spinner_quiet_mode_success_is_noop() {
        let spinner = CommandSpinner::new_maybe("test", true);
        // Should not panic
        spinner.success("done");
    }

    #[test]
    fn spinner_quiet_mode_fail_is_noop() {
        let spinner = CommandSpinner::new_maybe("test", true);
        // Should not panic
        spinner.fail("failed");
    }
}
