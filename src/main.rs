//! AllBeads - Distributed Protocol for Agentic Orchestration
//!
//! Main entry point for the AllBeads CLI and Sheriff daemon.

use allbeads::storage::BeadsRepo;

fn main() {
    // Initialize logging (respects RUST_LOG environment variable)
    if let Err(e) = allbeads::logging::init() {
        eprintln!("Failed to initialize logging: {}", e);
    }

    tracing::info!("AllBeads starting");
    println!("AllBeads - Distributed Protocol for Agentic Orchestration\n");

    // Example: Using BeadsRepo to interact with beads
    match run() {
        Ok(()) => {
            tracing::info!("AllBeads completed successfully");
        }
        Err(e) => {
            tracing::error!(error = %e, "AllBeads failed");
            eprintln!("Error: {}", e);
        }
    }
}

fn run() -> allbeads::Result<()> {
    // Check if bd is available
    tracing::debug!("Initializing BeadsRepo");
    let repo = BeadsRepo::new()?;

    if !repo.is_repo() {
        tracing::warn!("Not in a beads repository");
        println!("Not in a beads repository.");
        println!("Run 'bd init' to initialize beads in this directory.");
        return Ok(());
    }

    // Load beads into a FederatedGraph
    tracing::info!("Loading beads into federated graph");
    println!("Loading beads into federated graph...");
    let graph = repo.load_graph()?;
    tracing::debug!(
        total_beads = graph.stats().total_beads,
        "Graph loaded successfully"
    );

    // Display statistics
    let stats = graph.stats();
    println!("\nProject Statistics:");
    println!("  Total beads:      {}", stats.total_beads);
    println!("  Total shadows:    {}", stats.total_shadows);
    println!("  Total rigs:       {}", stats.total_rigs);
    println!("  Open beads:       {}", stats.open_beads);
    println!("  In progress:      {}", stats.in_progress_beads);
    println!("  Blocked:          {}", stats.blocked_beads);
    println!("  Closed:           {}", stats.closed_beads);

    // Show ready beads
    let ready = graph.ready_beads();
    println!("\nReady to work on: {} beads", ready.len());
    for bead in ready.iter().take(3) {
        println!("  - {}: {}", bead.id.as_str(), bead.title);
    }

    Ok(())
}
