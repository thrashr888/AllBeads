//! AllBeads - Distributed Protocol for Agentic Orchestration
//!
//! Main entry point for the AllBeads CLI.

mod commands;

use allbeads::aggregator::{Aggregator, AggregatorConfig, SyncMode};
use allbeads::cache::{Cache, CacheConfig};
use allbeads::config::{AllBeadsConfig, AuthStrategy, BossContext};
use allbeads::graph::{BeadId, IssueType, Priority, Status};
use allbeads::style;
use beads::Beads;
use clap::Parser;
use commands::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    // Initialize logging
    if let Err(e) = allbeads::logging::init() {
        eprintln!("Failed to initialize logging: {}", e);
    }

    // Check for help BEFORE clap parsing for main command only
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1
        || (args.len() == 2 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help"))
    {
        println!("{}", custom_help());
        return;
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Provenance review statistics
#[derive(Debug, Deserialize)]
struct ProvenanceReviews {
    passed: u32,
    iterated: u32,
}

/// Provenance summary from Aiki
#[derive(Debug, Deserialize)]
struct ProvenanceSummary {
    total_changes: u32,
    agents: Vec<(String, u32)>,
    #[serde(default)]
    reviews: Option<ProvenanceReviews>,
    #[serde(default)]
    time_in_review: Option<String>,
}

/// Query Aiki for provenance information about a bead
///
/// Returns a structured summary of changes associated with the bead ID
/// via Aiki's `summary --bead=<id>` command.
fn query_aiki_provenance(
    bead_id: &str,
    context_path: Option<&Path>,
) -> allbeads::Result<ProvenanceSummary> {
    use std::process::Command;

    // Determine where to run the aiki command
    let working_dir = context_path.unwrap_or_else(|| Path::new("."));

    // Try to run `aiki summary --bead=<id> --format=json`
    let output = Command::new("aiki")
        .args(["summary", &format!("--bead={}", bead_id), "--format=json"])
        .current_dir(working_dir)
        .output()
        .map_err(|e| {
            allbeads::AllBeadsError::Config(format!(
                "Failed to execute aiki command: {}. Is Aiki installed?",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(allbeads::AllBeadsError::Config(format!(
            "Aiki command failed: {}",
            stderr.trim()
        )));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to parse Aiki output: {}", e)))
}

/// Display Aiki tasks linked to a bead
fn show_aiki_tasks_for_bead(bead: &allbeads::graph::Bead) -> allbeads::Result<()> {
    use std::process::Command;

    println!("\n  {}", style::header("Aiki Tasks:"));

    // Show linked tasks from bead metadata
    if !bead.aiki_tasks.is_empty() {
        println!("    Linked tasks:");
        for task_id in &bead.aiki_tasks {
            println!("      • {}", style::highlight(task_id));
        }
    } else {
        println!("    No tasks linked to this bead");
        println!("\n    Link a task with:");
        println!("      ab aiki link {} <task-id>", bead.id.as_str());
        return Ok(());
    }

    // Try to query Aiki for task details if available
    let output = Command::new("aiki")
        .args(["task", "list", "--format=xml"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_and_display_aiki_tasks(&stdout, &bead.aiki_tasks);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "\n    {} Unable to fetch task details: {}",
                style::error("✗"),
                stderr.trim()
            );
        }
        Err(_) => {
            println!(
                "\n    {} Aiki not available (task details unavailable)",
                style::warning("⚠")
            );
        }
    }

    Ok(())
}

/// Parse Aiki XML task list and display relevant tasks
fn parse_and_display_aiki_tasks(xml: &str, linked_tasks: &[String]) {
    use std::collections::HashMap;

    // Simple XML parsing - look for <task id="..."> <title>...</title> <status>...</status>
    // This is a basic implementation; could use a proper XML parser later

    let mut tasks: HashMap<String, (String, String)> = HashMap::new(); // id -> (title, status)

    // Split by <task and process each task block
    for chunk in xml.split("<task") {
        if chunk.trim().is_empty() {
            continue;
        }

        // Extract id from id="..."
        let id = if let Some(start) = chunk.find("id=\"") {
            let rest = &chunk[start + 4..];
            rest.find('"').map(|end| &rest[..end])
        } else {
            None
        };

        // Extract title from <title>...</title>
        let title = if let Some(start) = chunk.find("<title>") {
            let rest = &chunk[start + 7..];
            rest.find("</title>").map(|end| &rest[..end])
        } else {
            None
        };

        // Extract status from <status>...</status>
        let status = if let Some(start) = chunk.find("<status>") {
            let rest = &chunk[start + 8..];
            rest.find("</status>").map(|end| &rest[..end])
        } else {
            None
        };

        if let (Some(id), Some(title), Some(status)) = (id, title, status) {
            tasks.insert(id.to_string(), (title.to_string(), status.to_string()));
        }
    }

    // Display details for linked tasks
    if !tasks.is_empty() {
        println!("\n    Task details:");
        for task_id in linked_tasks {
            if let Some((title, status)) = tasks.get(task_id) {
                let status_display = style::status_style(status);
                println!(
                    "      • {} - {} [{}]",
                    style::highlight(task_id),
                    title,
                    status_display
                );
            } else {
                println!(
                    "      • {} - {}",
                    style::highlight(task_id),
                    style::error("not found")
                );
            }
        }
    }
}

fn run(mut cli: Cli) -> allbeads::Result<()> {
    // Get bd-compatible global flags to pass through to wrapper commands
    let bd_flags = cli.bd_global_flags();

    // Take command - if None, show help
    let command = match cli.command.take() {
        Some(cmd) => cmd,
        None => {
            println!("{}", custom_help());
            return Ok(());
        }
    };

    // Handle init command first (creates config)
    if let Commands::Init {
        remote,
        target,
        janitor,
    } = command
    {
        return handle_init_command(&cli.config, remote.as_deref(), target.as_deref(), janitor);
    }

    // Handle context management commands (don't need graph)
    if let Commands::Context(ref ctx_cmd) = command {
        return handle_context_command(ctx_cmd, &cli.config);
    }

    // Handle onboard-repo command (don't need graph)
    if let Commands::OnboardRepo {
        ref path,
        yes,
        skip_init,
        skip_claude,
        skip_context,
    } = command
    {
        use allbeads::onboarding::OnboardingWorkflow;
        let workflow = OnboardingWorkflow::new(path, yes, skip_init, skip_claude, skip_context)?;
        return workflow.run();
    }

    // Handle folder tracking commands (don't need graph)
    if let Commands::Folder(ref folder_cmd) = command {
        return handle_folder_command(folder_cmd);
    }

    // Handle mail commands (don't need graph)
    if let Commands::Mail(ref mail_cmd) = command {
        return handle_mail_command(mail_cmd);
    }

    // Handle JIRA commands (don't need graph)
    if let Commands::Jira(ref jira_cmd) = command {
        return handle_jira_command(jira_cmd);
    }

    // Handle GitHub commands (don't need graph)
    if let Commands::GitHub(ref github_cmd) = command {
        return handle_github_command(github_cmd);
    }

    // Handle swarm commands (don't need graph)
    if let Commands::Swarm(ref swarm_cmd) = command {
        return handle_swarm_command(swarm_cmd);
    }

    // Handle config sync commands (don't need graph)
    if let Commands::Config(ref config_cmd) = command {
        return handle_config_command(config_cmd);
    }

    // Handle plugin commands (don't need graph)
    if let Commands::Plugin(ref plugin_cmd) = command {
        return handle_plugin_command(plugin_cmd);
    }

    // Handle coding agent commands (don't need graph)
    if let Commands::CodingAgent(ref agent_cmd) = command {
        return handle_coding_agent_command(agent_cmd);
    }

    // Handle handoff command (don't need graph)
    if let Commands::Handoff {
        ref id,
        ref agent,
        ready,
        list,
        agents,
        dry_run,
        worktree,
    } = command
    {
        return handle_handoff_command(
            id.as_deref(),
            agent.as_deref(),
            ready,
            list,
            agents,
            dry_run,
            worktree,
        );
    }

    // Handle governance commands (don't need graph)
    if let Commands::Check {
        strict,
        ref policy,
        fix,
        pre_commit,
        ref bead,
        ref format,
    } = command
    {
        return handle_check_command(
            strict,
            policy.as_deref(),
            fix,
            pre_commit,
            bead.as_deref(),
            format,
        );
    }

    if let Commands::Hooks(ref hooks_cmd) = command {
        return handle_hooks_command(hooks_cmd);
    }

    // Handle Aiki commands (don't need graph)
    if let Commands::Aiki(ref aiki_cmd) = command {
        return handle_aiki_command(aiki_cmd);
    }

    // Handle Agents commands (don't need graph)
    if let Commands::Agents(ref agents_cmd) = command {
        return handle_agents_command(agents_cmd, &cli.config);
    }

    // Handle Governance commands (need config but not full graph)
    if let Commands::Governance(ref gov_cmd) = command {
        return handle_governance_command(gov_cmd, &cli.config);
    }

    // Handle Scan commands (need async, no full graph)
    if let Commands::Scan(ref scan_cmd) = command {
        let rt = tokio::runtime::Runtime::new()?;
        return rt.block_on(handle_scan_command(scan_cmd, &cli.config, cli.json));
    }

    // Handle sync command
    if let Commands::Sync {
        all,
        ref context,
        ref message,
        status,
    } = command
    {
        return handle_sync_command(
            all,
            context.as_deref(),
            message.as_deref(),
            status,
            &cli.config,
        );
    }

    // Handle agent commands that don't need graph
    if let Commands::Quickstart = command {
        return handle_quickstart_command();
    }

    if let Commands::Setup = command {
        return handle_setup_command(&cli.config);
    }

    if let Commands::Human { ref message } = command {
        return handle_human_command(message);
    }

    // Handle rename-prefix (doesn't need graph)
    if let Commands::RenamePrefix {
        ref new_prefix,
        ref from,
        ref path,
    } = command
    {
        return handle_rename_prefix_command(new_prefix, from.as_deref(), path, &cli.config);
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

    // Parse context filter (strip @ prefix if present, normalize to lowercase for comparison)
    let context_filter: Vec<String> = if let Some(ref contexts) = cli.contexts {
        contexts
            .split(',')
            .map(|s| s.trim().trim_start_matches('@').to_string())
            .collect()
    } else {
        Vec::new()
    };

    // Validate context filter - ensure all specified contexts exist
    if !context_filter.is_empty() {
        let valid_context_names: Vec<&str> =
            config.contexts.iter().map(|c| c.name.as_str()).collect();
        for ctx in &context_filter {
            if !valid_context_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(ctx))
            {
                eprintln!(
                    "Error: Context '{}' not found. Available contexts: {}",
                    ctx,
                    valid_context_names.join(", ")
                );
                return Ok(());
            }
        }
    }

    // Set up aggregator
    let sync_mode = if cli.cached {
        SyncMode::LocalOnly
    } else {
        SyncMode::Fetch
    };

    let agg_config = AggregatorConfig {
        sync_mode,
        context_filter: context_filter.clone(),
        skip_errors: true,
    };

    // Extract project ID for TUI mail (before config is moved)
    let tui_project_id = config
        .contexts
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "default".to_string());

    // Clone config for use in CRUD wrapper commands
    let config_for_commands = config.clone();

    // Try to load from cache first
    let cache_config = CacheConfig::default();
    let cache = Cache::new(cache_config)?;

    let mut graph = if cli.cached || !cache.is_expired()? {
        tracing::debug!("Attempting to load from cache");
        if let Some(cached_graph) = cache.load_graph()? {
            tracing::info!("Using cached graph");
            cached_graph
        } else {
            tracing::info!("Cache miss, aggregating from Boss repositories");
            eprintln!("⏳ Loading beads from repositories...");
            let mut aggregator = Aggregator::new(config, agg_config)?;
            let graph = aggregator.aggregate()?;
            cache.store_graph(&graph)?;
            eprintln!(
                "✓ Loaded {} beads from {} contexts",
                graph.beads.len(),
                graph.rigs.len()
            );
            graph
        }
    } else {
        tracing::info!("Cache expired, aggregating from Boss repositories");
        eprintln!("⏳ Refreshing beads from repositories...");
        let mut aggregator = Aggregator::new(config, agg_config)?;
        let graph = aggregator.aggregate()?;
        cache.store_graph(&graph)?;
        eprintln!(
            "✓ Loaded {} beads from {} contexts",
            graph.beads.len(),
            graph.rigs.len()
        );
        graph
    };

    // Apply context filter to loaded graph (needed when loading from cache)
    if !context_filter.is_empty() {
        graph.beads.retain(|_, bead| {
            bead.labels.iter().any(|label| {
                if let Some(ctx_name) = label.strip_prefix('@') {
                    context_filter
                        .iter()
                        .any(|f| f.eq_ignore_ascii_case(ctx_name))
                } else {
                    false
                }
            })
        });
    }

    // Execute command
    match command {
        Commands::List {
            status,
            priority,
            context,
            label,
            issue_type,
            assignee,
            ready,
            all,
            limit,
            local,
        } => {
            // Fast path: use local bd list directly (skip aggregation)
            if local {
                let bd = Beads::new().map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Not in a beads repository: {}", e))
                })?;

                // Use bd ready if --ready flag is set
                let issues = if ready {
                    bd.ready().map_err(|e| {
                        allbeads::AllBeadsError::Config(format!("Failed to get ready beads: {}", e))
                    })?
                } else {
                    // Build bd list arguments
                    let status_arg = status.as_deref();
                    bd.list(status_arg, None).map_err(|e| {
                        allbeads::AllBeadsError::Config(format!("Failed to list beads: {}", e))
                    })?
                };

                // Apply additional filters that bd list doesn't support
                let mut filtered: Vec<_> = issues.iter().collect();

                if let Some(priority_str) = &priority {
                    let p = priority_str.trim_start_matches('P').parse::<u8>().ok();
                    filtered.retain(|i| i.priority == p);
                }

                if let Some(label_str) = &label {
                    filtered.retain(|i| i.labels.contains(label_str));
                }

                if let Some(type_str) = &issue_type {
                    let type_lower = type_str.to_lowercase();
                    filtered.retain(|i| i.issue_type.to_lowercase() == type_lower);
                }

                if let Some(assignee_str) = &assignee {
                    filtered.retain(|i| {
                        i.assignee
                            .as_ref()
                            .map_or(false, |a| a.contains(assignee_str))
                    });
                }

                // Filter closed unless --all
                if !all && status.is_none() && !ready {
                    filtered.retain(|i| i.status != "closed");
                }

                // Sort by priority
                filtered.sort_by_key(|i| i.priority.unwrap_or(2));

                // Apply limit
                let display_count = filtered.len().min(limit);
                let total = filtered.len();

                println!("Found {} beads (local):", total);
                println!();
                for issue in filtered.into_iter().take(limit) {
                    let p = issue.priority.unwrap_or(2);
                    println!(
                        "{} [P{}] [{}] {} - {}",
                        style::status_indicator(&issue.status),
                        p,
                        issue.issue_type,
                        style::issue_id(&issue.id),
                        issue.title
                    );
                }
                if display_count < total {
                    println!();
                    println!(
                        "  {} Showing {} of {} (use --limit 0 for all)",
                        style::dim("..."),
                        display_count,
                        total
                    );
                }
                return Ok(());
            }

            let mut beads: Vec<_> = graph.beads.values().collect();

            // Apply ready filter (open, no blockers)
            if ready {
                beads.retain(|b| b.status == Status::Open && b.dependencies.is_empty());
            }

            // Apply filters
            if let Some(status_str) = status {
                let status_filter = parse_status(&status_str)?;
                beads.retain(|b| b.status == status_filter);
            } else if !all && !ready {
                // Default: exclude closed unless --all or --ready
                beads.retain(|b| b.status != Status::Closed);
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

            if let Some(type_str) = issue_type {
                let type_filter = parse_issue_type(&type_str)?;
                beads.retain(|b| b.issue_type == type_filter);
            }

            if let Some(assignee_str) = assignee {
                beads.retain(|b| {
                    b.assignee
                        .as_ref()
                        .map_or(false, |a| a.contains(&assignee_str))
                });
            }

            // Sort by priority then status
            beads.sort_by_key(|b| (b.priority, status_to_sort_key(b.status)));

            // Apply limit
            let total = beads.len();
            let display_count = if limit == 0 { total } else { total.min(limit) };

            // Display results
            println!("Found {} beads:", total);
            println!();
            for bead in beads
                .into_iter()
                .take(if limit == 0 { usize::MAX } else { limit })
            {
                print_bead_summary(bead);
            }
            if display_count < total {
                println!();
                println!(
                    "  {} Showing {} of {} (use --limit 0 for all)",
                    style::dim("..."),
                    display_count,
                    total
                );
            }
        }

        Commands::Show {
            id,
            provenance,
            tasks,
        } => {
            let bead_id = BeadId::new(&id);
            if let Some(bead) = graph.get_bead(&bead_id) {
                print_bead_detailed(bead);

                // Show handoff info if bead has been handed off
                if bead.labels.iter().any(|l| l == "handed-off") {
                    show_handoff_info(&id, bead)?;
                }

                // Show provenance information if requested
                if provenance {
                    match query_aiki_provenance(&id, None) {
                        Ok(prov) => {
                            println!("\n  {}", style::header("Provenance Summary (from Aiki):"));
                            println!("    Total changes: {}", prov.total_changes);
                            if !prov.agents.is_empty() {
                                print!("    Agents: ");
                                let agents_str: Vec<String> = prov
                                    .agents
                                    .iter()
                                    .map(|(agent, count)| format!("{} ({})", agent, count))
                                    .collect();
                                println!("{}", agents_str.join(", "));
                            }
                            if let Some(reviews) = prov.reviews {
                                println!(
                                    "    Reviews: {} passed, {} required iteration",
                                    reviews.passed, reviews.iterated
                                );
                            }
                            if let Some(time) = prov.time_in_review {
                                println!("    Time in review loop: {}", time);
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "\n  {} Unable to fetch provenance: {}",
                                style::error("✗"),
                                e
                            );
                        }
                    }
                }

                // Show linked Aiki tasks if requested
                if tasks {
                    show_aiki_tasks_for_bead(bead)?;
                }
            } else {
                return Err(allbeads::AllBeadsError::IssueNotFound(id));
            }
        }

        Commands::Ready => {
            let mut ready = graph.ready_beads();
            // Sort by priority (lower number = higher priority, like bd)
            ready.sort_by_key(|b| b.priority);
            println!();
            println!(
                "{} Ready work ({} beads with no blockers):",
                style::header("○"),
                style::count_ready(ready.len())
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
            println!(
                "{} Blocked beads ({}):",
                style::error("●"),
                style::count_blocked(blocked.len())
            );
            println!();
            for bead in blocked {
                print_bead_summary(bead);
                if !bead.dependencies.is_empty() {
                    println!(
                        "  {} Blocked by: {}",
                        style::dim("→"),
                        bead.dependencies
                            .iter()
                            .map(|id| style::issue_id(id.as_str()).to_string())
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
                            if status_negated {
                                !matches
                            } else {
                                matches
                            }
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
                            if type_negated {
                                !matches
                            } else {
                                matches
                            }
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
                "title" => {
                    results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                }
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

        Commands::Duplicates {
            threshold,
            include_closed,
        } => {
            // Group beads by similarity (filter to open by default)
            let beads: Vec<_> = graph
                .beads
                .values()
                .filter(|b| include_closed || b.status != allbeads::graph::Status::Closed)
                .collect();
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
                    "{} No potential duplicates found (threshold: {:.0}%)",
                    style::success("✓"),
                    threshold * 100.0
                );
            } else {
                println!(
                    "{} Potential duplicates (threshold: {:.0}%): {} pairs",
                    style::warning("⚠"),
                    threshold * 100.0,
                    duplicates.len()
                );
                println!();
                for (similarity, bead1, bead2) in duplicates {
                    println!(
                        "{} Similarity: {:.0}%",
                        style::warning("~"),
                        similarity * 100.0
                    );
                    println!("  {}: {}", style::issue_id(bead1.id.as_str()), bead1.title);
                    println!("  {}: {}", style::issue_id(bead2.id.as_str()), bead2.title);
                    println!();
                }
            }
        }

        Commands::Stats => {
            let stats = graph.stats();
            let ready_count = graph.ready_beads().len();

            println!();
            println!("{}", style::header("Aggregated Beads Status"));
            println!();
            println!("{}", style::subheader("Summary"));
            println!(
                "  Total Beads:          {}",
                style::count_normal(stats.total_beads)
            );
            println!(
                "  Open:                 {}",
                style::count_ready(stats.open_beads)
            );
            println!(
                "  In Progress:          {}",
                style::count_in_progress(stats.in_progress_beads)
            );
            println!(
                "  Blocked:              {}",
                style::count_blocked(stats.blocked_beads)
            );
            println!(
                "  Closed:               {}",
                style::dim(&stats.closed_beads.to_string())
            );
            println!(
                "  Ready to Work:        {}",
                style::count_ready(ready_count)
            );
            println!();
            println!("{}", style::subheader("Extended"));
            println!(
                "  Shadows:              {}",
                style::dim(&stats.total_shadows.to_string())
            );
            println!(
                "  Rigs:                 {}",
                style::dim(&stats.total_rigs.to_string())
            );

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
                println!("{}", style::subheader("Contexts"));
                let mut contexts: Vec<_> = context_counts.iter().collect();
                contexts.sort_by_key(|(ctx, _)| ctx.as_str());

                for (context, count) in contexts {
                    let open_count = context_open.get(context).unwrap_or(&0);
                    let context_name = context.trim_start_matches('@');
                    println!(
                        "  {:<15} {} beads ({} open)",
                        style::path(context_name),
                        count,
                        style::count_ready(*open_count)
                    );
                }
            }

            // Cache stats
            let cache_stats = cache.stats()?;
            println!();
            println!("{}", style::subheader("Cache"));
            println!(
                "  Beads cached:         {}",
                style::dim(&cache_stats.bead_count.to_string())
            );
            println!(
                "  Rigs cached:          {}",
                style::dim(&cache_stats.rig_count.to_string())
            );
            if let Some(age) = cache_stats.age {
                println!(
                    "  Cache age:            {}",
                    style::dim(&format!("{:.1}s", age.as_secs_f64()))
                );
            }
            let expired_str = if cache_stats.is_expired {
                style::error("true")
            } else {
                style::success("false")
            };
            println!("  Expired:              {}", expired_str);
            println!();
            println!(
                "{}",
                style::dim("For more details, use 'ab list' to see individual beads.")
            );
        }

        Commands::Tui => {
            // Determine mail database path (in config directory)
            let mail_db_path = AllBeadsConfig::default_path()
                .parent()
                .map(|p| p.join("mail.db"));

            let tui_result = allbeads::tui::run_with_mail(graph, mail_db_path, &tui_project_id)?;

            // Handle onboarding request from GitHub picker
            if !tui_result.repos_to_onboard.is_empty() {
                println!(
                    "\nOnboarding {} repositories...\n",
                    tui_result.repos_to_onboard.len()
                );
                for repo_url in &tui_result.repos_to_onboard {
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    handle_onboard_repository(
                        repo_url,
                        true,  // non_interactive
                        false, // skip_clone
                        false, // skip_beads
                        false, // skip_skills
                        false, // skip_hooks
                        false, // skip_issues
                        None,  // context_name (auto-generate)
                        None,  // custom_path (use default)
                        &config_for_commands,
                    )?;
                    println!();
                }
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!(
                    "✓ Onboarding complete for {} repositories",
                    tui_result.repos_to_onboard.len()
                );
            }
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

        Commands::Info => {
            handle_info_command(&graph)?;
        }

        Commands::Prime => {
            handle_prime_command(&graph)?;
        }

        Commands::Onboard {
            target,
            wizard,
            non_interactive,
            skip_clone,
            skip_beads,
            skip_skills,
            skip_hooks,
            skip_issues,
            context_name,
            path,
        } => {
            if wizard {
                // Use the guided wizard
                use allbeads::onboarding::OnboardingWizard;
                let repo_path = if target == "." {
                    std::env::current_dir()?
                } else if target.starts_with("http") || target.starts_with("git@") {
                    eprintln!("Wizard mode requires a local path. Use without --wizard for URLs.");
                    return Ok(());
                } else {
                    std::path::PathBuf::from(&target)
                };
                let mut wiz = OnboardingWizard::new(&repo_path)?;
                wiz.run()?;
            } else {
                handle_onboard_repository(
                    &target,
                    non_interactive,
                    skip_clone,
                    skip_beads,
                    skip_skills,
                    skip_hooks,
                    skip_issues,
                    context_name.as_deref(),
                    path.as_deref(),
                    &config_for_commands,
                )?;
            }
        }

        Commands::Update {
            id,
            status,
            priority,
            assignee,
        } => {
            // Find which context this bead belongs to
            let bead_id = allbeads::graph::BeadId::from(id.as_str());
            if let Some(bead) = graph.beads.get(&bead_id) {
                // Get the context path from the bead's labels
                let context_label = bead
                    .labels
                    .iter()
                    .find(|l| l.starts_with('@'))
                    .map(|l| l.trim_start_matches('@'));

                if let Some(ctx_name) = context_label {
                    // Find the context path
                    if let Some(ctx) = config_for_commands
                        .contexts
                        .iter()
                        .find(|c| c.name == ctx_name)
                    {
                        if let Some(ctx_path) = &ctx.path {
                            println!(
                                "Updating {} in context @{}...",
                                style::issue_id(&id),
                                ctx_name
                            );

                            // Parse priority string to u8 if provided
                            let priority_u8 = priority
                                .as_ref()
                                .and_then(|p| p.trim_start_matches('P').parse::<u8>().ok());

                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            match bd.update(
                                &id,
                                status.as_deref(),
                                priority_u8,
                                assignee.as_deref(),
                                None, // title
                            ) {
                                Ok(output) => {
                                    if output.success {
                                        println!("{}", output.stdout);
                                    } else {
                                        eprintln!("{}", output.stderr);
                                    }
                                }
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        } else {
                            eprintln!("Context '{}' has no local path configured", ctx_name);
                        }
                    } else {
                        eprintln!("Context '{}' not found in config", ctx_name);
                    }
                } else {
                    eprintln!("Could not determine context for bead {}", id);
                }
            } else {
                eprintln!("Bead {} not found", id);
            }
        }

        Commands::Close { ids, reason } => {
            // Helper to find context by reading .beads/config.yaml prefix
            fn find_context_by_prefix<'a>(
                prefix: &str,
                contexts: &'a [allbeads::config::BossContext],
            ) -> Option<&'a allbeads::config::BossContext> {
                for ctx in contexts {
                    if let Some(path) = &ctx.path {
                        let config_path = std::path::Path::new(path).join(".beads/config.yaml");
                        if let Ok(content) = std::fs::read_to_string(&config_path) {
                            // Parse issue-prefix from YAML
                            for line in content.lines() {
                                if let Some(value) = line.strip_prefix("issue-prefix:") {
                                    let ctx_prefix =
                                        value.trim().trim_matches('"').trim_matches('\'');
                                    if ctx_prefix.eq_ignore_ascii_case(prefix) {
                                        return Some(ctx);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }

            // Group beads by context
            let mut by_context: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for id in &ids {
                let bead_id = allbeads::graph::BeadId::from(id.as_str());

                // First try to find in graph
                if let Some(bead) = graph.beads.get(&bead_id) {
                    if let Some(ctx_name) = bead
                        .labels
                        .iter()
                        .find(|l| l.starts_with('@'))
                        .map(|l| l.trim_start_matches('@').to_string())
                    {
                        by_context.entry(ctx_name).or_default().push(id.clone());
                        continue;
                    }
                }

                // Fallback: extract prefix from ID and find matching context
                if let Some(prefix) = id.split('-').next() {
                    if let Some(ctx) = find_context_by_prefix(prefix, &config_for_commands.contexts)
                    {
                        by_context
                            .entry(ctx.name.clone())
                            .or_default()
                            .push(id.clone());
                        continue;
                    }
                }

                eprintln!("Warning: Could not determine context for bead {}", id);
            }

            if by_context.is_empty() {
                eprintln!("No beads to close");
                return Ok(());
            }

            for (ctx_name, bead_ids) in by_context {
                if let Some(ctx) = config_for_commands
                    .contexts
                    .iter()
                    .find(|c| c.name == ctx_name)
                {
                    if let Some(ctx_path) = &ctx.path {
                        println!(
                            "Closing {} bead(s) in context @{}...",
                            bead_ids.len(),
                            ctx_name
                        );

                        let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                        let result = if let Some(r) = &reason {
                            // Use run() for close with reason (close_multiple doesn't support reason)
                            let mut args: Vec<&str> = vec!["close"];
                            let id_refs: Vec<&str> = bead_ids.iter().map(|s| s.as_str()).collect();
                            args.extend(id_refs);
                            let reason_arg = format!("--reason={}", r);
                            args.push(&reason_arg);
                            bd.run(&args)
                        } else {
                            let id_refs: Vec<&str> = bead_ids.iter().map(|s| s.as_str()).collect();
                            bd.close_multiple(&id_refs)
                        };

                        match result {
                            Ok(output) => {
                                if output.success {
                                    println!("{}", output.stdout);
                                } else {
                                    eprintln!("{}", output.stderr);
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                }
            }
        }

        Commands::Create {
            title,
            issue_type,
            priority,
            context,
        } => {
            // Find the target context
            let ctx_name = context.unwrap_or_else(|| {
                // Try to determine from current directory
                let cwd = std::env::current_dir().unwrap_or_default();
                config_for_commands
                    .contexts
                    .iter()
                    .find(|c| c.path.as_ref().is_some_and(|p| cwd.starts_with(p)))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "default".to_string())
            });

            if let Some(ctx) = config_for_commands
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
            {
                if let Some(ctx_path) = &ctx.path {
                    println!("Creating bead in context @{}...", ctx_name);

                    // Parse priority string to u8
                    let priority_u8 = priority.trim_start_matches('P').parse::<u8>().ok();

                    let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                    match bd.create(&title, &issue_type, priority_u8, None) {
                        Ok(output) => {
                            if output.success {
                                println!("{}", output.stdout);
                            } else {
                                eprintln!("{}", output.stderr);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Context '{}' has no local path configured", ctx_name);
                }
            } else {
                eprintln!("Context '{}' not found", ctx_name);
            }
        }

        Commands::Reopen { ids } => {
            // Group beads by context
            let mut by_context: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for id in &ids {
                let bead_id = allbeads::graph::BeadId::from(id.as_str());
                if let Some(bead) = graph.beads.get(&bead_id) {
                    if let Some(ctx_name) = bead
                        .labels
                        .iter()
                        .find(|l| l.starts_with('@'))
                        .map(|l| l.trim_start_matches('@').to_string())
                    {
                        by_context.entry(ctx_name).or_default().push(id.clone());
                    }
                }
            }

            for (ctx_name, bead_ids) in by_context {
                if let Some(ctx) = config_for_commands
                    .contexts
                    .iter()
                    .find(|c| c.name == ctx_name)
                {
                    if let Some(ctx_path) = &ctx.path {
                        println!(
                            "Reopening {} bead(s) in context @{}...",
                            bead_ids.len(),
                            ctx_name
                        );

                        let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                        let id_refs: Vec<&str> = bead_ids.iter().map(|s| s.as_str()).collect();
                        match bd.reopen_multiple(&id_refs) {
                            Ok(output) => {
                                if output.success {
                                    println!("{}", output.stdout);
                                } else {
                                    eprintln!("{}", output.stderr);
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                }
            }
        }

        Commands::Dep(dep_cmd) => {
            match dep_cmd {
                DepCommands::Add { issue, depends_on } => {
                    // Find which context the issue belongs to
                    let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                    if let Some(bead) = graph.beads.get(&bead_id) {
                        if let Some(ctx_name) = bead
                            .labels
                            .iter()
                            .find(|l| l.starts_with('@'))
                            .map(|l| l.trim_start_matches('@'))
                        {
                            if let Some(ctx) = config_for_commands
                                .contexts
                                .iter()
                                .find(|c| c.name == ctx_name)
                            {
                                if let Some(ctx_path) = &ctx.path {
                                    let bd =
                                        Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                    match bd.dep_add(&issue, &depends_on) {
                                        Ok(output) => println!("{}", output.stdout),
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Bead {} not found", issue);
                    }
                }
                DepCommands::Remove { issue, depends_on } => {
                    let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                    if let Some(bead) = graph.beads.get(&bead_id) {
                        if let Some(ctx_name) = bead
                            .labels
                            .iter()
                            .find(|l| l.starts_with('@'))
                            .map(|l| l.trim_start_matches('@'))
                        {
                            if let Some(ctx) = config_for_commands
                                .contexts
                                .iter()
                                .find(|c| c.name == ctx_name)
                            {
                                if let Some(ctx_path) = &ctx.path {
                                    let bd =
                                        Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                    match bd.dep_remove(&issue, &depends_on) {
                                        Ok(output) => println!("{}", output.stdout),
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Bead {} not found", issue);
                    }
                }
            }
        }

        Commands::Label(label_cmd) => {
            match label_cmd {
                LabelCommands::Add { issue, label } => {
                    let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                    if let Some(bead) = graph.beads.get(&bead_id) {
                        if let Some(ctx_name) = bead
                            .labels
                            .iter()
                            .find(|l| l.starts_with('@'))
                            .map(|l| l.trim_start_matches('@'))
                        {
                            if let Some(ctx) = config_for_commands
                                .contexts
                                .iter()
                                .find(|c| c.name == ctx_name)
                            {
                                if let Some(ctx_path) = &ctx.path {
                                    let bd =
                                        Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                    match bd.label_add(&issue, &label) {
                                        Ok(output) => println!("{}", output.stdout),
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Bead {} not found", issue);
                    }
                }
                LabelCommands::Remove { issue, label } => {
                    let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                    if let Some(bead) = graph.beads.get(&bead_id) {
                        if let Some(ctx_name) = bead
                            .labels
                            .iter()
                            .find(|l| l.starts_with('@'))
                            .map(|l| l.trim_start_matches('@'))
                        {
                            if let Some(ctx) = config_for_commands
                                .contexts
                                .iter()
                                .find(|c| c.name == ctx_name)
                            {
                                if let Some(ctx_path) = &ctx.path {
                                    let bd =
                                        Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                    match bd.label_remove(&issue, &label) {
                                        Ok(output) => println!("{}", output.stdout),
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Bead {} not found", issue);
                    }
                }
                LabelCommands::List => {
                    // List labels from all contexts
                    for ctx in &config_for_commands.contexts {
                        if let Some(ctx_path) = &ctx.path {
                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            println!("Labels in @{}:", ctx.name);
                            match bd.label_list() {
                                Ok(output) => println!("{}", output.stdout),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                }
            }
        }

        Commands::Comments(comment_cmd) => match comment_cmd {
            CommentCommands::List { issue } => {
                let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                if let Some(bead) = graph.beads.get(&bead_id) {
                    if let Some(ctx_name) = bead
                        .labels
                        .iter()
                        .find(|l| l.starts_with('@'))
                        .map(|l| l.trim_start_matches('@'))
                    {
                        if let Some(ctx) = config_for_commands
                            .contexts
                            .iter()
                            .find(|c| c.name == ctx_name)
                        {
                            if let Some(ctx_path) = &ctx.path {
                                let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                match bd.comments(&issue) {
                                    Ok(comments) => {
                                        if comments.is_empty() {
                                            println!("No comments on {}", issue);
                                        } else {
                                            for comment in comments {
                                                println!(
                                                    "--- {} ({}) ---",
                                                    comment.author,
                                                    comment.created_at.unwrap_or_default()
                                                );
                                                println!("{}\n", comment.content);
                                            }
                                        }
                                    }
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("Bead {} not found", issue);
                }
            }
            CommentCommands::Add { issue, content } => {
                let bead_id = allbeads::graph::BeadId::from(issue.as_str());
                if let Some(bead) = graph.beads.get(&bead_id) {
                    if let Some(ctx_name) = bead
                        .labels
                        .iter()
                        .find(|l| l.starts_with('@'))
                        .map(|l| l.trim_start_matches('@'))
                    {
                        if let Some(ctx) = config_for_commands
                            .contexts
                            .iter()
                            .find(|c| c.name == ctx_name)
                        {
                            if let Some(ctx_path) = &ctx.path {
                                let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                match bd.comment_add(&issue, &content) {
                                    Ok(output) => println!("{}", output.stdout),
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("Bead {} not found", issue);
                }
            }
        },

        Commands::Q {
            title,
            issue_type,
            priority,
            context,
        } => {
            // Find the target context
            let ctx_name = context.unwrap_or_else(|| {
                let cwd = std::env::current_dir().unwrap_or_default();
                config_for_commands
                    .contexts
                    .iter()
                    .find(|c| c.path.as_ref().is_some_and(|p| cwd.starts_with(p)))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "default".to_string())
            });

            if let Some(ctx) = config_for_commands
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
            {
                if let Some(ctx_path) = &ctx.path {
                    let priority_u8 = priority
                        .as_ref()
                        .and_then(|p| p.trim_start_matches('P').parse::<u8>().ok());

                    let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                    match bd.quick_create_full(&title, issue_type.as_deref(), priority_u8) {
                        Ok(id) => println!("{}", id),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Context '{}' has no local path configured", ctx_name);
                }
            } else {
                eprintln!("Context '{}' not found", ctx_name);
            }
        }

        Commands::Epic(epic_cmd) => {
            match epic_cmd {
                EpicCommands::List { open } => {
                    // List epics from all contexts
                    for ctx in &config_for_commands.contexts {
                        if let Some(ctx_path) = &ctx.path {
                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            let result: beads::Result<Vec<beads::Issue>> = if open {
                                bd.epic_list_open()
                            } else {
                                bd.epic_list()
                            };
                            match result {
                                Ok(epics) => {
                                    if !epics.is_empty() {
                                        println!("Epics in @{}:", ctx.name);
                                        for epic in epics {
                                            println!(
                                                "  {} [P{}] - {}",
                                                epic.id,
                                                epic.priority.unwrap_or(2),
                                                epic.title
                                            );
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Error listing epics in @{}: {}", ctx.name, e),
                            }
                        }
                    }
                }
                EpicCommands::Create {
                    title,
                    priority,
                    context,
                } => {
                    let ctx_name = context.clone().unwrap_or_else(|| {
                        let cwd = std::env::current_dir().unwrap_or_default();
                        config_for_commands
                            .contexts
                            .iter()
                            .find(|c| c.path.as_ref().is_some_and(|p| cwd.starts_with(p)))
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| "default".to_string())
                    });

                    if let Some(ctx) = config_for_commands
                        .contexts
                        .iter()
                        .find(|c| c.name == ctx_name)
                    {
                        if let Some(ctx_path) = &ctx.path {
                            let priority_u8 = priority.trim_start_matches('P').parse::<u8>().ok();
                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            match bd.create_epic(&title, priority_u8) {
                                Ok(output) => println!("{}", output.stdout),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                }
                EpicCommands::Show { id } => {
                    let bead_id = allbeads::graph::BeadId::from(id.as_str());
                    if let Some(bead) = graph.beads.get(&bead_id) {
                        if let Some(ctx_name) = bead
                            .labels
                            .iter()
                            .find(|l| l.starts_with('@'))
                            .map(|l| l.trim_start_matches('@'))
                        {
                            if let Some(ctx) = config_for_commands
                                .contexts
                                .iter()
                                .find(|c| c.name == ctx_name)
                            {
                                if let Some(ctx_path) = &ctx.path {
                                    let bd =
                                        Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                                    match bd.epic_show(&id) {
                                        Ok(epic) => {
                                            println!("{}: {}", epic.id, epic.title);
                                            println!("Status: {}", epic.status);
                                            if let Some(desc) = &epic.description {
                                                println!("Description: {}", desc);
                                            }
                                        }
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Epic {} not found", id);
                    }
                }
            }
        }

        Commands::Edit { id, field } => {
            let bead_id = allbeads::graph::BeadId::from(id.as_str());
            if let Some(bead) = graph.beads.get(&bead_id) {
                if let Some(ctx_name) = bead
                    .labels
                    .iter()
                    .find(|l| l.starts_with('@'))
                    .map(|l| l.trim_start_matches('@'))
                {
                    if let Some(ctx) = config_for_commands
                        .contexts
                        .iter()
                        .find(|c| c.name == ctx_name)
                    {
                        if let Some(ctx_path) = &ctx.path {
                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            match bd.edit(&id, field.as_deref()) {
                                Ok(output) => println!("{}", output.stdout),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                }
            } else {
                eprintln!("Bead {} not found", id);
            }
        }

        Commands::Delete { ids, yes: _ } => {
            // Group beads by context
            let mut by_context: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for id in &ids {
                let bead_id = allbeads::graph::BeadId::from(id.as_str());
                if let Some(bead) = graph.beads.get(&bead_id) {
                    if let Some(ctx_name) = bead
                        .labels
                        .iter()
                        .find(|l| l.starts_with('@'))
                        .map(|l| l.trim_start_matches('@').to_string())
                    {
                        by_context.entry(ctx_name).or_default().push(id.clone());
                    }
                }
            }

            for (ctx_name, bead_ids) in by_context {
                if let Some(ctx) = config_for_commands
                    .contexts
                    .iter()
                    .find(|c| c.name == ctx_name)
                {
                    if let Some(ctx_path) = &ctx.path {
                        println!(
                            "Deleting {} bead(s) in context @{}...",
                            bead_ids.len(),
                            ctx_name
                        );

                        let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                        let id_refs: Vec<&str> = bead_ids.iter().map(|s| s.as_str()).collect();
                        match bd.delete_multiple(&id_refs) {
                            Ok(output) => {
                                if output.success {
                                    println!("{}", output.stdout);
                                } else {
                                    eprintln!("{}", output.stderr);
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                }
            }
        }

        Commands::Duplicate { id, of } => {
            let bead_id = allbeads::graph::BeadId::from(id.as_str());
            if let Some(bead) = graph.beads.get(&bead_id) {
                if let Some(ctx_name) = bead
                    .labels
                    .iter()
                    .find(|l| l.starts_with('@'))
                    .map(|l| l.trim_start_matches('@'))
                {
                    if let Some(ctx) = config_for_commands
                        .contexts
                        .iter()
                        .find(|c| c.name == ctx_name)
                    {
                        if let Some(ctx_path) = &ctx.path {
                            let bd = Beads::with_workdir_and_flags(ctx_path, bd_flags.clone());
                            match bd.duplicate(&id, &of) {
                                Ok(output) => println!("{}", output.stdout),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                }
            } else {
                eprintln!("Bead {} not found", id);
            }
        }

        Commands::RenamePrefix { .. }
        | Commands::Context(_)
        | Commands::Init { .. }
        | Commands::OnboardRepo { .. }
        | Commands::Mail(_)
        | Commands::Folder(_)
        | Commands::Jira(_)
        | Commands::GitHub(_)
        | Commands::Swarm(_)
        | Commands::Config(_)
        | Commands::Quickstart
        | Commands::Setup
        | Commands::Human { .. }
        | Commands::Plugin(_)
        | Commands::CodingAgent(_)
        | Commands::Handoff { .. }
        | Commands::Sync { .. }
        | Commands::Check { .. }
        | Commands::Hooks(_)
        | Commands::Aiki(_)
        | Commands::Agents(_)
        | Commands::Governance(_)
        | Commands::Scan(_) => {
            // Handled earlier in the function
            unreachable!("Context, Init, OnboardRepo, Mail, Jira, GitHub, Swarm, Config, Plugin, Handoff, Sync, Quickstart, Setup, Human, Check, Hooks, Aiki, Agents, Governance, and Scan commands should be handled before aggregation")
        }
    }

    Ok(())
}

fn handle_rename_prefix_command(
    new_prefix: &str,
    from: Option<&str>,
    path: &str,
    config_path: &Option<String>,
) -> allbeads::Result<()> {
    use beads::Beads;

    // Load config for context search
    let config = if let Some(ref cp) = config_path {
        AllBeadsConfig::load(cp)?
    } else {
        AllBeadsConfig::load_default()?
    };

    // Determine target path: either from --from prefix search or --path
    let target_path = if let Some(old_prefix) = from {
        // Search all contexts for one with matching prefix
        let mut found_path: Option<std::path::PathBuf> = None;

        for ctx in &config.contexts {
            if let Some(ref ctx_path) = ctx.path {
                // First, check config.yaml for issue-prefix
                let config_file = ctx_path.join(".beads/config.yaml");
                if config_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_file) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('#') {
                                continue;
                            }
                            if trimmed.starts_with("issue-prefix:") {
                                let prefix = trimmed
                                    .trim_start_matches("issue-prefix:")
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'');
                                if prefix == old_prefix {
                                    found_path = Some(ctx_path.clone());
                                    break;
                                }
                            }
                        }
                    }
                }

                // If not found in config, check issues.jsonl for issue IDs with this prefix
                if found_path.is_none() {
                    let jsonl_file = ctx_path.join(".beads/issues.jsonl");
                    if jsonl_file.exists() {
                        if let Ok(content) = std::fs::read_to_string(&jsonl_file) {
                            if let Some(first_line) = content.lines().next() {
                                if let Ok(issue) = serde_json::from_str::<serde_json::Value>(first_line) {
                                    if let Some(id) = issue.get("id").and_then(|v| v.as_str()) {
                                        if let Some(dash_pos) = id.rfind('-') {
                                            let prefix = &id[..dash_pos];
                                            if prefix == old_prefix {
                                                found_path = Some(ctx_path.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if found_path.is_some() {
                    break;
                }
            }
        }

        match found_path {
            Some(p) => {
                println!("Found context with prefix '{}' at {}", old_prefix, p.display());
                p
            }
            None => {
                eprintln!("Error: No context found with prefix '{}'", old_prefix);
                eprintln!("Available contexts with local paths:");
                for ctx in &config.contexts {
                    if let Some(ref ctx_path) = ctx.path {
                        eprintln!("  - {} ({})", ctx.name, ctx_path.display());
                    }
                }
                return Ok(());
            }
        }
    } else {
        std::fs::canonicalize(path).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e))
        })?
    };

    let bd = Beads::with_workdir(&target_path);
    match bd.rename_prefix(new_prefix) {
        Ok(output) => {
            println!("{}", output.stdout);
            if !output.stderr.is_empty() {
                eprintln!("{}", output.stderr);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
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
    let _repo = git2::Repository::clone(remote_url, &target_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to clone repository: {}", e)))?;

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
fn run_janitor_analysis(repo_path: &Path) -> allbeads::Result<()> {
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
        println!(
            "  ... and {} more TODOs found (limited to 10)",
            todo_patterns.len() - 10
        );
    }

    // Commit janitor findings if we created any
    if created_count > 0 {
        let boss_repo = BossRepo::from_local(repo_path)?;
        boss_repo.add_beads()?;
        boss_repo.commit(
            &format!(
                "Janitor: Created {} issues from codebase analysis",
                created_count
            ),
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
fn scan_for_todos(repo_path: &std::path::Path) -> allbeads::Result<Vec<(String, usize, String)>> {
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
            "rs" | "py"
                | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "rb"
                | "php"
                | "swift"
                | "kt"
                | "scala"
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
            if line_upper.contains("TODO")
                || line_upper.contains("FIXME")
                || line_upper.contains("HACK")
            {
                // Extract the comment text
                let text = line.trim().to_string();
                if !text.is_empty() && results.len() < 100 {
                    results.push((relative_path.clone(), line_num + 1, text));
                }
            }
        }
        Ok(())
    }

    walk_dir(repo_path, repo_path, &mut results).map_err(allbeads::AllBeadsError::Io)?;

    Ok(results)
}

/// Run comprehensive janitor analysis on a repository
fn run_full_janitor_analysis(
    repo_path: &Path,
    verbose: bool,
    dry_run: bool,
) -> allbeads::Result<()> {
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
            description: "Repository is missing a security vulnerability reporting policy."
                .to_string(),
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
            category: if is_fixme {
                "Bug"
            } else if is_hack {
                "Tech Debt"
            } else {
                "Task"
            },
            title,
            description: format!("Found at {}:{}\n{}", file, line, text),
            issue_type: if is_fixme { "bug" } else { "task" },
            priority: if is_fixme { 2 } else { 3 },
        });
    }

    if todos.len() > 20 {
        println!(
            "  Found {} more code comments (showing first 20)",
            todos.len() - 20
        );
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

    let mut by_category: std::collections::HashMap<&str, Vec<&JanitorFinding>> =
        std::collections::HashMap::new();
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
fn detect_project_languages(repo_path: &std::path::Path) -> Vec<&'static str> {
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
        "Rust" => vec!["tests", "src"], // Rust uses tests/ or inline tests
        "Python" => vec!["tests", "test"],
        "JavaScript" | "TypeScript" => vec!["tests", "test", "__tests__", "spec"],
        "Go" => vec!["."], // Go tests are alongside code
        "Java" => vec!["src/test"],
        "Ruby" => vec!["test", "spec"],
        _ => vec!["tests", "test"],
    }
}

/// Scan for potential security patterns
fn scan_for_security_patterns(
    repo_path: &std::path::Path,
) -> allbeads::Result<Vec<(String, usize, String, String)>> {
    let mut results = Vec::new();

    // Patterns that might indicate security issues
    let patterns = [
        (
            "hardcoded secret",
            r#"(?i)(password|secret|api_key|apikey|token)\s*=\s*["'][^"']+["']"#,
        ),
        (
            "SQL injection risk",
            r#"(?i)execute\s*\(\s*["'].*\+|format!\s*\([^)]*\{[^}]*\}[^)]*sql"#,
        ),
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
                            let relative = path
                                .strip_prefix(base)
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .to_string();
                            for (line_num, line) in content.lines().enumerate() {
                                // Skip lines that are pattern definitions (avoid self-detection)
                                if line.contains("r#\"") || line.contains("name == &\"") {
                                    continue;
                                }
                                for (name, _pattern) in patterns {
                                    // Simple substring check (regex would be better but adds dependency)
                                    let line_lower = line.to_lowercase();
                                    if (name == &"hardcoded secret"
                                        && (line_lower.contains("password")
                                            || line_lower.contains("secret")
                                            || line_lower.contains("api_key"))
                                        && line.contains("=")
                                        && (line.contains("\"") || line.contains("'")))
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

    walk_for_security(repo_path, repo_path, &patterns, &mut results)
        .map_err(allbeads::AllBeadsError::Io)?;

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

fn parse_issue_type(s: &str) -> allbeads::Result<IssueType> {
    match s.to_lowercase().as_str() {
        "bug" => Ok(IssueType::Bug),
        "feature" => Ok(IssueType::Feature),
        "task" => Ok(IssueType::Task),
        "epic" => Ok(IssueType::Epic),
        "chore" => Ok(IssueType::Chore),
        "merge_request" | "merge-request" | "mr" => Ok(IssueType::MergeRequest),
        "molecule" => Ok(IssueType::Molecule),
        "gate" => Ok(IssueType::Gate),
        _ => Err(allbeads::AllBeadsError::Parse(format!(
            "Invalid type: {}. Must be one of: bug, feature, task, epic, chore, merge_request, molecule, gate",
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
    let priority_num = priority_to_num(bead.priority);
    let type_str = format_issue_type(bead.issue_type);
    let context_tags: Vec<_> = bead.labels.iter().filter(|l| l.starts_with('@')).collect();

    // Format: [P1] [task] id: title
    print!(
        "{} {} {} {} - {}",
        style::status_indicator(format_status(bead.status)),
        style::priority_style(priority_num),
        style::type_style(type_str),
        style::issue_id(bead.id.as_str()),
        bead.title
    );

    if !context_tags.is_empty() {
        print!(
            " {}",
            style::dim(&format!(
                "({})",
                context_tags
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        );
    }

    println!();
}

fn print_bead_detailed(bead: &allbeads::graph::Bead) {
    let priority_num = priority_to_num(bead.priority);
    let type_str = format_issue_type(bead.issue_type);
    let status_str = format_status(bead.status);

    // Header with ID and title
    println!(
        "{} {} - {}",
        style::issue_id(bead.id.as_str()),
        style::header(&bead.title),
        style::dim(&format!("({})", type_str))
    );
    println!();

    // Metadata
    println!(
        "  {} {}  {} {}",
        style::dim("Status:"),
        style::status_style(status_str),
        style::dim("Priority:"),
        style::priority_style(priority_num)
    );
    println!(
        "  {} {}  {} {}",
        style::dim("Created:"),
        &bead.created_at[..19], // Trim to date + time
        style::dim("by"),
        bead.created_by
    );
    println!("  {} {}", style::dim("Updated:"), &bead.updated_at[..19]);

    if let Some(ref assignee) = bead.assignee {
        println!("  {} {}", style::dim("Assignee:"), assignee);
    }

    if !bead.labels.is_empty() {
        println!(
            "  {} {}",
            style::dim("Labels:"),
            bead.labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !bead.dependencies.is_empty() {
        println!(
            "  {} {}",
            style::dim("Depends on:"),
            bead.dependencies
                .iter()
                .map(|id| style::issue_id(id.as_str()).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !bead.blocks.is_empty() {
        println!(
            "  {} {}",
            style::dim("Blocks:"),
            bead.blocks
                .iter()
                .map(|id| style::issue_id(id.as_str()).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if let Some(ref description) = bead.description {
        println!();
        println!("{}", style::subheader("Description:"));
        println!("{}", description);
    }

    if let Some(ref notes) = bead.notes {
        println!();
        println!("{}", style::subheader("Notes:"));
        println!("{}", notes);
    }
}

/// Show handoff info for a bead that has been handed off to an agent
fn show_handoff_info(bead_id: &str, bead: &allbeads::graph::Bead) -> allbeads::Result<()> {
    // Try to load comments from the beads crate
    let beads = match Beads::new() {
        Ok(b) => b,
        Err(_) => return Ok(()), // Silently skip if beads not available
    };

    let comments = match beads.comments(bead_id) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Silently skip if comments not found
    };

    println!();
    println!("{}", style::subheader("Handoff Info:"));

    // Parse comments to find handoff info
    let mut agent_info: Option<String> = None;
    let mut handoff_time: Option<String> = None;
    let mut task_url: Option<String> = None;

    for comment in &comments {
        let content = &comment.content;
        if content.starts_with("[HANDOFF]") {
            // Parse: [HANDOFF] Agent: Claude Code, Time: 2026-01-15T10:30:00Z
            if let Some(agent_part) = content.strip_prefix("[HANDOFF] Agent: ") {
                if let Some(comma_pos) = agent_part.find(", Time:") {
                    agent_info = Some(agent_part[..comma_pos].to_string());
                    let time_part = &agent_part[comma_pos + 7..]; // Skip ", Time:"
                    handoff_time = Some(time_part.trim().to_string());
                }
            }
        } else if content.starts_with("[TASK_URL]") {
            // Parse: [TASK_URL] https://...
            if let Some(url) = content.strip_prefix("[TASK_URL] ") {
                task_url = Some(url.trim().to_string());
            }
        }
    }

    if let Some(agent) = agent_info {
        println!("  {} {}", style::dim("Agent:"), style::highlight(&agent));
    }

    if let Some(time) = handoff_time {
        println!("  {} {}", style::dim("Handed off:"), time);
    }

    if let Some(url) = task_url {
        println!("  {} {}", style::dim("Task URL:"), style::path(&url));
    }

    // Show context path if we can determine it
    if let Some(ctx_label) = bead.labels.iter().find(|l| l.starts_with('@')) {
        let ctx_name = ctx_label.trim_start_matches('@');
        println!("  {} @{}", style::dim("Context:"), ctx_name);
    }

    Ok(())
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

fn priority_to_num(priority: Priority) -> u8 {
    match priority {
        Priority::P0 => 0,
        Priority::P1 => 1,
        Priority::P2 => 2,
        Priority::P3 => 3,
        Priority::P4 => 4,
    }
}

fn format_issue_type(issue_type: IssueType) -> &'static str {
    match issue_type {
        IssueType::Bug => "bug",
        IssueType::Feature => "feature",
        IssueType::Task => "task",
        IssueType::Epic => "epic",
        IssueType::Chore => "chore",
        IssueType::MergeRequest => "merge-request",
        IssueType::Molecule => "molecule",
        IssueType::Gate => "gate",
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

// === Distributed Configuration Commands (Phase 4 of PRD-01) ===

fn handle_config_command(cmd: &ConfigCommands) -> allbeads::Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| {
            allbeads::AllBeadsError::Config("Could not determine config directory".to_string())
        })?
        .join("allbeads");

    match cmd {
        ConfigCommands::Init {
            remote,
            gist,
            force,
        } => {
            handle_config_init(&config_dir, remote.as_deref(), gist.as_deref(), *force)?;
        }
        ConfigCommands::Pull { force } => {
            handle_config_pull(&config_dir, *force)?;
        }
        ConfigCommands::Push { message, force } => {
            handle_config_push(&config_dir, message.as_deref(), *force)?;
        }
        ConfigCommands::Status => {
            handle_config_status(&config_dir)?;
        }
        ConfigCommands::Diff => {
            handle_config_diff(&config_dir)?;
        }
        ConfigCommands::Clone { source, target } => {
            handle_config_clone(source, target.as_deref())?;
        }
    }
    Ok(())
}

/// Initialize distributed config sync
fn handle_config_init(
    config_dir: &Path,
    remote: Option<&str>,
    gist: Option<&str>,
    force: bool,
) -> allbeads::Result<()> {
    use git2::Repository;

    // Ensure config directory exists
    std::fs::create_dir_all(config_dir).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to create config directory: {}", e))
    })?;

    let git_dir = config_dir.join(".git");
    let is_repo = git_dir.exists();

    if is_repo && !force {
        // Check if remote already configured
        let repo = Repository::open(config_dir).map_err(|e| {
            allbeads::AllBeadsError::Git(format!("Failed to open config repo: {}", e))
        })?;

        let existing_remote = repo
            .find_remote("origin")
            .ok()
            .and_then(|r| r.url().map(|u| u.to_string()));

        if let Some(url) = existing_remote {
            println!();
            println!("{}", style::header("Config Sync Already Initialized"));
            println!();
            println!("  Remote: {}", style::path(&url));
            println!();
            println!("  Use --force to re-initialize with a new remote.");
            return Ok(());
        }
    }

    println!();
    println!("{}", style::header("Initialize Config Sync"));
    println!();

    // Determine remote URL
    let remote_url = if let Some(gist_id) = gist {
        // GitHub Gist URL format
        format!("https://gist.github.com/{}.git", gist_id)
    } else if let Some(url) = remote {
        url.to_string()
    } else {
        // No remote specified - just initialize local git repo
        println!("  Initializing local config repository...");

        if !is_repo {
            Repository::init(config_dir).map_err(|e| {
                allbeads::AllBeadsError::Git(format!("Failed to init config repo: {}", e))
            })?;
            println!("  {} Initialized git repository", style::success("✓"));
        } else {
            println!("  {} Git repository already exists", style::dim("○"));
        }

        // Create .gitignore
        let gitignore = config_dir.join(".gitignore");
        if !gitignore.exists() {
            std::fs::write(
                &gitignore,
                "# Ignore local-only files\n*.local\n*.local.yaml\ncache/\n",
            )
            .ok();
            println!("  {} Created .gitignore", style::success("✓"));
        }

        println!();
        println!("  Config sync initialized (local only).");
        println!("  To add a remote: ab config init --remote=<url>");
        return Ok(());
    };

    println!("  Remote: {}", style::path(&remote_url));

    // Initialize or reinitialize
    if !is_repo {
        Repository::init(config_dir).map_err(|e| {
            allbeads::AllBeadsError::Git(format!("Failed to init config repo: {}", e))
        })?;
        println!("  {} Initialized git repository", style::success("✓"));
    }

    let repo = Repository::open(config_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to open config repo: {}", e)))?;

    // Remove existing origin if force
    if force && repo.find_remote("origin").is_ok() {
        repo.remote_delete("origin").map_err(|e| {
            allbeads::AllBeadsError::Git(format!("Failed to remove existing remote: {}", e))
        })?;
        println!("  {} Removed existing remote", style::dim("○"));
    }

    // Add remote
    repo.remote("origin", &remote_url)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to add remote: {}", e)))?;
    println!("  {} Added remote 'origin'", style::success("✓"));

    // Create .gitignore
    let gitignore = config_dir.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(
            &gitignore,
            "# Ignore local-only files\n*.local\n*.local.yaml\ncache/\n",
        )
        .ok();
        println!("  {} Created .gitignore", style::success("✓"));
    }

    // Initial commit if needed
    let mut index = repo
        .index()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to get index: {}", e)))?;

    // Add all files
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to add files: {}", e)))?;
    index
        .write()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to write index: {}", e)))?;

    // Check if there are any commits
    if repo.head().is_err() {
        // No commits yet - create initial commit
        let tree_id = index
            .write_tree()
            .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to write tree: {}", e)))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to find tree: {}", e)))?;

        let sig = git2::Signature::now("AllBeads", "noreply@allbeads.dev").map_err(|e| {
            allbeads::AllBeadsError::Git(format!("Failed to create signature: {}", e))
        })?;

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial config sync setup",
            &tree,
            &[],
        )
        .map_err(|e| {
            allbeads::AllBeadsError::Git(format!("Failed to create initial commit: {}", e))
        })?;
        println!("  {} Created initial commit", style::success("✓"));
    }

    println!();
    println!("  Config sync initialized successfully!");
    println!();
    println!("  Next steps:");
    println!("    ab config push      # Push current config");
    println!("    ab config status    # Check sync status");

    Ok(())
}

/// Pull config changes from remote
fn handle_config_pull(config_dir: &Path, force: bool) -> allbeads::Result<()> {
    use git2::Repository;

    let git_dir = config_dir.join(".git");
    if !git_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(
            "Config sync not initialized. Run 'ab config init --remote=<url>' first.".to_string(),
        ));
    }

    println!();
    println!("{}", style::header("Pull Config Changes"));
    println!();

    let repo = Repository::open(config_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to open config repo: {}", e)))?;

    // Check if remote exists
    let remote = repo.find_remote("origin").map_err(|_| {
        allbeads::AllBeadsError::Config(
            "No remote configured. Run 'ab config init --remote=<url>' first.".to_string(),
        )
    })?;

    let remote_url = remote.url().unwrap_or("unknown");
    println!("  Remote: {}", style::path(remote_url));

    // Run git pull
    let output = std::process::Command::new("git")
        .args(if force {
            vec![
                "-C",
                config_dir.to_str().unwrap(),
                "pull",
                "--force",
                "origin",
                "main",
            ]
        } else {
            vec!["-C", config_dir.to_str().unwrap(), "pull", "origin", "main"]
        })
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to run git pull: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Already up to date") {
            println!("  {} Already up to date", style::success("✓"));
        } else {
            println!("  {} Pulled changes", style::success("✓"));
            println!();
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("fatal") || stderr.contains("error") {
            return Err(allbeads::AllBeadsError::Git(format!(
                "Pull failed: {}",
                stderr
            )));
        }
        println!(
            "  {}",
            style::warning(&format!("Warning: {}", stderr.trim()))
        );
    }

    Ok(())
}

/// Push config changes to remote
fn handle_config_push(
    config_dir: &Path,
    message: Option<&str>,
    force: bool,
) -> allbeads::Result<()> {
    use git2::Repository;

    let git_dir = config_dir.join(".git");
    if !git_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(
            "Config sync not initialized. Run 'ab config init --remote=<url>' first.".to_string(),
        ));
    }

    println!();
    println!("{}", style::header("Push Config Changes"));
    println!();

    let repo = Repository::open(config_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to open config repo: {}", e)))?;

    // Check if remote exists
    let remote = repo.find_remote("origin").map_err(|_| {
        allbeads::AllBeadsError::Config(
            "No remote configured. Run 'ab config init --remote=<url>' first.".to_string(),
        )
    })?;

    let remote_url = remote.url().unwrap_or("unknown");
    println!("  Remote: {}", style::path(remote_url));

    // Stage all changes
    let output = std::process::Command::new("git")
        .args(["-C", config_dir.to_str().unwrap(), "add", "-A"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to stage changes: {}", e)))?;

    if !output.status.success() {
        return Err(allbeads::AllBeadsError::Git(format!(
            "Failed to stage changes: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Check for changes
    let status_output = std::process::Command::new("git")
        .args(["-C", config_dir.to_str().unwrap(), "status", "--porcelain"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to check status: {}", e)))?;

    let has_changes = !String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .is_empty();

    if has_changes {
        // Commit changes
        let commit_msg = message.unwrap_or("Update config");
        let output = std::process::Command::new("git")
            .args([
                "-C",
                config_dir.to_str().unwrap(),
                "commit",
                "-m",
                commit_msg,
            ])
            .output()
            .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to commit: {}", e)))?;

        if output.status.success() {
            println!("  {} Committed changes", style::success("✓"));
        }
    } else {
        println!("  {} No changes to commit", style::dim("○"));
    }

    // Push to remote
    let push_args = if force {
        vec![
            "-C",
            config_dir.to_str().unwrap(),
            "push",
            "--force",
            "-u",
            "origin",
            "main",
        ]
    } else {
        vec![
            "-C",
            config_dir.to_str().unwrap(),
            "push",
            "-u",
            "origin",
            "main",
        ]
    };

    let output = std::process::Command::new("git")
        .args(&push_args)
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to push: {}", e)))?;

    if output.status.success() {
        println!("  {} Pushed to remote", style::success("✓"));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Everything up-to-date") {
            println!("  {} Already up to date", style::success("✓"));
        } else if stderr.contains("fatal") || stderr.contains("error") {
            return Err(allbeads::AllBeadsError::Git(format!(
                "Push failed: {}",
                stderr
            )));
        }
    }

    Ok(())
}

/// Show config sync status
fn handle_config_status(config_dir: &Path) -> allbeads::Result<()> {
    use git2::Repository;

    println!();
    println!("{}", style::header("Config Sync Status"));
    println!();

    println!(
        "  Config dir: {}",
        style::path(&config_dir.display().to_string())
    );

    let git_dir = config_dir.join(".git");
    if !git_dir.exists() {
        println!("  Status:     {}", style::warning("Not initialized"));
        println!();
        println!("  Run 'ab config init --remote=<url>' to initialize.");
        return Ok(());
    }

    let repo = Repository::open(config_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to open config repo: {}", e)))?;

    // Check remote
    if let Ok(remote) = repo.find_remote("origin") {
        if let Some(url) = remote.url() {
            println!("  Remote:     {}", style::path(url));
        }
    } else {
        println!("  Remote:     {}", style::warning("Not configured"));
    }

    // Get current branch
    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            println!("  Branch:     {}", name);
        }
    }

    // Check status
    let output = std::process::Command::new("git")
        .args(["-C", config_dir.to_str().unwrap(), "status", "--porcelain"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to check status: {}", e)))?;

    let status_output = String::from_utf8_lossy(&output.stdout);
    let changes: Vec<&str> = status_output.lines().collect();

    if changes.is_empty() {
        println!("  Changes:    {}", style::success("Clean"));
    } else {
        println!("  Changes:    {} modified files", changes.len());
        for change in changes.iter().take(5) {
            println!("              {}", change);
        }
        if changes.len() > 5 {
            println!("              ... and {} more", changes.len() - 5);
        }
    }

    // Check ahead/behind
    let output = std::process::Command::new("git")
        .args([
            "-C",
            config_dir.to_str().unwrap(),
            "rev-list",
            "--left-right",
            "--count",
            "HEAD...origin/main",
        ])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let counts = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = counts.trim().split('\t').collect();
            if parts.len() == 2 {
                let ahead: i32 = parts[0].parse().unwrap_or(0);
                let behind: i32 = parts[1].parse().unwrap_or(0);

                if ahead > 0 && behind > 0 {
                    println!(
                        "  Sync:       {} ahead, {} behind (diverged)",
                        ahead, behind
                    );
                } else if ahead > 0 {
                    println!("  Sync:       {} commits ahead", ahead);
                } else if behind > 0 {
                    println!("  Sync:       {} commits behind", behind);
                } else {
                    println!("  Sync:       {}", style::success("Up to date"));
                }
            }
        }
    }

    println!();

    Ok(())
}

/// Show diff with remote
fn handle_config_diff(config_dir: &Path) -> allbeads::Result<()> {
    let git_dir = config_dir.join(".git");
    if !git_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(
            "Config sync not initialized.".to_string(),
        ));
    }

    println!();
    println!("{}", style::header("Config Diff"));
    println!();

    // Show local changes
    let output = std::process::Command::new("git")
        .args(["-C", config_dir.to_str().unwrap(), "diff", "--stat"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to get diff: {}", e)))?;

    let diff = String::from_utf8_lossy(&output.stdout);
    if diff.trim().is_empty() {
        println!("  No local changes.");
    } else {
        println!("{}", diff);
    }

    Ok(())
}

/// Clone config from remote
fn handle_config_clone(source: &str, target: Option<&str>) -> allbeads::Result<()> {
    let target_dir = if let Some(t) = target {
        PathBuf::from(t)
    } else {
        dirs::config_dir()
            .ok_or_else(|| {
                allbeads::AllBeadsError::Config("Could not determine config directory".to_string())
            })?
            .join("allbeads")
    };

    println!();
    println!("{}", style::header("Clone Config"));
    println!();

    // Check if target exists
    if target_dir.exists() && target_dir.join(".git").exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Config already exists at {}. Use 'ab config pull' to update.",
            target_dir.display()
        )));
    }

    // Determine if source is a Gist ID or full URL
    let remote_url = if source.starts_with("http") || source.starts_with("git@") {
        source.to_string()
    } else {
        // Assume it's a Gist ID
        format!("https://gist.github.com/{}.git", source)
    };

    println!("  Source: {}", style::path(&remote_url));
    println!(
        "  Target: {}",
        style::path(&target_dir.display().to_string())
    );
    println!();

    // Clone the repository
    git2::Repository::clone(&remote_url, &target_dir)
        .map_err(|e| allbeads::AllBeadsError::Git(format!("Failed to clone config: {}", e)))?;

    println!("  {} Config cloned successfully!", style::success("✓"));
    println!();
    println!("  Your configuration is now synced from the remote.");
    println!("  Use 'ab config pull' to get updates.");

    Ok(())
}

// ============================================================================
// Plugin Commands
// ============================================================================

fn handle_plugin_command(cmd: &PluginCommands) -> allbeads::Result<()> {
    match cmd {
        PluginCommands::List {
            all,
            category,
            json,
        } => handle_plugin_list(*all, category.as_deref(), *json),
        PluginCommands::Info { name } => handle_plugin_info(name),
        PluginCommands::Status { name } => handle_plugin_status(name.as_deref()),
        PluginCommands::Detect { path, verbose } => handle_plugin_detect(path, *verbose),
        PluginCommands::Install { name, yes } => handle_plugin_install(name, *yes),
        PluginCommands::Uninstall { name, yes } => handle_plugin_uninstall(name, *yes),
        PluginCommands::Onboard { name, path, yes } => handle_plugin_onboard(name, path, *yes),
        PluginCommands::Recommend { path } => handle_plugin_recommend(path),
        PluginCommands::MarketplaceList { json } => handle_marketplace_list(*json),
        PluginCommands::MarketplaceAdd { source, name } => {
            handle_marketplace_add(source, name.as_deref())
        }
        PluginCommands::MarketplaceSync { name } => handle_marketplace_sync(name.as_deref()),
    }
}

fn handle_plugin_list(all: bool, category: Option<&str>, json: bool) -> allbeads::Result<()> {
    use allbeads::plugin::{ClaudePluginState, PluginCategory, PluginRegistry};

    let registry = PluginRegistry::builtin();
    let claude_state = ClaudePluginState::load();

    // Filter by category if specified
    let plugins: Vec<_> = registry
        .plugins
        .iter()
        .filter(|p| {
            if let Some(cat) = category {
                let cat_lower = cat.to_lowercase();
                match &p.category {
                    PluginCategory::Claude => cat_lower == "claude",
                    PluginCategory::Beads => cat_lower == "beads",
                    PluginCategory::Prose => cat_lower == "prose",
                    PluginCategory::DevTools => cat_lower == "devtools" || cat_lower == "dev",
                    PluginCategory::Testing => cat_lower == "testing" || cat_lower == "test",
                    PluginCategory::Other => cat_lower == "other",
                }
            } else {
                true
            }
        })
        .filter(|p| all || claude_state.is_installed(&p.name) || p.relevance.always_suggest)
        .collect();

    if json {
        let output: Vec<_> = plugins
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "description": p.description,
                    "category": format!("{:?}", p.category),
                    "installed": claude_state.is_installed(&p.name),
                    "enabled": claude_state.is_enabled(&p.name),
                    "has_onboarding": p.has_onboarding,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return Ok(());
    }

    println!();
    println!("{}", style::header("Plugins"));
    println!();

    if plugins.is_empty() {
        println!("  No plugins found. Use --all to see available plugins.");
        return Ok(());
    }

    for plugin in plugins {
        let installed = claude_state.is_installed(&plugin.name);
        let enabled = claude_state.is_enabled(&plugin.name);

        let status = if enabled {
            style::success("●")
        } else if installed {
            style::warning("○")
        } else {
            style::dim("○")
        };

        let category = format!("{:?}", plugin.category).to_lowercase();
        println!(
            "  {} {} {} - {}",
            status,
            style::highlight(&plugin.name),
            style::dim(&format!("[{}]", category)),
            plugin.description
        );
    }

    println!();
    println!(
        "  {} = enabled, {} = installed, {} = available",
        style::success("●"),
        style::warning("○"),
        style::dim("○")
    );

    Ok(())
}

fn handle_plugin_info(name: &str) -> allbeads::Result<()> {
    use allbeads::plugin::{ClaudePluginState, PluginRegistry};

    let registry = PluginRegistry::builtin();
    let claude_state = ClaudePluginState::load();

    let plugin = registry.find(name).ok_or_else(|| {
        allbeads::AllBeadsError::Config(format!("Plugin '{}' not found in registry", name))
    })?;

    println!();
    println!("{}", style::header(&format!("Plugin: {}", plugin.name)));
    println!();
    println!("  Description: {}", plugin.description);
    println!("  Category:    {:?}", plugin.category);

    if let Some(ref marketplace) = plugin.marketplace {
        println!("  Marketplace: {}", marketplace);
    }
    if let Some(ref repository) = plugin.repository {
        println!("  Repository:  {}", repository);
    }

    println!();
    println!("  Status:");
    println!(
        "    Installed: {}",
        if claude_state.is_installed(name) {
            style::success("Yes")
        } else {
            style::dim("No")
        }
    );
    println!(
        "    Enabled:   {}",
        if claude_state.is_enabled(name) {
            style::success("Yes")
        } else {
            style::dim("No")
        }
    );
    println!(
        "    Onboarding: {}",
        if plugin.has_onboarding {
            style::success("Available")
        } else {
            style::dim("Not available")
        }
    );

    if !plugin.relevance.languages.is_empty() {
        println!();
        println!("  Relevant for languages:");
        for lang in &plugin.relevance.languages {
            println!("    - {}", lang);
        }
    }

    if !plugin.relevance.files.is_empty() {
        println!();
        println!("  Detection files:");
        for file in &plugin.relevance.files {
            println!("    - {}", file);
        }
    }

    Ok(())
}

fn handle_plugin_status(name: Option<&str>) -> allbeads::Result<()> {
    use allbeads::plugin::{ClaudePluginState, PluginRegistry};

    let registry = PluginRegistry::builtin();
    let claude_state = ClaudePluginState::load();

    println!();
    println!("{}", style::header("Plugin Status"));
    println!();

    if let Some(name) = name {
        // Show status for specific plugin
        let plugin = registry.find(name);
        let installed = claude_state.is_installed(name);
        let enabled = claude_state.is_enabled(name);

        println!("  Plugin: {}", style::highlight(name));
        println!(
            "  In registry: {}",
            if plugin.is_some() {
                style::success("Yes")
            } else {
                style::dim("No")
            }
        );
        println!(
            "  Installed:   {}",
            if installed {
                style::success("Yes")
            } else {
                style::dim("No")
            }
        );
        println!(
            "  Enabled:     {}",
            if enabled {
                style::success("Yes")
            } else {
                style::dim("No")
            }
        );
    } else {
        // Show summary
        let installed_count = claude_state.installed_plugins.len();
        let enabled_count = claude_state.enabled_plugins.len();
        let registry_count = registry.plugins.len();

        println!("  Installed plugins: {}", installed_count);
        println!("  Enabled plugins:   {}", enabled_count);
        println!("  Available in registry: {}", registry_count);

        if !claude_state.installed_plugins.is_empty() {
            println!();
            println!("  Installed:");
            for plugin in &claude_state.installed_plugins {
                let status = if claude_state.is_enabled(&plugin.name) {
                    style::success("●")
                } else {
                    style::warning("○")
                };
                println!("    {} {}", status, plugin.name);
            }
        }
    }

    Ok(())
}

fn handle_plugin_detect(path: &str, verbose: bool) -> allbeads::Result<()> {
    use allbeads::plugin::PluginRegistry;
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!("{}", style::header("Plugin Detection"));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    // Detect languages and files
    let mut languages = Vec::new();
    let mut detected_files = Vec::new();

    // Check for common files
    let checks = [
        ("package.json", "javascript"),
        ("tsconfig.json", "typescript"),
        ("Cargo.toml", "rust"),
        ("go.mod", "go"),
        ("pyproject.toml", "python"),
        ("requirements.txt", "python"),
        ("Gemfile", "ruby"),
        ("pom.xml", "java"),
        ("build.gradle", "java"),
    ];

    for (file, lang) in checks {
        if project_path.join(file).exists() {
            detected_files.push(file.to_string());
            if !languages.contains(&lang.to_string()) {
                languages.push(lang.to_string());
            }
        }
    }

    // Check for .github
    if project_path.join(".github").exists() {
        detected_files.push(".github".to_string());
    }

    // Check for beads
    if project_path.join(".beads").exists() {
        detected_files.push(".beads".to_string());
    }

    if verbose {
        println!("  Detected languages:");
        for lang in &languages {
            println!("    - {}", lang);
        }
        println!();
        println!("  Detected files:");
        for file in &detected_files {
            println!("    - {}", file);
        }
        println!();
    }

    // Get recommendations
    let registry = PluginRegistry::builtin();
    let recommended = registry.recommend(&languages, &detected_files);

    println!("  Recommended plugins:");
    if recommended.is_empty() {
        println!("    (none detected based on project files)");
    } else {
        for plugin in recommended {
            println!(
                "    {} {} - {}",
                style::success("→"),
                style::highlight(&plugin.name),
                plugin.description
            );
        }
    }

    Ok(())
}

fn handle_plugin_install(name: &str, yes: bool) -> allbeads::Result<()> {
    use allbeads::plugin::{
        check_prerequisites, load_onboarding, OnboardingExecutor, PluginRegistry,
    };

    let registry = PluginRegistry::builtin();
    let plugin = registry.find(name);

    println!();
    println!("{}", style::header(&format!("Install Plugin: {}", name)));
    println!();

    let plugin = match plugin {
        Some(p) => p,
        None => {
            return Err(allbeads::AllBeadsError::Config(format!(
                "Plugin '{}' not found in registry. Use 'ab plugin list --all' to see available plugins.",
                name
            )));
        }
    };

    // If plugin has marketplace entry, suggest claude plugin install
    if let Some(ref marketplace) = plugin.marketplace {
        println!("  Step 1: Install via Claude marketplace");
        println!();
        println!("    claude plugin install {}", marketplace);
        println!();
    }

    // Check if current directory has an onboarding protocol
    let current_dir = std::env::current_dir().map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Could not get current directory: {}", e))
    })?;

    if let Some(onboarding) = load_onboarding(&current_dir) {
        println!("  Step 2: Run onboarding for this project");
        println!();

        // Check prerequisites
        let prereqs = check_prerequisites(&onboarding, &current_dir);
        let mut all_satisfied = true;

        if !prereqs.is_empty() {
            println!("  Prerequisites:");
            for (prereq_name, satisfied, hint) in &prereqs {
                if *satisfied {
                    println!("    {} {}", style::success("✓"), prereq_name);
                } else {
                    println!("    {} {}", style::error("✗"), prereq_name);
                    if let Some(h) = hint {
                        println!("      Install with: {}", h);
                    }
                    all_satisfied = false;
                }
            }
            println!();
        }

        if !all_satisfied {
            println!(
                "  {} Install missing prerequisites first.",
                style::warning("!")
            );
            return Ok(());
        }

        if yes {
            println!("  Executing onboarding steps...");
            println!();

            let mut executor = OnboardingExecutor::new(current_dir).auto_yes(true);
            let result = executor.execute(&onboarding);

            println!();
            if result.success {
                println!("  {} Plugin installed and configured!", style::success("✓"));
                println!("    Steps completed: {}", result.steps_completed);
                if result.steps_skipped > 0 {
                    println!("    Steps skipped: {}", result.steps_skipped);
                }
            } else {
                println!("  {} Some steps failed:", style::error("✗"));
                for err in &result.errors {
                    println!("    - {}", err);
                }
            }
        } else {
            println!("  Run with --yes to execute onboarding steps.");
        }
    } else if plugin.has_onboarding {
        println!("  This plugin supports onboarding but no protocol found in current directory.");
        println!(
            "  The plugin may install its onboarding protocol after marketplace installation."
        );
    }

    Ok(())
}

fn handle_plugin_uninstall(name: &str, yes: bool) -> allbeads::Result<()> {
    use allbeads::plugin::{load_onboarding, OnboardingExecutor};

    println!();
    println!("{}", style::header(&format!("Uninstall Plugin: {}", name)));
    println!();

    // Check if current directory has an onboarding protocol with uninstall steps
    let current_dir = std::env::current_dir().map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Could not get current directory: {}", e))
    })?;

    if let Some(onboarding) = load_onboarding(&current_dir) {
        if onboarding.uninstall.is_some() {
            println!("  Found uninstall steps for {}", onboarding.plugin);
            println!();

            if yes {
                println!("  Executing uninstall steps...");
                println!();

                let mut executor = OnboardingExecutor::new(current_dir).auto_yes(true);
                let result = executor.execute_uninstall(&onboarding);

                println!();
                if result.success {
                    println!("  {} Plugin uninstalled!", style::success("✓"));
                } else {
                    println!("  {} Some steps failed:", style::error("✗"));
                    for err in &result.errors {
                        println!("    - {}", err);
                    }
                }
            } else {
                println!("  Run with --yes to execute uninstall steps.");
            }
        } else {
            println!("  No uninstall steps defined for this plugin.");
        }
    }

    // Also suggest claude plugin uninstall
    println!();
    println!("  To uninstall from Claude marketplace:");
    println!();
    println!("    claude plugin uninstall {}", name);
    println!();

    Ok(())
}

fn handle_plugin_onboard(name: &str, path: &str, yes: bool) -> allbeads::Result<()> {
    use allbeads::plugin::{
        check_prerequisites, load_onboarding, OnboardingExecutor, PluginRegistry,
    };
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    let registry = PluginRegistry::builtin();
    let plugin = registry.find(name).ok_or_else(|| {
        allbeads::AllBeadsError::Config(format!("Plugin '{}' not found in registry", name))
    })?;

    println!();
    println!("{}", style::header(&format!("Onboard: {}", name)));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    if !plugin.has_onboarding {
        println!("  This plugin does not have an onboarding protocol.");
        println!("  Please refer to the plugin documentation for setup instructions.");
        return Ok(());
    }

    // Try to load onboarding from project
    if let Some(onboarding) = load_onboarding(&project_path) {
        println!("  Found onboarding protocol: {}", onboarding.plugin);
        println!("  Version: {}", onboarding.version);
        println!();

        // Check prerequisites
        let prereqs = check_prerequisites(&onboarding, &project_path);
        if !prereqs.is_empty() {
            println!("  Prerequisites:");
            let mut all_satisfied = true;
            for (prereq_name, satisfied, hint) in &prereqs {
                if *satisfied {
                    println!("    {} {}", style::success("✓"), prereq_name);
                } else {
                    println!("    {} {}", style::error("✗"), prereq_name);
                    if let Some(h) = hint {
                        println!("      Install with: {}", h);
                    }
                    all_satisfied = false;
                }
            }
            println!();

            if !all_satisfied && yes {
                println!(
                    "  {} Install missing prerequisites first.",
                    style::warning("!")
                );
                return Ok(());
            }
        }

        // Show steps
        println!("  Steps:");
        for (i, step) in onboarding.onboard.steps.iter().enumerate() {
            let step_name = match step {
                allbeads::plugin::OnboardingStep::Command { name, .. } => name,
                allbeads::plugin::OnboardingStep::Interactive { name, .. } => name,
                allbeads::plugin::OnboardingStep::Template { name, .. } => name,
                allbeads::plugin::OnboardingStep::Append { name, .. } => name,
            };
            println!("    {}. {}", i + 1, step_name);
        }
        println!();

        if yes {
            println!("  Executing onboarding steps...");
            println!();

            let mut executor = OnboardingExecutor::new(project_path).auto_yes(true);
            let result = executor.execute(&onboarding);

            println!();
            if result.success {
                println!("  {} Onboarding complete!", style::success("✓"));
                println!("    Steps completed: {}", result.steps_completed);
                if result.steps_skipped > 0 {
                    println!("    Steps skipped: {}", result.steps_skipped);
                }
            } else {
                println!("  {} Some steps failed:", style::error("✗"));
                for err in &result.errors {
                    println!("    - {}", err);
                }
            }
        } else {
            println!("  Run with --yes to execute these steps.");
        }
    } else {
        println!("  No onboarding protocol found in project.");
        println!("  Looking for: .claude-plugin/allbeads-onboarding.yaml");
        println!();
        println!("  The plugin may need to be installed first via:");
        if let Some(ref marketplace) = plugin.marketplace {
            println!("    claude plugin install {}", marketplace);
        } else {
            println!("    (manual installation required)");
        }
    }

    Ok(())
}

fn handle_plugin_recommend(path: &str) -> allbeads::Result<()> {
    use allbeads::plugin::{analyze_project, recommend_plugins, ClaudePluginState, PluginRegistry};
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!("{}", style::header("Plugin Recommendations"));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    // Analyze project
    let analysis = analyze_project(&project_path);

    // Show detected project info
    println!("  {}", style::header("Project Analysis"));
    println!();

    if !analysis.languages.is_empty() {
        println!(
            "  Languages: {}",
            style::highlight(&analysis.languages.join(", "))
        );
    }
    if !analysis.frameworks.is_empty() {
        println!(
            "  Frameworks: {}",
            style::highlight(&analysis.frameworks.join(", "))
        );
    }
    if analysis.is_monorepo {
        println!("  Type: {}", style::highlight("Monorepo"));
    }
    if analysis.has_git {
        print!("  Git: {}", style::success("✓"));
    }
    if analysis.has_beads {
        print!("  Beads: {}", style::success("✓"));
    }
    if analysis.has_git || analysis.has_beads {
        println!();
    }
    println!();

    // Get recommendations
    let registry = PluginRegistry::builtin();
    let claude_state = ClaudePluginState::load();
    let recommendations = recommend_plugins(&project_path, &registry, &claude_state);

    if recommendations.is_empty() {
        println!("  No specific plugins recommended for this project.");
        println!("  Use 'ab plugin list --all' to browse available plugins.");
    } else {
        println!("  {}", style::header("Recommended Plugins"));
        println!();

        for rec in &recommendations {
            // Status indicator
            let status_icon = if rec.is_configured {
                style::success("✓")
            } else if rec.is_installed {
                style::warning("○")
            } else {
                style::dim("·")
            };

            // Confidence indicator
            let confidence_bar = match rec.confidence_label() {
                "High" => format!("{}", style::success("███")),
                "Medium" => format!("{}", style::warning("██░")),
                _ => format!("{}", style::dim("█░░")),
            };

            println!(
                "  {} {} {} - {}",
                status_icon,
                confidence_bar,
                style::highlight(&rec.plugin.name),
                rec.plugin.description
            );

            // Show reasons
            let reason_strs: Vec<String> = rec.reasons.iter().map(|r| r.description()).collect();
            println!(
                "      {} {} ({}% confidence)",
                style::dim("→"),
                style::dim(&reason_strs.join(", ")),
                (rec.confidence * 100.0) as u32
            );
        }

        println!();
        println!(
            "  Legend: {} configured  {} installed  {} not installed",
            style::success("✓"),
            style::warning("○"),
            style::dim("·")
        );
        println!(
            "          {} high  {} medium  {} low confidence",
            style::success("███"),
            style::warning("██░"),
            style::dim("█░░")
        );
    }

    Ok(())
}

// ============================================================================
// Marketplace Commands
// ============================================================================

fn handle_marketplace_list(json: bool) -> allbeads::Result<()> {
    use allbeads::plugin::{load_known_marketplaces, load_marketplace_metadata, MarketplaceSource};

    let marketplaces = load_known_marketplaces();

    if json {
        let output: Vec<_> = marketplaces
            .iter()
            .map(|(name, m)| {
                let source_str = match &m.source {
                    MarketplaceSource::Github { repo } => format!("github:{}", repo),
                    MarketplaceSource::Git { url } => format!("git:{}", url),
                };
                serde_json::json!({
                    "name": name,
                    "source": source_str,
                    "install_location": m.install_location,
                    "last_updated": m.last_updated,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return Ok(());
    }

    println!();
    println!("{}", style::header("Registered Marketplaces"));
    println!();

    if marketplaces.is_empty() {
        println!("  No marketplaces registered.");
        println!();
        println!("  Marketplaces are registered automatically when you install plugins via:");
        println!("    claude plugin install <marketplace>/<plugin>");
        return Ok(());
    }

    for (name, marketplace) in &marketplaces {
        let source_str = match &marketplace.source {
            MarketplaceSource::Github { repo } => format!("github:{}", repo),
            MarketplaceSource::Git { url } => url.clone(),
        };

        println!(
            "  {} {}",
            style::highlight(name),
            style::dim(&format!("({})", source_str))
        );

        // Try to load metadata
        let install_path = if marketplace.install_location.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                home.join(&marketplace.install_location[2..])
            } else {
                PathBuf::from(&marketplace.install_location)
            }
        } else {
            PathBuf::from(&marketplace.install_location)
        };

        if let Some(metadata) = load_marketplace_metadata(&install_path) {
            let allbeads_count = metadata
                .plugins
                .iter()
                .filter(|p| p.allbeads_compatible)
                .count();
            println!(
                "    Plugins: {} ({} AllBeads-compatible)",
                metadata.plugins.len(),
                allbeads_count
            );
        }

        if let Some(ref updated) = marketplace.last_updated {
            // Parse and format the date
            if let Some(date) = updated.split('T').next() {
                println!("    Updated: {}", date);
            }
        }
        println!();
    }

    Ok(())
}

fn handle_marketplace_add(source: &str, name: Option<&str>) -> allbeads::Result<()> {
    println!();
    println!("{}", style::header("Add Marketplace"));
    println!();

    // Determine the full URL
    let (marketplace_url, marketplace_name) = if source.contains('/') && !source.contains("://") {
        // Assume GitHub shorthand: owner/repo
        let url = format!("https://github.com/{}", source);
        let inferred_name = source.split('/').next_back().unwrap_or(source);
        (url, name.unwrap_or(inferred_name).to_string())
    } else if source.starts_with("http") || source.starts_with("git@") {
        // Full URL
        let inferred_name = source
            .split('/')
            .next_back()
            .unwrap_or("marketplace")
            .trim_end_matches(".git");
        (
            source.to_string(),
            name.unwrap_or(inferred_name).to_string(),
        )
    } else {
        return Err(allbeads::AllBeadsError::Config(
            "Invalid source. Use GitHub shorthand (owner/repo) or full URL.".to_string(),
        ));
    };

    println!("  Name: {}", style::highlight(&marketplace_name));
    println!("  URL:  {}", style::path(&marketplace_url));
    println!();

    // Delegate to claude plugin marketplace add
    println!("  To register this marketplace with Claude, run:");
    println!();
    println!(
        "    claude plugin marketplace add {} {}",
        marketplace_name, marketplace_url
    );
    println!();
    println!("  After registration, plugins from this marketplace can be installed with:");
    println!("    claude plugin install {}/PLUGIN_NAME", marketplace_name);

    Ok(())
}

fn handle_marketplace_sync(name: Option<&str>) -> allbeads::Result<()> {
    use allbeads::plugin::load_known_marketplaces;

    println!();
    println!("{}", style::header("Sync Marketplaces"));
    println!();

    let marketplaces = load_known_marketplaces();

    if marketplaces.is_empty() {
        println!("  No marketplaces registered.");
        return Ok(());
    }

    let to_sync: Vec<_> = if let Some(n) = name {
        marketplaces
            .iter()
            .filter(|(k, _)| k.as_str() == n)
            .collect()
    } else {
        marketplaces.iter().collect()
    };

    if to_sync.is_empty() {
        if let Some(n) = name {
            return Err(allbeads::AllBeadsError::Config(format!(
                "Marketplace '{}' not found",
                n
            )));
        }
    }

    for (mkt_name, _marketplace) in to_sync {
        println!("  Syncing {}...", style::highlight(mkt_name));

        // Use claude plugin marketplace sync
        let result = std::process::Command::new("claude")
            .args(["plugin", "marketplace", "sync", mkt_name])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                println!("    {} Synced", style::success("✓"));
            }
            Ok(_) => {
                println!(
                    "    {} Sync failed (try 'claude plugin marketplace sync {}')",
                    style::warning("!"),
                    mkt_name
                );
            }
            Err(_) => {
                println!("    {} 'claude' command not found", style::error("✗"));
            }
        }
    }

    println!();
    Ok(())
}

// ============================================================================
// Coding Agent Commands
// ============================================================================

fn handle_coding_agent_command(cmd: &commands::CodingAgentCommands) -> allbeads::Result<()> {
    use commands::CodingAgentCommands;

    match cmd {
        CodingAgentCommands::List { path, json } => handle_agent_list(path, *json),
        CodingAgentCommands::Init { agent, path, yes } => handle_agent_init(agent, path, *yes),
        CodingAgentCommands::Sync { path, agent } => handle_agent_sync(path, agent.as_deref()),
        CodingAgentCommands::Preview { agent, path } => handle_agent_preview(agent, path),
        CodingAgentCommands::Detect { path } => handle_agent_detect(path),
    }
}

fn handle_agent_list(path: &str, json: bool) -> allbeads::Result<()> {
    use allbeads::coding_agent::detect_agents;
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    let agents = detect_agents(&project_path);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&agents).unwrap_or_default()
        );
        return Ok(());
    }

    println!();
    println!("{}", style::header("Coding Agents"));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    let configured: Vec<_> = agents.iter().filter(|a| a.configured).collect();
    let not_configured: Vec<_> = agents.iter().filter(|a| !a.configured).collect();

    if configured.is_empty() {
        println!("  No coding agents configured.");
        println!();
    } else {
        println!("  {}", style::header("Configured"));
        println!();
        for status in &configured {
            let sync_icon = if status.has_allbeads_context {
                style::success("✓")
            } else {
                style::dim("○")
            };
            println!(
                "  {} {} {}",
                sync_icon,
                style::highlight(status.agent.display_name()),
                style::dim(&format!(
                    "({})",
                    status.config_path.as_deref().unwrap_or("")
                ))
            );
        }
        println!();
    }

    if !not_configured.is_empty() {
        println!("  {}", style::header("Available"));
        println!();
        for status in &not_configured {
            println!(
                "  {} {} {}",
                style::dim("·"),
                status.agent.display_name(),
                style::dim(&format!("({})", status.agent.primary_config()))
            );
        }
        println!();
    }

    println!(
        "  Legend: {} synced  {} not synced  {} not configured",
        style::success("✓"),
        style::dim("○"),
        style::dim("·")
    );
    println!();
    println!("  Use 'ab agent init <agent>' to configure an agent.");
    println!("  Use 'ab agent sync' to sync AllBeads context.");

    Ok(())
}

fn handle_agent_init(agent_name: &str, path: &str, yes: bool) -> allbeads::Result<()> {
    use allbeads::coding_agent::{init_agent, CodingAgent};
    use std::path::Path;

    let agent = CodingAgent::parse(agent_name).ok_or_else(|| {
        allbeads::AllBeadsError::Config(format!(
            "Unknown agent '{}'. Available: claude, cursor, copilot, aider",
            agent_name
        ))
    })?;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!("{}", style::header("Initialize Agent"));
    println!();
    println!("  Agent: {}", style::highlight(agent.display_name()));
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    match init_agent(agent, &project_path, yes) {
        Ok(config_path) => {
            println!(
                "  {} Created {}",
                style::success("✓"),
                style::path(&config_path.display().to_string())
            );
            println!();
            println!("  Edit this file to customize the agent's behavior.");
            println!("  Use 'ab agent sync' to add AllBeads context.");
        }
        Err(e) => {
            println!("  {} {}", style::error("✗"), e);
        }
    }

    Ok(())
}

fn handle_agent_sync(path: &str, agent_filter: Option<&str>) -> allbeads::Result<()> {
    use allbeads::coding_agent::{detect_agents, sync_agent_context, AllBeadsContext, CodingAgent};
    use allbeads::plugin::analyze_project;
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!("{}", style::header("Sync Agent Context"));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    // Analyze project
    let analysis = analyze_project(&project_path);

    // Build context
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    // Try to get beads info
    let (open_issues, ready_issues, beads_prefix) = if project_path.join(".beads").exists() {
        // Run bd commands to get counts
        let open = std::process::Command::new("bd")
            .args(["list", "--status=open"])
            .current_dir(&project_path)
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .count()
                    .saturating_sub(1)
            })
            .unwrap_or(0);

        let ready = std::process::Command::new("bd")
            .args(["ready"])
            .current_dir(&project_path)
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .count()
                    .saturating_sub(1)
            })
            .unwrap_or(0);

        // Try to get prefix from config
        let prefix = std::fs::read_to_string(project_path.join(".beads/config.yaml"))
            .ok()
            .and_then(|c| {
                c.lines()
                    .find(|l| l.starts_with("prefix:"))
                    .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            });

        (open, ready, prefix)
    } else {
        (0, 0, None)
    };

    let context = AllBeadsContext {
        project_name,
        beads_prefix,
        open_issues,
        ready_issues,
        languages: analysis.languages,
        frameworks: analysis.frameworks,
    };

    // Get configured agents
    let agents = detect_agents(&project_path);
    let configured: Vec<_> = agents.iter().filter(|a| a.configured).collect();

    if configured.is_empty() {
        println!("  No coding agents configured.");
        println!("  Use 'ab agent init <agent>' to configure one.");
        return Ok(());
    }

    // Filter if specified
    let to_sync: Vec<_> = if let Some(filter) = agent_filter {
        if let Some(agent) = CodingAgent::parse(filter) {
            configured
                .iter()
                .filter(|s| s.agent == agent)
                .cloned()
                .collect()
        } else {
            return Err(allbeads::AllBeadsError::Config(format!(
                "Unknown agent '{}'",
                filter
            )));
        }
    } else {
        configured
    };

    for status in to_sync {
        print!(
            "  Syncing {}...",
            style::highlight(status.agent.display_name())
        );
        match sync_agent_context(status.agent, &project_path, &context) {
            Ok(()) => println!(" {}", style::success("✓")),
            Err(e) => println!(" {} {}", style::error("✗"), e),
        }
    }

    println!();
    println!("  Context synced:");
    println!("    Open issues: {}", context.open_issues);
    println!("    Ready: {}", context.ready_issues);
    if !context.languages.is_empty() {
        println!("    Languages: {}", context.languages.join(", "));
    }

    Ok(())
}

fn handle_agent_preview(agent_name: &str, path: &str) -> allbeads::Result<()> {
    use allbeads::coding_agent::{preview_agent_config, CodingAgent};
    use std::path::Path;

    let agent = CodingAgent::parse(agent_name).ok_or_else(|| {
        allbeads::AllBeadsError::Config(format!(
            "Unknown agent '{}'. Available: claude, cursor, copilot, aider",
            agent_name
        ))
    })?;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!(
        "{}",
        style::header(&format!("{} Configuration Preview", agent.display_name()))
    );
    println!();

    match preview_agent_config(agent, &project_path) {
        Ok(content) => {
            // Print with line numbers
            for (i, line) in content.lines().enumerate() {
                println!("{:4} {}", style::dim(&format!("{}", i + 1)), line);
            }
        }
        Err(e) => {
            println!("  {} {}", style::error("Error:"), e);
        }
    }

    Ok(())
}

fn handle_agent_detect(path: &str) -> allbeads::Result<()> {
    use allbeads::coding_agent::detect_agents;
    use std::path::Path;

    let project_path = Path::new(path)
        .canonicalize()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Invalid path '{}': {}", path, e)))?;

    println!();
    println!("{}", style::header("Agent Detection"));
    println!();
    println!(
        "  Project: {}",
        style::path(&project_path.display().to_string())
    );
    println!();

    let agents = detect_agents(&project_path);

    for status in &agents {
        let icon = if status.configured {
            style::success("✓")
        } else {
            style::dim("·")
        };

        print!("  {} {}", icon, status.agent.display_name());

        if status.configured {
            if let Some(ref config_path) = status.config_path {
                print!(" {}", style::dim(&format!("({})", config_path)));
            }
            if status.has_allbeads_context {
                print!(" {}", style::success("[synced]"));
            }
        } else {
            print!(" {}", style::dim("(not configured)"));
        }
        println!();
    }

    println!();
    println!("  Tip: Use 'ab agent init <agent>' to configure an agent.");

    Ok(())
}

// ============================================================================
// Handoff Command
// ============================================================================

fn handle_handoff_command(
    id: Option<&str>,
    agent: Option<&str>,
    ready: bool,
    list: bool,
    agents: bool,
    dry_run: bool,
    worktree: bool,
) -> allbeads::Result<()> {
    use allbeads::config::AllBeadsConfig;
    use allbeads::handoff::AgentType;
    use std::process::Command;

    // Show available agents
    if agents {
        return handle_handoff_agents();
    }

    // List handed off beads
    if list {
        return handle_handoff_list();
    }

    // Hand off all ready beads
    if ready {
        return handle_handoff_ready(agent);
    }

    // Hand off a specific bead
    let bead_id = id.ok_or_else(|| {
        allbeads::AllBeadsError::Config(
            "Bead ID required. Usage: ab handoff <bead-id> [--agent <agent>]".to_string(),
        )
    })?;

    // Parse agent type: explicit > config > prompt
    let agent_type = if let Some(agent_name) = agent {
        // Explicit --agent flag
        agent_name.parse::<AgentType>().map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Invalid agent '{}': {}", agent_name, e))
        })?
    } else if let Some(preferred) = allbeads::handoff::get_preferred_agent() {
        // Saved preference
        preferred
    } else {
        // First use - prompt user to select
        prompt_for_agent_selection()?
    };

    // Check if agent is available (skip in dry-run mode)
    let agent_cmd = agent_type.command();
    let cli_available = if dry_run || agent_type.is_web_agent() {
        false // Web agents don't have CLI
    } else {
        agent_type.is_installed()
    };

    // If CLI not available and no web fallback, error out
    if !dry_run && !cli_available && !agent_type.has_web_fallback() && !agent_type.is_web_agent() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Agent '{}' not found. Is {} installed?",
            agent_type.display_name(),
            agent_cmd
        )));
    }

    // Load config to find bead's context
    let config = AllBeadsConfig::load_default().ok();

    // Helper to find context by prefix
    fn find_context_path(prefix: &str, config: Option<&AllBeadsConfig>) -> Option<std::path::PathBuf> {
        let config = config?;
        for ctx in &config.contexts {
            if let Some(ref ctx_path) = ctx.path {
                // Check config.yaml for issue-prefix
                let config_path = ctx_path.join(".beads/config.yaml");
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    for line in content.lines() {
                        if let Some(value) = line.strip_prefix("issue-prefix:") {
                            let ctx_prefix = value.trim().trim_matches('"').trim_matches('\'');
                            if ctx_prefix.eq_ignore_ascii_case(prefix) {
                                return Some(ctx_path.clone());
                            }
                        }
                    }
                }
                // Also check if issues.jsonl has IDs with this prefix
                let jsonl_path = ctx_path.join(".beads/issues.jsonl");
                if let Ok(content) = std::fs::read_to_string(&jsonl_path) {
                    if let Some(first_line) = content.lines().next() {
                        if let Ok(issue) = serde_json::from_str::<serde_json::Value>(first_line) {
                            if let Some(id) = issue.get("id").and_then(|v| v.as_str()) {
                                if let Some(found_prefix) = id.split('-').next() {
                                    if found_prefix.eq_ignore_ascii_case(prefix) {
                                        return Some(ctx_path.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // Extract prefix from bead ID and find context
    let bead_prefix = bead_id.split('-').next().unwrap_or("");
    let context_path = find_context_path(bead_prefix, config.as_ref());

    // Load bead from the correct context
    let beads = if let Some(ref path) = context_path {
        Beads::with_workdir(path)
    } else {
        // Fall back to current directory
        Beads::new().map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to initialize beads: {}", e))
        })?
    };

    let issue = beads.show(bead_id).map_err(|e| {
        let hint = if context_path.is_none() {
            format!(
                "\n\nHint: Bead '{}' may be in a different context. \
                 Make sure the repository is added with 'ab context add'.",
                bead_id
            )
        } else {
            String::new()
        };
        allbeads::AllBeadsError::Config(format!("Failed to load bead '{}': {}{}", bead_id, e, hint))
    })?;

    // Build prompt from bead
    let prompt = build_handoff_prompt(&issue);

    // Create worktree if requested
    let working_dir = if worktree && !agent_type.is_web_agent() {
        let worktree_path = create_handoff_worktree(bead_id)?;
        println!(
            "  {} Created worktree at: {}",
            style::success("✓"),
            style::path(&worktree_path.display().to_string())
        );
        Some(worktree_path)
    } else {
        None
    };

    // Display handoff info
    println!();
    println!("{}", style::header("Handoff"));
    println!();
    println!("  Bead:    {} - {}", style::highlight(bead_id), issue.title);
    println!("  Agent:   {}", style::highlight(agent_type.display_name()));
    if let Some(ref wt_path) = working_dir {
        println!(
            "  Worktree: {}",
            style::path(&wt_path.display().to_string())
        );
    }
    println!(
        "  Command: {} {}",
        agent_cmd,
        agent_type.prompt_args(&prompt).join(" ")
    );
    println!();

    if dry_run {
        println!(
            "  {} Dry run - showing prompt that would be sent:",
            style::dim("→")
        );
        println!();
        println!("{}", style::dim("--- PROMPT START ---"));
        println!("{}", prompt);
        println!("{}", style::dim("--- PROMPT END ---"));
        println!();
        println!("  {} Would set AB_ACTIVE_BEAD={}", style::dim("→"), bead_id);
        println!(
            "  {} Would update bead status to in_progress",
            style::dim("→")
        );
        if worktree {
            println!("  {} Would create worktree for bead", style::dim("→"));
        }

        // Show web URL for web agents
        if agent_type.is_web_agent() {
            let repo_url = get_git_remote_url();
            if let Some(url) = agent_type.build_web_url(&prompt, repo_url.as_deref()) {
                println!(
                    "  {} Would open browser to: {}",
                    style::dim("→"),
                    style::highlight(&url)
                );
            }
        }

        return Ok(());
    }

    // Update bead status to in_progress
    println!(
        "  {} Updating bead status to in_progress...",
        style::dim("→")
    );
    beads
        .update(bead_id, Some("in_progress"), None, None, None)
        .map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to update bead '{}': {}", bead_id, e))
        })?;

    // Add handoff info as a comment
    let handoff_comment = format!(
        "[HANDOFF] Agent: {}, Time: {}",
        agent_type.display_name(),
        chrono::Utc::now().to_rfc3339()
    );
    if let Err(e) = beads.comment_add(bead_id, &handoff_comment) {
        // Non-fatal - log but continue
        eprintln!(
            "  {} Failed to add handoff comment: {}",
            style::warning("⚠"),
            e
        );
    }

    // Add handoff label for easy filtering
    if let Err(e) = beads.label_add(bead_id, "handed-off") {
        // Non-fatal - log but continue
        eprintln!(
            "  {} Failed to add handoff label: {}",
            style::warning("⚠"),
            e
        );
    }

    println!(
        "  {} Launching {}...",
        style::success("✓"),
        agent_type.display_name()
    );
    println!();

    // Build args and launch
    let args = agent_type.prompt_args(&prompt);

    // Set environment variable for agent context
    std::env::set_var("AB_ACTIVE_BEAD", bead_id);

    // Launch the agent: CLI if available, web fallback otherwise
    if cli_available {
        // Launch via CLI
        let mut cmd = Command::new(agent_cmd);
        cmd.args(&args).env("AB_ACTIVE_BEAD", bead_id);

        // Run in worktree if created
        if let Some(ref wt_path) = working_dir {
            cmd.current_dir(wt_path);
        }

        let status = cmd.status().map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to launch {}: {}", agent_cmd, e))
        })?;

        if !status.success() {
            return Err(allbeads::AllBeadsError::Config(format!(
                "Agent {} exited with error",
                agent_type.display_name()
            )));
        }
    } else if agent_type.is_web_agent() || agent_type.has_web_fallback() {
        // Web agents - open browser (fire and forget)
        // Try to get the repo URL for the web agent
        let repo_url = get_git_remote_url();

        if let Some(url) = agent_type.build_web_url(&prompt, repo_url.as_deref()) {
            // Add the URL to the bead as a comment
            let url_comment = format!("[TASK_URL] {}", url);
            if let Err(e) = beads.comment_add(bead_id, &url_comment) {
                eprintln!("  {} Failed to store task URL: {}", style::warning("⚠"), e);
            }

            // Open browser
            println!(
                "  {} Opening {} in browser...",
                style::dim("→"),
                agent_type.display_name()
            );
            println!("  {} {}", style::dim("URL:"), style::highlight(&url));

            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(&url).spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("xdg-open").arg(&url).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
            }

            println!();
            println!(
                "  {} Handed off to {} (fire and forget)",
                style::success("✓"),
                agent_type.display_name()
            );
            println!("  {} The agent will work asynchronously.", style::dim("→"));
        } else {
            println!(
                "  {} Web agent launch failed - no URL available for {}",
                style::warning("⚠"),
                agent_type.display_name()
            );
        }
    }

    Ok(())
}

/// Get the git remote URL (origin) for the current repo
/// Prompt user to select their preferred agent (first-time use)
fn prompt_for_agent_selection() -> allbeads::Result<allbeads::handoff::AgentType> {
    use allbeads::handoff::{get_installed_agents, save_preferred_agent, AgentType};
    use std::io::{self, Write};

    println!();
    println!("{}", style::header("First-time Agent Selection"));
    println!();
    println!(
        "  {} No preferred agent set. Let's choose one!",
        style::dim("→")
    );
    println!();

    // Get installed agents
    let installed = get_installed_agents();

    if installed.is_empty() {
        return Err(allbeads::AllBeadsError::Config(
            "No agents detected! Install claude, gemini, cursor, or another supported agent."
                .to_string(),
        ));
    }

    // Show numbered list
    println!("  Available agents:");
    for (i, agent) in installed.iter().enumerate() {
        let marker = if *agent == AgentType::Claude {
            format!("{} (recommended)", style::success("*"))
        } else {
            String::new()
        };
        println!("    {}. {} {}", i + 1, agent.display_name(), marker);
    }
    println!();

    // Prompt for selection
    print!("  Select agent [1-{}]: ", installed.len());
    io::stdout()
        .flush()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to flush stdout: {}", e)))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

    let selection: usize = input.trim().parse().map_err(|_| {
        allbeads::AllBeadsError::Config("Invalid selection. Please enter a number.".to_string())
    })?;

    if selection < 1 || selection > installed.len() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Invalid selection. Please enter 1-{}.",
            installed.len()
        )));
    }

    let selected = installed[selection - 1];

    // Save preference
    if let Err(e) = save_preferred_agent(selected) {
        eprintln!("  {} Failed to save preference: {}", style::warning("⚠"), e);
    } else {
        println!();
        println!(
            "  {} Saved {} as your preferred agent.",
            style::success("✓"),
            selected.display_name()
        );
        println!(
            "  {} Change with: ab handoff --agent <name>",
            style::dim("Tip:")
        );
    }

    println!();
    Ok(selected)
}

fn get_git_remote_url() -> Option<String> {
    use std::process::Command;
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }
    None
}

/// Create a git worktree for isolated agent work on a bead
fn create_handoff_worktree(bead_id: &str) -> allbeads::Result<PathBuf> {
    use std::process::Command;

    // Get the repository root
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to get git root: {}", e)))?;

    if !output.status.success() {
        return Err(allbeads::AllBeadsError::Config(
            "Not in a git repository".to_string(),
        ));
    }

    let repo_root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let worktrees_dir = repo_root.join(".worktrees");

    // Create worktrees directory if it doesn't exist
    if !worktrees_dir.exists() {
        std::fs::create_dir_all(&worktrees_dir).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to create worktrees directory: {}", e))
        })?;
    }

    // Sanitize bead ID for use as directory name
    let safe_name = bead_id.replace(['/', '\\', ':'], "-");
    let worktree_path = worktrees_dir.join(&safe_name);

    // If worktree already exists, return it
    if worktree_path.exists() {
        println!("  {} Using existing worktree", style::dim("→"));
        return Ok(worktree_path);
    }

    // Create branch name for this bead
    let branch_name = format!("ab/{}", safe_name);

    // Get current branch to base off
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to get HEAD: {}", e)))?;

    if !output.status.success() {
        return Err(allbeads::AllBeadsError::Config(
            "Failed to get current commit".to_string(),
        ));
    }

    // Create the worktree with a new branch
    println!("  {} Creating worktree for {}...", style::dim("→"), bead_id);
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch_name,
            worktree_path.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to create worktree: {}", e))
        })?;

    if !output.status.success() {
        // Try without -b if branch already exists
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap_or(""),
                &branch_name,
            ])
            .output()
            .map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to create worktree: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(allbeads::AllBeadsError::Config(format!(
                "Failed to create worktree: {}",
                stderr.trim()
            )));
        }
    }

    Ok(worktree_path)
}

fn build_handoff_prompt(issue: &beads::Issue) -> String {
    let mut prompt = format!(
        "You are working on bead {}.\n\n## Title\n{}\n",
        issue.id, issue.title
    );

    if let Some(ref desc) = issue.description {
        prompt.push_str(&format!("\n## Description\n{}\n", desc));
    }

    if !issue.dependencies.is_empty() {
        prompt.push_str("\n## Dependencies (blocked by)\n");
        for dep in &issue.dependencies {
            prompt.push_str(&format!("- {}\n", dep.id));
        }
    }

    if !issue.labels.is_empty() {
        prompt.push_str(&format!("\n## Labels\n{}\n", issue.labels.join(", ")));
    }

    prompt.push_str("\n## Instructions\nPlease work on this issue. When done, commit your changes and update the bead status.\n");

    prompt
}

fn handle_handoff_list() -> allbeads::Result<()> {
    println!();
    println!("{}", style::header("Handed Off Beads"));
    println!();

    // Load beads and filter by handoff label
    let beads = Beads::new().map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to initialize beads: {}", e))
    })?;
    let issues = beads
        .list(Some("in_progress"), None)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to list beads: {}", e)))?;

    // Filter to only those with handed-off label
    let handed_off: Vec<_> = issues
        .iter()
        .filter(|i| i.labels.iter().any(|l| l == "handed-off"))
        .collect();

    if handed_off.is_empty() {
        println!("  No beads currently handed off to agents.");
        println!();
        println!("  Use 'ab handoff <bead-id>' to hand off a bead.");
    } else {
        for issue in &handed_off {
            println!(
                "  {} {} - {}",
                style::highlight(&issue.id),
                style::dim("→"),
                issue.title
            );
        }
    }

    println!();
    Ok(())
}

fn handle_handoff_agents() -> allbeads::Result<()> {
    use allbeads::handoff::detect_installed_agents;

    println!();
    println!("{}", style::header("Available Agents"));
    println!();

    let agents = detect_installed_agents();

    // Terminal agents
    println!("  {} {}", style::dim("Terminal Agents:"), "");
    for (agent, installed) in &agents {
        if !agent.is_web_agent() && !agent.is_ide_agent() {
            let status = if *installed {
                style::success("✓")
            } else {
                style::dim("○")
            };
            let cmd = if *installed {
                format!("({})", agent.command())
            } else {
                String::new()
            };
            println!(
                "    {} {} {}",
                status,
                agent.display_name(),
                style::dim(&cmd)
            );
        }
    }

    // IDE agents
    println!();
    println!("  {} {}", style::dim("IDE Agents:"), "");
    for (agent, installed) in &agents {
        if agent.is_ide_agent() {
            let status = if *installed {
                style::success("✓")
            } else {
                style::dim("○")
            };
            let cmd = if *installed {
                format!("({})", agent.command())
            } else {
                String::new()
            };
            println!(
                "    {} {} {}",
                status,
                agent.display_name(),
                style::dim(&cmd)
            );
        }
    }

    // Web agents
    println!();
    println!("  {} {}", style::dim("Web Agents (browser):"), "");
    for (agent, _) in &agents {
        if agent.is_web_agent() {
            if let Some(url) = agent.web_url() {
                println!(
                    "    {} {} {}",
                    style::success("✓"),
                    agent.display_name(),
                    style::dim(&format!("({})", url))
                );
            }
        }
    }

    println!();
    println!(
        "  {} Use '--agent <name>' to specify an agent",
        style::dim("Tip:")
    );
    println!();

    Ok(())
}

fn handle_handoff_ready(agent: Option<&str>) -> allbeads::Result<()> {
    use allbeads::handoff::AgentType;

    let agent_type = if let Some(name) = agent {
        name.parse::<AgentType>().map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Invalid agent '{}': {}", name, e))
        })?
    } else {
        AgentType::Claude
    };

    println!();
    println!("{}", style::header("Ready Beads"));
    println!();

    // Load ready beads (unblocked open issues)
    let beads = Beads::new().map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to initialize beads: {}", e))
    })?;
    let ready_issues = beads.ready().map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to get ready beads: {}", e))
    })?;

    if ready_issues.is_empty() {
        println!("  No beads ready for handoff.");
        return Ok(());
    }

    println!(
        "  Found {} ready beads for {}:",
        ready_issues.len(),
        agent_type.display_name()
    );
    println!();

    for issue in &ready_issues {
        println!(
            "  {} {} - {}",
            style::dim("○"),
            style::highlight(&issue.id),
            issue.title
        );
    }

    println!();
    println!(
        "  Use 'ab handoff <id> --agent {}' to hand off a specific bead.",
        agent_type.command()
    );

    Ok(())
}

// ============================================================================
// Sync Command
// ============================================================================

fn handle_sync_command(
    all: bool,
    context: Option<&str>,
    message: Option<&str>,
    status: bool,
    config_path: &Option<String>,
) -> allbeads::Result<()> {
    println!();
    println!("{}", style::header("AllBeads Sync"));
    println!();

    // Load config
    let config = if let Some(path) = config_path {
        AllBeadsConfig::load(path)?
    } else {
        AllBeadsConfig::load_default().unwrap_or_else(|_| AllBeadsConfig::new())
    };

    // Get config directory
    let config_dir = if let Some(path) = config_path {
        PathBuf::from(path).parent().unwrap().to_path_buf()
    } else {
        AllBeadsConfig::default_path()
            .parent()
            .unwrap()
            .to_path_buf()
    };

    if status {
        // Show sync status only
        println!(
            "  Config directory: {}",
            style::path(&config_dir.display().to_string())
        );

        // Check if config dir is a git repo
        if config_dir.join(".git").exists() {
            match git2::Repository::open(&config_dir) {
                Ok(repo) => {
                    let statuses = repo.statuses(None).ok();
                    let changes = statuses.map(|s| s.len()).unwrap_or(0);
                    if changes > 0 {
                        println!(
                            "  Config status: {} uncommitted changes",
                            style::warning(&changes.to_string())
                        );
                    } else {
                        println!("  Config status: {}", style::success("clean"));
                    }
                }
                Err(_) => {
                    println!("  Config status: {}", style::dim("not a git repository"));
                }
            }
        } else {
            println!("  Config status: {}", style::dim("not tracked in git"));
        }

        // Show context status
        if !config.contexts.is_empty() {
            println!();
            println!("  Contexts:");
            for ctx in &config.contexts {
                if let Some(ref path) = ctx.path {
                    let beads_dir = path.join(".beads");
                    let has_beads = beads_dir.exists();
                    let status = if has_beads {
                        style::success("✓ beads")
                    } else {
                        style::dim("no beads")
                    };
                    println!(
                        "    {} {} - {}",
                        status,
                        style::highlight(&ctx.name),
                        path.display()
                    );
                } else {
                    println!(
                        "    {} {} - {}",
                        style::dim("?"),
                        style::highlight(&ctx.name),
                        style::dim("(no local path)")
                    );
                }
            }
        }

        return Ok(());
    }

    // Sync config directory if it's a git repo
    if config_dir.join(".git").exists() {
        println!("  Syncing config directory...");

        match git2::Repository::open(&config_dir) {
            Ok(repo) => {
                // Get statuses
                let statuses = repo.statuses(None)?;

                if statuses.is_empty() {
                    println!("    {}", style::dim("No changes to commit"));
                } else {
                    // Stage all changes
                    let mut index = repo.index()?;
                    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
                    index.write()?;

                    // Create commit
                    let tree_id = index.write_tree()?;
                    let tree = repo.find_tree(tree_id)?;
                    let sig = repo.signature().unwrap_or_else(|_| {
                        git2::Signature::now("AllBeads", "allbeads@local").unwrap()
                    });
                    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

                    let commit_msg = message.unwrap_or("sync");
                    let parents: Vec<&git2::Commit> = head.iter().collect();

                    repo.commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &parents)?;
                    println!("    {} Committed changes", style::success("✓"));
                }

                // Try to pull and push if remote exists
                if let Ok(remote) = repo.find_remote("origin") {
                    if remote.url().is_some() {
                        // Use git command for pull/push (git2 auth is complex)
                        let config_dir_str = config_dir.display().to_string();

                        // Pull
                        let pull_result = std::process::Command::new("git")
                            .args(["pull", "--rebase"])
                            .current_dir(&config_dir_str)
                            .output();

                        match pull_result {
                            Ok(output) if output.status.success() => {
                                println!("    {} Pulled from remote", style::success("✓"));
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                if !stderr.contains("up to date") {
                                    println!(
                                        "    {} Pull warning: {}",
                                        style::warning("!"),
                                        stderr.trim()
                                    );
                                }
                            }
                            Err(_) => {
                                println!(
                                    "    {} Could not pull (git command failed)",
                                    style::dim("○")
                                );
                            }
                        }

                        // Push
                        let push_result = std::process::Command::new("git")
                            .args(["push"])
                            .current_dir(&config_dir_str)
                            .output();

                        match push_result {
                            Ok(output) if output.status.success() => {
                                println!("    {} Pushed to remote", style::success("✓"));
                            }
                            Ok(_) => {
                                println!(
                                    "    {} Could not push (may need to pull first)",
                                    style::warning("!")
                                );
                            }
                            Err(_) => {
                                println!(
                                    "    {} Could not push (git command failed)",
                                    style::dim("○")
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("    {} Could not sync config: {}", style::error("✗"), e);
            }
        }
    } else {
        println!("  Config directory is not tracked in git");
        println!("  Use 'ab config init --remote <url>' to set up sync");
    }

    // Sync specific context or all contexts
    if all || context.is_some() {
        println!();

        let contexts_to_sync: Vec<_> = if let Some(ctx_name) = context {
            config
                .contexts
                .iter()
                .filter(|c| c.name == ctx_name)
                .collect()
        } else {
            config.contexts.iter().collect()
        };

        if contexts_to_sync.is_empty() {
            if let Some(ctx_name) = context {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Context '{}' not found",
                    ctx_name
                )));
            }
            println!("  No contexts configured");
        } else {
            for ctx in contexts_to_sync {
                println!("  Syncing context: {}", style::highlight(&ctx.name));

                let ctx_path = match &ctx.path {
                    Some(p) => p.clone(),
                    None => {
                        println!("    {} No local path configured", style::dim("○"));
                        continue;
                    }
                };

                let beads_dir = ctx_path.join(".beads");
                if !beads_dir.exists() {
                    println!("    {} No beads directory", style::dim("○"));
                    continue;
                }

                // Run bd sync in the context directory
                let sync_result = std::process::Command::new("bd")
                    .arg("sync")
                    .current_dir(&ctx_path)
                    .output();

                match sync_result {
                    Ok(output) if output.status.success() => {
                        println!("    {} Beads synced", style::success("✓"));
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.contains("Sync complete") || stdout.contains("no changes") {
                            println!("    {} Beads synced", style::success("✓"));
                        } else {
                            println!("    {} Sync issue: {}", style::warning("!"), stderr.trim());
                        }
                    }
                    Err(_) => {
                        println!(
                            "    {} 'bd' command not found - install beads CLI",
                            style::error("✗")
                        );
                    }
                }
            }
        }
    }

    println!();
    Ok(())
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
            // Determine if this is a local path or URL-only context
            let (repo_path_opt, remote_url, context_name) = if let Some(url_str) = url {
                // URL provided - this is the primary mode
                let inferred_name = if let Some(n) = name {
                    n.clone()
                } else {
                    // Extract repo name from URL
                    // Examples:
                    //   https://github.com/user/repo.git -> repo
                    //   git@github.com:user/repo.git -> repo
                    let url_path = url_str
                        .trim_end_matches(".git")
                        .rsplit('/')
                        .next()
                        .or_else(|| url_str.rsplit(':').next())
                        .unwrap_or("unknown");
                    url_path.to_string()
                };

                // Check if path provided and exists as git repo
                let path_opt = if let Some(ref p) = path {
                    match std::fs::canonicalize(p) {
                        Ok(abs_path) if abs_path.join(".git").exists() => Some(abs_path),
                        Ok(abs_path) => {
                            eprintln!(
                                "⚠️  Path '{}' exists but is not a git repository",
                                abs_path.display()
                            );
                            None
                        }
                        Err(_) => {
                            eprintln!("ℹ️  Path not found locally - tracking URL only");
                            None
                        }
                    }
                } else {
                    None
                };

                (path_opt, url_str.clone(), inferred_name)
            } else if let Some(p) = path.as_ref() {
                // Path provided but no URL - use git remote
                let repo_path = std::fs::canonicalize(p).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!(
                        "Failed to resolve path '{}': {}",
                        p, e
                    ))
                })?;

                let git_dir = repo_path.join(".git");
                if !git_dir.exists() {
                    return Err(allbeads::AllBeadsError::Config(format!(
                        "'{}' is not a git repository (no .git directory)",
                        repo_path.display()
                    )));
                }

                let inferred_name = if let Some(n) = name {
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

                // Get git remote URL
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
                    return Err(allbeads::AllBeadsError::Config(
                        "No 'origin' remote found. Add one with:\n  \
                         git remote add origin <url>\n\n\
                         Or specify the URL explicitly:\n  \
                         ab context add --url <url>"
                            .to_string(),
                    ));
                }

                let url_from_git = String::from_utf8_lossy(&output.stdout).trim().to_string();
                (Some(repo_path), url_from_git, inferred_name)
            } else {
                // Neither path nor URL provided
                return Err(allbeads::AllBeadsError::Config(
                    "Either provide a path to a local git repository or use --url <url> to track a remote repository".to_string()
                ));
            };

            // Check if context already exists
            if config.get_context(&context_name).is_some() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Context '{}' already exists",
                    context_name
                )));
            }

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

            // Print confirmation
            if let Some(ref path) = repo_path_opt {
                println!("✓ Added context '{}' from {}", context_name, path.display());
            } else {
                println!("✓ Added context '{}' (URL-only)", context_name);
            }
            println!("  URL:  {}", remote_url);
            println!("  Auth: {:?}", auth_strategy);

            // Create context
            let mut context = BossContext::new(&context_name, &remote_url, auth_strategy);
            context.path = repo_path_opt;

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

        ContextCommands::Onboarding { full, summary } => {
            use allbeads::onboarding::OnboardingReport;

            // Generate onboarding report
            let report = OnboardingReport::from_contexts(&config.contexts)?;

            if *summary {
                report.print_summary();
            } else {
                report.print();
            }

            // In full mode, show additional guidance
            if *full {
                eprintln!("\n📚 Getting Started with AllBeads Onboarding:\n");
                eprintln!("1. URL-only contexts will be cloned automatically when you run:");
                eprintln!("   cargo run -- list -C <context-name>");
                eprintln!("\n2. Once cloned, initialize beads in each repository:");
                eprintln!("   cd <repo-path> && bd init");
                eprintln!("\n3. Start tracking issues:");
                eprintln!("   bd create --title=\"Your first issue\" --type=task --priority=2");
                eprintln!("\n4. Enable integrations (optional):");
                eprintln!("   cargo run -- jira status");
                eprintln!("   cargo run -- github status");
                eprintln!("\n5. Launch the TUI to see everything:");
                eprintln!("   cargo run -- tui");
                eprintln!();
            }
        }

        ContextCommands::New {
            name,
            description,
            private,
            org,
            readme: _,
            gitignore,
            license,
            template,
            init_beads,
            init_agents,
            path,
            no_clone,
            wizard,
            non_interactive,
        } => {
            use allbeads::context_new::{create_new_repository, NewRepoConfig, NewRepoPrompt};

            // Determine mode: wizard, non-interactive, or standard
            let repo_config = if *wizard {
                // Interactive TUI wizard
                let prompt = NewRepoPrompt::new();
                prompt.run()?
            } else if *non_interactive {
                // Non-interactive: require name
                let repo_name = name.clone().ok_or_else(|| {
                    allbeads::AllBeadsError::Config(
                        "Repository name required in non-interactive mode. Use --name <name>"
                            .to_string(),
                    )
                })?;
                NewRepoConfig {
                    name: repo_name,
                    description: description.clone(),
                    private: *private,
                    org: org.clone(),
                    gitignore: gitignore.clone(),
                    license: license.clone(),
                    template: template.clone(),
                    init_beads: *init_beads,
                    init_agents: init_agents
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect(),
                    clone_path: path.clone(),
                    no_clone: *no_clone,
                }
            } else {
                // Standard mode: prompt for missing required fields
                let prompt = NewRepoPrompt::new();
                prompt.run_with_defaults(
                    name.clone(),
                    description.clone(),
                    *private,
                    org.clone(),
                    gitignore.clone(),
                    license.clone(),
                    template.clone(),
                    *init_beads,
                    init_agents.clone(),
                    path.clone(),
                    *no_clone,
                )?
            };

            // Create the repository
            let result = create_new_repository(&repo_config)?;

            // Add to config if we have a local path
            if let Some(ref local_path) = result.local_path {
                let auth_strategy = if result.clone_url.starts_with("https://") {
                    AuthStrategy::PersonalAccessToken
                } else {
                    AuthStrategy::SshAgent
                };

                let mut new_context =
                    BossContext::new(&result.repo_name, &result.clone_url, auth_strategy);
                new_context.path = Some(local_path.clone());

                config.add_context(new_context);
                config.save(&config_file)?;

                println!(
                    "\n{} Added context '{}' to AllBeads config",
                    style::success("✓"),
                    result.repo_name
                );
            }

            // Summary
            println!("\n{}", style::header("Repository Created Successfully"));
            println!("  Name:   {}", result.repo_name);
            println!("  URL:    {}", result.html_url);
            if let Some(ref path) = result.local_path {
                println!("  Path:   {}", path.display());
            }
            if result.beads_initialized {
                println!("  Beads:  initialized");
            }
            if !result.agents_configured.is_empty() {
                println!("  Agents: {}", result.agents_configured.join(", "));
            }
        }
    }

    Ok(())
}

// === Folder Tracking Commands (Phase 1 of PRD-01) ===

fn handle_folder_command(cmd: &FolderCommands) -> allbeads::Result<()> {
    use allbeads::context::{Context, FolderConfig, FolderStatus, TrackedFolder};

    // Get folder tracking file path
    let folders_file = AllBeadsConfig::default_path()
        .parent()
        .map(|p| p.join("folders.yaml"))
        .ok_or_else(|| {
            allbeads::AllBeadsError::Config("Could not determine config directory".to_string())
        })?;

    // Load or create folder tracking context
    let mut context = if folders_file.exists() {
        let content = std::fs::read_to_string(&folders_file).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to read folders.yaml: {}", e))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to parse folders.yaml: {}", e))
        })?
    } else {
        Context::new("default")
    };

    match cmd {
        FolderCommands::Add {
            paths,
            prefix,
            persona,
            setup: _,
        } => {
            let mut added = 0;
            let mut skipped = 0;

            for path_pattern in paths {
                // Expand ~ to home directory
                let expanded = if let Some(suffix) = path_pattern.strip_prefix("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(suffix)
                    } else {
                        PathBuf::from(path_pattern)
                    }
                } else {
                    PathBuf::from(path_pattern)
                };

                // Handle glob patterns
                let paths_to_add: Vec<PathBuf> = if path_pattern.contains('*') {
                    glob::glob(expanded.to_str().unwrap_or(""))
                        .map_err(|e| {
                            allbeads::AllBeadsError::Config(format!("Invalid glob pattern: {}", e))
                        })?
                        .filter_map(|r| r.ok())
                        .filter(|p| p.is_dir())
                        .collect()
                } else {
                    vec![expanded]
                };

                for path in paths_to_add {
                    // Resolve to absolute path
                    let abs_path = std::fs::canonicalize(&path).map_err(|e| {
                        allbeads::AllBeadsError::Config(format!(
                            "Failed to resolve path '{}': {}",
                            path.display(),
                            e
                        ))
                    })?;

                    // Check if already tracked
                    if context.get_folder(&abs_path).is_some() {
                        println!(
                            "  {} {} (already tracked)",
                            style::dim("○"),
                            abs_path.display()
                        );
                        skipped += 1;
                        continue;
                    }

                    // Detect status
                    let status = detect_folder_status(&abs_path);

                    // Create folder config if prefix/persona specified
                    let config = if prefix.is_some() || persona.is_some() {
                        Some(FolderConfig {
                            prefix: prefix.clone(),
                            persona: persona.clone(),
                            ..Default::default()
                        })
                    } else {
                        None
                    };

                    // Create tracked folder
                    let mut folder = TrackedFolder::new(&abs_path).with_status(status);
                    folder.config = config;

                    // Print status
                    println!(
                        "  {} {} {}",
                        style::folder_status_indicator(status.short_name()),
                        style::folder_status(status.short_name()),
                        abs_path.display()
                    );

                    context.add_folder(folder);
                    added += 1;
                }
            }

            // Save context
            if added > 0 {
                // Ensure parent directory exists
                if let Some(parent) = folders_file.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        allbeads::AllBeadsError::Config(format!(
                            "Failed to create config directory: {}",
                            e
                        ))
                    })?;
                }

                let yaml = serde_yaml::to_string(&context).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to serialize folders: {}", e))
                })?;
                std::fs::write(&folders_file, yaml).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to write folders.yaml: {}", e))
                })?;
            }

            println!();
            println!(
                "{} Added {} folder(s), {} skipped",
                style::success("✓"),
                style::count_ready(added),
                style::dim(&skipped.to_string())
            );
        }

        FolderCommands::List {
            status,
            json,
            verbose,
        } => {
            if context.folders.is_empty() {
                println!("No folders tracked. Use 'ab folder add <path>' to start tracking.");
                return Ok(());
            }

            // Filter by status if specified
            let folders: Vec<&TrackedFolder> = if let Some(status_str) = status {
                let filter_status = FolderStatus::parse(status_str).ok_or_else(|| {
                    allbeads::AllBeadsError::Config(format!("Invalid status: {}", status_str))
                })?;
                context
                    .folders
                    .iter()
                    .filter(|f| f.status == filter_status)
                    .collect()
            } else {
                context.folders.iter().collect()
            };

            if *json {
                let json_out = serde_json::to_string_pretty(&folders).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to serialize to JSON: {}", e))
                })?;
                println!("{}", json_out);
                return Ok(());
            }

            // Print header
            println!();
            println!(
                "{} Tracked Folders ({} total):",
                style::header("○"),
                context.folder_count()
            );
            println!();

            // Print status counts
            let counts = context.status_counts();
            print!("  ");
            for status in &[
                FolderStatus::Dry,
                FolderStatus::Git,
                FolderStatus::Beads,
                FolderStatus::Configured,
                FolderStatus::Wet,
            ] {
                let count = counts.get(status).unwrap_or(&0);
                if *count > 0 {
                    print!(
                        "{} {} {}  ",
                        style::folder_status_indicator(status.short_name()),
                        status.short_name(),
                        count
                    );
                }
            }
            println!();
            println!();

            // Print folders
            for folder in folders {
                let status_icon = style::folder_status_indicator(folder.status.short_name());
                let status_text = style::folder_status(folder.status.short_name());
                let path_display = folder.display_path();

                print!("  {} {:8} {}", status_icon, status_text, path_display);

                if let Some(ref config) = folder.config {
                    if let Some(ref prefix) = config.prefix {
                        print!("  {}", style::dim(&format!("[{}]", prefix)));
                    }
                }

                println!();

                if *verbose {
                    if folder.bead_count > 0 {
                        println!("      Beads: {}", folder.bead_count);
                    }
                    if let Some(ref added) = folder.added_at {
                        println!("      Added: {}", &added[..19]);
                    }
                }
            }

            println!();
            println!(
                "{}",
                style::dim("Legend: ○ dry → ◔ git → ◑ beads → ◕ configured → ● wet")
            );
        }

        FolderCommands::Remove { path, clean: _ } => {
            // Resolve path
            let abs_path = std::fs::canonicalize(path).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
            })?;

            if context.remove_folder(&abs_path).is_some() {
                // Save context
                let yaml = serde_yaml::to_string(&context).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to serialize folders: {}", e))
                })?;
                std::fs::write(&folders_file, yaml).map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to write folders.yaml: {}", e))
                })?;

                println!("Removed folder '{}'", abs_path.display());
            } else {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Folder '{}' not found in tracking",
                    abs_path.display()
                )));
            }
        }

        FolderCommands::Status { path } => {
            // Resolve path
            let abs_path = std::fs::canonicalize(path).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
            })?;

            let status = detect_folder_status(&abs_path);

            println!();
            println!("{}", style::header("Folder Status"));
            println!();
            println!("  Path:   {}", style::path(&abs_path.display().to_string()));
            println!(
                "  Status: {} {}",
                style::folder_status_indicator(status.short_name()),
                style::folder_status(status.short_name())
            );

            // Show what's missing to reach next level
            if let Some(next) = status.next() {
                println!();
                println!(
                    "  {} To reach '{}' status:",
                    style::dim("→"),
                    next.short_name()
                );
                match next {
                    FolderStatus::Git => println!("      Run: git init"),
                    FolderStatus::Beads => println!("      Run: bd init"),
                    FolderStatus::Configured => {
                        println!("      Run: ab folder add {} --prefix=<name>", path)
                    }
                    FolderStatus::Wet => println!("      Enable sync in config"),
                    _ => {}
                }
            } else {
                println!();
                println!("  {} Fully integrated!", style::success("✓"));
            }
        }

        FolderCommands::Setup { path, yes } => {
            handle_folder_setup(path, *yes, &folders_file, &mut context)?;
        }

        FolderCommands::Promote { path, to, yes } => {
            handle_folder_promote(path, to.as_deref(), *yes, &folders_file, &mut context)?;
        }

        FolderCommands::Worktree(wt_cmd) => {
            handle_worktree_command(wt_cmd)?;
        }

        FolderCommands::Monorepo { path } => {
            handle_monorepo_command(path)?;
        }

        FolderCommands::Template(tpl_cmd) => {
            handle_template_command(tpl_cmd)?;
        }
    }

    Ok(())
}

/// Handle worktree subcommands
fn handle_worktree_command(cmd: &WorktreeCommands) -> allbeads::Result<()> {
    match cmd {
        WorktreeCommands::List { path } => {
            let abs_path = std::fs::canonicalize(path).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
            })?;

            // Check if it's a git repository
            if !abs_path.join(".git").exists() {
                return Err(allbeads::AllBeadsError::Config(
                    "Not a git repository".to_string(),
                ));
            }

            // Get worktrees
            let output = std::process::Command::new("git")
                .args(["worktree", "list", "--porcelain"])
                .current_dir(&abs_path)
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run git worktree: {}", e))
                })?;

            if !output.status.success() {
                return Err(allbeads::AllBeadsError::Config(
                    "Failed to list worktrees".to_string(),
                ));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let worktrees = parse_worktree_list(&stdout);

            if worktrees.is_empty() {
                println!("No worktrees found.");
                return Ok(());
            }

            println!();
            println!("{}", style::header("Git Worktrees"));
            println!();

            for wt in &worktrees {
                let status = detect_folder_status(&PathBuf::from(&wt.path));
                let beads_info = if PathBuf::from(&wt.path).join(".beads").exists() {
                    // Count issues
                    let issues_file = PathBuf::from(&wt.path).join(".beads/issues.jsonl");
                    let count = if issues_file.exists() {
                        std::fs::read_to_string(&issues_file)
                            .map(|s| s.lines().count())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    format!("{} issues", count)
                } else {
                    "no beads".to_string()
                };

                let is_bare = wt.bare;
                let branch_display = if is_bare {
                    "(bare)".to_string()
                } else {
                    wt.branch
                        .clone()
                        .unwrap_or_else(|| "(detached)".to_string())
                };

                println!(
                    "  {} {} {}",
                    style::folder_status_indicator(status.short_name()),
                    style::path(&wt.path),
                    style::dim(&format!("({}) - {}", branch_display, beads_info))
                );
            }
            println!();
        }

        WorktreeCommands::Status { path } => {
            let abs_path = std::fs::canonicalize(path).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
            })?;

            let worktree_info = detect_worktree_info(&abs_path);

            println!();
            println!("{}", style::header("Worktree Status"));
            println!();
            println!(
                "  Path:        {}",
                style::path(&abs_path.display().to_string())
            );

            if worktree_info.is_worktree {
                println!("  Type:        {}", style::dim("Git worktree"));
                if let Some(ref main) = worktree_info.main_worktree {
                    println!(
                        "  Main:        {}",
                        style::path(&main.display().to_string())
                    );
                }
                if let Some(ref branch) = worktree_info.branch {
                    println!("  Branch:      {}", branch);
                }
            } else if abs_path.join(".git").exists() {
                println!("  Type:        {}", style::dim("Main repository"));
            } else {
                println!("  Type:        {}", style::dim("Not a git repository"));
            }

            // Beads info
            let status = detect_folder_status(&abs_path);
            println!(
                "  Status:      {} {}",
                style::folder_status_indicator(status.short_name()),
                style::folder_status(status.short_name())
            );

            if abs_path.join(".beads").exists() {
                // Detect beads mode
                let mode = if abs_path.join(".beads/beads.db").exists() {
                    "standard"
                } else if abs_path.join(".beads/issues.jsonl").exists() {
                    "jsonl-only"
                } else {
                    "unknown"
                };
                println!("  Beads Mode:  {}", style::dim(mode));
            }
            println!();
        }
    }

    Ok(())
}

/// Parse git worktree list --porcelain output
fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let mut current: Option<WorktreeInfo> = None;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            if let Some(wt) = current.take() {
                worktrees.push(wt);
            }
            current = Some(WorktreeInfo {
                path: line.strip_prefix("worktree ").unwrap_or("").to_string(),
                branch: None,
                bare: false,
            });
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current {
                let branch = line
                    .strip_prefix("branch refs/heads/")
                    .unwrap_or(line.strip_prefix("branch ").unwrap_or(""));
                wt.branch = Some(branch.to_string());
            }
        } else if line == "bare" {
            if let Some(ref mut wt) = current {
                wt.bare = true;
            }
        }
    }

    if let Some(wt) = current {
        worktrees.push(wt);
    }

    worktrees
}

#[derive(Debug)]
struct WorktreeInfo {
    path: String,
    branch: Option<String>,
    bare: bool,
}

/// Handle monorepo detection command
fn handle_monorepo_command(path: &str) -> allbeads::Result<()> {
    let abs_path = std::fs::canonicalize(path).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
    })?;

    let monorepo_info = detect_monorepo_structure(&abs_path);

    println!();
    println!("{}", style::header("Monorepo Structure"));
    println!();
    println!(
        "  Path:      {}",
        style::path(&abs_path.display().to_string())
    );

    if monorepo_info.is_monorepo {
        println!("  Type:      {}", style::success("Monorepo detected"));
        if let Some(ref tool) = monorepo_info.tool {
            println!("  Tool:      {}", tool);
        }
        println!();

        if !monorepo_info.packages.is_empty() {
            println!("{}", style::subheader("Packages"));
            for pkg in &monorepo_info.packages {
                let lang = pkg.language.as_deref().unwrap_or("unknown");
                println!(
                    "  {} {} {}",
                    style::dim("├──"),
                    style::path(&pkg.path),
                    style::dim(&format!("({})", lang))
                );
            }
            println!();
        }

        if !monorepo_info.services.is_empty() {
            println!("{}", style::subheader("Services"));
            for svc in &monorepo_info.services {
                let lang = svc.language.as_deref().unwrap_or("unknown");
                println!(
                    "  {} {} {}",
                    style::dim("├──"),
                    style::path(&svc.path),
                    style::dim(&format!("({})", lang))
                );
            }
            println!();
        }
    } else {
        println!(
            "  Type:      {}",
            style::dim("Single project (not a monorepo)")
        );
    }

    Ok(())
}

#[derive(Debug, Default)]
struct MonorepoInfo {
    is_monorepo: bool,
    tool: Option<String>,
    packages: Vec<PackageInfo>,
    services: Vec<PackageInfo>,
}

#[derive(Debug)]
struct PackageInfo {
    path: String,
    language: Option<String>,
}

/// Detect monorepo structure and packages
fn detect_monorepo_structure(path: &Path) -> MonorepoInfo {
    let mut info = MonorepoInfo::default();

    // Detect monorepo tool
    if path.join("pnpm-workspace.yaml").exists() {
        info.is_monorepo = true;
        info.tool = Some("pnpm".to_string());
    } else if path.join("lerna.json").exists() {
        info.is_monorepo = true;
        info.tool = Some("lerna".to_string());
    } else if path.join("nx.json").exists() {
        info.is_monorepo = true;
        info.tool = Some("nx".to_string());
    } else if path.join("turbo.json").exists() {
        info.is_monorepo = true;
        info.tool = Some("turborepo".to_string());
    } else if path.join("Cargo.toml").exists() {
        // Check for Rust workspace
        if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
            if content.contains("[workspace]") {
                info.is_monorepo = true;
                info.tool = Some("cargo workspace".to_string());
            }
        }
    }

    // Scan for packages/ directory
    let packages_dir = path.join("packages");
    if packages_dir.exists() && packages_dir.is_dir() {
        info.is_monorepo = true;
        if let Ok(entries) = std::fs::read_dir(&packages_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let pkg_path = entry.path();
                    let language = detect_package_language(&pkg_path);
                    info.packages.push(PackageInfo {
                        path: entry.file_name().to_string_lossy().to_string(),
                        language,
                    });
                }
            }
        }
    }

    // Scan for services/ directory
    let services_dir = path.join("services");
    if services_dir.exists() && services_dir.is_dir() {
        info.is_monorepo = true;
        if let Ok(entries) = std::fs::read_dir(&services_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let svc_path = entry.path();
                    let language = detect_package_language(&svc_path);
                    info.services.push(PackageInfo {
                        path: entry.file_name().to_string_lossy().to_string(),
                        language,
                    });
                }
            }
        }
    }

    // Scan for apps/ directory (common in turborepo)
    let apps_dir = path.join("apps");
    if apps_dir.exists() && apps_dir.is_dir() {
        info.is_monorepo = true;
        if let Ok(entries) = std::fs::read_dir(&apps_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let app_path = entry.path();
                    let language = detect_package_language(&app_path);
                    info.packages.push(PackageInfo {
                        path: format!("apps/{}", entry.file_name().to_string_lossy()),
                        language,
                    });
                }
            }
        }
    }

    info
}

