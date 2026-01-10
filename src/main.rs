//! AllBeads - Distributed Protocol for Agentic Orchestration
//!
//! Main entry point for the AllBeads CLI.

use allbeads::aggregator::{Aggregator, AggregatorConfig, SyncMode};
use allbeads::cache::{Cache, CacheConfig};
use allbeads::config::{AllBeadsConfig, AuthStrategy, BossContext};
use allbeads::graph::{BeadId, Priority, Status};
use clap::{Parser, Subcommand};
use crossterm::style::Stylize;
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
    /// Initialize AllBeads configuration or clone a remote repo with beads
    Init {
        /// Remote repository URL to clone and initialize
        #[arg(short, long)]
        remote: Option<String>,

        /// Target directory for cloned repo (default: derived from URL)
        #[arg(short, long)]
        target: Option<String>,

        /// Run janitor agent to scan codebase and create issues
        #[arg(short, long)]
        janitor: bool,
    },

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
        /// Search query (optional with filters)
        query: Option<String>,

        /// Filter by context
        #[arg(short = 'c', long)]
        context: Option<String>,

        /// Filter by status (open, in_progress, blocked, deferred, closed). Prefix with ^ to negate (e.g., ^closed)
        #[arg(short = 's', long)]
        status: Option<String>,

        /// Filter by minimum priority (inclusive, 0-4 or P0-P4)
        #[arg(long)]
        priority_min: Option<String>,

        /// Filter by maximum priority (inclusive, 0-4 or P0-P4)
        #[arg(long)]
        priority_max: Option<String>,

        /// Filter by type (bug, feature, task, epic, chore). Prefix with ^ to negate (e.g., ^epic)
        #[arg(short = 't', long = "type")]
        issue_type: Option<String>,

        /// Filter by label
        #[arg(short = 'l', long)]
        label: Option<Vec<String>>,

        /// Filter by assignee
        #[arg(short = 'a', long)]
        assignee: Option<String>,

        /// Sort by field: priority, created, updated, status, id, title, type
        #[arg(long, default_value = "priority")]
        sort: String,

        /// Reverse sort order
        #[arg(short = 'r', long)]
        reverse: bool,

        /// Limit results (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
    },

    /// Find potential duplicate beads
    Duplicates {
        /// Similarity threshold (0.0-1.0, default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        threshold: f64,
    },

    /// Show aggregated statistics
    Stats,

    /// Launch Terminal UI (Kanban + Mail)
    Tui,

    /// Clear the local cache
    ClearCache,

    /// Manage contexts (Boss repositories)
    #[command(subcommand)]
    Context(ContextCommands),

    /// Agent Mail commands
    #[command(subcommand)]
    Mail(MailCommands),

    /// Run janitor analysis on a repository
    Janitor {
        /// Path to repository (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Include verbose analysis details
        #[arg(short, long)]
        verbose: bool,

        /// Only scan, don't create beads (dry run)
        #[arg(long)]
        dry_run: bool,
    },

    /// Run the Sheriff daemon (background sync)
    Sheriff {
        /// Path to manifest file (manifests/default.xml)
        #[arg(short, long)]
        manifest: Option<String>,

        /// Poll interval in seconds (default: 5)
        #[arg(short, long, default_value = "5")]
        poll_interval: u64,

        /// Run in foreground (print events to stdout)
        #[arg(short, long)]
        foreground: bool,
    },

    /// JIRA integration commands
    #[command(subcommand)]
    Jira(JiraCommands),

    /// GitHub integration commands
    #[command(subcommand, name = "github")]
    GitHub(GitHubCommands),

    /// Agent swarm management commands
    #[command(subcommand)]
    Swarm(SwarmCommands),
}

#[derive(Subcommand, Debug)]
enum MailCommands {
    /// Send a test notification message
    Test {
        /// Message to send
        #[arg(default_value = "Hello from AllBeads!")]
        message: String,
    },

    /// Show inbox messages
    Inbox,

    /// Show unread message count
    Unread,
}

#[derive(Subcommand, Debug)]
enum JiraCommands {
    /// Pull issues from JIRA with ai-agent label
    Pull {
        /// JIRA project key (e.g., PROJ)
        #[arg(short, long)]
        project: String,

        /// JIRA server URL (e.g., https://company.atlassian.net)
        #[arg(short, long)]
        url: String,

        /// Label filter (default: ai-agent)
        #[arg(short, long, default_value = "ai-agent")]
        label: String,

        /// Show raw issue data
        #[arg(long)]
        verbose: bool,
    },

    /// Show JIRA configuration status
    Status,
}

#[derive(Subcommand, Debug)]
enum GitHubCommands {
    /// Pull issues from GitHub with ai-agent label
    Pull {
        /// GitHub owner/organization
        #[arg(short, long)]
        owner: String,

        /// Repository name (optional, pulls from all if not specified)
        #[arg(short, long)]
        repo: Option<String>,

        /// Label filter (default: ai-agent)
        #[arg(short, long, default_value = "ai-agent")]
        label: String,

        /// Show raw issue data
        #[arg(long)]
        verbose: bool,
    },

    /// Show GitHub configuration status
    Status,
}

#[derive(Subcommand, Debug)]
enum SwarmCommands {
    /// List all agents
    List {
        /// Filter by context
        #[arg(short = 'c', long)]
        context: Option<String>,

        /// Only show active agents
        #[arg(short, long)]
        active: bool,
    },

    /// Show aggregated swarm statistics
    Stats,

    /// Set budget for a context
    Budget {
        /// Context name
        context: String,

        /// Budget limit in USD
        limit: f64,
    },

    /// Spawn a test agent (for demonstration)
    SpawnDemo {
        /// Agent name
        #[arg(default_value = "test-agent")]
        name: String,

        /// Context
        #[arg(short = 'c', long, default_value = "default")]
        context: String,

        /// Agent persona (general, refactor-bot, test-writer, security-specialist)
        #[arg(short, long, default_value = "general")]
        persona: String,
    },

    /// Kill an agent
    Kill {
        /// Agent ID
        id: String,
    },

    /// Pause an agent
    Pause {
        /// Agent ID
        id: String,
    },

