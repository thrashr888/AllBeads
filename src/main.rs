//! AllBeads - Distributed Protocol for Agentic Orchestration
//!
//! Main entry point for the AllBeads CLI.

use allbeads::aggregator::{Aggregator, AggregatorConfig, SyncMode};
use allbeads::cache::{Cache, CacheConfig};
use allbeads::config::AllBeadsConfig;
use allbeads::graph::{BeadId, Priority, Status};
use clap::{Parser, Subcommand};
use std::process;

/// AllBeads - Multi-context task aggregator and orchestrator
#[derive(Parser, Debug)]
#[command(name = "allbeads")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to config file (default: ~/.config/allbeads/config.yaml)
    #[arg(short, long)]
    config: Option<String>,

    /// Filter to specific contexts (comma-separated)
    #[arg(short = 'C', long)]
    contexts: Option<String>,

    /// Use cached data only (don't fetch updates)
    #[arg(long)]
    cached: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List beads with optional filters
    List {
        /// Filter by status (open, in_progress, blocked, closed)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by priority (P0-P4 or 0-4)
        #[arg(short, long)]
        priority: Option<String>,

        /// Filter by context (@work, @personal)
        #[arg(short = 'c', long)]
        context: Option<String>,

        /// Filter by label/tag
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Show detailed information about a bead
    Show {
        /// Bead ID (e.g., ab-123)
        id: String,
    },

    /// Show beads that are ready to work on
    Ready,

    /// Show aggregated statistics
    Stats,

    /// Clear the local cache
    ClearCache,
}