/// Detect language of a package/service
fn detect_package_language(path: &Path) -> Option<String> {
    if path.join("Cargo.toml").exists() {
        Some("Rust".to_string())
    } else if path.join("package.json").exists() {
        if path.join("tsconfig.json").exists() {
            Some("TypeScript".to_string())
        } else {
            Some("JavaScript".to_string())
        }
    } else if path.join("go.mod").exists() {
        Some("Go".to_string())
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        Some("Python".to_string())
    } else if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        Some("Java".to_string())
    } else {
        None
    }
}

/// Detect if path is a worktree and get info
fn detect_worktree_info(path: &Path) -> allbeads::context::DetectedInfo {
    let mut info = allbeads::context::DetectedInfo::default();

    // Check if this is a worktree by looking at .git
    let git_path = path.join(".git");
    if git_path.exists() && git_path.is_file() {
        // .git is a file - this is a worktree
        info.is_worktree = true;

        // Read the .git file to find the main worktree
        if let Ok(content) = std::fs::read_to_string(&git_path) {
            // Format: "gitdir: /path/to/.git/worktrees/name"
            if let Some(gitdir) = content.strip_prefix("gitdir: ") {
                let gitdir = gitdir.trim();
                // Go up from .git/worktrees/name to find main .git
                let gitdir_path = PathBuf::from(gitdir);
                let main_git = gitdir_path
                    .parent() // worktrees
                    .and_then(|p| p.parent()) // .git
                    .and_then(|p| p.parent()); // main worktree
                info.main_worktree = main_git.map(|p| p.to_path_buf());
            }
        }

        // Get current branch for worktree
        if let Ok(output) = std::process::Command::new("git")
            .args([
                "-C",
                path.to_str().unwrap_or("."),
                "branch",
                "--show-current",
            ])
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    info.branch = Some(branch);
                }
            }
        }
    }

    info
}

