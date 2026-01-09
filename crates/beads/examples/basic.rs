//! Basic usage example for the beads crate
//!
//! This example demonstrates the core functionality of the beads wrapper:
//! - Checking if bd is installed
//! - Listing issues
//! - Getting statistics
//! - Querying ready and blocked issues
//!
//! To run this example:
//! ```sh
//! cd crates/beads
//! cargo run --example basic
//! ```

use beads::{Beads, Result};

fn main() -> Result<()> {
    println!("Beads CLI Wrapper Example\n");

    // Create a new Beads instance
    let bd = match Beads::new() {
        Ok(bd) => bd,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nMake sure 'bd' is installed and in your PATH.");
            return Err(e);
        }
    };

    // Check if we're in a beads repository
    if !bd.is_repo() {
        eprintln!("Not in a beads-enabled repository.");
        eprintln!("Run 'bd init' to initialize beads in this repository.");
        return Ok(());
    }

    println!("✓ Connected to beads repository\n");

    // Get and display statistics
    println!("=== Project Statistics ===");
    match bd.stats() {
        Ok(stats) => {
            println!("Total issues:      {}", stats.total);
            println!("Open:              {}", stats.open);
            println!("In Progress:       {}", stats.in_progress);
            println!("Blocked:           {}", stats.blocked);
            println!("Closed:            {}", stats.closed);
            println!("Epics:             {}", stats.epics);
        }
        Err(e) => eprintln!("Failed to get stats: {}", e),
    }

    // List open issues
    println!("\n=== Open Issues ===");
    match bd.list_open() {
        Ok(issues) => {
            if issues.is_empty() {
                println!("No open issues found.");
            } else {
                for issue in issues.iter().take(5) {
                    println!(
                        "{}: {} [{}]",
                        issue.id, issue.title, issue.status
                    );
                }
                if issues.len() > 5 {
                    println!("... and {} more", issues.len() - 5);
                }
            }
        }
        Err(e) => eprintln!("Failed to list issues: {}", e),
    }

    // Show ready issues (no blockers)
    println!("\n=== Ready to Work On ===");
    match bd.ready() {
        Ok(ready) => {
            if ready.is_empty() {
                println!("No issues ready to work on.");
            } else {
                for issue in ready.iter().take(3) {
                    let priority = issue.priority
                        .map(|p| format!("P{}", p))
                        .unwrap_or_else(|| "P?".to_string());
                    println!(
                        "{}: {} [{}] - {}",
                        issue.id, issue.title, priority, issue.issue_type
                    );
                }
                if ready.len() > 3 {
                    println!("... and {} more ready", ready.len() - 3);
                }
            }
        }
        Err(e) => eprintln!("Failed to get ready issues: {}", e),
    }

    // Show blocked issues
    println!("\n=== Blocked Issues ===");
    match bd.blocked() {
        Ok(blocked) => {
            if blocked.is_empty() {
                println!("No blocked issues.");
            } else {
                for issue in blocked.iter().take(3) {
                    let blockers = issue.blocker_ids().len();
                    println!(
                        "{}: {} (blocked by {} issue{})",
                        issue.id,
                        issue.title,
                        blockers,
                        if blockers == 1 { "" } else { "s" }
                    );
                }
                if blocked.len() > 3 {
                    println!("... and {} more blocked", blocked.len() - 3);
                }
            }
        }
        Err(e) => eprintln!("Failed to get blocked issues: {}", e),
    }

    // Show epics
    println!("\n=== Open Epics ===");
    match bd.list_open_epics() {
        Ok(epics) => {
            if epics.is_empty() {
                println!("No open epics.");
            } else {
                for epic in epics {
                    let priority = epic.priority
                        .map(|p| format!("P{}", p))
                        .unwrap_or_else(|| "P?".to_string());
                    println!("{}: {} [{}]", epic.id, epic.title, priority);
                }
            }
        }
        Err(e) => eprintln!("Failed to list epics: {}", e),
    }

    println!("\nExample completed successfully!");
    Ok(())
}
