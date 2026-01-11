# AllBeads

**A Boss Repository Architecture for Multi-Repo AI Agent Orchestration**

AllBeads is a meta-orchestration system that federates issue tracking (beads) across multiple git repositories, enabling AI agents to coordinate work across distributed microservices with unified dependency management and enterprise integration.

## What is AllBeads?

AllBeads implements the "Boss Repository" pattern - a control plane that:

- **Federates beads** from multiple repositories into a unified dependency graph
- **Synchronizes state** bi-directionally with JIRA and GitHub Issues
- **Visualizes cross-repo work** through a terminal-based dashboard
- **Enables strategic coordination** between AI agents working across distributed codebases
- **Provides agent-to-agent messaging** for distributed coordination

Think of it as a "meta-issue-tracker" that sits above your microservices, giving agents and architects a coherent view of work spanning 10, 20, or 50+ repositories.

## Architecture

AllBeads consists of five core components:

### 1. Sheriff Daemon
Background synchronization service that:
- Polls member repositories for beads updates
- Creates "Shadow Beads" in the Boss repo for Epic-level work
- Syncs state with JIRA and GitHub Issues
- Manages the federated dependency graph
- Runs in foreground or background mode

### 2. Boss Board TUI
Terminal-based dashboard providing:
- Multi-view visualization (Kanban, Mail views)
- Real-time updates from the Sheriff daemon
- Interactive navigation and filtering
- Cross-repository dependency visualization

### 3. Agent Mail System
Distributed messaging protocol:
- Message types: LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT
- Postmaster daemon for message routing
- Resource locking for coordination
- SQLite-backed persistence

### 4. Federated Graph
Data structure representing:
- Shadow Beads pointing to native beads in member repositories
- Cross-repo dependencies (`bead://repo-name/bead-id` URIs)
- Rig configurations (member repository metadata)
- Aggregated work state across the entire organization

### 5. Enterprise Integration
External system adapters:
- **JIRA**: REST API adapter with JQL search and status sync
- **GitHub**: GraphQL + REST API for issue management
- **Plugin Architecture**: Extensible for Linear, Asana, etc.

See [DEMO.md](DEMO.md) for usage examples.

## Getting Started

### Prerequisites

