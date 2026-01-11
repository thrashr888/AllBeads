//! CLI command definitions
//!
//! All CLI structs and subcommand enums are defined here.

use clap::{Parser, Subcommand};

/// AllBeads - Multi-context task aggregator and orchestrator
#[derive(Parser, Debug)]
#[command(name = "allbeads")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to config file (default: ~/.config/allbeads/config.yaml)
    #[arg(short, long)]
    pub config: Option<String>,

    /// Filter to specific contexts (comma-separated)
    #[arg(short = 'C', long)]
    pub contexts: Option<String>,

    /// Use cached data only (don't fetch updates)
    #[arg(long)]
    pub cached: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // === Setup & Configuration ===
    /// Initialize AllBeads configuration or clone a remote repo
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

    /// Setup wizard for configuration
    Setup,

    /// Quickstart guide for AllBeads
    Quickstart,

    /// Manage contexts (Boss repositories)
    #[command(subcommand)]
    Context(ContextCommands),

    /// Manage tracked folders (Dry→Wet progression)
    #[command(subcommand)]
    Folder(FolderCommands),

    /// Clear the local cache
    ClearCache,

    // === Viewing Beads ===
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

    /// Show beads that are ready to work on (no blockers)
    Ready,

    /// Show all blocked beads
    Blocked,

    /// Show aggregated statistics
    Stats,

    // === Analysis & Search ===
    /// Search beads by text (title, description, notes)
    Search {
        /// Search query (optional with filters)
        query: Option<String>,

        /// Filter by context
        #[arg(short = 'c', long)]
        context: Option<String>,

        /// Filter by status. Prefix with ^ to negate (e.g., ^closed)
        #[arg(short = 's', long)]
        status: Option<String>,

        /// Filter by minimum priority (inclusive, 0-4 or P0-P4)
        #[arg(long)]
        priority_min: Option<String>,

        /// Filter by maximum priority (inclusive, 0-4 or P0-P4)
        #[arg(long)]
        priority_max: Option<String>,

        /// Filter by type (bug, feature, task, epic, chore). Prefix with ^ to negate
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

    // === TUI & Interface ===
    /// Launch Terminal UI (Kanban + Mail + Graph + Swarm)
    Tui,

    // === Agent Integration ===
    /// Show project info and status for AI agents
    Info,

    /// Prime agent memory with project context
    Prime,

    /// Onboard to a project (for AI agents)
    Onboard {
        /// Show detailed workflow guide
        #[arg(long)]
        full: bool,
    },

    /// Send a message to human operator
    Human {
        /// Message to send to human
        message: Option<String>,
    },

    /// Agent Mail commands
    #[command(subcommand)]
    Mail(MailCommands),

    /// Agent swarm management commands
    #[command(subcommand)]
    Swarm(SwarmCommands),

    // === Distributed Configuration ===
    /// Manage distributed configuration sync
    #[command(subcommand)]
    Config(ConfigCommands),

    // === Daemons & Sync ===
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

    // === Enterprise Integration ===
    /// JIRA integration commands
    #[command(subcommand)]
    Jira(JiraCommands),

    /// GitHub integration commands
    #[command(subcommand, name = "github")]
    GitHub(GitHubCommands),
}

#[derive(Subcommand, Debug)]
pub enum MailCommands {
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
pub enum JiraCommands {
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
pub enum GitHubCommands {
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
pub enum SwarmCommands {
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
pub enum ConfigCommands {
    /// Initialize distributed config sync with a git remote
    Init {
        /// Remote repository URL for config sync
        #[arg(long, conflicts_with = "gist")]
        remote: Option<String>,

        /// GitHub Gist ID for lightweight config sync
        #[arg(long, conflicts_with = "remote")]
        gist: Option<String>,

        /// Force re-initialization (overwrites existing remote)
        #[arg(short, long)]
        force: bool,
    },

    /// Pull config changes from remote
    Pull {
        /// Force pull, discarding local changes
        #[arg(short, long)]
        force: bool,
    },

    /// Push config changes to remote
    Push {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,

        /// Force push (use with caution)
        #[arg(short, long)]
        force: bool,
    },

    /// Show config sync status
    Status,

    /// Show config diff with remote
    Diff,

    /// Clone config from a remote to a new machine
    Clone {
        /// Remote repository URL or Gist ID
        source: String,

        /// Target directory (default: ~/.config/allbeads)
        #[arg(short, long)]
        target: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ContextCommands {
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

#[derive(Subcommand, Debug)]
pub enum FolderCommands {
    /// Add folders to tracking
    Add {
        /// Paths to track (supports globs like ~/Workspace/*)
        #[arg(required = true)]
        paths: Vec<String>,

        /// Override prefix for beads
        #[arg(short, long)]
        prefix: Option<String>,

        /// Agent persona for these folders
        #[arg(long)]
        persona: Option<String>,

        /// Start interactive setup after adding
        #[arg(long)]
        setup: bool,
    },

    /// List tracked folders with Dry→Wet status
    List {
        /// Filter by status (dry, git, beads, configured, wet)
        #[arg(short, long)]
        status: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove a folder from tracking
    Remove {
        /// Path to remove
        path: String,

        /// Also clean up AllBeads config in the folder
        #[arg(long)]
        clean: bool,
    },

    /// Show folder status summary
    Status {
        /// Path to check (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Interactive setup wizard for a folder
    Setup {
        /// Path to set up (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Skip confirmation prompts (use defaults)
        #[arg(short, long)]
        yes: bool,
    },

    /// Promote a folder to the next status level
    Promote {
        /// Path to promote (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Target status level
        #[arg(long)]
        to: Option<String>,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Manage git worktrees
    #[command(subcommand)]
    Worktree(WorktreeCommands),

    /// Detect and display monorepo structure
    Monorepo {
        /// Path to check (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Manage project templates
    #[command(subcommand)]
    Template(TemplateCommands),
}

#[derive(Subcommand, Debug)]
pub enum TemplateCommands {
    /// Create a template from an existing project
    Create {
        /// Template name
        name: String,

        /// Source project path to create template from
        #[arg(long, default_value = ".")]
        from: String,

        /// Template description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Apply a template to the current or specified directory
    Apply {
        /// Template name to apply
        name: String,

        /// Target directory (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// List available templates
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show template details
    Show {
        /// Template name
        name: String,
    },

    /// Delete a template
    Delete {
        /// Template name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorktreeCommands {
    /// List all worktrees for a repository
    List {
        /// Path to any worktree in the repo (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Show worktree details and beads status
    Status {
        /// Path to worktree (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },
}