    /// Resume a paused agent
    Resume {
        /// Agent ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ContextCommands {
    /// Add a new context (from current directory or explicit path)
    Add {
        /// Path to git repository (default: current directory)
        /// Name and URL are inferred from git config
        #[arg(default_value = ".")]
        path: String,

        /// Override context name (default: folder name)
        #[arg(short, long)]
        name: Option<String>,

        /// Override repository URL (default: git remote origin)
        #[arg(short, long)]
        url: Option<String>,

        /// Authentication strategy (auto-detected from URL if not specified)
        /// Options: ssh_agent, personal_access_token, gh_enterprise_token
        #[arg(short, long)]
        auth: Option<String>,
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
    if let Commands::Init { remote, target, janitor } = &cli.command {
        return handle_init_command(&cli.config, remote.as_deref(), target.as_deref(), *janitor);
    }

    // Handle context management commands (don't need graph)
    if let Commands::Context(ref ctx_cmd) = cli.command {
        return handle_context_command(ctx_cmd, &cli.config);
    }

    // Handle mail commands (don't need graph)
    if let Commands::Mail(ref mail_cmd) = cli.command {
        return handle_mail_command(mail_cmd);
    }

    // Handle JIRA commands (don't need graph)
    if let Commands::Jira(ref jira_cmd) = cli.command {
        return handle_jira_command(jira_cmd);
    }

    // Handle GitHub commands (don't need graph)
    if let Commands::GitHub(ref github_cmd) = cli.command {
        return handle_github_command(github_cmd);
    }

    // Handle swarm commands (don't need graph)
    if let Commands::Swarm(ref swarm_cmd) = cli.command {
        return handle_swarm_command(swarm_cmd);
    }

    // Load configuration
    let config = if let Some(config_path) = cli.config.clone() {
        AllBeadsConfig::load(config_path)?
    } else {
        match AllBeadsConfig::load_default() {
            Ok(config) => config,
            Err(allbeads::AllBeadsError::Config(msg)) if msg.contains("Config file not found") => {
                return Err(allbeads::AllBeadsError::Config(
                    "No configuration found. Run 'ab init' first to create one.\n\n\
                     Then add contexts with:\n  \
                     ab context add <name> <repo-path>"
                        .to_string(),
                ));
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

    // Extract project ID for TUI mail (before config is moved)
    let tui_project_id = config
        .contexts
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "default".to_string());

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
            println!();
            println!(
                "📋 Ready work ({} beads with no blockers):",
                ready.len().to_string().green()
            );
            println!();
            for bead in ready {
                print_bead_summary(bead);
            }
        }

        Commands::Blocked => {
            let mut blocked: Vec<_> = graph
                .beads
                .values()
                .filter(|b| {
                    b.status == Status::Blocked
                        || (!b.dependencies.is_empty() && b.status != Status::Closed)
                })
                .collect();

            blocked.sort_by_key(|b| (b.priority, status_to_sort_key(b.status)));

            println!();
            println!("🚫 Blocked beads ({}):", blocked.len().to_string().red());
            println!();
            for bead in blocked {
                print_bead_summary(bead);
                if !bead.dependencies.is_empty() {
                    println!(
                        "  → Blocked by: {}",
                        bead.dependencies
                            .iter()
                            .map(|id| id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }

        Commands::Search {
            query,
            context,
            status,
            priority_min,
            priority_max,
            issue_type,
            label,
            assignee,
            sort,
            reverse,
            limit,
        } => {
            let query_lower = query.as_ref().map(|q| q.to_lowercase());

            // Parse priority bounds
            let min_priority = priority_min.as_ref().and_then(|p| parse_priority_arg(p));
            let max_priority = priority_max.as_ref().and_then(|p| parse_priority_arg(p));

            // Parse status filter (supports negation with ^ or ! prefix)
            let (status_filter, status_negated) = status
                .as_ref()
                .map(|s| {
                    let (negated, val) = if let Some(stripped) = s.strip_prefix('^') {
                        (true, stripped)
                    } else if let Some(stripped) = s.strip_prefix('!') {
                        (true, stripped)
                    } else {
                        (false, s.as_str())
                    };
                    let parsed = match val.to_lowercase().as_str() {
                        "open" => Some(allbeads::graph::Status::Open),
                        "in_progress" | "inprogress" => Some(allbeads::graph::Status::InProgress),
                        "blocked" => Some(allbeads::graph::Status::Blocked),
                        "deferred" => Some(allbeads::graph::Status::Deferred),
                        "closed" => Some(allbeads::graph::Status::Closed),
                        _ => None,
                    };
                    (parsed, negated)
                })
                .unwrap_or((None, false));

            // Parse type filter (supports negation with ^ or ! prefix)
            let (type_filter, type_negated) = issue_type
                .as_ref()
                .map(|t| {
                    let (negated, val) = if let Some(stripped) = t.strip_prefix('^') {
                        (true, stripped)
                    } else if let Some(stripped) = t.strip_prefix('!') {
                        (true, stripped)
                    } else {
                        (false, t.as_str())
                    };
                    let parsed = match val.to_lowercase().as_str() {
                        "bug" => Some(allbeads::graph::IssueType::Bug),
                        "feature" => Some(allbeads::graph::IssueType::Feature),
                        "task" => Some(allbeads::graph::IssueType::Task),
                        "epic" => Some(allbeads::graph::IssueType::Epic),
                        "chore" => Some(allbeads::graph::IssueType::Chore),
                        _ => None,
                    };
                    (parsed, negated)
                })
                .unwrap_or((None, false));

            let mut results: Vec<_> = graph
                .beads
                .values()
                .filter(|b| {
                    // Text search (if query provided)
                    let matches_text = if let Some(ref q) = query_lower {
                        b.title.to_lowercase().contains(q)
                            || b.id.as_str().to_lowercase().contains(q)
                            || b.description
                                .as_ref()
                                .map(|d| d.to_lowercase().contains(q))
                                .unwrap_or(false)
                            || b.notes
                                .as_ref()
                                .map(|n| n.to_lowercase().contains(q))
                                .unwrap_or(false)
                    } else {
                        true // No query = match all
                    };

                    // Context filter
                    let matches_context = if let Some(ref context_str) = context {
                        let context_tag = if context_str.starts_with('@') {
                            context_str.clone()
                        } else {
                            format!("@{}", context_str)
                        };
                        b.labels.contains(&context_tag)
                    } else {
                        true
                    };

                    // Status filter (with negation support)
                    let matches_status = status_filter
                        .as_ref()
                        .map(|s| {
                            let matches = b.status == *s;
                            if status_negated { !matches } else { matches }
                        })
                        .unwrap_or(true);

                    // Priority filter
                    let matches_priority = {
                        let min_ok = min_priority
                            .as_ref()
                            .map(|min| b.priority >= *min)
                            .unwrap_or(true);
                        let max_ok = max_priority
                            .as_ref()
                            .map(|max| b.priority <= *max)
                            .unwrap_or(true);
                        min_ok && max_ok
                    };

                    // Type filter (with negation support)
                    let matches_type = type_filter
                        .as_ref()
                        .map(|t| {
                            let matches = b.issue_type == *t;
                            if type_negated { !matches } else { matches }
                        })
                        .unwrap_or(true);

                    // Label filter (must have ALL specified labels)
                    let matches_labels = label
                        .as_ref()
                        .map(|labels| labels.iter().all(|l| b.labels.contains(l)))
                        .unwrap_or(true);

                    // Assignee filter
                    let matches_assignee = assignee
                        .as_ref()
                        .map(|a| {
                            b.assignee
                                .as_ref()
                                .map(|ba| ba.to_lowercase().contains(&a.to_lowercase()))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true);

                    matches_text
                        && matches_context
                        && matches_status
                        && matches_priority
                        && matches_type
                        && matches_labels
                        && matches_assignee
                })
                .collect();

            // Sort results
            match sort.to_lowercase().as_str() {
                "priority" => results.sort_by_key(|b| b.priority),
                "created" => results.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
                "updated" => results.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
                "status" => results.sort_by_key(|b| status_to_sort_key(b.status)),
                "id" => results.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str())),
                "title" => results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
                "type" => results.sort_by_key(|b| format!("{:?}", b.issue_type)),
                _ => results.sort_by_key(|b| (b.priority, status_to_sort_key(b.status))),
            }

            if reverse {
                results.reverse();
            }

            // Apply limit
            let total = results.len();
            results.truncate(limit);

            // Print results
            let query_display = query.as_deref().unwrap_or("*");
            if total > limit {
                println!(
                    "Search results for '{}': showing {} of {} beads",
                    query_display, limit, total
                );
            } else {
                println!("Search results for '{}': {} beads", query_display, total);
            }
            println!();
            for bead in results {
                print_bead_summary(bead);
            }
        }

        Commands::Duplicates { threshold } => {
            // Group beads by similarity
            let beads: Vec<_> = graph.beads.values().collect();
            let mut duplicates: Vec<(f64, &allbeads::graph::Bead, &allbeads::graph::Bead)> =
                Vec::new();

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
                println!(
                    "No potential duplicates found (threshold: {:.0}%)",
                    threshold * 100.0
                );
            } else {
                println!(
                    "Potential duplicates (threshold: {:.0}%): {} pairs",
                    threshold * 100.0,
                    duplicates.len()
                );
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
            let ready_count = graph.ready_beads().len();

            println!();
            println!("📊 Aggregated Beads Status");
            println!();
            println!("Summary:");
            println!("  Total Beads:          {}", stats.total_beads);
            println!(
                "  Open:                 {}",
                stats.open_beads.to_string().green()
            );
            println!(
                "  In Progress:          {}",
                stats.in_progress_beads.to_string().yellow()
            );
            println!(
                "  Blocked:              {}",
                stats.blocked_beads.to_string().red()
            );
            println!("  Closed:               {}", stats.closed_beads);
            println!(
                "  Ready to Work:        {}",
                ready_count.to_string().green()
            );
            println!();
            println!("Extended:");
            println!("  Shadows:              {}", stats.total_shadows);
            println!("  Rigs:                 {}", stats.total_rigs);

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
                    println!(
                        "  {:<15} {} beads ({} open)",
                        context_name,
                        count,
                        open_count.to_string().green()
                    );
                }
            }

            // Cache stats
            let cache_stats = cache.stats()?;
            println!();
            println!("Cache:");
            println!("  Beads cached:         {}", cache_stats.bead_count);
            println!("  Rigs cached:          {}", cache_stats.rig_count);
            if let Some(age) = cache_stats.age {
                println!("  Cache age:            {:.1}s", age.as_secs_f64());
            }
            let expired_str = if cache_stats.is_expired {
                "true".red().to_string()
            } else {
                "false".green().to_string()
            };
            println!("  Expired:              {}", expired_str);
            println!();
            println!("For more details, use 'ab list' to see individual beads.");
        }

        Commands::Tui => {
            // Determine mail database path (in config directory)
            let mail_db_path = AllBeadsConfig::default_path()
                .parent()
                .map(|p| p.join("mail.db"));

            allbeads::tui::run_with_mail(graph, mail_db_path, &tui_project_id)?;
        }

        Commands::ClearCache => {
            cache.clear()?;
            println!("Cache cleared successfully");
        }

        Commands::Janitor {
            path,
            verbose,
            dry_run,
        } => {
            let repo_path = PathBuf::from(&path);
            if !repo_path.exists() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Path does not exist: {}",
                    repo_path.display()
                )));
            }

            println!("Running janitor analysis on {}...", repo_path.display());
            println!();

            if dry_run {
                println!("(Dry run mode - no beads will be created)");
                println!();
            }

            run_full_janitor_analysis(&repo_path, verbose, dry_run)?;
        }