/// Template definition for project setup
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectTemplate {
    /// Template name
    name: String,
    /// Template description
    #[serde(default)]
    description: String,
    /// Init settings
    #[serde(default)]
    init: TemplateInit,
    /// Beads configuration
    #[serde(default)]
    beads: TemplateBeads,
    /// General config
    #[serde(default)]
    config: TemplateConfig,
    /// Files to copy
    #[serde(default)]
    files: Vec<TemplateFile>,
    /// Source project path (where template was created from)
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TemplateInit {
    #[serde(default)]
    git: bool,
    #[serde(default)]
    beads: bool,
    #[serde(default)]
    claude: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TemplateBeads {
    #[serde(default = "default_beads_mode")]
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sync_branch: Option<String>,
}

fn default_beads_mode() -> String {
    "standard".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TemplateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    persona: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateFile {
    source: String,
    dest: String,
}

/// Get the templates directory path
fn get_templates_dir() -> allbeads::Result<PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        allbeads::AllBeadsError::Config("Could not determine config directory".to_string())
    })?;
    Ok(config_dir.join("allbeads").join("templates"))
}

/// Handle template subcommands
fn handle_template_command(cmd: &TemplateCommands) -> allbeads::Result<()> {
    match cmd {
        TemplateCommands::Create {
            name,
            from,
            description,
        } => {
            handle_template_create(name, from, description.as_deref())?;
        }
        TemplateCommands::Apply { name, path, yes } => {
            handle_template_apply(name, path, *yes)?;
        }
        TemplateCommands::List { json } => {
            handle_template_list(*json)?;
        }
        TemplateCommands::Show { name } => {
            handle_template_show(name)?;
        }
        TemplateCommands::Delete { name, yes } => {
            handle_template_delete(name, *yes)?;
        }
    }
    Ok(())
}

