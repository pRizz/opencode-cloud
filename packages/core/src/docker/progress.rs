//! Progress reporting utilities for Docker operations
//!
//! This module provides progress bars and spinners for Docker image
//! builds and pulls, using indicatif for terminal output.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;

/// Progress reporter for Docker operations
///
/// Manages multiple progress bars for concurrent operations like
/// multi-layer image pulls and build steps.
pub struct ProgressReporter {
    multi: MultiProgress,
    bars: HashMap<String, ProgressBar>,
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
        spinner.set_message(message.to_string());
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
    pub fn update_spinner(&mut self, id: &str, message: &str) {
        if let Some(spinner) = self.bars.get(id) {
            spinner.set_message(message.to_string());
        } else {
            // Create new spinner if doesn't exist
            self.add_spinner(id, message);
        }
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
}
