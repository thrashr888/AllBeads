//! CLI command definitions
//!
//! All CLI structs and subcommand enums are defined here.

use clap::{Parser, Subcommand};

/// Generate the custom help output matching bd's style
pub fn custom_help() -> String {
    // ANSI codes for cyan (like bd uses)
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";

    format!(
        r#"Multi-context bead aggregator and orchestrator

Usage:
  ab [flags]
  ab [command]

{cyan}Aggregation:{reset}
  list               List beads with optional filters
  show               Show detailed information about a bead
  ready              Show beads that are ready to work on (no blockers)
  blocked            Show all blocked beads
  search             Search beads by text (title, description, notes)
  duplicates         Find potential duplicate beads
  stats              Show aggregated statistics

{cyan}Wrapper Commands:{reset}
  create             Create a bead in a specific context (delegates to bd)
  update             Update a bead (delegates to bd in the bead's context)
  close              Close bead(s) (delegates to bd in the bead's context)
  reopen             Reopen closed bead(s)
  dep                Manage dependencies (add/remove)
  label              Manage labels (add/remove/list)
  comments           Manage comments (list/add)
  q                  Quick capture - create and output only ID
  epic               Epic management (list/create/show)
  edit               Edit a bead in $EDITOR
  delete             Delete bead(s)
  duplicate          Mark a bead as duplicate of another

{cyan}Context Management:{reset}
  init               Initialize AllBeads configuration or clone a remote repo
  setup              Setup wizard for configuration
  quickstart         Quickstart guide for AllBeads
  context            Manage contexts (Boss repositories)
                       - onboarding: Track repo adoption and onboarding status
  onboard            Onboard a repository (clone, bd init, skills, add context)
  onboard-repo       Interactive onboarding for current repository (deprecated: use 'onboard')
  folder             Manage tracked folders (Dry→Wet progression)
  clear-cache        Clear the local cache

{cyan}Integrations:{reset}
  jira               JIRA integration commands
  github             GitHub integration commands
  plugin             Manage plugins and onboarding

{cyan}Daemon & Sync:{reset}
  sync               Sync AllBeads state (config and/or context beads)
  sheriff            Run the Sheriff daemon (background sync)
  mail               Agent Mail commands

{cyan}Agent Support:{reset}
  info               Show project info and status for AI agents
  prime              Prime agent memory with project context
  onboard            Onboard to a project (for AI agents)
  human              Send a message to human operator
  swarm              Agent swarm management commands
  agent              Coding agent configuration (Claude Code, Cursor, etc.)

{cyan}Analysis:{reset}
  janitor            Run janitor analysis on a repository

{cyan}UI:{reset}
  tui                Launch Terminal UI (Kanban + Mail + Graph + Swarm)

{cyan}Governance:{reset}
  check              Check governance policies against current beads
  hooks              Manage git hooks for policy enforcement

{cyan}Aiki Integration:{reset}
  aiki               Aiki integration utilities (activate/deactivate beads)

{cyan}Configuration:{reset}
  config             Manage distributed configuration sync

{cyan}Additional Commands:{reset}
  help               Help about any command

Flags:
  -c, --config string        Path to config file (default: ~/.config/allbeads/config.yaml)
  -C, --contexts string      Filter to specific contexts (comma-separated)
      --cached               Use cached data only (don't fetch updates)

{cyan}Output Control:{reset}
      --json                 Output in JSON format
  -q, --quiet                Suppress non-essential output (errors only)
  -v, --verbose              Enable verbose/debug output

{cyan}Database/Storage:{reset}
      --db string            Database path (default: auto-discover .beads/*.db)
      --no-db                Use no-db mode: load from JSONL, no SQLite
      --readonly             Read-only mode: block write operations

{cyan}Sync Behavior:{reset}
      --no-auto-flush        Disable automatic JSONL sync after CRUD operations
      --no-auto-import       Disable automatic JSONL import when newer than DB
      --no-daemon            Force direct storage mode, bypass daemon if running
      --sandbox              Sandbox mode: disables daemon and auto-sync
      --allow-stale          Allow operations on potentially stale data

{cyan}Other:{reset}
      --actor string         Actor name for audit trail (default: $AB_ACTOR or $USER)
      --lock-timeout string  SQLite busy timeout (default 30s)
      --profile              Generate CPU profile for performance analysis
  -h, --help                 help for ab
  -V, --version              Print version information

Use "ab [command] --help" for more information about a command."#,
        reset = reset,
        cyan = cyan
    )
}

/// AllBeads - Multi-context task aggregator and orchestrator
#[derive(Parser, Debug)]
#[command(name = "allbeads")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to config file (default: ~/.config/allbeads/config.yaml)
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Filter to specific contexts (comma-separated)
    #[arg(short = 'C', long, global = true)]
    pub contexts: Option<String>,

    /// Use cached data only (don't fetch updates)
    #[arg(long, global = true)]
    pub cached: bool,

    // =========================================================================
    // OUTPUT CONTROL FLAGS (bd-compatible)
    // =========================================================================
    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output (errors only)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Enable verbose/debug output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    // =========================================================================
    // DATABASE/STORAGE FLAGS (bd-compatible)
    // =========================================================================
    /// Database path (default: auto-discover .beads/*.db)
    #[arg(long, global = true)]
    pub db: Option<String>,

    /// Use no-db mode: load from JSONL, no SQLite
    #[arg(long, global = true)]
    pub no_db: bool,

    /// Read-only mode: block write operations (for worker sandboxes)
    #[arg(long, global = true)]
    pub readonly: bool,

    // =========================================================================
    // SYNC BEHAVIOR FLAGS (bd-compatible)
    // =========================================================================
    /// Disable automatic JSONL sync after CRUD operations
    #[arg(long, global = true)]
    pub no_auto_flush: bool,

    /// Disable automatic JSONL import when newer than DB
    #[arg(long, global = true)]
    pub no_auto_import: bool,

    /// Force direct storage mode, bypass daemon if running
    #[arg(long, global = true)]
    pub no_daemon: bool,

    /// Sandbox mode: disables daemon and auto-sync
    #[arg(long, global = true)]
    pub sandbox: bool,

    /// Allow operations on potentially stale data (skip staleness check)
    #[arg(long, global = true)]
    pub allow_stale: bool,

    // =========================================================================
    // OTHER FLAGS (bd-compatible)
    // =========================================================================
    /// Actor name for audit trail (default: $AB_ACTOR or $USER)
    #[arg(long, global = true)]
    pub actor: Option<String>,

    /// SQLite busy timeout (0 = fail immediately if locked) (default 30s)
    #[arg(long, global = true)]
    pub lock_timeout: Option<String>,

    /// Generate CPU profile for performance analysis
    #[arg(long, global = true)]
    pub profile: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    /// Build bd-compatible global flags from CLI args
    /// Returns a vector of arguments to pass to bd commands
    pub fn bd_global_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        // Output control
        if self.json {
            flags.push("--json".to_string());
        }
        if self.quiet {
            flags.push("--quiet".to_string());
        }
        if self.verbose {
            flags.push("--verbose".to_string());
        }

        // Database/storage
        if let Some(ref db) = self.db {
            flags.push(format!("--db={}", db));
        }
        if self.no_db {
            flags.push("--no-db".to_string());
        }
        if self.readonly {
            flags.push("--readonly".to_string());
        }

        // Sync behavior
        if self.no_auto_flush {
            flags.push("--no-auto-flush".to_string());
        }
        if self.no_auto_import {
            flags.push("--no-auto-import".to_string());
        }
        if self.no_daemon {
            flags.push("--no-daemon".to_string());
        }
        if self.sandbox {
            flags.push("--sandbox".to_string());
        }
        if self.allow_stale {
            flags.push("--allow-stale".to_string());
        }

        // Other
        if let Some(ref actor) = self.actor {
            flags.push(format!("--actor={}", actor));
        }
        if let Some(ref timeout) = self.lock_timeout {
            flags.push(format!("--lock-timeout={}", timeout));
        }
        if self.profile {
            flags.push("--profile".to_string());
        }

        flags
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // =========================================================================
    // AGGREGATION COMMANDS - View beads across all contexts
    // =========================================================================
    /// List beads with optional filters
    List {
        /// Filter by status (open, in_progress, blocked, closed)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by priority (P0-P4 or 0-4)
        #[arg(short, long)]
        priority: Option<String>,

        /// Filter by context (@work, @personal)
        #[arg(long)]
        context: Option<String>,

        /// Filter by label/tag
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Show detailed information about a bead
    Show {
        /// Bead ID (e.g., ab-123)
        id: String,

        /// Show provenance information from Aiki
        #[arg(long)]
        provenance: bool,

        /// Show linked Aiki tasks
        #[arg(long)]
        tasks: bool,
    },

    /// Show beads that are ready to work on (no blockers)
    Ready,

    /// Show all blocked beads
    Blocked,

    /// Search beads by text (title, description, notes)
    Search {
        /// Search query (optional with filters)
        query: Option<String>,

        /// Filter by context
        #[arg(long)]
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

        /// Include closed beads (default: open only)
        #[arg(long)]
        include_closed: bool,
    },

    /// Show aggregated statistics
    Stats,

    // =========================================================================
    // WRAPPER COMMANDS - Delegate to bd in the correct context
    // =========================================================================
    /// Create a bead in a specific context (delegates to bd)
    Create {
        /// Title of the new bead
        #[arg(short, long)]
        title: String,

        /// Type (bug, feature, task, epic, chore)
        #[arg(short = 'T', long = "type", default_value = "task")]
        issue_type: String,

        /// Priority (P0-P4 or 0-4)
        #[arg(short, long, default_value = "2")]
        priority: String,

        /// Context to create in (defaults to current directory's context)
        #[arg(long)]
        context: Option<String>,
    },

    /// Update a bead (delegates to bd in the bead's context)
    Update {
        /// Bead ID (e.g., ab-123, rk-456)
        id: String,

        /// Set status (open, in_progress, blocked, closed)
        #[arg(long)]
        status: Option<String>,

        /// Set priority (P0-P4 or 0-4)
        #[arg(long)]
        priority: Option<String>,

        /// Set assignee
        #[arg(long)]
        assignee: Option<String>,
    },

    /// Close a bead (delegates to bd in the bead's context)
    Close {
        /// Bead ID(s) to close
        ids: Vec<String>,

        /// Reason for closing
        #[arg(long)]
        reason: Option<String>,
    },

    /// Reopen closed bead(s) (delegates to bd in the bead's context)
    Reopen {
        /// Bead ID(s) to reopen
        ids: Vec<String>,
    },

    /// Manage dependencies between beads
    #[command(subcommand)]
    Dep(DepCommands),

    /// Manage labels on beads
    #[command(subcommand)]
    Label(LabelCommands),

    /// Manage comments on beads
    #[command(subcommand)]
    Comments(CommentCommands),

    /// Quick capture - create bead and output only the ID
    Q {
        /// Title of the new bead
        title: String,

        /// Type (bug, feature, task, epic, chore)
        #[arg(short = 'T', long = "type")]
        issue_type: Option<String>,

        /// Priority (P0-P4 or 0-4)
        #[arg(short, long)]
        priority: Option<String>,

        /// Context to create in (defaults to current directory's context)
        #[arg(long)]
        context: Option<String>,
    },

    /// Epic management commands
    #[command(subcommand)]
    Epic(EpicCommands),

    /// Edit a bead field in $EDITOR
    Edit {
        /// Bead ID to edit
        id: String,

        /// Field to edit (title, description, notes)
        #[arg(long)]
        field: Option<String>,
    },

    /// Delete bead(s) (delegates to bd in the bead's context)
    Delete {
        /// Bead ID(s) to delete
        ids: Vec<String>,

        /// Skip confirmation
        #[arg(long, short)]
        yes: bool,
    },

    /// Mark a bead as duplicate of another
    Duplicate {
        /// Bead ID to mark as duplicate
        id: String,

        /// Bead ID that this is a duplicate of
        #[arg(long)]
        of: String,
    },

    // =========================================================================
    // CONTEXT COMMANDS - Manage Boss repositories
    // =========================================================================
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

    /// Onboard a repository into AllBeads
    ///
    /// Supports GitHub URLs, local paths, or current directory.
    /// Clones if needed, runs bd init, configures skills, and adds to AllBeads context.
    Onboard {
        /// Repository URL, local path, or current directory ('.')
        /// Examples: https://github.com/user/repo, git@github.com:user/repo.git, /path/to/repo, .
        target: String,

        /// Use defaults without interactive prompts
        #[arg(long)]
        non_interactive: bool,

        /// Skip cloning (repository already exists locally)
        #[arg(long)]
        skip_clone: bool,

        /// Skip beads initialization (bd init)
        #[arg(long)]
        skip_beads: bool,

        /// Skip skills marketplace configuration
        #[arg(long)]
        skip_skills: bool,

        /// Skip Git hooks installation (handled by bd init)
        #[arg(long)]
        skip_hooks: bool,

        /// Skip issue import/population
        #[arg(long)]
        skip_issues: bool,

        /// Override context name (default: repo name)
        #[arg(long)]
        context_name: Option<String>,

        /// Override clone path (default: workspace_directory/repo_name)
        #[arg(long)]
        path: Option<String>,
    },

    /// Interactive onboarding for current repository
    #[command(name = "onboard-repo")]
    OnboardRepo {
        /// Path to repository (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Skip interactive prompts and use defaults
        #[arg(short, long)]
        yes: bool,

        /// Skip bd init (assume already initialized)
        #[arg(long)]
        skip_init: bool,

        /// Skip CLAUDE.md setup
        #[arg(long)]
        skip_claude: bool,

        /// Skip adding to AllBeads contexts
        #[arg(long)]
        skip_context: bool,
    },

    /// Manage tracked folders (Dry→Wet progression)
    #[command(subcommand)]
    Folder(FolderCommands),

    /// Clear the local cache
    ClearCache,

    // =========================================================================
    // INTEGRATION COMMANDS - External systems
    // =========================================================================
    /// JIRA integration commands
    #[command(subcommand)]
    Jira(JiraCommands),

    /// GitHub integration commands
    #[command(subcommand, name = "github")]
    GitHub(GitHubCommands),

    /// Manage plugins and onboarding
    #[command(subcommand)]
    Plugin(PluginCommands),

    // =========================================================================
    // DAEMON COMMANDS - Background services
    // =========================================================================
    /// Sync AllBeads state (config and/or context beads)
    Sync {
        /// Sync all contexts' beads (runs bd sync in each context)
        #[arg(long)]
        all: bool,

        /// Specific context to sync (default: sync config only)
        context: Option<String>,

        /// Commit message for config sync
        #[arg(short, long)]
        message: Option<String>,

        /// Show status only, don't sync
        #[arg(long)]
        status: bool,
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

    /// Agent Mail commands
    #[command(subcommand)]
    Mail(MailCommands),

    // =========================================================================
    // AGENT COMMANDS - AI agent integration
    // =========================================================================
    /// Show project info and status for AI agents
    Info,

    /// Prime agent memory with project context
    Prime,

    /// Send a message to human operator
    Human {
        /// Message to send to human
        message: Option<String>,
    },

    /// Agent swarm management commands
    #[command(subcommand)]
    Swarm(SwarmCommands),

    /// Coding agent configuration (Claude Code, Cursor, Copilot, etc.)
    #[command(subcommand, name = "agent")]
    CodingAgent(CodingAgentCommands),

    // =========================================================================
    // ANALYSIS COMMANDS - Code and repository analysis
    // =========================================================================
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

    // =========================================================================
    // UI COMMANDS - User interface
    // =========================================================================
    /// Launch Terminal UI (Kanban + Mail + Graph + Swarm)
    Tui,

    // =========================================================================
    // GOVERNANCE COMMANDS - Policy enforcement and compliance
    // =========================================================================
    /// Check governance policies against current beads
    Check {
        /// Run in strict mode (exit non-zero on any violation)
        #[arg(long)]
        strict: bool,

        /// Check specific policy only
        #[arg(long)]
        policy: Option<String>,

        /// Show fix suggestions for violations
        #[arg(long)]
        fix: bool,

        /// Pre-commit mode (optimized, quiet if passing)
        #[arg(long)]
        pre_commit: bool,

        /// Check specific bead
        #[arg(long)]
        bead: Option<String>,

        /// Output format (text, json, yaml)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Manage git hooks for policy enforcement
    #[command(subcommand)]
    Hooks(HooksCommands),

    /// Aiki integration utilities
    #[command(subcommand)]
    Aiki(AikiCommands),

    // =========================================================================
    // GOVERNANCE COMMANDS - Policy enforcement and agent management
    // =========================================================================
    /// Detect and manage AI agents in repositories
    #[command(subcommand)]
    Agents(AgentsCommands),

    /// Check and enforce governance policies
    #[command(subcommand)]
    Governance(GovernanceCommands),

    /// Scan GitHub user/org for repositories
    #[command(subcommand)]
    Scan(ScanCommands),

    // =========================================================================
    // CONFIG COMMANDS - Distributed configuration
    // =========================================================================
    /// Manage distributed configuration sync
    #[command(subcommand)]
    Config(ConfigCommands),
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
        #[arg(long)]
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
        #[arg(long, default_value = "default")]
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
pub enum HooksCommands {
    /// Install git hooks for policy enforcement
    Install {
        /// Install specific hook (pre-commit, commit-msg, post-commit, pre-push)
        #[arg(long)]
        hook: Option<String>,

        /// Install all available hooks
        #[arg(long)]
        all: bool,

        /// Dry run (show what would be installed)
        #[arg(long)]
        dry_run: bool,
    },

    /// Uninstall git hooks
    Uninstall {
        /// Uninstall specific hook
        #[arg(long)]
        hook: Option<String>,

        /// Uninstall all hooks
        #[arg(long)]
        all: bool,
    },

    /// List installed hooks
    List,

    /// Test hooks without committing
    Test {
        /// Test specific hook
        #[arg(long)]
        hook: Option<String>,
    },

    /// Check hook installation status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum AikiCommands {
    /// Activate a bead (set AB_ACTIVE_BEAD environment variable)
    Activate {
        /// Bead ID to activate
        bead_id: String,
    },

    /// Deactivate the current bead (unset AB_ACTIVE_BEAD)
    Deactivate,

    /// Show the currently active bead
    Status,

    /// Output shell initialization code (for eval)
    HookInit,

    /// Link an Aiki task to a bead
    Link {
        /// Bead ID
        bead_id: String,

        /// Aiki task ID
        task_id: String,
    },

    /// Unlink an Aiki task from a bead
    Unlink {
        /// Bead ID
        bead_id: String,

        /// Aiki task ID
        task_id: String,
    },

    /// List Aiki tasks linked to a bead
    Tasks {
        /// Bead ID
        bead_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AgentsCommands {
    /// Detect AI agents in a repository
    Detect {
        /// Path to repository (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List agents across all managed contexts
    List {
        /// Filter by agent type (claude, copilot, cursor, etc.)
        #[arg(long)]
        agent: Option<String>,

        /// Only show high-confidence detections
        #[arg(long)]
        high_confidence: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show agent adoption summary
    Summary,

    /// Record agent scan results to usage database
    Track {
        /// Context name (default: current directory name)
        #[arg(long)]
        context: Option<String>,

        /// Path to repository (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Show historical usage statistics
    Stats {
        /// Number of days to show (default: 30)
        #[arg(long, default_value = "30")]
        days: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum GovernanceCommands {
    /// Check policies against all managed repositories
    Check {
        /// Check specific repository only
        #[arg(long)]
        repo: Option<String>,

        /// Only check specific policy
        #[arg(long)]
        policy: Option<String>,

        /// Treat all policies as advisory (never block)
        #[arg(long)]
        advisory_only: bool,

        /// Treat soft mandatory as hard mandatory
        #[arg(long)]
        strict: bool,

        /// Override soft mandatory violations (with justification)
        #[arg(long, name = "REASON")]
        override_reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show policy status summary
    Status,

    /// List policy violations
    Violations {
        /// Filter by enforcement level (advisory, soft_mandatory, hard_mandatory)
        #[arg(long)]
        enforcement: Option<String>,

        /// Filter by repository
        #[arg(long)]
        repo: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add exemption for a repository
    Exempt {
        /// Repository name
        repo: String,

        /// Policy to exempt
        policy: String,

        /// Reason for exemption
        #[arg(long)]
        reason: String,

        /// Exemption expiration date (YYYY-MM-DD)
        #[arg(long)]
        expires: Option<String>,
    },

    /// Remove exemption
    Unexempt {
        /// Repository name
        repo: String,

        /// Policy to remove exemption for
        policy: String,
    },

    /// Show audit log
    Audit {
        /// Filter by repository
        #[arg(long)]
        repo: Option<String>,

        /// Number of days to show (default: 7)
        #[arg(long, default_value = "7")]
        days: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScanCommands {
    /// Scan a GitHub user's repositories
    User {
        /// GitHub username to scan
        username: String,

        /// Minimum number of stars
        #[arg(long)]
        min_stars: Option<u32>,

        /// Filter by language
        #[arg(long)]
        language: Option<String>,

        /// Only show repos with activity within N days
        #[arg(long)]
        activity: Option<u32>,

        /// Exclude forks
        #[arg(long)]
        exclude_forks: bool,

        /// Exclude archived repos
        #[arg(long)]
        exclude_archived: bool,

        /// Show all results (including low priority)
        #[arg(long)]
        all: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Scan a GitHub organization's repositories
    Org {
        /// GitHub organization name to scan
        org: String,

        /// Minimum number of stars
        #[arg(long)]
        min_stars: Option<u32>,

        /// Filter by language
        #[arg(long)]
        language: Option<String>,

        /// Only show repos with activity within N days
        #[arg(long)]
        activity: Option<u32>,

        /// Exclude forks
        #[arg(long)]
        exclude_forks: bool,

        /// Exclude archived repos
        #[arg(long)]
        exclude_archived: bool,

        /// Exclude private repos (requires auth token)
        #[arg(long)]
        exclude_private: bool,

        /// Show all results (including low priority)
        #[arg(long)]
        all: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare scanned repos with managed contexts
    Compare {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// List installed and available plugins
    List {
        /// Show all available plugins (not just installed)
        #[arg(short, long)]
        all: bool,

        /// Filter by category (claude, beads, prose, etc.)
        #[arg(short, long)]
        category: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show detailed plugin information
    Info {
        /// Plugin name
        name: String,
    },

    /// Show plugin status for current project
    Status {
        /// Plugin name (optional, shows all if not specified)
        name: Option<String>,
    },

    /// Detect plugins from Claude settings and project
    Detect {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Show verbose detection output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Install a plugin
    Install {
        /// Plugin name
        name: String,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Run plugin onboarding for current project
    Onboard {
        /// Plugin name
        name: String,

        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Recommend plugins for current project
    Recommend {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// List registered marketplaces
    #[command(name = "marketplace-list")]
    MarketplaceList {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a marketplace source
    #[command(name = "marketplace-add")]
    MarketplaceAdd {
        /// Marketplace URL or GitHub repo (e.g., owner/repo)
        source: String,

        /// Custom name for the marketplace
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Sync marketplace metadata
    #[command(name = "marketplace-sync")]
    MarketplaceSync {
        /// Only sync specific marketplace
        name: Option<String>,
    },
}

// =========================================================================
// WRAPPER SUBCOMMANDS
// =========================================================================

#[derive(Subcommand, Debug)]
pub enum DepCommands {
    /// Add a dependency (issue depends on another)
    Add {
        /// Issue that will depend on the other
        issue: String,

        /// Issue that will be depended on (blocker)
        depends_on: String,
    },

    /// Remove a dependency
    Remove {
        /// Issue to remove dependency from
        issue: String,

        /// Issue to remove as dependency
        depends_on: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum LabelCommands {
    /// Add a label to an issue
    Add {
        /// Issue ID
        issue: String,

        /// Label to add
        label: String,
    },

    /// Remove a label from an issue
    Remove {
        /// Issue ID
        issue: String,

        /// Label to remove
        label: String,
    },

    /// List all labels in the project
    List,
}

#[derive(Subcommand, Debug)]
pub enum CommentCommands {
    /// List comments on an issue
    List {
        /// Issue ID
        issue: String,
    },

    /// Add a comment to an issue
    Add {
        /// Issue ID
        issue: String,

        /// Comment content
        content: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum EpicCommands {
    /// List all epics
    List {
        /// Show only open epics
        #[arg(long)]
        open: bool,
    },

    /// Create a new epic
    Create {
        /// Epic title
        #[arg(short, long)]
        title: String,

        /// Priority (P0-P4 or 0-4)
        #[arg(short, long, default_value = "2")]
        priority: String,

        /// Context to create in (defaults to current directory's context)
        #[arg(long)]
        context: Option<String>,
    },

    /// Show epic details with children
    Show {
        /// Epic ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ContextCommands {
    /// Add a new context (from local path or remote URL)
    Add {
        /// Path to git repository (optional if --url provided)
        /// Name and URL are inferred from git config if path exists
        path: Option<String>,

        /// Override context name (default: folder/repo name)
        #[arg(short, long)]
        name: Option<String>,

        /// Repository URL (required if no path, or overrides remote origin)
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

    /// Show onboarding status for all contexts
    Onboarding {
        /// Show detailed onboarding guide for each repo
        #[arg(long)]
        full: bool,

        /// Show only summary statistics
        #[arg(long)]
        summary: bool,
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

#[derive(Subcommand, Debug)]
pub enum CodingAgentCommands {
    /// List configured coding agents
    List {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize a coding agent configuration
    Init {
        /// Agent name (claude, cursor, copilot, aider)
        agent: String,

        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },

    /// Sync AllBeads context to all configured agents
    Sync {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Only sync specific agent
        #[arg(short, long)]
        agent: Option<String>,
    },

    /// Preview agent configuration
    Preview {
        /// Agent name (claude, cursor, copilot, aider)
        agent: String,

        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Show agent detection status
    Detect {
        /// Path to project (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },
}