fn main() {
    // Initialize logging
    if let Err(e) = allbeads::logging::init() {
        eprintln!("Failed to initialize logging: {}", e);
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> allbeads::Result<()> {
    // Load configuration
    let config = if let Some(config_path) = cli.config {
        AllBeadsConfig::load(config_path)?
    } else {
        AllBeadsConfig::load_default()?
    };

    tracing::info!(contexts = config.contexts.len(), "Configuration loaded");

    // Parse context filter
    let context_filter: Vec<String> = if let Some(contexts) = cli.contexts {
        contexts.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };

    // Set up aggregator
    let sync_mode = if cli.cached {
        SyncMode::LocalOnly
    } else {
        SyncMode::Fetch
    };

    let agg_config = AggregatorConfig {
        sync_mode,
        context_filter,
        skip_errors: true,
    };

    // Try to load from cache first
    let cache_config = CacheConfig::default();
    let cache = Cache::new(cache_config)?;

    let graph = if cli.cached || !cache.is_expired()? {
        tracing::debug!("Attempting to load from cache");
        if let Some(cached_graph) = cache.load_graph()? {
            tracing::info!("Using cached graph");
            cached_graph
        } else {
            tracing::info!("Cache miss, aggregating from Boss repositories");
            let mut aggregator = Aggregator::new(config, agg_config)?;
            let graph = aggregator.aggregate()?;
            cache.store_graph(&graph)?;
            graph
        }
    } else {
        tracing::info!("Cache expired, aggregating from Boss repositories");
        let mut aggregator = Aggregator::new(config, agg_config)?;
        let graph = aggregator.aggregate()?;
        cache.store_graph(&graph)?;
        graph
    };

    // Execute command
    match cli.command {
        Commands::List {
            status,
            priority,
            context,
            label,
        } => {
            let mut beads: Vec<_> = graph.beads.values().collect();

            // Apply filters
            if let Some(status_str) = status {
                let status_filter = parse_status(&status_str)?;
                beads.retain(|b| b.status == status_filter);
            }

            if let Some(priority_str) = priority {
                let priority_filter = parse_priority(&priority_str)?;
                beads.retain(|b| b.priority == priority_filter);
            }

            if let Some(context_str) = context {
                let context_tag = if context_str.starts_with('@') {
                    context_str
                } else {
                    format!("@{}", context_str)
                };
                beads.retain(|b| b.labels.contains(&context_tag));
            }

            if let Some(label_str) = label {
                beads.retain(|b| b.labels.contains(&label_str));
            }

            // Sort by priority then status
            beads.sort_by_key(|b| (b.priority, status_to_sort_key(b.status)));

            // Display results
            println!("Found {} beads:", beads.len());
            println!();
            for bead in beads {
                print_bead_summary(bead);
            }
        }

        Commands::Show { id } => {
            let bead_id = BeadId::new(&id);
            if let Some(bead) = graph.get_bead(&bead_id) {
                print_bead_detailed(bead);
            } else {
                return Err(allbeads::AllBeadsError::IssueNotFound(id));
            }
        }

        Commands::Ready => {
            let ready = graph.ready_beads();
            println!("Ready to work on: {} beads", ready.len());
            println!();
            for bead in ready {
                print_bead_summary(bead);
            }
        }

        Commands::Stats => {
            let stats = graph.stats();
            println!("AllBeads Statistics:");
            println!();
            println!("  Total beads:      {}", stats.total_beads);
            println!("  Total shadows:    {}", stats.total_shadows);
            println!("  Total rigs:       {}", stats.total_rigs);
            println!();
            println!("  Open:             {}", stats.open_beads);
            println!("  In Progress:      {}", stats.in_progress_beads);
            println!("  Blocked:          {}", stats.blocked_beads);
            println!("  Closed:           {}", stats.closed_beads);

            // Cache stats
            let cache_stats = cache.stats()?;
            println!();
            println!("Cache:");
            println!("  Beads cached:     {}", cache_stats.bead_count);
            println!("  Rigs cached:      {}", cache_stats.rig_count);
            if let Some(age) = cache_stats.age {
                println!("  Cache age:        {:.1}s", age.as_secs_f64());
            }
            println!("  Expired:          {}", cache_stats.is_expired);
        }

        Commands::ClearCache => {
            cache.clear()?;
            println!("Cache cleared successfully");
        }
    }

    Ok(())
}

fn parse_status(s: &str) -> allbeads::Result<Status> {
    match s.to_lowercase().as_str() {
        "open" => Ok(Status::Open),
        "in_progress" | "in-progress" => Ok(Status::InProgress),
        "blocked" => Ok(Status::Blocked),
        "deferred" => Ok(Status::Deferred),
        "closed" => Ok(Status::Closed),
        "tombstone" => Ok(Status::Tombstone),
        _ => Err(allbeads::AllBeadsError::Parse(format!(
            "Invalid status: {}. Must be one of: open, in_progress, blocked, deferred, closed, tombstone",
            s
        ))),
    }
}

fn parse_priority(s: &str) -> allbeads::Result<Priority> {
    match s.to_uppercase().as_str() {
        "P0" | "0" => Ok(Priority::P0),
        "P1" | "1" => Ok(Priority::P1),
        "P2" | "2" => Ok(Priority::P2),
        "P3" | "3" => Ok(Priority::P3),
        "P4" | "4" => Ok(Priority::P4),
        _ => Err(allbeads::AllBeadsError::Parse(format!(
            "Invalid priority: {}. Must be one of: P0-P4 or 0-4",
            s
        ))),
    }
}

fn status_to_sort_key(status: Status) -> u8 {
    match status {
        Status::Open => 0,
        Status::InProgress => 1,
        Status::Blocked => 2,
        Status::Deferred => 3,
        Status::Closed => 4,
        Status::Tombstone => 5,
    }
}

fn print_bead_summary(bead: &allbeads::graph::Bead) {
    let status_str = format_status(bead.status);
    let priority_str = format_priority(bead.priority);
    let context_tags: Vec<_> = bead
        .labels
        .iter()
        .filter(|l| l.starts_with('@'))
        .collect();

    print!("[{}] [{}] {}: {}", priority_str, status_str, bead.id.as_str(), bead.title);

    if !context_tags.is_empty() {
        print!(" ({})", context_tags.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    println!();
}

fn print_bead_detailed(bead: &allbeads::graph::Bead) {
    println!("{}: {}", bead.id.as_str(), bead.title);
    println!("Status:       {}", format_status(bead.status));
    println!("Priority:     {}", format_priority(bead.priority));
    println!("Type:         {:?}", bead.issue_type);
    println!("Created:      {} by {}", bead.created_at, bead.created_by);
    println!("Updated:      {}", bead.updated_at);

    if let Some(ref assignee) = bead.assignee {
        println!("Assignee:     {}", assignee);
    }

    if !bead.labels.is_empty() {
        println!("Labels:       {}", bead.labels.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    if !bead.dependencies.is_empty() {
        println!("Depends on:   {}", bead.dependencies.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(", "));
    }

    if !bead.blocks.is_empty() {
        println!("Blocks:       {}", bead.blocks.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(", "));
    }

    if let Some(ref description) = bead.description {
        println!();
        println!("Description:");
        println!("{}", description);
    }

    if let Some(ref notes) = bead.notes {
        println!();
        println!("Notes:");
        println!("{}", notes);
    }
}

fn format_status(status: Status) -> &'static str {
    match status {
        Status::Open => "open",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Deferred => "deferred",
        Status::Closed => "closed",
        Status::Tombstone => "tombstone",
    }
}

fn format_priority(priority: Priority) -> &'static str {
    match priority {
        Priority::P0 => "P0",
        Priority::P1 => "P1",
        Priority::P2 => "P2",
        Priority::P3 => "P3",
        Priority::P4 => "P4",
    }
}
