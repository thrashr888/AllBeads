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

**This is a greenfield Rust project.** We have:

- ✅ Comprehensive architecture specification ([PRD](specs/PRD-00.md))
- ✅ Rust project initialized (`cargo init`)
- ✅ Beads issue tracking configured (prefix: `ab`)
- ⬜ Implementation (in progress)

## Getting Started

### Prerequisites

- Rust toolchain (2024 edition)
- `bd` (beads CLI) - [Installation instructions](https://github.com/steveyegge/beads)
- Git

### Development Setup

```bash
# Clone the repository
git clone <repo-url>
cd AllBeads

# Check the build
cargo check

# Run tests (when available)
cargo test

# See available beads issues
bd list
```

### Project Structure

```
AllBeads/
├── specs/
│   └── PRD-00.md           # Complete architecture specification
├── src/
│   └── main.rs             # CLI entry point
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
