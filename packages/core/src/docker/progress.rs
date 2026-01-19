//! Progress reporting utilities for Docker operations
//!
//! This module provides progress bars and spinners for Docker image
//! builds and pulls, using indicatif for terminal output.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum time between spinner message updates to prevent flickering
const SPINNER_UPDATE_THROTTLE: Duration = Duration::from_millis(150);

/// Format duration as MM:SS, or HH:MM:SS if over an hour
fn format_elapsed(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Progress reporter for Docker operations
///
/// Manages multiple progress bars for concurrent operations like
/// multi-layer image pulls and build steps.
pub struct ProgressReporter {
    multi: MultiProgress,
    bars: HashMap<String, ProgressBar>,
    last_update: HashMap<String, Instant>,
    last_message: HashMap<String, String>,
    start_time: Instant,
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
            last_update: HashMap::new(),
            last_message: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Create a spinner for indeterminate progress (e.g., build steps)
    pub fn add_spinner(&mut self, id: &str, message: &str) -> &ProgressBar {
        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("valid template")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        // Include elapsed time in message (formatted as MM:SS or HH:MM:SS)
        let elapsed = format_elapsed(self.start_time.elapsed());
        spinner.set_message(format!("{} ({})", message, elapsed));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        self.bars.insert(id.to_string(), spinner);
        self.bars.get(id).expect("just inserted")
    }

    /// Create a progress bar for determinate progress (e.g., layer download)
    ///
    /// `total` is in bytes
    pub fn add_bar(&mut self, id: &str, total: u64) -> &ProgressBar {
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
        let now = Instant::now();
        let is_step_message = message.starts_with("Step ");

        // Check if we should throttle this update
        if !is_step_message {
            if let Some(last) = self.last_update.get(id) {
                if now.duration_since(*last) < SPINNER_UPDATE_THROTTLE {
                    return; // Throttle: too soon since last update
                }
            }

            // Skip if message is identical to last one
            if let Some(last_msg) = self.last_message.get(id) {
                if last_msg == message {
                    return;
                }
            }
        }

        // Perform the update with elapsed time
        let elapsed = format_elapsed(self.start_time.elapsed());
        let message_with_time = format!("{} ({})", message, elapsed);

        if let Some(spinner) = self.bars.get(id) {
            spinner.set_message(message_with_time);
        } else {
            // Create new spinner if doesn't exist
            self.add_spinner(id, message);
        }

        // Track update time and message
        self.last_update.insert(id.to_string(), now);
        self.last_message
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
    fn format_elapsed_shows_seconds_only() {
        let duration = Duration::from_secs(45);
        assert_eq!(format_elapsed(duration), "00:45");
    }

    #[test]
    fn format_elapsed_shows_minutes_and_seconds() {
        let duration = Duration::from_secs(90); // 1m 30s
        assert_eq!(format_elapsed(duration), "01:30");
    }

    #[test]
    fn format_elapsed_shows_hours_when_needed() {
        let duration = Duration::from_secs(3661); // 1h 1m 1s
        assert_eq!(format_elapsed(duration), "01:01:01");
    }

    #[test]
    fn format_elapsed_zero() {
        let duration = Duration::from_secs(0);
        assert_eq!(format_elapsed(duration), "00:00");
    }
}
