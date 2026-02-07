//! Summary display
//!
//! Shows configuration summary before saving.

use crate::wizard::WizardState;
use comfy_table::{Cell, Table};
use console::style;
use opencode_cloud_core::config::paths::get_config_path;

/// Display configuration summary
///
/// Shows all collected configuration values and onboarding steps.
pub fn display_summary(state: &WizardState) {
    println!("{}", style("Configuration Summary").bold());
    println!("{}", style("-".repeat(22)).dim());

    let mut table = Table::new();
    table.load_preset(comfy_table::presets::NOTHING);

    table.add_row(vec![
        Cell::new("Auth setup:"),
        Cell::new("Initial One-Time Password (IOTP) + passkey"),
    ]);
    table.add_row(vec![Cell::new("Bootstrap user:"), Cell::new("opencoder")]);
    table.add_row(vec![Cell::new("Port:"), Cell::new(state.port)]);
    table.add_row(vec![Cell::new("Binding:"), Cell::new(&state.bind)]);

    let image_time = if state.image_source == "prebuilt" {
        "(~2 min download)"
    } else {
        "(30-60 min build)"
    };
    table.add_row(vec![
        Cell::new("Image:"),
        Cell::new(format!("{} {}", state.image_source, image_time)),
    ]);

    let mounts_summary = if state.mounts.is_empty() {
        "None (Docker volumes only)".to_string()
    } else {
        state.mounts.join("\n")
    };
    table.add_row(vec![Cell::new("Mounts:"), Cell::new(mounts_summary)]);

    println!("{table}");

    println!();
    println!("{}", style("After setup").bold());
    println!(
        "{}",
        style("- Start the service and use the IOTP from logs on the web login page.").dim()
    );
    println!(
        "{}",
        style("- Enroll a passkey to complete first-time onboarding.").dim()
    );
    println!(
        "{}",
        style("- Add additional users later with: occ user add <username>").dim()
    );

    // Show config file location
    if let Some(path) = get_config_path() {
        println!();
        println!("Config will be saved to: {}", style(path.display()).dim());
    }
}
