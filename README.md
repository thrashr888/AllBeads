# AllBeads

**A Boss Repository Architecture for Multi-Repo AI Agent Orchestration**

AllBeads is a meta-orchestration system that federates issue tracking (beads) across multiple git repositories, enabling AI agents to coordinate work across distributed microservices with unified dependency management and enterprise integration.

## What is AllBeads?

AllBeads implements the "Boss Repository" pattern - a control plane that:

- **Federates beads** from multiple repositories into a unified dependency graph
- **Synchronizes state** bi-directionally with JIRA and GitHub Issues
- **Visualizes cross-repo work** through a terminal-based dashboard
- **Enables strategic coordination** between AI agents working across distributed codebases

Think of it as a "meta-issue-tracker" that sits above your microservices, giving agents and architects a coherent view of work spanning 10, 20, or 50+ repositories.

## Architecture

AllBeads consists of four core components:

### 1. Sheriff Daemon
Background service that:
- Polls member repositories for beads updates
- Creates "Shadow Beads" in the Boss repo for Epic-level work
- Syncs state with JIRA and GitHub Issues
- Manages the federated dependency graph

### 2. Boss Board TUI
Terminal-based dashboard providing:
- Multi-view visualization (Kanban, Dependency Graph, Agent Status)
- Real-time updates from the Sheriff daemon
- Interactive navigation and filtering
- Cross-repository dependency visualization

### 3. Federated Graph
Data structure representing:
- Shadow Beads pointing to native beads in member repositories
- Cross-repo dependencies (`bead://repo-name/bead-id` URIs)
- Rig configurations (member repository metadata)
- Aggregated work state across the entire organization

### 4. Manifest System
XML-based configuration defining:
- Member repositories (location, branch, remote)
- Agent personas (security-specialist, ux-designer, etc.)
- Bead prefixes for namespacing
- External integration mappings

## Current State

**Phase 1 (The Reader) - Complete**

AllBeads provides read-only aggregation of multiple Boss repositories:

- ✅ Multi-repository aggregation from git remotes (SSH and HTTPS)
- ✅ SQLite cache layer with automatic expiration
- ✅ Context-aware filtering (@work, @personal, etc.)
- ✅ Full CLI with filtering, search, and display commands
- ✅ Kanban TUI with keyboard navigation
- ✅ bd JSONL format compatibility
- ✅ Integration tests covering core functionality

**Phase 2 (The Mailroom) - In Progress**

Agent-to-agent messaging protocol:

- ✅ Message types defined (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT)
- ⬜ Postmaster daemon
- ⬜ Message persistence and routing

See [demo.md](demo.md) for usage examples and [ARCHITECTURE.md](ARCHITECTURE.md) for technical details.

## Getting Started

### Prerequisites

- Rust toolchain (2024 edition)
- `bd` (beads CLI) - [Installation instructions](https://github.com/steveyegge/beads)
- Git

### Installation

```bash
# Clone the repository
git clone https://github.com/thrashr888/AllBeads.git
cd AllBeads

# Build the project
cargo build --release

# Initialize AllBeads (creates config directory and file)
./target/release/allbeads init
```

### Quick Start

```bash
# Setup alias for convenience
alias ab='./target/release/allbeads'

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

# Filter by priority
ab list --priority P1

# Show ready-to-work beads (no blockers)
ab ready

# Show bead details
ab show ab-123

# Launch Kanban TUI
ab tui
```

See [demo.md](demo.md) for more examples.

### CLI Reference

#### Initialization

```bash
# Initialize AllBeads (creates ~/.config/allbeads/config.yaml)
allbeads init
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

# Filter by context
allbeads list --context <context-name>

# Show beads ready to work (no blockers)
allbeads ready

# Show detailed information about a bead
allbeads show <bead-id>
```

#### TUI (Terminal User Interface)

```bash
# Launch Kanban board
allbeads tui

# TUI Keyboard shortcuts:
#   j/k or ↑/↓    - Move up/down in current column
#   h/l or ←/→    - Switch between columns
#   Enter         - View bead details
#   Esc           - Back to board
#   q             - Quit
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
│   └── PRD-00.md           # Complete architecture specification
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library exports
│   ├── aggregator/         # Multi-repo aggregation
│   ├── cache/              # SQLite caching
│   ├── config/             # Configuration management
│   ├── git/                # Git operations
│   ├── graph/              # Bead graph data structures
│   ├── mail/               # Agent Mail protocol
│   ├── storage/            # JSONL parsing
│   └── tui/                # Kanban TUI
├── tests/
│   └── integration_test.rs # Integration tests
├── .beads/                 # Issue tracking database
├── Cargo.toml              # Rust dependencies
├── CLAUDE.md               # AI agent development guide
├── ARCHITECTURE.md         # Technical architecture
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

## Documentation

- **[PRD](specs/PRD-00.md)**: 20,000+ word architectural specification covering the Boss Repository pattern, Sheriff daemon, federated graph, TUI design, and enterprise integration strategy
- **[CLAUDE.md](CLAUDE.md)**: Development guide for AI agents and developers, including Rust patterns, architecture overview, and common workflows

## Issue Tracking

This project uses [beads](https://github.com/steveyegge/beads) for issue tracking.

```bash
# Create a new issue
bd create --title="Implement Sheriff daemon" --type=feature --priority=1

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
- **TUI Framework**: ratatui
- **Git Operations**: git2 or gix
- **HTTP Client**: reqwest
- **Error Handling**: anyhow + thiserror
- **Serialization**: serde (JSON/YAML/XML)

See [CLAUDE.md](CLAUDE.md) for complete list of recommended crates.

## Inspiration & Related Projects

- **[beads](https://github.com/steveyegge/beads)**: Git-native issue tracking for AI agents
- **[Gas Town](https://github.com/steveyegge/gastown)**: Multi-agent workspace orchestration
- **[Conductor](https://conductor.build)**: AI-powered development with git worktrees
- **Google repo**: Multi-repository management tool

AllBeads builds on these concepts to create a federated orchestration layer for enterprise-scale AI agent coordination.

## Contributing

This is an early-stage project. Key areas needing implementation:

1. **Core data structures**: Shadow Bead, Rig, Federated Graph
2. **Sheriff daemon**: Event loop, polling, sync logic
3. **Manifest parser**: XML parsing compatible with git-repo standard
4. **Boss Board TUI**: Terminal dashboard with multiple views
5. **Integration adapters**: JIRA and GitHub sync

See `bd ready` for current work items.

---

*AllBeads: Orchestrating AI agent swarms across the polyrepo frontier* 🤖