        Commands::Sheriff {
            manifest,
            poll_interval,
            foreground,
        } => {
            use allbeads::sheriff::{Sheriff, SheriffConfig};
            use std::time::Duration;

            // Build sheriff config
            let mut sheriff_config = SheriffConfig::new(".")
                .with_poll_interval(Duration::from_secs(poll_interval))
                .with_verbose(foreground)
                .with_project_id(&tui_project_id);

            if let Some(manifest_path) = manifest {
                sheriff_config = sheriff_config.with_manifest(manifest_path);
            }

            // Create sheriff
            let mut sheriff = Sheriff::new(sheriff_config)?;
            sheriff.init()?;

            if foreground {
                println!("Sheriff daemon starting in foreground mode...");
                println!("Press Ctrl+C to stop");
                println!();

                // Subscribe to events and print them
                let mut events = sheriff.subscribe();
                let event_handle = tokio::spawn(async move {
                    while let Ok(event) = events.recv().await {
                        match event {
                            allbeads::sheriff::SheriffEvent::Started => {
                                println!("[Sheriff] Daemon started");
                            }
                            allbeads::sheriff::SheriffEvent::Stopped => {
                                println!("[Sheriff] Daemon stopped");
                            }
                            allbeads::sheriff::SheriffEvent::PollStarted => {
                                println!("[Sheriff] Poll cycle started");
                            }
                            allbeads::sheriff::SheriffEvent::PollCompleted {
                                rigs_polled,
                                changes,
                            } => {
                                println!(
                                    "[Sheriff] Poll completed: {} rigs, {} changes",
                                    rigs_polled, changes
                                );
                            }
                            allbeads::sheriff::SheriffEvent::RigSynced { rig_id, result } => {
                                println!(
                                    "[Sheriff] Rig {} synced: {} created, {} updated, {} deleted",
                                    rig_id.as_str(),
                                    result.created.len(),
                                    result.updated.len(),
                                    result.deleted.len()
                                );
                            }
                            allbeads::sheriff::SheriffEvent::Error { message } => {
                                eprintln!("[Sheriff] Error: {}", message);
                            }
                            _ => {}
                        }
                    }
                });

                // Run the daemon
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    sheriff.run().await?;
                    event_handle.abort();
                    Ok::<(), allbeads::AllBeadsError>(())
                })?;
            } else {
                println!("Sheriff daemon background mode not yet implemented.");
                println!("Use --foreground flag to run in foreground.");
            }
        }

        Commands::Context(_) | Commands::Init { .. } | Commands::Mail(_) | Commands::Jira(_) | Commands::GitHub(_) | Commands::Swarm(_) => {
            // Handled earlier in the function
            unreachable!("Context, Init, Mail, Jira, GitHub, and Swarm commands should be handled before aggregation")
        }
    }

    Ok(())
}

