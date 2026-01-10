//! Example demonstrating issue creation and updates
//!
//! This example shows how to:
//! - Create new issues with different types
//! - Update issue status and priority
//! - Add dependencies between issues
//! - Close issues
//!
//! To run this example:
//! ```sh
//! cd crates/beads
//! cargo run --example create_update
//! ```

use beads::{Beads, Result};

fn main() -> Result<()> {
    println!("Beads Issue Creation and Update Example\n");

    let bd = Beads::new()?;

    if !bd.is_repo() {
        eprintln!("Not in a beads-enabled repository.");
        eprintln!("Run 'bd init' to initialize beads.");
        return Ok(());
    }

    // Create an epic
    println!("Creating an epic...");
    let output = bd.create_epic("Q1 2026 Goals", Some(1))?;
    println!("✓ {}", output.stdout.trim());

    // Create a feature
    println!("\nCreating a feature...");
    let output = bd.create("Implement user authentication", "feature", Some(2), None)?;
    println!("✓ {}", output.stdout.trim());

    // Create a task
    println!("\nCreating a task...");
    let output = bd.create_full(
        "Write unit tests for auth module",
        "task",
        Some(2),
        Some("Add comprehensive test coverage for authentication"),
        Some("dev@example.com"),
        None,
        Some(&["testing", "auth"]),
    )?;
    println!("✓ {}", output.stdout.trim());

    // Create a bug
    println!("\nCreating a bug...");
    let output = bd.create("Fix login redirect issue", "bug", Some(1), None)?;
    println!("✓ {}", output.stdout.trim());

    // Show the newly created issues
    println!("\n=== Recent Issues ===");
    let issues = bd.list_open()?;
    for issue in issues.iter().take(5) {
        let priority = issue
            .priority
            .map(|p| format!("P{}", p))
            .unwrap_or_else(|| "P?".to_string());
        println!(
            "{}: {} [{}] - {}",
            issue.id, issue.title, priority, issue.issue_type
        );
    }

    println!("\nExample completed! Issues created successfully.");
    println!("You can view them with: bd list");

    Ok(())
}
