//! User list subcommand
//!
//! Lists users in the container with their status.

use anyhow::Result;
use clap::Args;
use comfy_table::{Cell, Color, Table};
use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, list_users};

/// Arguments for the user list command
#[derive(Args)]
pub struct UserListArgs {}

/// List users in the container
pub async fn cmd_user_list(_args: &UserListArgs, quiet: bool, _verbose: u8) -> Result<()> {
    let client = DockerClient::new()?;

    // Get users from container
    let users = list_users(&client, CONTAINER_NAME).await?;

    // Handle empty list
    if users.is_empty() {
        if !quiet {
            println!("No users configured.");
        }
        return Ok(());
    }

    // Quiet mode: just usernames, one per line
    if quiet {
        for user in &users {
            println!("{}", user.username);
        }
        return Ok(());
    }

    // Table output
    let mut table = Table::new();
    table.set_header(vec!["Username", "Status", "UID", "Home", "Shell"]);

    for user in &users {
        let status_cell = if user.locked {
            Cell::new("disabled").fg(Color::Yellow)
        } else {
            Cell::new("enabled").fg(Color::Green)
        };

        table.add_row(vec![
            Cell::new(&user.username),
            status_cell,
            Cell::new(user.uid.to_string()),
            Cell::new(&user.home),
            Cell::new(&user.shell),
        ]);
    }

    println!("{table}");

    Ok(())
}