- `bd` (beads CLI) - [Installation instructions](https://github.com/steveyegge/beads)
- Git

### Installation

```bash
# Clone the repository
git clone https://github.com/thrashr888/AllBeads.git
cd AllBeads

# Build the project (requires Rust toolchain)
cargo build --release

# Add to PATH or create alias
alias ab='./target/release/allbeads'

# Initialize AllBeads (creates config directory and file)
ab init
```

### Quick Start

```bash
# Add the current repository (auto-detects name, URL, and auth)
cd /path/to/your-repo
ab context add .

# Or add with explicit URL (SSH or HTTPS)
ab context add . --url git@github.com:org/repo.git

# View aggregated statistics
ab stats

# List all beads
ab list

# Filter by status
ab list --status open

# Show ready-to-work beads (no blockers)
ab ready

# Launch TUI (Kanban + Mail)
ab tui

# Run Sheriff daemon in foreground
ab sheriff --foreground

# Check JIRA/GitHub integration status
ab jira status
ab github status
```

See [DEMO.md](DEMO.md) for more examples.

### CLI Reference

> **Note:** The examples below use `allbeads` (the binary name). If you've set up the `ab` alias as shown above, you can use `ab` instead.

#### Initialization

```bash
# Initialize AllBeads (creates ~/.config/allbeads/config.yaml)
allbeads init

# Initialize from existing remote repository
allbeads init --remote git@github.com:org/boss-repo.git
```

#### Context Management

```bash
# Add a repository (infers name from folder, URL from git remote)
allbeads context add <path>

# Add with explicit name and URL
allbeads context add <path> --name <name> --url <url>

# Specify authentication strategy
allbeads context add <path> --auth ssh_agent
allbeads context add <path> --auth personal_access_token

# List all configured contexts
allbeads context list

# Remove a context
allbeads context remove <name>
```

#### Viewing Beads

```bash
# Show aggregated statistics
allbeads stats

# List all beads
allbeads list

# Filter by status (open, in_progress, blocked, deferred, closed)
allbeads list --status <status>

# Filter by priority (P0-P4 or 0-4)
allbeads list --priority <priority>

# Show beads ready to work (no blockers)
allbeads ready

# Show blocked beads
allbeads blocked

# Show detailed information about a bead
allbeads show <bead-id>

# Search beads
allbeads search "query"
allbeads search --status open --type feature
```

#### Sheriff Daemon

```bash
# Run in foreground (recommended for development)
allbeads sheriff --foreground

# Run with custom poll interval (seconds)
allbeads sheriff --foreground --poll-interval 10

# Run with specific manifest
allbeads sheriff --manifest manifests/work.xml --foreground
```

#### Agent Mail

```bash
# Send a test notification
allbeads mail send --to agent-1 --subject "Test" --body "Hello"

# List messages for the human inbox
allbeads mail list

# Check unread count
allbeads mail unread
```

#### Janitor Analysis

```bash
# Analyze repository for potential issues
allbeads janitor /path/to/repo

# Dry run (show what would be created)
allbeads janitor /path/to/repo --dry-run
```

#### Enterprise Integration

```bash
# JIRA commands
allbeads jira status                    # Check configuration
allbeads jira pull --project PROJ --url https://company.atlassian.net

# GitHub commands
allbeads github status                  # Check configuration
allbeads github pull --owner myorg      # Pull from organization
allbeads github pull --owner myorg --repo myrepo  # Pull from specific repo
```

#### TUI Dashboard

```bash
# Launch TUI (Kanban + Mail views)
allbeads tui

# Keyboard shortcuts:
#   Tab           - Switch between Kanban and Mail views
#   j/k or Up/Down - Move up/down
#   h/l or Left/Right - Switch columns (Kanban)
#   Enter         - View details
#   Esc           - Back
#   q             - Quit
```

#### Plugin System

```bash
# List available plugins
allbeads plugin list
allbeads plugin list --all              # Include not-installed

# Get plugin recommendations for current project
allbeads plugin recommend

# Plugin information and status
allbeads plugin info <name>
allbeads plugin status

# Marketplace commands
allbeads plugin marketplace-list
allbeads plugin marketplace-sync
```

#### Coding Agents

```bash
# List configured coding agents
allbeads agent list

# Detect agents in project
allbeads agent detect

# Initialize agent configuration
allbeads agent init claude              # Claude Code (CLAUDE.md)
allbeads agent init cursor              # Cursor (.cursorrules)
allbeads agent init copilot             # GitHub Copilot
allbeads agent init aider               # Aider

# Sync AllBeads context to agent configs
allbeads agent sync

# Preview agent configuration
allbeads agent preview <agent>
```

#### Sync

```bash
# Sync AllBeads config (if in git)
allbeads sync

# Sync all context beads
allbeads sync --all

# Sync specific context
allbeads sync mycontext

# Check sync status
allbeads sync --status
```

#### Cache Management

```bash
# Clear the local cache (forces refresh on next command)
allbeads clear-cache

# Use cached data only (don't fetch updates)
allbeads --cached <command>
```

### Configuration

Config file location: `~/.config/allbeads/config.yaml`

Example configuration:

```yaml
contexts:
  - name: work
    type: git
    url: https://github.com/org/boss-work.git
    path: /Users/you/workspace/boss-work
    auth_strategy: ssh_agent
    integrations:
      jira:
        url: https://company.atlassian.net
        project: PROJ
      github:
        url: https://github.com
        owner: myorg

  - name: personal
    type: git
    url: git@github.com:you/boss-personal.git
    path: /Users/you/workspace/boss-personal
    auth_strategy: ssh_agent

agent_mail:
  port: 8085
  storage: ~/.config/allbeads/mail.db

visualization:
  default_view: kanban
  theme: dark
  refresh_interval: 60
```

### Project Structure

```
AllBeads/
├── specs/
│   ├── PRD-00.md           # Core architecture specification
│   └── PRD-01-*.md         # Feature specifications
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library exports
│   ├── commands.rs         # CLI command definitions
│   ├── aggregator/         # Multi-repo aggregation
│   ├── cache/              # SQLite caching
│   ├── coding_agent.rs     # Multi-agent support (Claude, Cursor, etc.)
│   ├── config/             # Configuration management
│   ├── context/            # Context & folder tracking
│   ├── git/                # Git operations
│   ├── graph/              # Bead/Shadow/Rig data structures
│   ├── integrations/       # JIRA, GitHub adapters
│   ├── mail/               # Agent Mail protocol
│   ├── manifest/           # XML manifest parsing
│   ├── plugin.rs           # Plugin system & marketplace
│   ├── sheriff/            # Sheriff daemon
│   ├── storage/            # JSONL parsing
│   ├── swarm/              # Agent swarm management
│   └── tui/                # Terminal UI
├── tests/
│   └── integration_test.rs # Integration tests
├── .beads/                 # Issue tracking database
├── Cargo.toml              # Rust dependencies
├── CLAUDE.md               # AI agent development guide
└── README.md               # This file
```

## Key Concepts

### The Boss Repository Pattern

Traditional approaches:
- **Monorepo**: All code in one repository (doesn't scale)
- **Polyrepo**: Independent repositories (loses coordination)

AllBeads approach:
- **Boss Repo**: Lightweight control plane that federates state across polyrepos without merging code

### Shadow Beads

- **Native Bead**: Lives in a member repo's `.beads/` directory
- **Shadow Bead**: Lives in Boss repo, points to Native Bead, adds cross-repo context

Shadow Beads enable the Boss to track Epic-level work that spans multiple repositories while letting each repo maintain autonomy over its own issues.

### Rigs

A "Rig" is a member repository managed by the Boss. Each Rig:
- Has its own `.beads/` directory with native beads
- Is defined in the Boss's manifest file
- May have an assigned agent persona (security-specialist, frontend-expert, etc.)
- Contributes Shadow Beads for Epic-level work to the Boss graph

### Agent Mail

The messaging protocol enables agents to:
- Send notifications between agents
- Request and release resource locks
- Broadcast announcements
- Track heartbeats for agent health monitoring

## Documentation

- **[PRD](specs/PRD-00.md)**: 20,000+ word architectural specification
- **[DEMO.md](DEMO.md)**: Usage examples and command reference
- **[CLAUDE.md](CLAUDE.md)**: Development guide for AI agents

## Issue Tracking

This project uses [beads](https://github.com/steveyegge/beads) for issue tracking.

```bash
# Create a new issue
bd create --title="Implement feature X" --type=feature --priority=1

# List open issues
bd list --status=open

# See available work
bd ready

# Update issue status
bd update ab-xxx --status=in_progress
bd close ab-xxx
```

Issues are prefixed with `ab-` (AllBeads).

## Technology Stack

- **Language**: Rust (edition 2024)
- **Async Runtime**: tokio
- **TUI Framework**: ratatui + crossterm
- **Git Operations**: git2
- **HTTP Client**: reqwest
- **Error Handling**: anyhow + thiserror
- **Serialization**: serde (JSON/YAML/XML)
- **Database**: SQLite (rusqlite)
- **Logging**: tracing

## Contributing

See `bd ready` for current work items.

---

*AllBeads: Orchestrating AI agent swarms across the polyrepo frontier*