/// Create a template from an existing project
fn handle_template_create(
    name: &str,
    from: &str,
    description: Option<&str>,
) -> allbeads::Result<()> {
    use dialoguer::Confirm;

    let source_path = std::fs::canonicalize(from).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", from, e))
    })?;

    println!();
    println!("{}", style::header("Create Template"));
    println!();
    println!("  Name:   {}", style::highlight(name));
    println!(
        "  Source: {}",
        style::path(&source_path.display().to_string())
    );
    println!();

    // Detect project characteristics
    let has_git = source_path.join(".git").exists();
    let has_beads = source_path.join(".beads").exists();
    let has_claude = source_path.join("CLAUDE.md").exists();
    let detected = detect_project_info(&source_path);

    // Build template
    let mut template = ProjectTemplate {
        name: name.to_string(),
        description: description.unwrap_or("").to_string(),
        init: TemplateInit {
            git: has_git,
            beads: has_beads,
            claude: has_claude,
        },
        beads: TemplateBeads {
            mode: if has_beads {
                if source_path.join(".beads/beads.db").exists() {
                    "standard".to_string()
                } else {
                    "jsonl-only".to_string()
                }
            } else {
                "standard".to_string()
            },
            sync_branch: None,
        },
        config: TemplateConfig {
            persona: None,
            prefix: None,
        },
        files: Vec::new(),
        source: Some(source_path.display().to_string()),
        created: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
    };

    // Collect files to include
    let include_files = vec![
        ("CLAUDE.md", has_claude),
        (
            ".cargo/config.toml",
            source_path.join(".cargo/config.toml").exists(),
        ),
        (
            "rust-toolchain.toml",
            source_path.join("rust-toolchain.toml").exists(),
        ),
        (".prettierrc", source_path.join(".prettierrc").exists()),
        (
            ".eslintrc.json",
            source_path.join(".eslintrc.json").exists(),
        ),
        ("tsconfig.json", source_path.join("tsconfig.json").exists()),
        (
            "pyproject.toml",
            source_path.join("pyproject.toml").exists(),
        ),
    ];

    for (file, exists) in include_files {
        if exists {
            template.files.push(TemplateFile {
                source: format!("{}.template", file),
                dest: file.to_string(),
            });
        }
    }

    println!("  Detected:");
    if has_git {
        println!("    {} Git repository", style::success("✓"));
    }
    if has_beads {
        println!("    {} Beads initialized", style::success("✓"));
    }
    if has_claude {
        println!("    {} CLAUDE.md present", style::success("✓"));
    }
    if !detected.languages.is_empty() {
        println!(
            "    {} Languages: {:?}",
            style::success("✓"),
            detected.languages
        );
    }
    println!();
    println!("  Files to include: {}", template.files.len());
    for f in &template.files {
        println!("    - {}", f.dest);
    }
    println!();

    let proceed = Confirm::new()
        .with_prompt("  Create template?")
        .default(true)
        .interact()
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Input error: {}", e)))?;

    if !proceed {
        println!("  {}", style::dim("Template creation cancelled"));
        return Ok(());
    }

    // Create templates directory
    let templates_dir = get_templates_dir()?;
    std::fs::create_dir_all(&templates_dir).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to create templates directory: {}", e))
    })?;

    // Create template subdirectory
    let template_dir = templates_dir.join(name);
    if template_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Template '{}' already exists. Use 'ab folder template delete {}' first.",
            name, name
        )));
    }
    std::fs::create_dir_all(&template_dir).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to create template directory: {}", e))
    })?;

    // Copy template files
    for file in &template.files {
        let source_file = source_path.join(&file.dest);
        let dest_file = template_dir.join(&file.source);

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if source_file.exists() {
            std::fs::copy(&source_file, &dest_file).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to copy {}: {}", file.dest, e))
            })?;
        }
    }

    // Save template.yaml
    let template_yaml = template_dir.join("template.yaml");
    let yaml = serde_yaml::to_string(&template).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to serialize template: {}", e))
    })?;
    std::fs::write(&template_yaml, yaml).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to write template.yaml: {}", e))
    })?;

    println!();
    println!("  {} Template '{}' created!", style::success("✓"), name);
    println!(
        "  Location: {}",
        style::path(&template_dir.display().to_string())
    );
    println!();

    Ok(())
}