fn handle_init_command(
    config_path: &Option<String>,
    remote: Option<&str>,
    target: Option<&str>,
    janitor: bool,
) -> allbeads::Result<()> {
    // Handle remote repository initialization
    if let Some(remote_url) = remote {
        return handle_remote_init(remote_url, target, janitor);
    }

    // Standard local config initialization
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
    println!("  1. Add a context (from within a git repo with beads):");
    println!("     cd /path/to/repo && ab context add");
    println!();
    println!("  2. View aggregated beads:");
    println!("     ab stats");
    println!("     ab list");
    println!("     ab tui");

    Ok(())
}

/// Initialize a remote repository with beads
fn handle_remote_init(
    remote_url: &str,
    target: Option<&str>,
    janitor: bool,
) -> allbeads::Result<()> {
    use allbeads::git::BossRepo;
    use allbeads::storage::BeadsRepo;

    // Derive target directory from URL if not specified
    let target_dir = if let Some(t) = target {
        PathBuf::from(t)
    } else {
        // Extract repo name from URL
        let repo_name = remote_url
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or("repo");
        PathBuf::from(repo_name)
    };

    // Check if target already exists
    if target_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Target directory already exists: {}",
            target_dir.display()
        )));
    }

    println!("Cloning {} to {}...", remote_url, target_dir.display());

    // Clone the repository
    let _repo = git2::Repository::clone(remote_url, &target_dir).map_err(|e| {
        allbeads::AllBeadsError::Git(format!("Failed to clone repository: {}", e))
    })?;

    println!("✓ Repository cloned");

    // Check if .beads/ already exists
    let beads_dir = target_dir.join(".beads");
    let already_has_beads = beads_dir.exists();

    if !already_has_beads {
        // Initialize beads using BeadsRepo
        let beads_repo = BeadsRepo::with_workdir(&target_dir);
        beads_repo.init()?;
        println!("✓ Initialized .beads/ directory");

        // Create an initial Analysis bead using the create API
        beads_repo.create("Initial codebase analysis", "task", Some(1))?;
        println!("✓ Created initial Analysis bead");

        // Commit the .beads/ directory using BossRepo
        let boss_repo = BossRepo::from_local(&target_dir)?;
        boss_repo.add_beads()?;
        boss_repo.commit(
            "Initialize beads tracking\n\nAdded .beads/ directory with initial Analysis bead",
            "AllBeads",
            "noreply@allbeads.dev",
        )?;
        println!("✓ Committed .beads/ directory");
    } else {
        println!("✓ Repository already has .beads/ directory");
    }

    // Run janitor if requested
    if janitor {
        println!();
        println!("Running janitor analysis...");
        run_janitor_analysis(&target_dir)?;
    }

    println!();
    println!("Repository initialized successfully!");
    println!();
    println!("Next steps:");
    println!("  cd {} && ab context add", target_dir.display());
    println!("  ab list");

    Ok(())
}

/// Run janitor analysis to scan codebase and create issues
fn run_janitor_analysis(repo_path: &PathBuf) -> allbeads::Result<()> {
    use allbeads::git::BossRepo;
    use allbeads::storage::BeadsRepo;

    let beads_repo = BeadsRepo::with_workdir(repo_path);
    let mut created_count = 0;

    // Check for missing README
    if !repo_path.join("README.md").exists() && !repo_path.join("README").exists() {
        beads_repo.create("Add README documentation", "chore", Some(2))?;
        println!("  Created: Add README documentation");
        created_count += 1;
    }

    // Check for missing license
    let license_files = ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"];
    let has_license = license_files.iter().any(|f| repo_path.join(f).exists());
    if !has_license {
        beads_repo.create("Add LICENSE file", "chore", Some(3))?;
        println!("  Created: Add LICENSE file");
        created_count += 1;
    }

    // Check for common config files
    let has_gitignore = repo_path.join(".gitignore").exists();
    if !has_gitignore {
        beads_repo.create("Add .gitignore file", "chore", Some(3))?;
        println!("  Created: Add .gitignore file");
        created_count += 1;
    }

    // Look for TODO/FIXME comments in source files
    let todo_patterns = scan_for_todos(repo_path)?;
    for (_file, _line, text) in todo_patterns.iter().take(10) {
        let title = if text.len() > 60 {
            format!("TODO: {}...", &text[..57])
        } else {
            format!("TODO: {}", text)
        };
        beads_repo.create(&title, "task", Some(3))?;
        println!("  Created: {}", title);
        created_count += 1;
    }

    if todo_patterns.len() > 10 {
        println!("  ... and {} more TODOs found (limited to 10)", todo_patterns.len() - 10);
    }

    // Commit janitor findings if we created any
    if created_count > 0 {
        let boss_repo = BossRepo::from_local(repo_path)?;
        boss_repo.add_beads()?;
        boss_repo.commit(
            &format!("Janitor: Created {} issues from codebase analysis", created_count),
            "AllBeads Janitor",
            "janitor@allbeads.dev",
        )?;
        println!();
        println!("✓ Created {} issues from janitor analysis", created_count);
    } else {
        println!("✓ No issues found - codebase looks clean!");
    }

    Ok(())
}

/// Scan repository for TODO/FIXME comments
fn scan_for_todos(repo_path: &PathBuf) -> allbeads::Result<Vec<(String, usize, String)>> {
    let mut results = Vec::new();

    // Walk directory looking for source files
    fn walk_dir(
        dir: &std::path::Path,
        base: &std::path::Path,
        results: &mut Vec<(String, usize, String)>,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            // Skip common ignored directories
            let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name.starts_with('.')
                || dir_name == "node_modules"
                || dir_name == "target"
                || dir_name == "vendor"
                || dir_name == "dist"
                || dir_name == "build"
            {
                return Ok(());
            }

            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, base, results)?;
                } else if is_source_file(&path) {
                    scan_file_for_todos(&path, base, results)?;
                }
            }
        }
        Ok(())
    }

    fn is_source_file(path: &std::path::Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(
            ext,
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
                | "rb" | "php" | "swift" | "kt" | "scala"
        )
    }

    fn scan_file_for_todos(
        path: &std::path::Path,
        base: &std::path::Path,
        results: &mut Vec<(String, usize, String)>,
    ) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(base)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        for (line_num, line) in content.lines().enumerate() {
            let line_upper = line.to_uppercase();
            if line_upper.contains("TODO") || line_upper.contains("FIXME") || line_upper.contains("HACK") {
                // Extract the comment text
                let text = line.trim().to_string();
                if !text.is_empty() && results.len() < 100 {
                    results.push((relative_path.clone(), line_num + 1, text));
                }
            }
        }
        Ok(())
    }

    walk_dir(repo_path, repo_path, &mut results).map_err(|e| {
        allbeads::AllBeadsError::Io(e)
    })?;

    Ok(results)
}

