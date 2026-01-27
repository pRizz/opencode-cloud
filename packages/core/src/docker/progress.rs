//! Progress reporting utilities for Docker operations
//!
//! This module provides progress bars and spinners for Docker image
//! builds and pulls, using indicatif for terminal output.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum time between spinner message updates to prevent flickering
const SPINNER_UPDATE_THROTTLE: Duration = Duration::from_millis(150);

/// Strip ANSI escape codes from a string
///
/// Docker build output often contains ANSI color codes that can interfere
/// with our spinner display. This removes them for clean output.
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence: ESC [
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (the command character)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // Also handle ESC followed by other sequences (less common)
        } else {
            result.push(c);
        }
    }

    result
}

/// Progress reporter for Docker operations
///
/// Manages multiple progress bars for concurrent operations like
/// multi-layer image pulls and build steps.
pub struct ProgressReporter {
    multi: MultiProgress,
    bars: HashMap<String, ProgressBar>,
    last_update_by_id: HashMap<String, Instant>,
    last_message_by_id: HashMap<String, String>,
    /// Optional context prefix shown before step messages (e.g., "Building Docker image")
    context: Option<String>,
    /// When true, print build output lines directly instead of spinners
    plain_output: bool,
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
            last_update_by_id: HashMap::new(),
            last_message_by_id: HashMap::new(),
            context: None,
            plain_output: false,
        }
    }

    /// Create a new progress reporter with a context prefix
    ///
    /// The context is shown before step messages, e.g., "Building Docker image · Step 1/10"
    pub fn with_context(context: &str) -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
            last_update_by_id: HashMap::new(),
            last_message_by_id: HashMap::new(),
            context: Some(context.to_string()),
            plain_output: false,
        }
    }

    /// Create a progress reporter that prints build output directly
    pub fn with_context_plain(context: &str) -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
            last_update_by_id: HashMap::new(),
            last_message_by_id: HashMap::new(),
            context: Some(context.to_string()),
            plain_output: true,
        }
    }

    /// Check if plain output mode is enabled
    pub fn is_plain_output(&self) -> bool {
        self.plain_output
    }

    /// Format a message with context prefix if set
    fn format_message(&self, message: &str) -> String {
        // Strip ANSI escape codes that Docker may include in its output
        let stripped = strip_ansi_codes(message);

        // Collapse message to single line for spinner display:
        // - Replace all whitespace sequences (including newlines) with single space
        // - Trim leading/trailing whitespace
        let clean_msg = stripped.split_whitespace().collect::<Vec<_>>().join(" ");

        // Format: "Context · message" or "message"
        match &self.context {
            Some(ctx) => format!("{ctx} · {clean_msg}"),
            None => clean_msg,
        }
    }

    /// Create a spinner for indeterminate progress (e.g., build steps)
    pub fn add_spinner(&mut self, id: &str, message: &str) -> &ProgressBar {
        if self.plain_output {
            let spinner = ProgressBar::hidden();
            self.bars.insert(id.to_string(), spinner);
            return self.bars.get(id).expect("just inserted");
        }

        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed}] {msg}")
                .expect("valid template")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        spinner.set_message(self.format_message(message));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        self.bars.insert(id.to_string(), spinner);
        self.bars.get(id).expect("just inserted")
    }

    /// Create a progress bar for determinate progress (e.g., layer download)
    ///
    /// `total` is in bytes
    pub fn add_bar(&mut self, id: &str, total: u64) -> &ProgressBar {
        if self.plain_output {
            let bar = ProgressBar::hidden();
            self.bars.insert(id.to_string(), bar);
            return self.bars.get(id).expect("just inserted");
        }

        let bar = self.multi.add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}",
                )
                .expect("valid template")
                .progress_chars("=>-"),
        );
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        self.bars.insert(id.to_string(), bar);
        self.bars.get(id).expect("just inserted")
    }

    /// Update progress for a layer (used during image pull)
    ///
    /// `current` and `total` are in bytes, `status` is the Docker status message
    pub fn update_layer(&mut self, layer_id: &str, current: u64, total: u64, status: &str) {
        if self.plain_output {
            return;
        }

        if let Some(bar) = self.bars.get(layer_id) {
            // Update total if it changed (Docker sometimes updates this)
            if bar.length() != Some(total) && total > 0 {
                bar.set_length(total);
            }
            bar.set_position(current);
            bar.set_message(status.to_string());
        } else {
            // Create new bar for this layer
            let bar = self.add_bar(layer_id, total);
            bar.set_position(current);
            bar.set_message(status.to_string());
        }
    }

    /// Update spinner message (used during build)
    ///
    /// Updates are throttled to prevent flickering from rapid message changes.
    /// "Step X/Y" messages always update immediately as they indicate significant progress.
    pub fn update_spinner(&mut self, id: &str, message: &str) {
        if self.plain_output {
            let clean = strip_ansi_codes(message);
            let formatted = match &self.context {
                Some(ctx) => format!("{ctx} · {clean}"),
                None => clean,
            };
            eprintln!("{formatted}");
            return;
        }

        let now = Instant::now();
        let is_step_message = message.starts_with("Step ");

        // Check if we should throttle this update
        if !is_step_message {
            if let Some(last) = self.last_update_by_id.get(id) {
                if now.duration_since(*last) < SPINNER_UPDATE_THROTTLE {
                    return; // Throttle: too soon since last update
                }
            }

            // Skip if message is identical to last one
            if let Some(last_msg) = self.last_message_by_id.get(id) {
                if last_msg == message {
                    return;
                }
            }
        }

        // Perform the update with context and elapsed time
        let formatted = self.format_message(message);

        if let Some(spinner) = self.bars.get(id) {
            spinner.set_message(formatted);
        } else {
            // Create new spinner if doesn't exist
            self.add_spinner(id, message);
        }

        // Track update time and message
        self.last_update_by_id.insert(id.to_string(), now);
        self.last_message_by_id
            .insert(id.to_string(), message.to_string());
    }

    /// Mark a layer/step as complete
    pub fn finish(&mut self, id: &str, message: &str) {
        if let Some(bar) = self.bars.get(id) {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Mark all progress as complete
    pub fn finish_all(&self, message: &str) {
        for bar in self.bars.values() {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Mark all progress as failed
    pub fn abandon_all(&self, message: &str) {
        for bar in self.bars.values() {
            bar.abandon_with_message(message.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_reporter_creation() {
        let reporter = ProgressReporter::new();
        assert!(reporter.bars.is_empty());
    }

    #[test]
    fn progress_reporter_default() {
        let reporter = ProgressReporter::default();
        assert!(reporter.bars.is_empty());
    }

    #[test]
    fn add_spinner_creates_entry() {
        let mut reporter = ProgressReporter::new();
        reporter.add_spinner("test", "Testing...");
        assert!(reporter.bars.contains_key("test"));
    }

    #[test]
    fn add_bar_creates_entry() {
        let mut reporter = ProgressReporter::new();
        reporter.add_bar("layer1", 1000);
        assert!(reporter.bars.contains_key("layer1"));
    }

    #[test]
    fn update_layer_creates_if_missing() {
        let mut reporter = ProgressReporter::new();
        reporter.update_layer("layer1", 500, 1000, "Downloading");
        assert!(reporter.bars.contains_key("layer1"));
    }

    #[test]
    fn update_spinner_creates_if_missing() {
        let mut reporter = ProgressReporter::new();
        reporter.update_spinner("step1", "Building...");
        assert!(reporter.bars.contains_key("step1"));
    }

    #[test]
    fn finish_handles_missing_id() {
        let mut reporter = ProgressReporter::new();
        // Should not panic on missing id
        reporter.finish("nonexistent", "Done");
    }

    #[test]
    fn finish_all_handles_empty() {
        let reporter = ProgressReporter::new();
        // Should not panic on empty
        reporter.finish_all("Done");
    }

    #[test]
    fn abandon_all_handles_empty() {
        let reporter = ProgressReporter::new();
        // Should not panic on empty
        reporter.abandon_all("Failed");
    }

    #[test]
    fn with_context_sets_context() {
        let reporter = ProgressReporter::with_context("Building Docker image");
        assert!(reporter.context.is_some());
        assert_eq!(reporter.context.unwrap(), "Building Docker image");
    }

    #[test]
    fn format_message_includes_context_for_steps() {
        let reporter = ProgressReporter::with_context("Building Docker image");
        let msg = reporter.format_message("Step 1/10 : FROM ubuntu");
        // Format: Context · message
        assert!(msg.starts_with("Building Docker image · Step 1/10"));
    }

    #[test]
    fn format_message_includes_context_for_all_messages() {
        let reporter = ProgressReporter::with_context("Building Docker image");
        let msg = reporter.format_message("Compiling foo v1.0");
        // Format: Context · message
        assert!(msg.starts_with("Building Docker image · Compiling foo"));
    }

    #[test]
    fn format_message_without_context() {
        let reporter = ProgressReporter::new();
        let msg = reporter.format_message("Step 1/10 : FROM ubuntu");
        // Format: message (no context, no dot)
        assert!(msg.starts_with("Step 1/10"));
        assert!(!msg.contains("·"));
    }

    #[test]
    fn format_message_collapses_whitespace() {
        let reporter = ProgressReporter::new();
        // All whitespace (including newlines) collapsed to single spaces for spinner display
        let msg = reporter.format_message("Compiling foo\n     Compiling bar\n");
        assert!(!msg.contains('\n'));
        assert!(msg.contains("Compiling foo Compiling bar"));
    }

    #[test]
    fn strip_ansi_codes_removes_color_codes() {
        // Red text: \x1b[31m ... \x1b[0m
        let input = "\x1b[31mError:\x1b[0m something failed";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "Error: something failed");
    }

    #[test]
    fn strip_ansi_codes_handles_plain_text() {
        let input = "Just plain text";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "Just plain text");
    }

    #[test]
    fn strip_ansi_codes_handles_multiple_codes() {
        // Bold green: \x1b[1;32m
        let input = "\x1b[1;32mSuccess\x1b[0m and \x1b[33mwarning\x1b[0m";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "Success and warning");
    }

    #[test]
    fn format_message_strips_ansi_codes() {
        let reporter = ProgressReporter::new();
        let msg = reporter.format_message("\x1b[31mCompiling\x1b[0m foo");
        assert!(msg.contains("Compiling foo"));
        assert!(!msg.contains("\x1b"));
    }
}