/// Apply a template to a directory
fn handle_template_apply(name: &str, path: &str, yes: bool) -> allbeads::Result<()> {
    use dialoguer::Confirm;

    let templates_dir = get_templates_dir()?;
    let template_dir = templates_dir.join(name);
    let template_yaml = template_dir.join("template.yaml");

    if !template_yaml.exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Template '{}' not found. Use 'ab folder template list' to see available templates.",
            name
        )));
    }

    // Load template
    let content = std::fs::read_to_string(&template_yaml)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to read template: {}", e)))?;
    let template: ProjectTemplate = serde_yaml::from_str(&content)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to parse template: {}", e)))?;

    let target_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));

    println!();
    println!("{}", style::header("Apply Template"));
    println!();
    println!("  Template: {}", style::highlight(&template.name));
    if !template.description.is_empty() {
        println!("  Desc:     {}", style::dim(&template.description));
    }
    println!(
        "  Target:   {}",
        style::path(&target_path.display().to_string())
    );
    println!();

    println!("  Actions:");
    if template.init.git && !target_path.join(".git").exists() {
        println!("    - Initialize git repository");
    }
    if template.init.beads && !target_path.join(".beads").exists() {
        println!("    - Initialize beads ({})", template.beads.mode);
    }
    for f in &template.files {
        println!("    - Copy {}", f.dest);
    }
    println!();

    let proceed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt("  Apply template?")
            .default(true)
            .interact()
            .map_err(|e| allbeads::AllBeadsError::Config(format!("Input error: {}", e)))?
    };

    if !proceed {
        println!("  {}", style::dim("Template application cancelled"));
        return Ok(());
    }

    // Ensure target directory exists
    std::fs::create_dir_all(&target_path).ok();

    // Initialize git if needed
    if template.init.git && !target_path.join(".git").exists() {
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&target_path)
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                println!("  {} Git initialized", style::success("✓"));
            }
        }
    }

    // Initialize beads if needed
    if template.init.beads && !target_path.join(".beads").exists() {
        let output = std::process::Command::new("bd")
            .args(["init"])
            .current_dir(&target_path)
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                println!("  {} Beads initialized", style::success("✓"));
            }
        }
    }

    // Copy files
    for file in &template.files {
        let source_file = template_dir.join(&file.source);
        let dest_file = target_path.join(&file.dest);

        if source_file.exists() {
            if let Some(parent) = dest_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            std::fs::copy(&source_file, &dest_file).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to copy {}: {}", file.dest, e))
            })?;
            println!("  {} Copied {}", style::success("✓"), file.dest);
        }
    }

    println!();
    println!("  {} Template '{}' applied!", style::success("✓"), name);
    println!();

    Ok(())
}

