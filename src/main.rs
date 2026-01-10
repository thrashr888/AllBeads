//! AllBeads - Distributed Protocol for Agentic Orchestration
//!
//! Main entry point for the AllBeads CLI.

use allbeads::aggregator::{Aggregator, AggregatorConfig, SyncMode};
use allbeads::cache::{Cache, CacheConfig};
use allbeads::config::{AllBeadsConfig, BossContext, AuthStrategy};
use allbeads::graph::{BeadId, Priority, Status};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
    /// Initialize AllBeads configuration
    Init,

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

    /// Show all blocked beads
    Blocked,

    /// Search beads by text (title, description, notes)
    Search {
        /// Search query
        query: String,

        /// Filter by context
        #[arg(short = 'c', long)]
        context: Option<String>,
    },

    /// Find potential duplicate beads
    Duplicates {
        /// Similarity threshold (0.0-1.0, default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        threshold: f64,
    },

    /// Show aggregated statistics
    Stats,

    /// Launch Kanban board (Terminal UI)
    Kanban,

    /// Clear the local cache
    ClearCache,

    /// Manage contexts (Boss repositories)
    #[command(subcommand)]
    Context(ContextCommands),
}

#[derive(Subcommand, Debug)]
enum ContextCommands {
    /// Add a new context
    Add {
        /// Context name (e.g., work, personal)
        name: String,

        /// Repository URL (HTTPS or SSH)
        url: String,

        /// Local path (default: ~/workspace/<name>)
        #[arg(short, long)]
        path: Option<String>,

        /// Authentication strategy (ssh_agent, gh_token, gh_enterprise_token)
        #[arg(short, long, default_value = "ssh_agent")]
        auth: String,
    },

    /// List all contexts
    List,

    /// Remove a context
    Remove {
        /// Context name to remove
        name: String,
    },
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
    // Handle init command first (creates config)
    if let Commands::Init = cli.command {
        return handle_init_command(&cli.config);
    }

    // Handle context management commands (don't need graph)
    if let Commands::Context(ref ctx_cmd) = cli.command {
        return handle_context_command(ctx_cmd, &cli.config);
    }

    // Load configuration
    let config = if let Some(config_path) = cli.config.clone() {
        AllBeadsConfig::load(config_path)?
    } else {
        match AllBeadsConfig::load_default() {
            Ok(config) => config,
            Err(allbeads::AllBeadsError::Config(msg)) if msg.contains("Config file not found") => {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "No configuration found. Run 'ab init' first to create one.\n\n\
                     Then add contexts with:\n  \
                     ab context add <name> <repo-path>"
                )));
            }
            Err(e) => return Err(e),
        }
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

        Commands::Blocked => {
            let mut blocked: Vec<_> = graph.beads.values()
                .filter(|b| b.status == Status::Blocked || (!b.dependencies.is_empty() && b.status != Status::Closed))
                .collect();

            blocked.sort_by_key(|b| (b.priority, status_to_sort_key(b.status)));

            println!("Blocked beads: {}", blocked.len());
            println!();
            for bead in blocked {
                print_bead_summary(bead);
                if !bead.dependencies.is_empty() {
                    println!("  → Blocked by: {}", bead.dependencies.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(", "));
                }
            }
        }

        Commands::Search { query, context } => {
            let query_lower = query.to_lowercase();
            let mut results: Vec<_> = graph.beads.values()
                .filter(|b| {
                    // Search in title, description, notes, and ID
                    let matches_text = b.title.to_lowercase().contains(&query_lower)
                        || b.id.as_str().to_lowercase().contains(&query_lower)
                        || b.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
                        || b.notes.as_ref().map(|n| n.to_lowercase().contains(&query_lower)).unwrap_or(false);

                    if let Some(ref context_str) = context {
                        let context_tag = if context_str.starts_with('@') {
                            context_str.clone()
                        } else {
                            format!("@{}", context_str)
                        };
                        matches_text && b.labels.contains(&context_tag)
                    } else {
                        matches_text
                    }
                })
                .collect();

            results.sort_by_key(|b| (b.priority, status_to_sort_key(b.status)));

            println!("Search results for '{}': {} beads", query, results.len());
            println!();
            for bead in results {
                print_bead_summary(bead);
            }
        }

        Commands::Duplicates { threshold } => {
            // Group beads by similarity
            let beads: Vec<_> = graph.beads.values().collect();
            let mut duplicates: Vec<(f64, &allbeads::graph::Bead, &allbeads::graph::Bead)> = Vec::new();

            for i in 0..beads.len() {
                for j in (i + 1)..beads.len() {
                    let similarity = calculate_similarity(&beads[i].title, &beads[j].title);
                    if similarity >= threshold {
                        duplicates.push((similarity, beads[i], beads[j]));
                    }
                }
            }

            duplicates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

            if duplicates.is_empty() {
                println!("No potential duplicates found (threshold: {:.0}%)", threshold * 100.0);
            } else {
                println!("Potential duplicates (threshold: {:.0}%): {} pairs", threshold * 100.0, duplicates.len());
                println!();
                for (similarity, bead1, bead2) in duplicates {
                    println!("Similarity: {:.0}%", similarity * 100.0);
                    println!("  {}: {}", bead1.id.as_str(), bead1.title);
                    println!("  {}: {}", bead2.id.as_str(), bead2.title);
                    println!();
                }
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

            // Per-context breakdown
            use std::collections::HashMap;
            let mut context_counts: HashMap<String, usize> = HashMap::new();
            let mut context_open: HashMap<String, usize> = HashMap::new();

            for bead in graph.beads.values() {
                // Find context label (@contextname)
                for label in &bead.labels {
                    if label.starts_with('@') {
                        let context = label.to_string();
                        *context_counts.entry(context.clone()).or_insert(0) += 1;
                        if bead.status == Status::Open {
                            *context_open.entry(context).or_insert(0) += 1;
                        }
                        break;
                    }
                }
            }

            if !context_counts.is_empty() {
                println!();
                println!("Contexts:");
                let mut contexts: Vec<_> = context_counts.iter().collect();
                contexts.sort_by_key(|(ctx, _)| ctx.as_str());

                for (context, count) in contexts {
                    let open_count = context_open.get(context).unwrap_or(&0);
                    let context_name = context.trim_start_matches('@');
                    println!("  {:<15} {} beads ({} open)", context_name, count, open_count);
                }
            }

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

        Commands::Kanban => {
            allbeads::tui::run(graph)?;
        }

        Commands::ClearCache => {
            cache.clear()?;
            println!("Cache cleared successfully");
        }

        Commands::Context(_) | Commands::Init => {
            // Handled earlier in the function
            unreachable!("Context and Init commands should be handled before aggregation")
        }
    }

    Ok(())
}