/// Run comprehensive janitor analysis on a repository
fn run_full_janitor_analysis(repo_path: &PathBuf, verbose: bool, dry_run: bool) -> allbeads::Result<()> {
    use allbeads::git::BossRepo;
    use allbeads::storage::BeadsRepo;

    let mut findings: Vec<JanitorFinding> = Vec::new();

    // Check for missing documentation
    println!("Checking documentation...");
    if !repo_path.join("README.md").exists() && !repo_path.join("README").exists() {
        findings.push(JanitorFinding {
            category: "Documentation",
            title: "Add README documentation".to_string(),
            description: "Repository is missing a README file.".to_string(),
            issue_type: "chore",
            priority: 2,
        });
    }

    let license_files = ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"];
    if !license_files.iter().any(|f| repo_path.join(f).exists()) {
        findings.push(JanitorFinding {
            category: "Documentation",
            title: "Add LICENSE file".to_string(),
            description: "Repository is missing a LICENSE file.".to_string(),
            issue_type: "chore",
            priority: 3,
        });
    }

    if !repo_path.join("CONTRIBUTING.md").exists() {
        findings.push(JanitorFinding {
            category: "Documentation",
            title: "Add CONTRIBUTING guidelines".to_string(),
            description: "Repository is missing contributing guidelines.".to_string(),
            issue_type: "chore",
            priority: 4,
        });
    }

    // Check for configuration files
    println!("Checking configuration...");
    if !repo_path.join(".gitignore").exists() {
        findings.push(JanitorFinding {
            category: "Configuration",
            title: "Add .gitignore file".to_string(),
            description: "Repository is missing a .gitignore file.".to_string(),
            issue_type: "chore",
            priority: 3,
        });
    }

    // Check for security files
    println!("Checking security...");
    if !repo_path.join("SECURITY.md").exists() {
        findings.push(JanitorFinding {
            category: "Security",
            title: "Add SECURITY.md policy".to_string(),
            description: "Repository is missing a security vulnerability reporting policy.".to_string(),
            issue_type: "chore",
            priority: 3,
        });
    }

    // Detect language and check for test directories
    println!("Checking test coverage...");
    let detected_langs = detect_project_languages(repo_path);

    for lang in &detected_langs {
        if verbose {
            println!("  Detected language: {}", lang);
        }
        let test_dirs = get_test_directories(lang);
        let has_tests = test_dirs.iter().any(|d| repo_path.join(d).exists());

        if !has_tests {
            findings.push(JanitorFinding {
                category: "Testing",
                title: format!("Add {} tests", lang),
                description: format!("No test directory found for {} code.", lang),
                issue_type: "task",
                priority: 2,
            });
        }
    }

    // Scan for TODO/FIXME comments
    println!("Scanning for code comments...");
    let todos = scan_for_todos(repo_path)?;

    for (file, line, text) in todos.iter().take(20) {
        let title = if text.len() > 50 {
            format!("{}...", &text[..50])
        } else {
            text.clone()
        };

        let is_fixme = text.to_uppercase().contains("FIXME");
        let is_hack = text.to_uppercase().contains("HACK");

        findings.push(JanitorFinding {
            category: if is_fixme { "Bug" } else if is_hack { "Tech Debt" } else { "Task" },
            title,
            description: format!("Found at {}:{}\n{}", file, line, text),
            issue_type: if is_fixme { "bug" } else { "task" },
            priority: if is_fixme { 2 } else { 3 },
        });
    }

    if todos.len() > 20 {
        println!("  Found {} more code comments (showing first 20)", todos.len() - 20);
    }

    // Check for potential security issues (basic patterns)
    println!("Scanning for potential issues...");
    let security_issues = scan_for_security_patterns(repo_path)?;
    for (file, line, pattern, context) in security_issues.iter().take(10) {
        findings.push(JanitorFinding {
            category: "Security",
            title: format!("Review potential {}", pattern),
            description: format!("Found at {}:{}\n{}", file, line, context),
            issue_type: "bug",
            priority: 1,
        });
    }

    // Print summary
    println!();
    println!("=== Janitor Analysis Summary ===");
    println!();

    let mut by_category: std::collections::HashMap<&str, Vec<&JanitorFinding>> = std::collections::HashMap::new();
    for finding in &findings {
        by_category
            .entry(finding.category)
            .or_default()
            .push(finding);
    }

    for (category, items) in &by_category {
        println!("{} ({} items):", category, items.len());
        for item in items.iter().take(5) {
            println!("  [P{}] {}", item.priority, item.title);
            if verbose {
                for line in item.description.lines().take(2) {
                    println!("       {}", line);
                }
            }
        }
        if items.len() > 5 {
            println!("  ... and {} more", items.len() - 5);
        }
        println!();
    }

    println!("Total findings: {}", findings.len());

    // Create beads if not dry run
    if !dry_run && !findings.is_empty() {
        println!();
        println!("Creating beads...");

        let beads_repo = BeadsRepo::with_workdir(repo_path);

        // Check if beads is initialized
        if !repo_path.join(".beads").exists() {
            beads_repo.init()?;
            println!("  Initialized .beads/ directory");
        }

        let mut created = 0;
        for finding in &findings {
            beads_repo.create(&finding.title, finding.issue_type, Some(finding.priority))?;
            created += 1;
        }

        // Commit findings
        if created > 0 {
            let boss_repo = BossRepo::from_local(repo_path)?;
            boss_repo.add_beads()?;
            boss_repo.commit(
                &format!("Janitor: Created {} issues from codebase analysis", created),
                "AllBeads Janitor",
                "janitor@allbeads.dev",
            )?;
            println!("✓ Created {} beads", created);
        }
    }

    Ok(())
}

/// A finding from janitor analysis
struct JanitorFinding {
    category: &'static str,
    title: String,
    description: String,
    issue_type: &'static str,
    priority: u8,
}

/// Detect programming languages used in the project
fn detect_project_languages(repo_path: &PathBuf) -> Vec<&'static str> {
    let mut langs = Vec::new();

    // Rust
    if repo_path.join("Cargo.toml").exists() {
        langs.push("Rust");
    }

    // Python
    if repo_path.join("pyproject.toml").exists()
        || repo_path.join("setup.py").exists()
        || repo_path.join("requirements.txt").exists()
    {
        langs.push("Python");
    }

    // JavaScript/TypeScript
    if repo_path.join("package.json").exists() {
        if repo_path.join("tsconfig.json").exists() {
            langs.push("TypeScript");
        } else {
            langs.push("JavaScript");
        }
    }

    // Go
    if repo_path.join("go.mod").exists() {
        langs.push("Go");
    }

    // Java
    if repo_path.join("pom.xml").exists() || repo_path.join("build.gradle").exists() {
        langs.push("Java");
    }

    // Ruby
    if repo_path.join("Gemfile").exists() {
        langs.push("Ruby");
    }

    langs
}