/// List available templates
fn handle_template_list(json: bool) -> allbeads::Result<()> {
    let templates_dir = get_templates_dir()?;

    if !templates_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!();
            println!("{}", style::header("Templates"));
            println!();
            println!("  {}", style::dim("No templates found"));
            println!("  Use 'ab folder template create <name> --from=<path>' to create one");
            println!();
        }
        return Ok(());
    }

    let mut templates = Vec::new();

    for entry in std::fs::read_dir(&templates_dir).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to read templates directory: {}", e))
    })? {
        let entry = entry
            .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to read entry: {}", e)))?;

        let template_yaml = entry.path().join("template.yaml");
        if template_yaml.exists() {
            if let Ok(content) = std::fs::read_to_string(&template_yaml) {
                if let Ok(template) = serde_yaml::from_str::<ProjectTemplate>(&content) {
                    templates.push(template);
                }
            }
        }
    }

    if json {
        let output = serde_json::to_string_pretty(&templates).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to serialize templates: {}", e))
        })?;
        println!("{}", output);
    } else {
        println!();
        println!("{}", style::header("Templates"));
        println!();

        if templates.is_empty() {
            println!("  {}", style::dim("No templates found"));
            println!("  Use 'ab folder template create <name> --from=<path>' to create one");
        } else {
            for template in &templates {
                println!(
                    "  {} {}",
                    style::highlight(&template.name),
                    if template.description.is_empty() {
                        "".to_string()
                    } else {
                        format!("- {}", style::dim(&template.description))
                    }
                );
                println!(
                    "    Files: {}  Git: {}  Beads: {}",
                    template.files.len(),
                    if template.init.git {
                        style::success("✓")
                    } else {
                        style::dim("-")
                    },
                    if template.init.beads {
                        style::success("✓")
                    } else {
                        style::dim("-")
                    }
                );
            }
        }
        println!();
    }

    Ok(())
}

/// Show template details
fn handle_template_show(name: &str) -> allbeads::Result<()> {
    let templates_dir = get_templates_dir()?;
    let template_yaml = templates_dir.join(name).join("template.yaml");

    if !template_yaml.exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Template '{}' not found",
            name
        )));
    }

    let content = std::fs::read_to_string(&template_yaml)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to read template: {}", e)))?;
    let template: ProjectTemplate = serde_yaml::from_str(&content)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to parse template: {}", e)))?;

    println!();
    println!("{}", style::header(&format!("Template: {}", template.name)));
    println!();

    if !template.description.is_empty() {
        println!("  Description: {}", template.description);
    }
    if let Some(ref created) = template.created {
        println!("  Created:     {}", created);
    }
    if let Some(ref source) = template.source {
        println!("  Source:      {}", style::path(source));
    }
    println!();

    println!("  Init:");
    println!(
        "    Git:    {}",
        if template.init.git { "Yes" } else { "No" }
    );
    println!(
        "    Beads:  {}",
        if template.init.beads { "Yes" } else { "No" }
    );
    println!(
        "    Claude: {}",
        if template.init.claude { "Yes" } else { "No" }
    );
    println!();

    println!("  Beads Config:");
    println!("    Mode: {}", template.beads.mode);
    if let Some(ref branch) = template.beads.sync_branch {
        println!("    Sync Branch: {}", branch);
    }
    println!();

    if let Some(ref persona) = template.config.persona {
        println!("  Config:");
        println!("    Persona: {}", persona);
    }
    println!();

    if !template.files.is_empty() {
        println!("  Files:");
        for f in &template.files {
            println!("    {} → {}", f.source, f.dest);
        }
        println!();
    }

    Ok(())
}

/// Delete a template
fn handle_template_delete(name: &str, yes: bool) -> allbeads::Result<()> {
    use dialoguer::Confirm;

    let templates_dir = get_templates_dir()?;
    let template_dir = templates_dir.join(name);

    if !template_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Template '{}' not found",
            name
        )));
    }

    println!();
    println!("{}", style::header("Delete Template"));
    println!();
    println!("  Template: {}", style::highlight(name));
    println!(
        "  Location: {}",
        style::path(&template_dir.display().to_string())
    );
    println!();

    let proceed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt("  Delete this template?")
            .default(false)
            .interact()
            .map_err(|e| allbeads::AllBeadsError::Config(format!("Input error: {}", e)))?
    };

    if !proceed {
        println!("  {}", style::dim("Deletion cancelled"));
        return Ok(());
    }

    std::fs::remove_dir_all(&template_dir).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to delete template: {}", e))
    })?;

    println!("  {} Template '{}' deleted", style::success("✓"), name);
    println!();

    Ok(())
}

/// Handle the interactive setup wizard for a folder
fn handle_folder_setup(
    path: &str,
    yes: bool,
    folders_file: &Path,
    context: &mut allbeads::context::Context,
) -> allbeads::Result<()> {
    use allbeads::context::{FolderConfig, FolderStatus, TrackedFolder};
    use dialoguer::{Confirm, Input, Select};

    // Resolve path
    let abs_path = std::fs::canonicalize(path).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
    })?;

    let current_status = detect_folder_status(&abs_path);

    println!();
    println!("{}", style::header("Folder Setup Wizard"));
    println!();
    println!("  Path:   {}", style::path(&abs_path.display().to_string()));
    println!(
        "  Status: {} {}",
        style::folder_status_indicator(current_status.short_name()),
        style::folder_status(current_status.short_name())
    );
    println!();

    let mut new_status = current_status;
    let mut folder_config = FolderConfig::default();

    // Step 1: Git Repository
    if current_status == FolderStatus::Dry {
        println!("{}", style::subheader("Step 1/5: Git Repository"));
        println!("  This folder is not a git repository.");
        println!();

        let init_git = if yes {
            true
        } else {
            Confirm::new()
                .with_prompt("  Initialize git?")
                .default(true)
                .interact()
                .unwrap_or(true)
        };

        if init_git {
            let branch_name: String = if yes {
                "main".to_string()
            } else {
                Input::new()
                    .with_prompt("  Default branch name")
                    .default("main".to_string())
                    .interact_text()
                    .unwrap_or_else(|_| "main".to_string())
            };

            // Run git init
            let output = std::process::Command::new("git")
                .args(["init", "-b", &branch_name])
                .current_dir(&abs_path)
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run git init: {}", e))
                })?;

            if output.status.success() {
                println!("  {} Initialized git repository", style::success("✓"));
                new_status = FolderStatus::Git;
            } else {
                println!("  {} Failed to initialize git", style::error("✗"));
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    println!("    {}", style::dim(stderr.trim()));
                }
            }
        } else {
            println!("  {} Skipping git initialization", style::dim("○"));
        }
        println!();
    }

    // Step 2: Beads Issue Tracker
    if new_status == FolderStatus::Git || current_status == FolderStatus::Git {
        println!("{}", style::subheader("Step 2/5: Beads Issue Tracker"));

        let init_beads = if yes {
            true
        } else {
            Confirm::new()
                .with_prompt("  Initialize beads?")
                .default(true)
                .interact()
                .unwrap_or(true)
        };

        if init_beads {
            // Get prefix
            let default_prefix = abs_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("proj")
                .to_lowercase()
                .chars()
                .take(10)
                .collect::<String>();

            let prefix: String = if yes {
                default_prefix
            } else {
                Input::new()
                    .with_prompt("  Issue prefix")
                    .default(default_prefix)
                    .interact_text()
                    .unwrap_or_else(|_| "proj".to_string())
            };

            folder_config.prefix = Some(prefix.clone());

            // Beads mode selection
            let mode_options = &[
                "Standard (SQLite + JSONL)",
                "JSONL-only",
                "Sync branch mode",
            ];
            let mode_idx = if yes {
                0
            } else {
                Select::new()
                    .with_prompt("  Beads mode")
                    .items(mode_options)
                    .default(0)
                    .interact()
                    .unwrap_or(0)
            };

            let bd_args = match mode_idx {
                1 => vec!["init", "--prefix", &prefix, "--no-db"],
                2 => vec!["init", "--prefix", &prefix, "--sync-branch"],
                _ => vec!["init", "--prefix", &prefix],
            };

            // Run bd init
            let output = std::process::Command::new("bd")
                .args(&bd_args)
                .current_dir(&abs_path)
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run bd init: {}", e))
                })?;

            if output.status.success() {
                println!(
                    "  {} Initialized beads with prefix '{}'",
                    style::success("✓"),
                    prefix
                );
                new_status = FolderStatus::Beads;
            } else {
                println!("  {} Failed to initialize beads", style::error("✗"));
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    println!("    {}", style::dim(stderr.trim()));
                }
            }
        } else {
            println!("  {} Skipping beads initialization", style::dim("○"));
        }
        println!();
    }

    // Step 3: Language & Project Type
    println!("{}", style::subheader("Step 3/5: Language & Project Type"));
    let detected = detect_project_info(&abs_path);

    if !detected.languages.is_empty() {
        let lang_names: Vec<&str> = detected
            .languages
            .iter()
            .map(|l| match l {
                allbeads::context::Language::Rust => "Rust",
                allbeads::context::Language::TypeScript => "TypeScript",
                allbeads::context::Language::JavaScript => "JavaScript",
                allbeads::context::Language::Python => "Python",
                allbeads::context::Language::Go => "Go",
                allbeads::context::Language::Java => "Java",
                allbeads::context::Language::Ruby => "Ruby",
                _ => "Other",
            })
            .collect();
        println!("  Detected: {}", lang_names.join(", "));
    } else {
        println!("  No languages detected");
    }

    if detected.is_monorepo {
        println!("  {} Monorepo detected", style::dim("○"));
    }

    println!("  {} Configuration saved", style::success("✓"));
    println!();

    // Step 4: Agent Integration
    println!("{}", style::subheader("Step 4/5: Agent Integration"));

    let personas = &[
        "General",
        "Security Specialist",
        "Frontend Developer",
        "Backend Developer",
        "DevOps Engineer",
        "Data Engineer",
    ];

    let persona_idx = if yes {
        0
    } else {
        Select::new()
            .with_prompt("  Agent persona")
            .items(personas)
            .default(0)
            .interact()
            .unwrap_or(0)
    };

    let persona = personas[persona_idx].to_lowercase().replace(' ', "-");
    folder_config.persona = Some(persona.clone());

    // Check for CLAUDE.md
    let claude_md_exists = abs_path.join("CLAUDE.md").exists();
    if !claude_md_exists {
        let create_claude = if yes {
            true
        } else {
            Confirm::new()
                .with_prompt("  Initialize Claude Code? (create CLAUDE.md)")
                .default(true)
                .interact()
                .unwrap_or(true)
        };

        if create_claude {
            // Create basic CLAUDE.md
            let claude_content = format!(
                r#"# CLAUDE.md

Project configuration for Claude Code.

## Project Type

Persona: {}

## Commands

```bash
# Build
cargo build

# Test
cargo test

# Run
cargo run
```
"#,
                persona
            );

            std::fs::write(abs_path.join("CLAUDE.md"), claude_content).map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to create CLAUDE.md: {}", e))
            })?;
            println!("  {} Created CLAUDE.md", style::success("✓"));
        }
    } else {
        println!("  {} CLAUDE.md already exists", style::dim("○"));
    }
    println!();

    // Step 5: AllBeads Integration
    println!("{}", style::subheader("Step 5/5: AllBeads Integration"));

    let enable_sync = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt("  Enable automatic sync?")
            .default(true)
            .interact()
            .unwrap_or(true)
    };

    folder_config.sync_enabled = enable_sync;

    if enable_sync {
        new_status = FolderStatus::Configured;
        // TODO: Actually enable sync in sheriff daemon
        println!("  {} Sync enabled", style::success("✓"));
    }
    println!();

    // Update or add folder to context
    if let Some(folder) = context.get_folder_mut(&abs_path) {
        folder.status = new_status;
        folder.config = Some(folder_config.clone());
        folder.detected = detected;
    } else {
        let mut folder = TrackedFolder::new(&abs_path).with_status(new_status);
        folder.config = Some(folder_config.clone());
        folder.detected = detected;
        context.add_folder(folder);
    }

    // Save context
    if let Some(parent) = folders_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            allbeads::AllBeadsError::Config(format!("Failed to create config directory: {}", e))
        })?;
    }

    let yaml = serde_yaml::to_string(&context).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to serialize folders: {}", e))
    })?;
    std::fs::write(folders_file, yaml).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to write folders.yaml: {}", e))
    })?;

    // Summary
    println!("{}", style::subheader("Summary"));
    println!(
        "  Status: {} {} {} {} {}",
        style::folder_status_indicator(current_status.short_name()),
        current_status.short_name(),
        style::dim("→"),
        style::folder_status_indicator(new_status.short_name()),
        style::folder_status(new_status.short_name())
    );
    if let Some(ref prefix) = folder_config.prefix {
        println!("  Prefix: {}", prefix);
    }
    if let Some(ref persona) = folder_config.persona {
        println!("  Persona: {}", persona);
    }
    println!(
        "  Sync: {}",
        if folder_config.sync_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();
    println!("  {} Setup complete!", style::success("✓"));

    Ok(())
}

/// Handle the promote command
fn handle_folder_promote(
    path: &str,
    to: Option<&str>,
    yes: bool,
    folders_file: &Path,
    context: &mut allbeads::context::Context,
) -> allbeads::Result<()> {
    use allbeads::context::FolderStatus;
    use dialoguer::Confirm;

    // Resolve path
    let abs_path = std::fs::canonicalize(path).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to resolve path '{}': {}", path, e))
    })?;

    let current_status = detect_folder_status(&abs_path);

    // Determine target status
    let target_status = if let Some(target_str) = to {
        FolderStatus::parse(target_str).ok_or_else(|| {
            allbeads::AllBeadsError::Config(format!("Invalid target status: {}", target_str))
        })?
    } else {
        current_status.next().ok_or_else(|| {
            allbeads::AllBeadsError::Config("Already at maximum status (wet)".to_string())
        })?
    };

    if target_status <= current_status {
        return Err(allbeads::AllBeadsError::Config(format!(
            "Target status '{}' must be higher than current status '{}'",
            target_status.short_name(),
            current_status.short_name()
        )));
    }

    println!();
    println!("{}", style::header("Promote Folder"));
    println!();
    println!(
        "  Path:    {}",
        style::path(&abs_path.display().to_string())
    );
    println!(
        "  Current: {} {}",
        style::folder_status_indicator(current_status.short_name()),
        current_status.short_name()
    );
    println!(
        "  Target:  {} {}",
        style::folder_status_indicator(target_status.short_name()),
        style::folder_status(target_status.short_name())
    );
    println!();

    let proceed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt("  Proceed with promotion?")
            .default(true)
            .interact()
            .unwrap_or(false)
    };

    if !proceed {
        println!("  {} Cancelled", style::dim("○"));
        return Ok(());
    }

    // Promote through each level
    let mut status = current_status;
    while status < target_status {
        let next = status.next().unwrap();
        match next {
            FolderStatus::Git => {
                // Initialize git
                let output = std::process::Command::new("git")
                    .args(["init"])
                    .current_dir(&abs_path)
                    .output()
                    .map_err(|e| {
                        allbeads::AllBeadsError::Config(format!("Failed to run git init: {}", e))
                    })?;

                if output.status.success() {
                    println!("  {} Initialized git", style::success("✓"));
                    status = FolderStatus::Git;
                } else {
                    return Err(allbeads::AllBeadsError::Config(
                        "Failed to initialize git".to_string(),
                    ));
                }
            }
            FolderStatus::Beads => {
                // Initialize beads
                let output = std::process::Command::new("bd")
                    .args(["init"])
                    .current_dir(&abs_path)
                    .output()
                    .map_err(|e| {
                        allbeads::AllBeadsError::Config(format!("Failed to run bd init: {}", e))
                    })?;

                if output.status.success() {
                    println!("  {} Initialized beads", style::success("✓"));
                    status = FolderStatus::Beads;
                } else {
                    return Err(allbeads::AllBeadsError::Config(
                        "Failed to initialize beads".to_string(),
                    ));
                }
            }
            FolderStatus::Configured => {
                println!("  {} Marked as configured", style::success("✓"));
                status = FolderStatus::Configured;
            }
            FolderStatus::Wet => {
                println!("  {} Marked as fully integrated", style::success("✓"));
                status = FolderStatus::Wet;
            }
            _ => {}
        }
    }

    // Update folder in context
    if let Some(folder) = context.get_folder_mut(&abs_path) {
        folder.status = status;
    } else {
        let folder = allbeads::context::TrackedFolder::new(&abs_path).with_status(status);
        context.add_folder(folder);
    }

    // Save context
    let yaml = serde_yaml::to_string(&context).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to serialize folders: {}", e))
    })?;
    std::fs::write(folders_file, yaml).map_err(|e| {
        allbeads::AllBeadsError::Config(format!("Failed to write folders.yaml: {}", e))
    })?;

    println!();
    println!("  {} Promotion complete!", style::success("✓"));

    Ok(())
}

/// Detect project information (languages, frameworks, etc.)
fn detect_project_info(path: &Path) -> allbeads::context::DetectedInfo {
    use allbeads::context::{DetectedInfo, Language};

    let mut info = DetectedInfo::default();

    // Detect languages by file presence
    if path.join("Cargo.toml").exists() {
        info.languages.push(Language::Rust);
    }
    if path.join("package.json").exists() {
        info.languages.push(Language::JavaScript);
        // Check for TypeScript
        if path.join("tsconfig.json").exists() {
            info.languages.insert(0, Language::TypeScript);
        }
    }
    if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        info.languages.push(Language::Python);
    }
    if path.join("go.mod").exists() {
        info.languages.push(Language::Go);
    }
    if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        info.languages.push(Language::Java);
    }
    if path.join("Gemfile").exists() {
        info.languages.push(Language::Ruby);
    }

    // Detect monorepo
    if path.join("lerna.json").exists()
        || path.join("pnpm-workspace.yaml").exists()
        || path.join("nx.json").exists()
        || (path.join("packages").exists() && path.join("packages").is_dir())
    {
        info.is_monorepo = true;
    }

    // Detect agents
    info.has_claude = path.join("CLAUDE.md").exists();
    info.has_cursor = path.join(".cursorrules").exists();
    info.has_copilot = path.join(".github/copilot-instructions.md").exists();
    info.has_aider = path.join(".aider.conf.yml").exists();

    // Git remote
    if path.join(".git").exists() {
        if let Ok(output) = std::process::Command::new("git")
            .args([
                "-C",
                path.to_str().unwrap_or("."),
                "remote",
                "get-url",
                "origin",
            ])
            .output()
        {
            if output.status.success() {
                info.git_remote = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }
    }

    info
}