fn handle_init_command(config_path: &Option<String>) -> allbeads::Result<()> {
    let config_file = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        AllBeadsConfig::default_path()
    };

    // Check if already initialized
    if config_file.exists() {
        println!("Configuration already exists at {}", config_file.display());
        println!();
        println!("To add contexts, run:");
        println!("  ab context add <name> <repo-path>");
        return Ok(());
    }

    // Create parent directory if needed
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            allbeads::AllBeadsError::Config(format!(
                "Failed to create config directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Create empty config
    let config = AllBeadsConfig::new();
    config.save(&config_file)?;

    println!("✓ Created configuration at {}", config_file.display());
    println!();
    println!("Next steps:");
    println!("  1. Add a context (repository with beads):");
    println!("     ab context add myproject /path/to/repo");
    println!();
    println!("  2. View aggregated beads:");
    println!("     ab stats");
    println!("     ab list");
    println!("     ab kanban");

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

/// Calculate similarity between two strings using Jaccard similarity
/// Returns a value between 0.0 (no similarity) and 1.0 (identical)
fn calculate_similarity(s1: &str, s2: &str) -> f64 {
    use std::collections::HashSet;

    // Normalize strings: lowercase and split into words
    let s1_lower = s1.to_lowercase();
    let s2_lower = s2.to_lowercase();

    let words1: HashSet<_> = s1_lower.split_whitespace().collect();
    let words2: HashSet<_> = s2_lower.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    // Jaccard similarity: intersection / union
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    intersection as f64 / union as f64
}

fn handle_context_command(cmd: &ContextCommands, config_path: &Option<String>) -> allbeads::Result<()> {
    let config_file = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        AllBeadsConfig::default_path()
    };

    let mut config = if config_file.exists() {
        AllBeadsConfig::load(&config_file)?
    } else {
        AllBeadsConfig::new()
    };

    match cmd {
        ContextCommands::Add { name, url, path, auth } => {
            // Check if context already exists
            if config.get_context(name).is_some() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Context '{}' already exists",
                    name
                )));
            }

            // Parse auth strategy
            let auth_strategy = match auth.to_lowercase().as_str() {
                "ssh_agent" => AuthStrategy::SshAgent,
                "personal_access_token" | "pat" => AuthStrategy::PersonalAccessToken,
                "gh_enterprise_token" => AuthStrategy::GhEnterpriseToken,
                _ => return Err(allbeads::AllBeadsError::Parse(format!(
                    "Invalid auth strategy: {}. Must be one of: ssh_agent, personal_access_token, gh_enterprise_token",
                    auth
                ))),
            };

            // Validate URL and auth strategy compatibility
            if auth_strategy == AuthStrategy::SshAgent && url.starts_with("https://") {
                eprintln!("⚠️  Warning: Using HTTPS URL with ssh_agent authentication may fail.");
                eprintln!("   Suggestion: Use SSH URL instead:");
                let ssh_url = url
                    .replace("https://github.com/", "git@github.com:")
                    .replace("https://", "git@")
                    .replace(".git/", ".git")
                    + if !url.ends_with(".git") { ".git" } else { "" };
                eprintln!("   {}", ssh_url);
                eprintln!();
                eprintln!("   To add with SSH URL:");
                eprintln!("   allbeads context add {} {}", name, ssh_url);
                eprintln!();
            }

            // Determine path
            let repo_path = if let Some(p) = path {
                Some(PathBuf::from(p))
            } else {
                // Default: ~/workspace/<name>
                let mut default_path = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."));
                default_path.push("workspace");
                default_path.push(name);
                Some(default_path)
            };

            // Create context
            let mut context = BossContext::new(name, url, auth_strategy);
            context.path = repo_path;

            config.add_context(context);
            config.save(&config_file)?;

            println!("Added context '{}' ({}) to {}", name, url, config_file.display());
        }

        ContextCommands::List => {
            if config.contexts.is_empty() {
                println!("No contexts configured");
                return Ok(());
            }

            println!("Configured contexts ({}):", config.contexts.len());
            println!();
            for context in &config.contexts {
                println!("  {}", context.name);
                println!("    URL:  {}", context.url);
                if let Some(ref path) = context.path {
                    println!("    Path: {}", path.display());
                }
                println!("    Auth: {:?}", context.auth_strategy);
                println!();
            }
        }

        ContextCommands::Remove { name } => {
            if config.remove_context(name).is_some() {
                config.save(&config_file)?;
                println!("Removed context '{}'", name);
            } else {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Context '{}' not found",
                    name
                )));
            }
        }
    }

    Ok(())
}