/// Get expected test directories for a language
fn get_test_directories(lang: &str) -> Vec<&'static str> {
    match lang {
        "Rust" => vec!["tests", "src"],  // Rust uses tests/ or inline tests
        "Python" => vec!["tests", "test"],
        "JavaScript" | "TypeScript" => vec!["tests", "test", "__tests__", "spec"],
        "Go" => vec!["."],  // Go tests are alongside code
        "Java" => vec!["src/test"],
        "Ruby" => vec!["test", "spec"],
        _ => vec!["tests", "test"],
    }
}

/// Scan for potential security patterns
fn scan_for_security_patterns(repo_path: &PathBuf) -> allbeads::Result<Vec<(String, usize, String, String)>> {
    let mut results = Vec::new();

    // Patterns that might indicate security issues
    let patterns = [
        ("hardcoded secret", r#"(?i)(password|secret|api_key|apikey|token)\s*=\s*["'][^"']+["']"#),
        ("SQL injection risk", r#"(?i)execute\s*\(\s*["'].*\+|format!\s*\([^)]*\{[^}]*\}[^)]*sql"#),
        ("unsafe eval", r#"(?i)\beval\s*\("#),
    ];

    fn walk_for_security(
        dir: &std::path::Path,
        base: &std::path::Path,
        patterns: &[(&str, &str)],
        results: &mut Vec<(String, usize, String, String)>,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name.starts_with('.')
                || dir_name == "node_modules"
                || dir_name == "target"
                || dir_name == "vendor"
            {
                return Ok(());
            }

            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk_for_security(&path, base, patterns, results)?;
                } else {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "rs" | "py" | "js" | "ts" | "go" | "java" | "rb") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let relative = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                            for (line_num, line) in content.lines().enumerate() {
                                for (name, _pattern) in patterns {
                                    // Simple substring check (regex would be better but adds dependency)
                                    let line_lower = line.to_lowercase();
                                    if (name == &"hardcoded secret" &&
                                        (line_lower.contains("password") || line_lower.contains("secret") || line_lower.contains("api_key"))
                                        && line.contains("=") && (line.contains("\"") || line.contains("'")))
                                    || (name == &"unsafe eval" && line_lower.contains("eval("))
                                    {
                                        results.push((
                                            relative.clone(),
                                            line_num + 1,
                                            name.to_string(),
                                            line.trim().to_string(),
                                        ));
                                        if results.len() >= 20 {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    walk_for_security(repo_path, repo_path, &patterns, &mut results).map_err(allbeads::AllBeadsError::Io)?;

    Ok(results)
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

fn parse_priority_arg(s: &str) -> Option<Priority> {
    parse_priority(s).ok()
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
    let context_tags: Vec<_> = bead.labels.iter().filter(|l| l.starts_with('@')).collect();

    print!(
        "[{}] [{}] {}: {}",
        priority_str,
        status_str,
        bead.id.as_str(),
        bead.title
    );

    if !context_tags.is_empty() {
        print!(
            " ({})",
            context_tags
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
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
        println!(
            "Labels:       {}",
            bead.labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !bead.dependencies.is_empty() {
        println!(
            "Depends on:   {}",
            bead.dependencies
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !bead.blocks.is_empty() {
        println!(
            "Blocks:       {}",
            bead.blocks
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
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

fn handle_context_command(
    cmd: &ContextCommands,
    config_path: &Option<String>,
) -> allbeads::Result<()> {
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
        ContextCommands::Add {
            path,
            name,
            url,
            auth,
        } => {
            // Resolve path to absolute
            let repo_path = std::fs::canonicalize(path).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
            })?;

            // Check if it's a git repository
            let git_dir = repo_path.join(".git");
            if !git_dir.exists() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "'{}' is not a git repository (no .git directory)",
                    repo_path.display()
                )));
            }

            // Infer name from folder if not provided
            let context_name = if let Some(n) = name {
                n.clone()
            } else {
                repo_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        allbeads::AllBeadsError::Config(
                            "Could not infer context name from path".to_string(),
                        )
                    })?
            };

            // Check if context already exists
            if config.get_context(&context_name).is_some() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Context '{}' already exists",
                    context_name
                )));
            }

            // Get git remote URL if not provided
            let remote_url = if let Some(u) = url {
                u.clone()
            } else {
                // Run: git -C <path> remote get-url origin
                let output = std::process::Command::new("git")
                    .args([
                        "-C",
                        repo_path.to_str().unwrap(),
                        "remote",
                        "get-url",
                        "origin",
                    ])
                    .output()
                    .map_err(|e| {
                        allbeads::AllBeadsError::Config(format!("Failed to run git: {}", e))
                    })?;

                if !output.status.success() {
                    return Err(allbeads::AllBeadsError::Config(format!(
                        "No 'origin' remote found. Add one with:\n  \
                         git remote add origin <url>\n\n\
                         Or specify the URL explicitly:\n  \
                         ab context add {} --url <url>",
                        path
                    )));
                }

                String::from_utf8_lossy(&output.stdout).trim().to_string()
            };

            // Parse or auto-detect auth strategy
            let auth_strategy = if let Some(ref auth_str) = auth {
                match auth_str.to_lowercase().as_str() {
                    "ssh_agent" => AuthStrategy::SshAgent,
                    "personal_access_token" | "pat" => AuthStrategy::PersonalAccessToken,
                    "gh_enterprise_token" => AuthStrategy::GhEnterpriseToken,
                    _ => return Err(allbeads::AllBeadsError::Parse(format!(
                        "Invalid auth strategy: {}. Must be one of: ssh_agent, personal_access_token, gh_enterprise_token",
                        auth_str
                    ))),
                }
            } else {
                // Auto-detect from URL
                if remote_url.starts_with("https://") {
                    eprintln!("ℹ️  HTTPS URL detected, using personal_access_token auth");
                    AuthStrategy::PersonalAccessToken
                } else {
                    // SSH URL (git@... or ssh://...)
                    AuthStrategy::SshAgent
                }
            };

            // Print before moving auth_strategy
            println!(
                "✓ Added context '{}' from {}",
                context_name,
                repo_path.display()
            );
            println!("  URL:  {}", remote_url);
            println!("  Auth: {:?}", auth_strategy);

            // Create context
            let mut context = BossContext::new(&context_name, &remote_url, auth_strategy);
            context.path = Some(repo_path);

            config.add_context(context);
            config.save(&config_file)?;
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

fn handle_mail_command(cmd: &MailCommands) -> allbeads::Result<()> {
    use allbeads::mail::{
        Address, Message, MessageType, NotifyPayload, Postmaster, RequestPayload, Severity,
    };

    // Get mail database path
    let mail_db_path = AllBeadsConfig::default_path()
        .parent()
        .map(|p| p.join("mail.db"))
        .ok_or_else(|| {
            allbeads::AllBeadsError::Config("Could not determine mail database path".to_string())
        })?;

    // Get project ID from config
    let project_id = match AllBeadsConfig::load_default() {
        Ok(config) => config
            .contexts
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "default".to_string()),
        Err(_) => "default".to_string(),
    };

    let mut postmaster = Postmaster::with_project_id(mail_db_path, &project_id)?;

    match cmd {
        MailCommands::Test { message } => {
            // Send a variety of test messages
            let human = Address::human();

            // 1. Simple notification
            let msg1 = Message::new(
                Address::new("worker", &project_id)?,
                human.clone(),
                MessageType::Notify(NotifyPayload::new(message).with_severity(Severity::Info)),
            );
            postmaster.send(msg1)?;
            println!("Sent: [NOTIFY] {}", message);

            // 2. Request for approval
            let msg2 = Message::new(
                Address::new("build-bot", &project_id)?,
                human.clone(),
                MessageType::Request(
                    RequestPayload::new("Approve deployment to production?")
                        .with_options(vec![
                            "Approve".to_string(),
                            "Deny".to_string(),
                            "Defer".to_string(),
                        ]),
                ),
            );
            postmaster.send(msg2)?;
            println!("Sent: [REQUEST] Approve deployment to production?");

            // 3. Warning notification
            let msg3 = Message::new(
                Address::new("monitor", &project_id)?,
                human.clone(),
                MessageType::Notify(
                    NotifyPayload::new("API rate limit at 80%").with_severity(Severity::Warning),
                ),
            );
            postmaster.send(msg3)?;
            println!("Sent: [NOTIFY] API rate limit at 80%");

            // 4. Success notification
            let msg4 = Message::new(
                Address::new("ci", &project_id)?,
                human,
                MessageType::Notify(
                    NotifyPayload::new("Build succeeded! All 42 tests passed.")
                        .with_severity(Severity::Info),
                ),
            );
            postmaster.send(msg4)?;
            println!("Sent: [NOTIFY] Build succeeded! All 42 tests passed.");

            println!();
            println!("4 test messages sent to inbox. Run 'ab tui' to view them.");
        }

        MailCommands::Inbox => {
            let human = Address::human();
            let messages = postmaster.inbox(&human)?;

            if messages.is_empty() {
                println!("Inbox is empty.");
                println!("Run 'ab mail test' to send some test messages.");
            } else {
                println!("Inbox ({} messages):", messages.len());
                println!();
                for msg in messages {
                    let is_unread =
                        msg.status == allbeads::mail::DeliveryStatus::Delivered;
                    let marker = if is_unread { "*" } else { " " };
                    let type_str = match &msg.message.message_type {
                        MessageType::Notify(_) => "[NOTIFY]",
                        MessageType::Request(_) => "[REQUEST]",
                        MessageType::Lock(_) => "[LOCK]",
                        MessageType::Unlock(_) => "[UNLOCK]",
                        MessageType::Broadcast(_) => "[BROADCAST]",
                        MessageType::Heartbeat(_) => "[HEARTBEAT]",
                        MessageType::Response(_) => "[RESPONSE]",
                    };
                    let summary = match &msg.message.message_type {
                        MessageType::Notify(n) => n.message.clone(),
                        MessageType::Request(r) => r.message.clone(),
                        MessageType::Lock(l) => format!("Lock: {}", l.path),
                        MessageType::Unlock(u) => format!("Unlock: {}", u.path),
                        MessageType::Broadcast(b) => b.message.clone(),
                        MessageType::Heartbeat(h) => format!("Status: {:?}", h.status),
                        MessageType::Response(r) => {
                            r.message.clone().unwrap_or_else(|| format!("{:?}", r.status))
                        }
                    };
                    let time = msg.message.timestamp.format("%H:%M");
                    println!(
                        "{} {} {} from {}: {}",
                        marker, time, type_str, msg.message.from, summary
                    );
                }
            }
        }

        MailCommands::Unread => {
            let human = Address::human();
            let count = postmaster.unread_count(&human)?;
            if count == 0 {
                println!("No unread messages.");
            } else {
                println!("{} unread message(s).", count);
            }
        }
    }

    Ok(())
}

fn handle_jira_command(cmd: &JiraCommands) -> allbeads::Result<()> {
    use allbeads::config::JiraIntegration;
    use allbeads::integrations::JiraAdapter;

    match cmd {
        JiraCommands::Pull {
            project,
            url,
            label,
            verbose,
        } => {
            // Check for token in environment
            let token = std::env::var("JIRA_API_TOKEN").ok();
            if token.is_none() {
                eprintln!("Warning: JIRA_API_TOKEN environment variable not set.");
                eprintln!("Set it with: export JIRA_API_TOKEN='your-api-token'");
                eprintln!();
            }

            let config = JiraIntegration {
                url: url.clone(),
                project: project.clone(),
                token_env: Some("JIRA_API_TOKEN".to_string()),
            };

            let mut adapter = JiraAdapter::new(config);
            if let Some(t) = token {
                adapter.set_auth_token(t);
            }

            println!("Pulling issues from JIRA project {} with label '{}'...", project, label);
            println!();

            // Run async pull
            let rt = tokio::runtime::Runtime::new()?;
            let issues = rt.block_on(async { adapter.pull_agent_issues(label).await })?;

            if issues.is_empty() {
                println!("No issues found with label '{}'", label);
            } else {
                println!("Found {} issues:", issues.len());
                println!();
                for issue in &issues {
                    let status = &issue.fields.status.name;
                    let priority = issue
                        .fields
                        .priority
                        .as_ref()
                        .map(|p| p.name.as_str())
                        .unwrap_or("None");

                    println!(
                        "[{}] [{}] {}: {}",
                        priority, status, issue.key, issue.fields.summary
                    );

                    if *verbose {
                        if let Some(ref desc) = issue.fields.description {
                            let short_desc = if desc.len() > 100 {
                                format!("{}...", &desc[..100])
                            } else {
                                desc.clone()
                            };
                            println!("  Description: {}", short_desc);
                        }
                        println!("  URL: {}/browse/{}", url, issue.key);
                        println!();
                    }
                }
            }
        }

        JiraCommands::Status => {
            let has_token = std::env::var("JIRA_API_TOKEN").is_ok();
            println!("JIRA Integration Status");
            println!();
            println!(
                "  API Token: {}",
                if has_token {
                    "Set (JIRA_API_TOKEN)".to_string()
                } else {
                    "Not set".to_string()
                }
            );
            println!();
            println!("To configure JIRA integration:");
            println!("  1. Create an API token at: https://id.atlassian.com/manage/api-tokens");
            println!("  2. Set the environment variable:");
            println!("     export JIRA_API_TOKEN='your-api-token'");
            println!();
            println!("Usage:");
            println!("  ab jira pull --project PROJ --url https://company.atlassian.net");
        }
    }

    Ok(())
}

fn handle_github_command(cmd: &GitHubCommands) -> allbeads::Result<()> {
    use allbeads::config::GitHubIntegration;
    use allbeads::integrations::GitHubAdapter;

    match cmd {
        GitHubCommands::Pull {
            owner,
            repo,
            label,
            verbose,
        } => {
            // Check for token in environment
            let token = std::env::var("GITHUB_TOKEN")
                .or_else(|_| std::env::var("GH_TOKEN"))
                .ok();
            if token.is_none() {
                eprintln!("Warning: GITHUB_TOKEN or GH_TOKEN environment variable not set.");
                eprintln!("Set it with: export GITHUB_TOKEN='your-personal-access-token'");
                eprintln!();
            }

            let config = GitHubIntegration {
                url: "https://api.github.com".to_string(),
                owner: owner.clone(),
                repo_pattern: repo.clone(),
            };

            let mut adapter = GitHubAdapter::new(config);
            if let Some(t) = token {
                adapter.set_auth_token(t);
            }

            let repo_display = repo.as_deref().unwrap_or("all repositories");
            println!(
                "Pulling issues from GitHub {}/{} with label '{}'...",
                owner, repo_display, label
            );
            println!();

            // Run async pull
            let rt = tokio::runtime::Runtime::new()?;
            let issues = rt.block_on(async { adapter.pull_agent_issues(label).await })?;

            if issues.is_empty() {
                println!("No issues found with label '{}'", label);
            } else {
                println!("Found {} issues:", issues.len());
                println!();
                for issue in &issues {
                    let state_icon = if issue.state == "OPEN" { "O" } else { "C" };
                    let labels: Vec<_> = issue.labels.nodes.iter().map(|l| l.name.as_str()).collect();
                    let labels_str = if labels.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", labels.join(", "))
                    };

                    println!(
                        "[{}] {}#{}: {}{}",
                        state_icon,
                        issue.repository.name_with_owner,
                        issue.number,
                        issue.title,
                        labels_str
                    );

                    if *verbose {
                        if let Some(ref body) = issue.body {
                            let short_body = if body.len() > 100 {
                                format!("{}...", &body[..100])
                            } else {
                                body.clone()
                            };
                            println!("  Body: {}", short_body);
                        }
                        println!("  URL: {}", issue.url);
                        println!();
                    }
                }
            }
        }

        GitHubCommands::Status => {
            let has_token = std::env::var("GITHUB_TOKEN")
                .or_else(|_| std::env::var("GH_TOKEN"))
                .is_ok();
            println!("GitHub Integration Status");
            println!();
            println!(
                "  API Token: {}",
                if has_token {
                    "Set (GITHUB_TOKEN or GH_TOKEN)".to_string()
                } else {
                    "Not set".to_string()
                }
            );
            println!();
            println!("To configure GitHub integration:");
            println!("  1. Create a personal access token at: https://github.com/settings/tokens");
            println!("     (requires 'repo' scope for private repos, 'public_repo' for public)");
            println!("  2. Set the environment variable:");
            println!("     export GITHUB_TOKEN='your-personal-access-token'");
            println!();
            println!("Usage:");
            println!("  ab github pull --owner myorg");
            println!("  ab github pull --owner myorg --repo myrepo");
        }
    }

    Ok(())
}

fn handle_swarm_command(cmd: &SwarmCommands) -> allbeads::Result<()> {
    use allbeads::swarm::{AgentManager, AgentPersona, SpawnRequest};

    // Create a shared agent manager (in a real app, this would be persisted)
    let manager = AgentManager::new();

    match cmd {
        SwarmCommands::List { context, active } => {
            let agents = if let Some(ctx) = context {
                manager.list_by_context(ctx)
            } else if *active {
                manager.list_active()
            } else {
                manager.list()
            };

            if agents.is_empty() {
                println!("No agents found.");
                println!();
                println!("Spawn a demo agent with: ab swarm spawn-demo");
            } else {
                println!("Agents ({}):", agents.len());
                println!();
                for agent in &agents {
                    let status_emoji = agent.status.emoji();
                    let rig_str = agent
                        .rig
                        .as_ref()
                        .map(|r| format!(" [{}]", r))
                        .unwrap_or_default();
                    println!(
                        "{} {} ({}) - {}{} - ${:.2} - {}",
                        status_emoji,
                        agent.name,
                        agent.id,
                        agent.persona,
                        rig_str,
                        agent.cost.total_usd,
                        agent.format_runtime()
                    );
                    if !agent.status_message.is_empty() {
                        println!("    {}", agent.status_message);
                    }
                }
            }
        }

        SwarmCommands::Stats => {
            let stats = manager.stats();

            println!();
            println!("Agent Swarm Statistics");
            println!();
            println!("Agents:");
            println!("  Total:     {}", stats.total_agents);
            println!("  Active:    {}", stats.active_agents);
            println!("  Completed: {}", stats.completed_agents);
            println!("  Errored:   {}", stats.errored_agents);
            println!();
            println!("Cost:");
            println!("  Total:     ${:.2}", stats.total_cost);
            if stats.total_budget > 0.0 {
                println!("  Budget:    ${:.2}", stats.total_budget);
                let percent = (stats.total_cost / stats.total_budget) * 100.0;
                println!("  Used:      {:.1}%", percent);
            }
        }

        SwarmCommands::Budget { context, limit } => {
            manager.set_budget(context, *limit);
            println!("Set budget for context '{}' to ${:.2}", context, limit);
        }

        SwarmCommands::SpawnDemo { name, context, persona } => {
            let agent_persona = match persona.to_lowercase().as_str() {
                "general" => AgentPersona::General,
                "refactor-bot" | "refactorbot" => AgentPersona::RefactorBot,
                "test-writer" | "testwriter" => AgentPersona::TestWriter,
                "security-specialist" | "securityspecialist" => AgentPersona::SecuritySpecialist,
                "frontend-expert" | "frontendexpert" => AgentPersona::FrontendExpert,
                "backend-developer" | "backenddeveloper" => AgentPersona::BackendDeveloper,
                "devops" => AgentPersona::DevOps,
                "tech-writer" | "techwriter" => AgentPersona::TechWriter,
                _ => AgentPersona::Custom(persona.clone()),
            };

            let request = SpawnRequest::new(name, context, "Demo task - exploring codebase")
                .with_persona(agent_persona);

            match manager.spawn(request) {
                Ok(agent_id) => {
                    println!("Spawned demo agent:");
                    println!("  ID:      {}", agent_id);
                    println!("  Name:    {}", name);
                    println!("  Context: {}", context);
                    println!("  Persona: {}", persona);
                    println!();
                    println!("Note: This is a demo agent - it will not actually perform any work.");
                    println!("In a full implementation, agents would be connected to AI providers.");
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        SwarmCommands::Kill { id } => {
            match manager.kill(id) {
                Ok(()) => println!("Killed agent '{}'", id),
                Err(e) => return Err(e),
            }
        }

        SwarmCommands::Pause { id } => {
            match manager.pause(id) {
                Ok(()) => println!("Paused agent '{}'", id),
                Err(e) => return Err(e),
            }
        }

        SwarmCommands::Resume { id } => {
            match manager.resume(id) {
                Ok(()) => println!("Resumed agent '{}'", id),
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}