/// Detect the current status of a folder (Dry to Wet progression)
fn detect_folder_status(path: &Path) -> allbeads::context::FolderStatus {
    use allbeads::context::FolderStatus;

    // Check if .beads/ exists (implies git exists too)
    if path.join(".beads").exists() {
        // Check if configured in AllBeads
        // For now, we'll consider beads = beads status
        // TODO: Check for actual AllBeads config
        return FolderStatus::Beads;
    }

    // Check if .git exists
    if path.join(".git").exists() {
        return FolderStatus::Git;
    }

    FolderStatus::Dry
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
                    RequestPayload::new("Approve deployment to production?").with_options(vec![
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
                    let is_unread = msg.status == allbeads::mail::DeliveryStatus::Delivered;
                    let marker = if is_unread { "*" } else { " " };
                    let type_str = match &msg.message.message_type {
                        MessageType::Notify(_) => "[NOTIFY]",
                        MessageType::Request(_) => "[REQUEST]",
                        MessageType::Lock(_) => "[LOCK]",
                        MessageType::Unlock(_) => "[UNLOCK]",
                        MessageType::Broadcast(_) => "[BROADCAST]",
                        MessageType::Heartbeat(_) => "[HEARTBEAT]",
                        MessageType::Response(_) => "[RESPONSE]",
                        MessageType::AikiEvent(_) => "[AIKI]",
                    };
                    let summary = match &msg.message.message_type {
                        MessageType::Notify(n) => n.message.clone(),
                        MessageType::Request(r) => r.message.clone(),
                        MessageType::Lock(l) => format!("Lock: {}", l.path),
                        MessageType::Unlock(u) => format!("Unlock: {}", u.path),
                        MessageType::Broadcast(b) => b.message.clone(),
                        MessageType::Heartbeat(h) => format!("Status: {:?}", h.status),
                        MessageType::Response(r) => r
                            .message
                            .clone()
                            .unwrap_or_else(|| format!("{:?}", r.status)),
                        MessageType::AikiEvent(a) => {
                            format!("Review {:?} for bead {}", a.event, a.bead_id)
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

            println!(
                "Pulling issues from JIRA project {} with label '{}'...",
                project, label
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
                    let labels: Vec<_> =
                        issue.labels.nodes.iter().map(|l| l.name.as_str()).collect();
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

/// Handle swarm commands by wrapping bd swarm
fn handle_swarm_command(cmd: &SwarmCommands) -> allbeads::Result<()> {
    use std::process::Command;

    match cmd {
        SwarmCommands::Create {
            epic_id,
            coordinator,
            force,
        } => {
            let mut args = vec!["swarm", "create", epic_id];

            let coord_arg;
            if let Some(coord) = coordinator {
                coord_arg = format!("--coordinator={}", coord);
                args.push(&coord_arg);
            }

            if *force {
                args.push("--force");
            }

            let output = Command::new("bd").args(&args).output().map_err(|e| {
                allbeads::AllBeadsError::Config(format!("Failed to run bd swarm: {}", e))
            })?;

            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(allbeads::AllBeadsError::Config(format!(
                    "bd swarm create failed: {}",
                    stderr
                )));
            }
        }

        SwarmCommands::List => {
            let output = Command::new("bd")
                .args(["swarm", "list"])
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run bd swarm: {}", e))
                })?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    println!("No swarm molecules found.");
                    println!();
                    println!("Create one with: ab swarm create <epic-id>");
                } else {
                    print!("{}", stdout);
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(allbeads::AllBeadsError::Config(format!(
                    "bd swarm list failed: {}",
                    stderr
                )));
            }
        }

        SwarmCommands::Status => {
            let output = Command::new("bd")
                .args(["swarm", "status"])
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run bd swarm: {}", e))
                })?;

            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(allbeads::AllBeadsError::Config(format!(
                    "bd swarm status failed: {}",
                    stderr
                )));
            }
        }

        SwarmCommands::Validate { epic_id } => {
            let output = Command::new("bd")
                .args(["swarm", "validate", epic_id])
                .output()
                .map_err(|e| {
                    allbeads::AllBeadsError::Config(format!("Failed to run bd swarm: {}", e))
                })?;

            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(allbeads::AllBeadsError::Config(format!(
                    "bd swarm validate failed: {}",
                    stderr
                )));
            }
        }
    }

    Ok(())
}

// === Agent Integration Commands (Phase 7) ===

/// Handle the `info` command - show project info and status for AI agents
fn handle_info_command(graph: &allbeads::graph::FederatedGraph) -> allbeads::Result<()> {
    let stats = graph.stats();
    let ready_count = graph.ready_beads().len();

    println!();
    println!("{}", style::header("AllBeads Project Info"));
    println!();
    println!("{}", style::subheader("Summary"));
    println!();
    println!(
        "  Total beads:    {}",
        style::count_normal(stats.total_beads)
    );
    println!("  Open:           {}", style::count_ready(stats.open_beads));
    println!(
        "  In Progress:    {}",
        style::count_in_progress(stats.in_progress_beads)
    );
    println!(
        "  Blocked:        {}",
        style::count_blocked(stats.blocked_beads)
    );
    println!(
        "  Closed:         {}",
        style::dim(&stats.closed_beads.to_string())
    );
    println!("  Ready to work:  {}", style::count_ready(ready_count));
    println!();

    // Show contexts
    use std::collections::HashMap;
    let mut context_counts: HashMap<String, (usize, usize)> = HashMap::new();
    for bead in graph.beads.values() {
        for label in &bead.labels {
            if label.starts_with('@') {
                let entry = context_counts.entry(label.clone()).or_insert((0, 0));
                entry.0 += 1;
                if bead.status == Status::Open {
                    entry.1 += 1;
                }
                break;
            }
        }
    }

    if !context_counts.is_empty() {
        println!("{}", style::subheader("Contexts"));
        println!();
        let mut contexts: Vec<_> = context_counts.iter().collect();
        contexts.sort_by_key(|(ctx, _)| ctx.as_str());
        for (context, (total, open)) in contexts {
            println!(
                "  {}: {} beads ({} open)",
                style::path(context),
                total,
                style::count_ready(*open)
            );
        }
        println!();
    }

    // Show recent activity
    let mut recent: Vec<_> = graph.beads.values().collect();
    recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let recent: Vec<_> = recent.into_iter().take(5).collect();

    if !recent.is_empty() {
        println!("{}", style::subheader("Recent Activity"));
        println!();
        for bead in recent {
            println!(
                "  {} {} {}",
                style::status_indicator(format_status(bead.status)),
                style::issue_id(bead.id.as_str()),
                bead.title
            );
        }
        println!();
    }

    println!("{}", style::subheader("Quick Actions"));
    println!();
    println!("  {} View ready work:    ab ready", style::dim("○"));
    println!("  {} View blocked work:  ab blocked", style::dim("●"));
    println!("  {} Launch TUI:         ab tui", style::dim("□"));
    println!(
        "  {} Search beads:       ab search \"query\"",
        style::dim("?")
    );

    Ok(())
}

/// Handle the `prime` command - prime agent memory with project context
fn handle_prime_command(graph: &allbeads::graph::FederatedGraph) -> allbeads::Result<()> {
    println!("# AllBeads Context Priming");
    println!();
    println!("This command helps AI agents recover context about the project.");
    println!();

    // Project summary
    let stats = graph.stats();
    println!("## Project Status");
    println!();
    println!(
        "AllBeads is aggregating {} beads across {} contexts.",
        stats.total_beads,
        graph.rigs.len()
    );
    println!();

    // Active work
    let in_progress: Vec<_> = graph
        .beads
        .values()
        .filter(|b| b.status == Status::InProgress)
        .collect();

    if !in_progress.is_empty() {
        println!("## In-Progress Work");
        println!();
        for bead in &in_progress {
            println!("### {} - {}", bead.id.as_str(), bead.title);
            if let Some(ref desc) = bead.description {
                let short = if desc.len() > 200 {
                    format!("{}...", &desc[..200])
                } else {
                    desc.clone()
                };
                println!("{}", short);
            }
            println!();
        }
    }

    // Blocked work needing attention
    let blocked: Vec<_> = graph
        .beads
        .values()
        .filter(|b| {
            b.status == Status::Blocked || (!b.dependencies.is_empty() && b.status == Status::Open)
        })
        .take(5)
        .collect();

    if !blocked.is_empty() {
        println!("## Blocked Work (top 5)");
        println!();
        for bead in &blocked {
            println!(
                "- {}: {} (blocked by: {})",
                bead.id.as_str(),
                bead.title,
                bead.dependencies
                    .iter()
                    .map(|d| d.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!();
    }

    // Ready work
    let ready = graph.ready_beads();
    if !ready.is_empty() {
        println!("## Ready Work (top 10)");
        println!();
        for bead in ready.iter().take(10) {
            println!(
                "- [{}] {}: {}",
                format_priority(bead.priority),
                bead.id.as_str(),
                bead.title
            );
        }
        println!();
    }

    println!("## Commands");
    println!();
    println!("- `ab info` - Project overview");
    println!("- `ab ready` - Show work ready to start");
    println!("- `ab show <id>` - Show bead details");
    println!("- `ab tui` - Interactive dashboard");

    Ok(())
}

/// Handle the `quickstart` command - show quickstart guide
fn handle_quickstart_command() -> allbeads::Result<()> {
    println!("# AllBeads Quickstart Guide");
    println!();
    println!("AllBeads is a distributed protocol for agentic orchestration and communication.");
    println!("It aggregates beads (issues) from multiple git repositories into a unified view.");
    println!();
    println!("## Setup");
    println!();
    println!("1. Initialize AllBeads:");
    println!("   ```");
    println!("   ab init");
    println!("   ```");
    println!();
    println!("2. Add repositories (contexts):");
    println!("   ```");
    println!("   cd /path/to/repo && ab context add");
    println!("   ```");
    println!();
    println!("3. View aggregated beads:");
    println!("   ```");
    println!("   ab stats    # Summary");
    println!("   ab list     # All beads");
    println!("   ab tui      # Interactive dashboard");
    println!("   ```");
    println!();
    println!("## Essential Commands");
    println!();
    println!("### Viewing Work");
    println!("- `ab list` - List all beads");
    println!("- `ab ready` - Show unblocked work");
    println!("- `ab blocked` - Show blocked work");
    println!("- `ab show <id>` - Show bead details");
    println!("- `ab search \"query\"` - Search beads");
    println!();
    println!("### TUI Dashboard");
    println!("- `ab tui` - Launch interactive dashboard");
    println!("- Tab: Switch views (Kanban/Mail/Graph/Swarm)");
    println!("- j/k: Navigate up/down");
    println!("- Enter: Toggle detail view");
    println!("- q: Quit");
    println!();
    println!("### Agent Integration");
    println!("- `ab info` - Project status for agents");
    println!("- `ab prime` - Context recovery after compaction");
    println!("- `ab mail inbox` - View agent messages");
    println!();
    println!("## Learn More");
    println!();
    println!("- Documentation: See AGENTS.md in the repo");
    println!("- Demo: Run `ab help` for all commands");

    Ok(())
}

/// Handle the `setup` command - interactive setup wizard
fn handle_setup_command(config_path: &Option<String>) -> allbeads::Result<()> {
    let config_file = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        AllBeadsConfig::default_path()
    };

    println!("# AllBeads Setup Wizard");
    println!();

    // Check if already initialized
    if config_file.exists() {
        let config = AllBeadsConfig::load(&config_file)?;
        println!(
            "AllBeads is already configured at: {}",
            config_file.display()
        );
        println!();
        println!("Current configuration:");
        println!("  Contexts: {}", config.contexts.len());
        for ctx in &config.contexts {
            println!("    - {}: {}", ctx.name, ctx.url);
        }
        println!();
        println!("To add more contexts:");
        println!("  cd /path/to/repo && ab context add");
        println!();
        println!("To remove a context:");
        println!("  ab context remove <name>");
        println!();
        println!("To start fresh, delete the config file:");
        println!("  rm {}", config_file.display());
        return Ok(());
    }

    // Not initialized - provide setup instructions
    println!("AllBeads is not yet configured.");
    println!();
    println!("## Step 1: Initialize");
    println!();
    println!("Run the following command to create the configuration:");
    println!("  ab init");
    println!();
    println!("Or clone an existing Boss repository:");
    println!("  ab init --remote git@github.com:org/boss-repo.git");
    println!();
    println!("## Step 2: Add Contexts");
    println!();
    println!("Navigate to each repository you want to track and add it:");
    println!("  cd /path/to/repo && ab context add");
    println!();
    println!("The repository must have a .beads/ directory (created by the beads issue tracker).");
    println!();
    println!("## Step 3: View Beads");
    println!();
    println!("Once configured, you can view aggregated beads:");
    println!("  ab stats    # Summary");
    println!("  ab list     # All beads");
    println!("  ab tui      # Interactive dashboard");

    Ok(())
}

/// Handle the `onboard` command - onboard a repository into AllBeads
/// Handle repository onboarding through the 9-stage workflow.
///
/// Implements SPEC-onboarding.md:
/// 1. Discovery/Selection
/// 2. Clone (if needed)
/// 3. Initialize Beads (via bd init)
/// 4. Populate Issues
/// 5. Add Skills (marketplace config)
/// 6. Integrations (optional)
/// 7. CI/CD Detection
/// 8. Add to AllBeads Config
/// 9. Summary
#[allow(clippy::too_many_arguments)]
fn handle_onboard_repository(
    target: &str,
    non_interactive: bool,
    skip_clone: bool,
    skip_beads: bool,
    skip_skills: bool,
    _skip_hooks: bool, // Handled by bd init
    skip_issues: bool,
    context_name: Option<&str>,
    custom_path: Option<&str>,
    config: &AllBeadsConfig,
) -> allbeads::Result<()> {
    use allbeads::onboarding::repository;

    println!("🚀 AllBeads Repository Onboarding");
    println!();

    // Stage 1: Discovery & Validation
    println!("Stage 1: Discovery");
    let repo_info = repository::discover_repository(target, custom_path, config)?;
    println!("  Repository: {}", repo_info.name);
    println!("  Path: {}", repo_info.path.display());
    if let Some(ref url) = repo_info.url {
        println!("  URL: {}", url);
    }
    if let Some(ref org) = repo_info.organization {
        println!("  Organization: {}", org);
    }
    println!("  Exists locally: {}", repo_info.exists_locally);
    println!();

    // Stage 2: Clone (if needed)
    if !skip_clone && !repo_info.exists_locally {
        if let Some(ref url) = repo_info.url {
            println!("Stage 2: Clone");
            repository::clone_repository(url, &repo_info.path, non_interactive)?;
            println!();
        }
    } else if repo_info.exists_locally {
        println!("Stage 2: Clone (skipped - already exists)");
        println!();
    }

    // Stage 3: Initialize Beads (bd init)
    if !skip_beads {
        println!("Stage 3: Initialize Beads");
        repository::initialize_beads(&repo_info.path, non_interactive)?;
        println!();
    } else {
        println!("Stage 3: Initialize Beads (skipped)");
        println!();
    }

    // Stage 4: Populate Issues (create beads for missing agent configs)
    if !skip_beads && !skip_issues {
        println!("Stage 4: Populate Issues");
        let issues = repository::populate_onboarding_issues(&repo_info.path)?;
        if issues.is_empty() {
            println!("  ✓ All agent configurations already exist");
        } else {
            println!("  Found {} missing agent configurations:", issues.len());
            for issue in &issues {
                println!("    • {}", issue.title);
            }
            let (epic_id, created) = repository::create_onboarding_beads(&repo_info.path, &issues)?;
            if let Some(epic) = epic_id {
                println!("  ✓ Created 'Agent Onboarding' epic: {}", epic);
            }
            println!("  ✓ Created {} onboarding tasks", created);
        }
        println!();
    } else if skip_issues {
        println!("Stage 4: Populate Issues (skipped)");
        println!();
    } else {
        println!("Stage 4: Populate Issues (skipped - beads not initialized)");
        println!();
    }

    // Stage 5: Configure Skills
    if !skip_skills {
        println!("Stage 5: Configure Skills");
        repository::configure_skills(&repo_info.path)?;
        println!();
    } else {
        println!("Stage 5: Configure Skills (skipped)");
        println!();
    }

    // Stage 6: Git Hooks (handled by bd init)
    println!("Stage 6: Git Hooks (handled by bd init)");
    println!();

    // Stage 7: Detect CI/CD
    println!("Stage 7: CI/CD Detection");
    repository::detect_ci_cd(&repo_info.path)?;
    println!();

    // Stage 8: Add to AllBeads Config
    let ctx_name = context_name.unwrap_or(&repo_info.name);
    println!("Stage 8: Add to AllBeads Config");
    if repo_info.url.is_some() {
        repository::add_to_allbeads_config(ctx_name, &repo_info, config)?;
    } else {
        println!("  Skipping config update (local path, no URL)");
    }
    println!();

    // Stage 9: Commit & Push Changes
    println!("Stage 9: Commit & Push Changes");
    repository::commit_and_push_onboarding(&repo_info.path, non_interactive)?;
    println!();

    // Stage 10: Summary
    println!("Stage 10: Summary");
    println!("═══════════════════════════════════════════════════════════════");
    repository::print_onboarding_summary(&repo_info, ctx_name, skip_beads, skip_skills);

    Ok(())
}

/// Handle the `human` command - communication channel to human operator
fn handle_human_command(message: &Option<String>) -> allbeads::Result<()> {
    use allbeads::mail::{Address, Message, MessageType, NotifyPayload, Postmaster, Severity};

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

    if let Some(msg) = message {
        // Send message to human
        let human = Address::human();
        let agent_addr = Address::new("agent", &project_id)?;

        let mail = Message::new(
            agent_addr,
            human,
            MessageType::Notify(NotifyPayload::new(msg).with_severity(Severity::Info)),
        );

        postmaster.send(mail)?;
        println!("Message sent to human operator.");
        println!();
        println!("The human can view it with:");
        println!("  ab mail inbox");
        println!("  ab tui  (Mail tab)");
    } else {
        // Interactive mode info
        println!("# Human Communication Channel");
        println!();
        println!("Use this command to send messages to the human operator.");
        println!();
        println!("## Usage");
        println!();
        println!("Send a message:");
        println!("  ab human \"Your message here\"");
        println!();
        println!("The human will see it in:");
        println!("  ab mail inbox");
        println!("  ab tui  (Mail tab)");
        println!();
        println!("## Example Messages");
        println!();
        println!("- Status update: `ab human \"Completed refactoring auth module\"`");
        println!("- Request help: `ab human \"Need clarification on API design\"`");
        println!("- Report issue: `ab human \"Found potential security issue in login flow\"`");
    }

    Ok(())
}

fn handle_check_command(
    strict: bool,
    policy: Option<&str>,
    fix: bool,
    pre_commit: bool,
    bead: Option<&str>,
    format: &str,
) -> allbeads::Result<()> {
    use allbeads::governance::{load_policies_for_context, PolicyChecker};
    use allbeads::graph::FederatedGraph;
    use allbeads::storage::issue_to_bead;
    use beads::Beads;
    use std::process;

    // Load policies from .beads/policies.yaml
    let policies = load_policies_for_context(".");

    if policies.is_empty() {
        if !pre_commit {
            eprintln!(
                "No policies configured. Create .beads/policies.yaml to enable governance checks."
            );
        }
        return Ok(());
    }

    // Create policy checker and add policies
    let mut checker = PolicyChecker::new();
    for p in policies {
        // Filter by specific policy if requested
        if let Some(policy_name) = policy {
            if p.name != policy_name {
                continue;
            }
        }
        checker.add_policy(p);
    }

    // Load beads from current directory
    let beads_path = std::env::current_dir()?.join(".beads");
    if !beads_path.exists() {
        return Err(allbeads::AllBeadsError::Config(
            "Not in a beads repository. Run 'bd init' first.".to_string(),
        ));
    }

    let bd = Beads::with_workdir(&beads_path);
    let beads_list = bd
        .list(None, None)
        .map_err(|e| allbeads::AllBeadsError::Config(format!("Failed to list beads: {}", e)))?;

    // Convert to graph for checking
    let mut graph = FederatedGraph::new();
    for bead_issue in beads_list {
        let graph_bead = issue_to_bead(bead_issue)?;

        // Filter by specific bead if requested
        if let Some(bead_id) = bead {
            if graph_bead.id.to_string() != bead_id {
                continue;
            }
        }

        graph.add_bead(graph_bead);
    }

    // Run checks
    let results = checker.check_graph(&graph);

    // Count violations
    let violations: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    let has_violations = !violations.is_empty();

    // Output results
    if pre_commit {
        // Pre-commit mode: only output if there are violations
        if has_violations {
            eprintln!("Error: Policy violations detected\n");
            for result in &violations {
                eprintln!("✗ {}: {}", result.policy_name, result.message);
                if !result.affected_beads.is_empty() {
                    eprintln!("  Affected beads: {}", result.affected_beads.join(", "));
                }
            }
            eprintln!("\nCommit blocked. Fix violations and try again.");
            if fix {
                eprintln!("\nRun 'bd check --fix' for suggestions.");
            }
            process::exit(1);
        }
        // No output if passing
    } else {
        // Normal mode: show all results
        match format {
            "json" | "yaml" => {
                // Format results for serialization
                let output: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "policy_name": r.policy_name,
                            "passed": r.passed,
                            "message": r.message,
                            "affected_beads": r.affected_beads,
                            "timestamp": r.timestamp,
                        })
                    })
                    .collect();

                if format == "json" {
                    let json = serde_json::to_string_pretty(&output)?;
                    println!("{}", json);
                } else {
                    let yaml = serde_yaml::to_string(&output)?;
                    println!("{}", yaml);
                }
            }
            _ => {
                println!("Checking governance policies...\n");

                let mut passed = 0;
                let mut failed = 0;

                for result in &results {
                    if result.passed {
                        println!("✓ {}: PASS", result.policy_name);
                        passed += 1;
                    } else {
                        println!("✗ {}: FAIL", result.policy_name);
                        println!("    {}", result.message);
                        if !result.affected_beads.is_empty() {
                            println!("    Affected beads: {}", result.affected_beads.join(", "));
                        }
                        failed += 1;
                    }
                }

                println!("\nSummary: {} passed, {} failed", passed, failed);

                if fix && has_violations {
                    println!("\nResolution suggestions:");
                    println!("  1. Review violations above");
                    println!("  2. Fix issues or adjust policy in .beads/policies.yaml");
                    println!("  3. Run 'bd check' again to verify");
                }
            }
        }

        // Exit with non-zero if strict mode and violations exist
        if strict && has_violations {
            process::exit(1);
        }
    }

    Ok(())
}

fn handle_hooks_command(cmd: &HooksCommands) -> allbeads::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let git_hooks_dir = PathBuf::from(".git/hooks");

    if !git_hooks_dir.exists() {
        return Err(allbeads::AllBeadsError::Config(
            "Not in a git repository. No .git/hooks directory found.".to_string(),
        ));
    }

    match cmd {
        HooksCommands::Install { hook, all, dry_run } => {
            let hooks_to_install = if *all {
                vec!["pre-commit", "commit-msg", "post-commit", "pre-push"]
            } else if let Some(h) = hook {
                vec![h.as_str()]
            } else {
                vec!["pre-commit"]
            };

            println!("Installing git hooks...\n");

            for hook_name in hooks_to_install {
                let hook_path = git_hooks_dir.join(hook_name);
                let hook_content = get_hook_template(hook_name);

                if *dry_run {
                    println!("Would install: {}", hook_path.display());
                    continue;
                }

                fs::write(&hook_path, hook_content)?;

                // Make executable (Unix only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&hook_path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&hook_path, perms)?;
                }

                println!("  ✓ Created {}", hook_path.display());
            }

            if !dry_run {
                println!("\nHooks installed successfully.");
                println!("\nTest the hooks:");
                println!("  bd hooks test");
            }

            Ok(())
        }

        HooksCommands::Uninstall { hook, all } => {
            let hooks_to_remove = if *all {
                vec!["pre-commit", "commit-msg", "post-commit", "pre-push"]
            } else if let Some(h) = hook {
                vec![h.as_str()]
            } else {
                return Err(allbeads::AllBeadsError::Config(
                    "Specify --hook=<name> or --all".to_string(),
                ));
            };

            println!("Uninstalling git hooks...\n");

            for hook_name in hooks_to_remove {
                let hook_path = git_hooks_dir.join(hook_name);

                if hook_path.exists() {
                    fs::remove_file(&hook_path)?;
                    println!("  ✓ Removed {}", hook_path.display());
                } else {
                    println!("  ⊗ Not installed: {}", hook_name);
                }
            }

            println!("\nHooks uninstalled.");
            Ok(())
        }

        HooksCommands::List => {
            println!("Installed hooks:\n");

            let all_hooks = vec!["pre-commit", "commit-msg", "post-commit", "pre-push"];
            let mut found_any = false;

            for hook_name in all_hooks {
                let hook_path = git_hooks_dir.join(hook_name);
                if hook_path.exists() {
                    println!("  ✓ {}", hook_name);
                    found_any = true;
                }
            }

            if !found_any {
                println!("  (none)");
                println!("\nInstall hooks with: bd hooks install");
            }

            Ok(())
        }

        HooksCommands::Test { hook } => {
            let hook_name = hook.as_deref().unwrap_or("pre-commit");
            let hook_path = git_hooks_dir.join(hook_name);

            if !hook_path.exists() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Hook '{}' not installed. Run 'bd hooks install' first.",
                    hook_name
                )));
            }

            println!("Testing {} hook...\n", hook_name);

            // Run the hook script
            let output = std::process::Command::new(&hook_path).output()?;

            if output.status.success() {
                println!("✓ Hook passed");
            } else {
                println!("✗ Hook failed");
                if !output.stderr.is_empty() {
                    eprintln!("\nError output:");
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                }
            }

            Ok(())
        }

        HooksCommands::Status => {
            println!("Hook installation status:\n");

            let all_hooks = vec!["pre-commit", "commit-msg", "post-commit", "pre-push"];

            for hook_name in all_hooks {
                let hook_path = git_hooks_dir.join(hook_name);
                if hook_path.exists() {
                    println!("  ✓ {} - installed", hook_name);
                } else {
                    println!("  ✗ {} - not installed", hook_name);
                }
            }

            Ok(())
        }
    }
}

fn handle_aiki_command(cmd: &AikiCommands) -> allbeads::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let env_file = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".config/allbeads/env");

    match cmd {
        AikiCommands::Activate { bead_id } => {
            // Create parent directory if it doesn't exist
            if let Some(parent) = env_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write the environment variable to the file
            fs::write(
                &env_file,
                format!("export AB_ACTIVE_BEAD=\"{}\"\n", bead_id),
            )?;

            println!("✓ Activated bead: {}", bead_id);
            println!("\nTo use this in your current shell, run:");
            println!("  source {}", env_file.display());
            println!("\nOr eval the output:");
            println!("  eval \"$(ab aiki hook-init)\"");
            println!("\nTo make this automatic, add to your shell rc file:");
            println!("  eval \"$(ab aiki hook-init)\"");

            Ok(())
        }

        AikiCommands::Deactivate => {
            if env_file.exists() {
                fs::remove_file(&env_file)?;
                println!("✓ Deactivated bead");
                println!("\nIn your current shell, run:");
                println!("  unset AB_ACTIVE_BEAD");
            } else {
                println!("No active bead");
            }

            Ok(())
        }

        AikiCommands::Status => {
            if let Ok(content) = fs::read_to_string(&env_file) {
                // Extract the bead ID from the file
                if let Some(line) = content.lines().next() {
                    if let Some(id) = line
                        .strip_prefix("export AB_ACTIVE_BEAD=\"")
                        .and_then(|s| s.strip_suffix("\""))
                    {
                        println!("Active bead: {}", id);
                        println!("Env file: {}", env_file.display());
                        return Ok(());
                    }
                }
            }

            println!("No active bead");
            println!("\nActivate a bead with:");
            println!("  ab aiki activate <bead-id>");

            Ok(())
        }

        AikiCommands::HookInit => {
            // Output shell code that sources the env file
            println!("# AllBeads Aiki integration");
            println!("# Add this to your ~/.bashrc or ~/.zshrc:");
            println!("# eval \"$(ab aiki hook-init)\"");
            println!();
            println!("ALLBEADS_ENV_FILE=\"{}\"", env_file.display());
            println!("if [ -f \"$ALLBEADS_ENV_FILE\" ]; then");
            println!("    source \"$ALLBEADS_ENV_FILE\"");
            println!("fi");

            Ok(())
        }

        AikiCommands::Link { bead_id, task_id } => {
            // Load beads from JSONL, add the task link, save back
            use allbeads::storage::{read_beads, write_beads};

            let beads_path = std::path::PathBuf::from(".beads");
            if !beads_path.exists() {
                return Err(allbeads::AllBeadsError::Config(
                    "Not in a beads repository. Run 'bd init' first.".to_string(),
                ));
            }

            let issues_file = beads_path.join("issues.jsonl");
            let mut beads = read_beads(&issues_file)?;

            // Find the bead
            let bead = beads
                .iter_mut()
                .find(|b| b.id.as_str() == bead_id)
                .ok_or_else(|| {
                    allbeads::AllBeadsError::Config(format!("Bead {} not found", bead_id))
                })?;

            if bead.has_aiki_task(task_id) {
                println!("Task {} is already linked to bead {}", task_id, bead_id);
                return Ok(());
            }

            bead.add_aiki_task(task_id.clone());

            // Write all beads back
            write_beads(&issues_file, &beads)?;

            println!("✓ Linked task {} to bead {}", task_id, bead_id);
            Ok(())
        }

        AikiCommands::Unlink { bead_id, task_id } => {
            // Load beads from JSONL, remove the task link, save back
            use allbeads::storage::{read_beads, write_beads};

            let beads_path = std::path::PathBuf::from(".beads");
            if !beads_path.exists() {
                return Err(allbeads::AllBeadsError::Config(
                    "Not in a beads repository. Run 'bd init' first.".to_string(),
                ));
            }

            let issues_file = beads_path.join("issues.jsonl");
            let mut beads = read_beads(&issues_file)?;

            // Find the bead
            let bead = beads
                .iter_mut()
                .find(|b| b.id.as_str() == bead_id)
                .ok_or_else(|| {
                    allbeads::AllBeadsError::Config(format!("Bead {} not found", bead_id))
                })?;

            if !bead.remove_aiki_task(task_id) {
                println!("Task {} was not linked to bead {}", task_id, bead_id);
                return Ok(());
            }

            // Write all beads back
            write_beads(&issues_file, &beads)?;

            println!("✓ Unlinked task {} from bead {}", task_id, bead_id);
            Ok(())
        }

        AikiCommands::Tasks { bead_id } => {
            // Load beads and display linked tasks
            use allbeads::storage::read_beads;

            let beads_path = std::path::PathBuf::from(".beads");
            if !beads_path.exists() {
                return Err(allbeads::AllBeadsError::Config(
                    "Not in a beads repository. Run 'bd init' first.".to_string(),
                ));
            }

            let issues_file = beads_path.join("issues.jsonl");
            let beads = read_beads(&issues_file)?;

            // Find the bead
            let bead = beads
                .iter()
                .find(|b| b.id.as_str() == bead_id)
                .ok_or_else(|| {
                    allbeads::AllBeadsError::Config(format!("Bead {} not found", bead_id))
                })?;

            if bead.aiki_tasks.is_empty() {
                println!("No Aiki tasks linked to bead {}", bead_id);
                println!("\nLink a task with:");
                println!("  ab aiki link {} <task-id>", bead_id);
                return Ok(());
            }

            println!("Aiki tasks linked to {}:", bead_id);
            for task_id in &bead.aiki_tasks {
                println!("  {}", task_id);
            }
            println!("\nTotal: {} task(s)", bead.aiki_tasks.len());

            Ok(())
        }
    }
}

fn get_hook_template(hook_name: &str) -> String {
    match hook_name {
        "pre-commit" => r#"#!/bin/sh
# AllBeads pre-commit hook for policy enforcement
# Auto-generated by ab hooks install

# Find AllBeads binary (prefer cargo for development)
if [ -f "Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    # Development mode: use cargo run
    ALLBEADS="cargo run --quiet --"
elif command -v allbeads >/dev/null 2>&1; then
    # Production: use installed allbeads binary
    ALLBEADS="allbeads"
else
    echo "Error: AllBeads not found. Install with 'cargo install allbeads' or run from repo."
    exit 1
fi

# Run policy checks in pre-commit mode
$ALLBEADS check --pre-commit --strict

exit $?
"#
        .to_string(),

        "commit-msg" => r#"#!/bin/sh
# AllBeads commit-msg hook for bead reference validation
# Auto-generated by ab hooks install

# TODO: Validate bead references in commit message
# For now, just pass through
exit 0
"#
        .to_string(),

        "post-commit" => r#"#!/bin/sh
# AllBeads post-commit hook for metadata updates
# Auto-generated by ab hooks install

# TODO: Update bead metadata with commit info
# For now, just pass through
exit 0
"#
        .to_string(),

        "pre-push" => r#"#!/bin/sh
# AllBeads pre-push hook for full validation
# Auto-generated by ab hooks install

# Find AllBeads binary (prefer cargo for development)
if [ -f "Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    # Development mode: use cargo run
    ALLBEADS="cargo run --quiet --"
elif command -v allbeads >/dev/null 2>&1; then
    # Production: use installed allbeads binary
    ALLBEADS="allbeads"
else
    echo "Error: AllBeads not found. Install with 'cargo install allbeads' or run from repo."
    exit 1
fi

# Run full policy checks before push
$ALLBEADS check --strict

exit $?
"#
        .to_string(),

        _ => {
            format!("#!/bin/sh\n# Unknown hook: {}\nexit 0\n", hook_name)
        }
    }
}

/// Handle the `agents` command - detect and manage AI agents
fn handle_agents_command(
    cmd: &AgentsCommands,
    config_path: &Option<String>,
) -> allbeads::Result<()> {
    use allbeads::governance::{detect_agents, print_agent_scan, AgentType};

    match cmd {
        AgentsCommands::Detect { path, json } => {
            let repo_path = std::path::Path::new(path);

            if !repo_path.exists() {
                return Err(allbeads::AllBeadsError::Config(format!(
                    "Path does not exist: {}",
                    path
                )));
            }

            let result = detect_agents(repo_path);

            if *json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("AI Agent Detection: {}", path);
                println!("═══════════════════════════════════════════════════════════════");
                print_agent_scan(&result);

                if !result.has_agents() {
                    println!();
                    println!("Tip: Add CLAUDE.md to configure Claude Code for this repository");
                }
            }

            Ok(())
        }

        AgentsCommands::List {
            agent,
            high_confidence,
            json,
        } => {
            // Load config to get contexts
            let config = if let Some(config_path) = config_path {
                AllBeadsConfig::load(config_path)?
            } else {
                AllBeadsConfig::load_default()?
            };

            let mut all_detections: Vec<(String, allbeads::governance::AgentScanResult)> =
                Vec::new();

            for context in &config.contexts {
                if let Some(ref path) = context.path {
                    if path.exists() {
                        let result = detect_agents(path);
                        if result.has_agents() {
                            all_detections.push((context.name.clone(), result));
                        }
                    }
                }
            }

            // Filter by agent type if specified
            let agent_filter: Option<AgentType> =
                agent
                    .as_ref()
                    .and_then(|a| match a.to_lowercase().as_str() {
                        "claude" => Some(AgentType::Claude),
                        "copilot" => Some(AgentType::Copilot),
                        "cursor" => Some(AgentType::Cursor),
                        "aider" => Some(AgentType::Aider),
                        "cody" => Some(AgentType::Cody),
                        "continue" => Some(AgentType::Continue),
                        "windsurf" => Some(AgentType::Windsurf),
                        "amazonq" => Some(AgentType::AmazonQ),
                        "kiro" => Some(AgentType::Kiro),
                        "opencode" => Some(AgentType::OpenCode),
                        "droid" | "factory" => Some(AgentType::Droid),
                        "codex" => Some(AgentType::Codex),
                        "gemini" => Some(AgentType::Gemini),
                        "agent" | "generic" => Some(AgentType::GenericAgent),
                        _ => None,
                    });

            if *json {
                let filtered: Vec<_> = all_detections
                    .iter()
                    .map(|(name, result)| {
                        let detections: Vec<_> = result
                            .detections
                            .iter()
                            .filter(|d| {
                                let agent_match = agent_filter.is_none_or(|af| d.agent == af);
                                let conf_match = !*high_confidence
                                    || d.confidence
                                        == allbeads::governance::DetectionConfidence::High;
                                agent_match && conf_match
                            })
                            .collect();
                        serde_json::json!({
                            "context": name,
                            "detections": detections
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&filtered)?);
            } else {
                println!("AI Agents Across Contexts");
                println!("═══════════════════════════════════════════════════════════════");

                if all_detections.is_empty() {
                    println!("  No AI agents detected in any context");
                    return Ok(());
                }

                for (context_name, result) in &all_detections {
                    let filtered: Vec<_> = result
                        .detections
                        .iter()
                        .filter(|d| {
                            let agent_match = agent_filter.is_none_or(|af| d.agent == af);
                            let conf_match = !*high_confidence
                                || d.confidence == allbeads::governance::DetectionConfidence::High;
                            agent_match && conf_match
                        })
                        .collect();

                    if !filtered.is_empty() {
                        println!();
                        println!("@{}", context_name);
                        for detection in filtered {
                            let conf = detection.confidence.symbol();
                            let agent = detection.agent.name();
                            println!("  [{conf}] {agent}");
                        }
                    }
                }
            }

            Ok(())
        }

        AgentsCommands::Summary => {
            // Load config to get contexts
            let config = AllBeadsConfig::load_default()?;

            let mut agent_counts: std::collections::HashMap<AgentType, usize> =
                std::collections::HashMap::new();
            let mut total_repos = 0;
            let mut repos_with_agents = 0;

            for context in &config.contexts {
                if let Some(ref path) = context.path {
                    if path.exists() {
                        total_repos += 1;
                        let result = detect_agents(path);
                        if result.has_agents() {
                            repos_with_agents += 1;
                        }
                        for detection in &result.detections {
                            *agent_counts.entry(detection.agent).or_insert(0) += 1;
                        }
                    }
                }
            }

            println!("AI Agent Adoption Summary");
            println!("═══════════════════════════════════════════════════════════════");
            println!();
            println!(
                "Repositories: {}/{} with agents ({:.0}%)",
                repos_with_agents,
                total_repos,
                if total_repos > 0 {
                    (repos_with_agents as f64 / total_repos as f64) * 100.0
                } else {
                    0.0
                }
            );
            println!();
            println!("Agent Distribution:");

            let mut counts: Vec<_> = agent_counts.iter().collect();
            counts.sort_by(|a, b| b.1.cmp(a.1));

            for (agent, count) in counts {
                println!("  {}: {} repos", agent.name(), count);
            }

            if agent_counts.is_empty() {
                println!("  No agents detected");
            }

            Ok(())
        }

        AgentsCommands::Track { context, path } => {
            use allbeads::governance::UsageStorage;

            let repo_path = std::path::Path::new(path);
            let context_name = context.clone().unwrap_or_else(|| {
                repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            });

            let result = detect_agents(repo_path);
            let storage = UsageStorage::open_default()?;

            storage.record_scan(&context_name, path, &result)?;

            println!(
                "✓ Recorded agent scan for {} ({} agents detected)",
                context_name,
                result.detections.len()
            );
            if result.has_agents() {
                for detection in &result.detections {
                    println!("  • {}", detection.agent.name());
                }
            }

            Ok(())
        }

        AgentsCommands::Stats { days, json } => {
            use allbeads::governance::{print_usage_stats, UsageStorage};

            let storage = UsageStorage::open_default()?;
            let stats = storage.get_stats(*days)?;
            let trends = storage.get_trends(*days)?;

            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "period_days": days,
                        "total_scans": stats.total_scans,
                        "repos_with_agents": stats.repos_with_agents,
                        "repos_without_agents": stats.repos_without_agents,
                        "adoption_rate": stats.adoption_rate,
                        "agent_counts": stats.agent_counts,
                        "trends": trends
                    })
                );
            } else if stats.total_scans == 0 {
                println!("No usage data recorded yet.");
                println!();
                println!("Run 'ab agents track' to record agent scans.");
            } else {
                print_usage_stats(&stats, &trends, *days);
            }

            Ok(())
        }
    }
}

/// Handle the `governance` command - check and enforce policies
fn handle_governance_command(
    cmd: &GovernanceCommands,
    _config_path: &Option<String>,
) -> allbeads::Result<()> {
    use allbeads::governance::{
        check_all_policies, check_policy, default_policies_path, Enforcement, PolicyExemption,
        RepoPolicyConfig,
    };

    match cmd {
        GovernanceCommands::Check {
            repo,
            policy,
            advisory_only,
            strict,
            override_reason,
            json,
        } => {
            // Determine repo path and name
            let repo_path = match repo {
                Some(r) => std::path::PathBuf::from(r),
                None => std::env::current_dir()?,
            };
            let repo_name = repo_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Load policy config
            let policy_config_path = default_policies_path();
            let config = RepoPolicyConfig::load(&policy_config_path)?;

            if !*json {
                println!("Governance Policy Check");
                println!("═══════════════════════════════════════════════════════════════");

                if *advisory_only {
                    println!("Mode: Advisory (never blocks)");
                } else if *strict {
                    println!("Mode: Strict (soft mandatory treated as hard)");
                } else {
                    println!("Mode: Normal");
                }

                println!("Repository: {} ({})", repo_name, repo_path.display());
                if let Some(ref p) = policy {
                    println!("Policy: {}", p);
                }
                if let Some(ref reason) = override_reason {
                    println!("Override: {}", reason);
                }
                println!();
            }

            // Run policy checks
            let results = if let Some(ref policy_name) = policy {
                // Check specific policy
                if let Some(p) = config.policies.get(policy_name) {
                    vec![check_policy(&repo_path, p)]
                } else {
                    if !*json {
                        println!("Policy '{}' not found", policy_name);
                    }
                    vec![]
                }
            } else {
                // Check all policies
                check_all_policies(&repo_path, &repo_name, &config)
            };

            // Process results
            let mut passed = 0;
            let mut warnings = 0;
            let mut failures = 0;
            let mut blocked = false;

            for result in &results {
                let status_icon = if result.passed {
                    passed += 1;
                    "✓"
                } else {
                    match result.enforcement {
                        Enforcement::Advisory => {
                            warnings += 1;
                            "⚠"
                        }
                        Enforcement::SoftMandatory => {
                            if *advisory_only || override_reason.is_some() {
                                warnings += 1;
                                "⚠"
                            } else {
                                failures += 1;
                                blocked = true;
                                "✗"
                            }
                        }
                        Enforcement::HardMandatory => {
                            failures += 1;
                            blocked = true;
                            "✗"
                        }
                    }
                };

                if !*json {
                    let enforcement_label = match result.enforcement {
                        Enforcement::Advisory => "advisory",
                        Enforcement::SoftMandatory => "soft_mandatory",
                        Enforcement::HardMandatory => "hard_mandatory",
                    };
                    println!(
                        "  {} {} ({}) - {}",
                        status_icon, result.policy_name, enforcement_label, result.message
                    );
                    if !result.passed {
                        if let Some(ref remediation) = result.remediation {
                            println!("    └─ Remediation: {}", remediation);
                        }
                    }
                }
            }

            if !*json {
                println!();
                println!(
                    "Summary: {} passed, {} warnings, {} failures",
                    passed, warnings, failures
                );
                if blocked {
                    if override_reason.is_some() {
                        println!("⚠️  Policy check would block but override provided");
                    } else {
                        println!("✗ Policy check failed - operation blocked");
                    }
                } else {
                    println!("✓ Policy check passed");
                }
            }

            if *json {
                let results_json: Vec<_> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "policy": r.policy_name,
                            "passed": r.passed,
                            "enforcement": format!("{:?}", r.enforcement).to_lowercase(),
                            "message": r.message,
                            "remediation": r.remediation
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({
                        "repository": repo_name,
                        "path": repo_path.display().to_string(),
                        "passed": passed,
                        "warnings": warnings,
                        "failures": failures,
                        "blocked": blocked,
                        "results": results_json
                    })
                );
            }

            Ok(())
        }

        GovernanceCommands::Status => {
            let policy_config_path = default_policies_path();
            let config = RepoPolicyConfig::load(&policy_config_path)?;

            println!("Governance Policy Status");
            println!("═══════════════════════════════════════════════════════════════");
            println!();
            println!("Config path: {}", policy_config_path.display());
            println!();

            println!("Policies ({}):", config.policies.len());
            let mut policies: Vec<_> = config.policies.iter().collect();
            policies.sort_by_key(|(name, _)| *name);

            for (name, policy) in policies {
                let icon = match policy.enforcement {
                    Enforcement::Advisory => "○",
                    Enforcement::SoftMandatory => "◐",
                    Enforcement::HardMandatory => "●",
                };
                let enforcement_label = match policy.enforcement {
                    Enforcement::Advisory => "advisory",
                    Enforcement::SoftMandatory => "soft_mandatory",
                    Enforcement::HardMandatory => "hard_mandatory",
                };
                let status = if policy.enabled {
                    "enabled"
                } else {
                    "disabled"
                };
                println!("  {} {} ({}) - {}", icon, name, enforcement_label, status);
                println!("    {}", policy.description);
            }

            if !config.exemptions.is_empty() {
                println!();
                println!("Exemptions ({}):", config.exemptions.len());
                for exemption in &config.exemptions {
                    let expires = exemption
                        .expires
                        .as_ref()
                        .map(|e| format!(" (expires: {})", e))
                        .unwrap_or_default();
                    println!(
                        "  {} / {} - {}{}",
                        exemption.repo, exemption.policy, exemption.reason, expires
                    );
                }
            }

            println!();
            println!("Enforcement levels:");
            println!("  ○ Advisory - warn only");
            println!("  ◐ Soft Mandatory - blocks, can override with reason");
            println!("  ● Hard Mandatory - always blocks, no override");

            Ok(())
        }

        GovernanceCommands::Violations {
            enforcement,
            repo,
            json,
        } => {
            // Determine repo path and name
            let repo_path = match repo {
                Some(r) => std::path::PathBuf::from(r),
                None => std::env::current_dir()?,
            };
            let repo_name = repo_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Load policy config
            let policy_config_path = default_policies_path();
            let config = RepoPolicyConfig::load(&policy_config_path)?;

            // Run policy checks
            let results = check_all_policies(&repo_path, &repo_name, &config);

            // Filter by enforcement level if specified
            let enforcement_filter: Option<Enforcement> =
                enforcement
                    .as_ref()
                    .and_then(|e| match e.to_lowercase().as_str() {
                        "advisory" => Some(Enforcement::Advisory),
                        "soft_mandatory" | "soft" => Some(Enforcement::SoftMandatory),
                        "hard_mandatory" | "hard" => Some(Enforcement::HardMandatory),
                        _ => None,
                    });

            // Collect violations (failed checks)
            let violations: Vec<_> = results
                .into_iter()
                .filter(|r| !r.passed)
                .filter(|r| {
                    enforcement_filter
                        .map(|f| r.enforcement == f)
                        .unwrap_or(true)
                })
                .collect();

            if !*json {
                println!("Policy Violations");
                println!("═══════════════════════════════════════════════════════════════");
                println!("Repository: {} ({})", repo_name, repo_path.display());
                if let Some(ref e) = enforcement {
                    println!("Filter: {} enforcement", e);
                }
                println!();

                if violations.is_empty() {
                    println!("✓ No violations detected");
                } else {
                    println!("Found {} violation(s):", violations.len());
                    println!();
                    for v in &violations {
                        let icon = match v.enforcement {
                            Enforcement::Advisory => "⚠",
                            Enforcement::SoftMandatory => "◐",
                            Enforcement::HardMandatory => "●",
                        };
                        let enforcement_label = match v.enforcement {
                            Enforcement::Advisory => "advisory",
                            Enforcement::SoftMandatory => "soft_mandatory",
                            Enforcement::HardMandatory => "hard_mandatory",
                        };
                        println!("  {} {} ({})", icon, v.policy_name, enforcement_label);
                        println!("    {}", v.message);
                        if let Some(ref remediation) = v.remediation {
                            println!("    └─ Remediation: {}", remediation);
                        }
                    }
                }
            }

            if *json {
                let violations_json: Vec<_> = violations
                    .iter()
                    .map(|v| {
                        serde_json::json!({
                            "policy": v.policy_name,
                            "enforcement": format!("{:?}", v.enforcement).to_lowercase(),
                            "message": v.message,
                            "remediation": v.remediation
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({
                        "repository": repo_name,
                        "path": repo_path.display().to_string(),
                        "count": violations.len(),
                        "violations": violations_json
                    })
                );
            }

            Ok(())
        }

        GovernanceCommands::Exempt {
            repo,
            policy,
            reason,
            expires,
        } => {
            let policy_config_path = default_policies_path();
            let mut config = RepoPolicyConfig::load(&policy_config_path)?;

            // Check if exemption already exists
            let already_exists = config
                .exemptions
                .iter()
                .any(|e| e.repo == *repo && e.policy == *policy);

            if already_exists {
                println!("⚠️  Exemption already exists for {} / {}", repo, policy);
                return Ok(());
            }

            // Add the new exemption
            config.exemptions.push(PolicyExemption {
                repo: repo.clone(),
                policy: policy.clone(),
                reason: reason.clone(),
                expires: expires.clone(),
                approved_by: std::env::var("USER").ok(),
            });

            // Save the config
            config.save(&policy_config_path)?;

            println!("✓ Added exemption:");
            println!("  Repository: {}", repo);
            println!("  Policy: {}", policy);
            println!("  Reason: {}", reason);
            if let Some(ref exp) = expires {
                println!("  Expires: {}", exp);
            }
            println!();
            println!("Saved to: {}", policy_config_path.display());

            Ok(())
        }

        GovernanceCommands::Unexempt { repo, policy } => {
            let policy_config_path = default_policies_path();
            let mut config = RepoPolicyConfig::load(&policy_config_path)?;

            let original_len = config.exemptions.len();
            config
                .exemptions
                .retain(|e| !(e.repo == *repo && e.policy == *policy));

            if config.exemptions.len() == original_len {
                println!("⚠️  No exemption found for {} / {}", repo, policy);
                return Ok(());
            }

            // Save the config
            config.save(&policy_config_path)?;

            println!("✓ Removed exemption:");
            println!("  Repository: {}", repo);
            println!("  Policy: {}", policy);
            println!();
            println!("Saved to: {}", policy_config_path.display());

            Ok(())
        }

        GovernanceCommands::Audit { repo, days, json } => {
            println!("Governance Audit Log");
            println!("═══════════════════════════════════════════════════════════════");

            if let Some(ref r) = repo {
                println!("Repository: {}", r);
            }
            println!("Last {} days", days);
            println!();
            println!("No audit entries (audit logging not yet implemented)");

            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "entries": []
                    })
                );
            }

            Ok(())
        }
    }
}

/// Output scan results in the specified format
fn output_scan_result(
    result: &allbeads::governance::ScanResult,
    format: &commands::OutputFormat,
    show_all: bool,
    fields: &allbeads::governance::FieldSet,
) {
    use allbeads::governance::{
        format_scan_result_csv_with_fields, format_scan_result_junit, print_scan_result,
    };

    match format {
        commands::OutputFormat::Text => print_scan_result(result, show_all),
        commands::OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(result).unwrap_or_default()
            );
        }
        commands::OutputFormat::Csv => {
            print!("{}", format_scan_result_csv_with_fields(result, fields));
        }
        commands::OutputFormat::Junit => {
            print!("{}", format_scan_result_junit(result));
        }
    }
}

/// Helper to check if format is text (for showing progress)
fn is_text_format(format: &commands::OutputFormat) -> bool {
    matches!(format, commands::OutputFormat::Text)
}

/// Handle the `scan` command - scan GitHub user/org for repositories
async fn handle_scan_command(
    cmd: &commands::ScanCommands,
    _config_path: &Option<String>,
    global_json: bool,
) -> allbeads::Result<()> {
    use allbeads::governance::{GitHubScanner, ScanFilter, ScanOptions};

    // Determine effective format: global --json flag overrides per-command format
    let effective_format = |format: &commands::OutputFormat| -> commands::OutputFormat {
        if global_json {
            commands::OutputFormat::Json
        } else {
            *format
        }
    };

    // Determine if we should show progress (only for text format)
    let show_progress = match cmd {
        commands::ScanCommands::User { format, .. }
        | commands::ScanCommands::Org { format, .. }
        | commands::ScanCommands::GitHub { format, .. }
        | commands::ScanCommands::Repo { format, .. }
        | commands::ScanCommands::Compare { format, .. } => !global_json && is_text_format(format),
    };

    // Extract and parse fields from command
    let fields_str = match cmd {
        commands::ScanCommands::User { fields, .. }
        | commands::ScanCommands::Org { fields, .. }
        | commands::ScanCommands::GitHub { fields, .. }
        | commands::ScanCommands::Repo { fields, .. } => fields.clone(),
        commands::ScanCommands::Compare { .. } => None,
    };

    let fields = match fields_str {
        Some(ref s) => match allbeads::governance::FieldSet::parse(s) {
            Ok(fs) => fs,
            Err(e) => {
                eprintln!("Error parsing --fields: {}", e);
                return Err(allbeads::AllBeadsError::Config(e));
            }
        },
        None => allbeads::governance::FieldSet::basic(),
    };

    let scan_options = ScanOptions {
        show_progress,
        fields,
        ..Default::default()
    };

    // Get GitHub token from environment
    let token = std::env::var("GITHUB_TOKEN").ok();

    if token.is_none() && show_progress {
        eprintln!("Warning: GITHUB_TOKEN not set. Rate limits will be strict and private repos won't be visible.");
        eprintln!("Set GITHUB_TOKEN environment variable for better results.");
        eprintln!();
    }

    let scanner = GitHubScanner::new(token);

    match cmd {
        commands::ScanCommands::User {
            username,
            min_stars,
            language,
            activity,
            exclude_forks,
            exclude_archived,
            all,
            fields: _,
            format,
        } => {
            let filter = ScanFilter {
                min_stars: *min_stars,
                language: language.clone(),
                activity_days: *activity,
                exclude_forks: *exclude_forks,
                exclude_archived: *exclude_archived,
                exclude_private: false,
                topics: Vec::new(),
            };

            let result = scanner
                .scan_user_with_options(username, &filter, &scan_options)
                .await?;
            output_scan_result(
                &result,
                &effective_format(format),
                *all,
                &scan_options.fields,
            );

            Ok(())
        }

        commands::ScanCommands::Org {
            org,
            min_stars,
            language,
            activity,
            exclude_forks,
            exclude_archived,
            exclude_private,
            all,
            fields: _,
            format,
        } => {
            let filter = ScanFilter {
                min_stars: *min_stars,
                language: language.clone(),
                activity_days: *activity,
                exclude_forks: *exclude_forks,
                exclude_archived: *exclude_archived,
                exclude_private: *exclude_private,
                topics: Vec::new(),
            };

            let result = scanner
                .scan_org_with_options(org, &filter, &scan_options)
                .await?;
            output_scan_result(
                &result,
                &effective_format(format),
                *all,
                &scan_options.fields,
            );

            Ok(())
        }

        commands::ScanCommands::Compare { format } => {
            let format = effective_format(format);
            // For now, just show a summary comparing managed vs GitHub
            use allbeads::config::AllBeadsConfig;

            let config_path = AllBeadsConfig::default_path();
            let config = AllBeadsConfig::load(&config_path)?;

            let contexts: Vec<_> = config
                .contexts
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "name": c.name,
                        "url": c.url,
                        "path": c.path.as_ref().map(|p| p.display().to_string())
                    })
                })
                .collect();

            match format {
                commands::OutputFormat::Text => {
                    println!("Managed Contexts vs GitHub");
                    println!("═══════════════════════════════════════════════════════════════");
                    println!();
                    println!("Managed contexts in AllBeads: {}", config.contexts.len());
                    for ctx in &config.contexts {
                        let path_str = ctx
                            .path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(no local path)".to_string());
                        println!("  • {} - {}", ctx.name, path_str);
                    }
                    println!();
                    println!("To scan GitHub for unmanaged repos, run:");
                    println!("  ab scan user <username>");
                    println!("  ab scan org <organization>");
                }
                commands::OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({ "contexts": contexts }))?
                    );
                }
                commands::OutputFormat::Csv => {
                    println!("name,url,path");
                    for ctx in &config.contexts {
                        let path_str = ctx
                            .path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        println!("{},{},{}", ctx.name, ctx.url, path_str);
                    }
                }
                commands::OutputFormat::Junit => {
                    // JUnit doesn't make sense for compare, output empty testsuite
                    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
                    println!(
                        "<testsuites name=\"allbeads-compare\" tests=\"{}\" failures=\"0\">",
                        config.contexts.len()
                    );
                    println!(
                        "  <testsuite name=\"managed-contexts\" tests=\"{}\">",
                        config.contexts.len()
                    );
                    for ctx in &config.contexts {
                        println!(
                            "    <testcase name=\"{}\" classname=\"context\" />",
                            ctx.name
                        );
                    }
                    println!("  </testsuite>");
                    println!("</testsuites>");
                }
            }

            Ok(())
        }

        commands::ScanCommands::GitHub {
            target,
            min_stars,
            language,
            activity,
            exclude_forks,
            exclude_archived,
            all,
            fields: _,
            format,
        } => {
            // Auto-detect target type: repo URL, user, or org
            let filter = ScanFilter {
                min_stars: *min_stars,
                language: language.clone(),
                activity_days: *activity,
                exclude_forks: *exclude_forks,
                exclude_archived: *exclude_archived,
                exclude_private: false,
                topics: Vec::new(),
            };

            // Check if it's a repo URL (contains /)
            if target.contains('/') {
                // Could be github.com/user/repo, user/repo, or full URL
                let parts: Vec<&str> = target
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .trim_start_matches("github.com/")
                    .split('/')
                    .collect();

                if parts.len() >= 2 {
                    // It's a repo reference
                    let owner = parts[0];
                    let repo = parts[1].trim_end_matches(".git");
                    let result = scanner
                        .scan_single_repo_with_options(owner, repo, &scan_options)
                        .await?;
                    output_scan_result(
                        &result,
                        &effective_format(format),
                        true,
                        &scan_options.fields,
                    );
                    return Ok(());
                }
            }

            // Try as user first, then org if that fails
            if show_progress {
                eprintln!("Scanning {}...", target);
            }
            let result = scanner
                .scan_user_with_options(target, &filter, &scan_options)
                .await;

            match result {
                Ok(scan_result) if !scan_result.repositories.is_empty() => {
                    output_scan_result(
                        &scan_result,
                        &effective_format(format),
                        *all,
                        &scan_options.fields,
                    );
                }
                _ => {
                    // Try as org
                    if show_progress {
                        eprintln!("Not a user, trying as organization...");
                    }
                    let result = scanner
                        .scan_org_with_options(target, &filter, &scan_options)
                        .await?;
                    output_scan_result(
                        &result,
                        &effective_format(format),
                        *all,
                        &scan_options.fields,
                    );
                }
            }

            Ok(())
        }

        commands::ScanCommands::Repo {
            url,
            fields: _,
            format,
        } => {
            let format = effective_format(format);
            // Parse repo URL
            let parts: Vec<&str> = url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("github.com/")
                .split('/')
                .collect();

            if parts.len() < 2 {
                return Err(allbeads::AllBeadsError::Config(
                    "Invalid repo URL. Expected format: user/repo or github.com/user/repo"
                        .to_string(),
                ));
            }

            let owner = parts[0];
            let repo = parts[1].trim_end_matches(".git");
            let result = scanner
                .scan_single_repo_with_options(owner, repo, &scan_options)
                .await?;
            output_scan_result(&result, &format, true, &scan_options.fields);

            Ok(())
        }
    }
}
