# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**AllBeads** is a Rust implementation of the "Boss Repository Architecture" - a meta-orchestration system for AI agent workflows across multiple git repositories.

### What AllBeads Does

From specs/PRD-00.md, AllBeads implements:
- **Sheriff Daemon**: Synchronization engine that federates beads (issues) across multiple repositories
- **Boss Board TUI**: Terminal-based dashboard for visualizing cross-repo dependencies and work status
- **Federated Graph**: Unified dependency graph aggregating work from distributed "Rig" repositories
- **Enterprise Integration**: Bi-directional sync with JIRA and GitHub Issues
- **Agent Mail System**: Messaging protocol for agent-to-agent communication
- **Janitor Workflow**: Automated issue discovery and repository analysis

### Current State

Phases 1-4 are complete. Phase 5 (The Swarm) is in progress.

**Implemented:**
- Multi-repository aggregation from git remotes (SSH/HTTPS)
- SQLite cache layer with automatic expiration
- Context-aware filtering (@work, @personal, etc.)
- Full CLI with filtering, search, and display commands
- Kanban TUI with keyboard navigation
- Mail TUI for agent messages
- Agent Mail protocol (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT)
- Postmaster daemon with message routing
- Sheriff daemon with git sync (foreground mode)
- `allbeads init --remote` for existing repositories
- Janitor workflow for automated issue discovery
- JIRA bi-directional sync (REST API)
- GitHub Issues integration (GraphQL + REST)
- Plugin architecture for extensibility

**In Progress (Phase 5):**
- Agent lifecycle management (spawn, monitor, kill)
- Cost tracking and budget management
- Advanced dependency resolution across contexts

## Rust Development Commands

### Building and Running

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the project
cargo run

# Run with arguments
cargo run -- [args]

# Check code without building
cargo check
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run doc tests
cargo test --doc
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Run clippy as strict as possible
cargo clippy -- -W clippy::pedantic
```

### Dependencies

```bash
# Add a dependency
cargo add <crate>

# Add a dev dependency
cargo add --dev <crate>

# Update dependencies
cargo update

# Check for outdated dependencies (requires cargo-outdated)
cargo outdated
```

### Documentation

```bash
# Build and open documentation
cargo doc --open

# Build docs including private items
cargo doc --document-private-items
```

## Architecture Overview

### Core Components

#### 1. Sheriff Daemon (`src/sheriff/`)
The synchronization engine:
- `daemon.rs`: Main event loop with configurable poll intervals
- `sync.rs`: State synchronization between Boss and Rigs
- `external_sync.rs`: JIRA/GitHub bi-directional sync
- Concurrent repository polling using tokio async runtime
- Git operations for fetching beads from member repositories

#### 2. Boss Board TUI (`src/tui/`)
Terminal interface using ratatui:
- `kanban.rs`: Kanban board view with columns (Open, In Progress, Closed)
- `mail.rs`: Agent Mail inbox view
- Real-time updates, interactive navigation
- Color-coded priorities (P0=red through P4=gray)

#### 3. Agent Mail (`src/mail/`)
Messaging protocol for agent coordination:
- `postmaster.rs`: Message routing and delivery
- `server.rs`: HTTP server for mail API
- `locks.rs`: Resource locking protocol
- `address.rs`: Agent addressing system
- Message types: LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT

#### 4. Data Structures (`src/graph/`)
Core entities:
- `bead.rs`: Native Bead representation
- `shadow_bead.rs`: Shadow Bead pointing to Rigs with cross-repo context
- `rig.rs`: Member repository configuration
- `federated_graph.rs`: Aggregated dependency graph
- `ids.rs`: Type-safe BeadId and RigId

#### 5. Integration Adapters (`src/integrations/`)
External system sync:
- `jira.rs`: JIRA REST API adapter with JQL search
- `github.rs`: GitHub GraphQL + REST API adapter
- `plugin.rs`: Plugin architecture for extensibility

#### 6. Janitor (`src/janitor/`)
Repository analysis and issue discovery:
- `analyzer.rs`: Static analysis for potential issues
- `rules.rs`: Configurable analysis rules

### Directory Structure

```
src/
  main.rs              # CLI entry point (clap)
  lib.rs               # Library exports
  error.rs             # Custom error types
  aggregator/          # Multi-repo aggregation
  cache/               # SQLite caching
  config/              # Configuration management
    boss_context.rs    # Context configuration
    mod.rs             # Config loading/saving
  git/                 # Git operations (git2)
  graph/               # Core data structures
    bead.rs            # Bead entity
    shadow_bead.rs     # Shadow Bead with builder
    rig.rs             # Rig configuration
    federated_graph.rs # Graph operations
    ids.rs             # Type-safe IDs
  integrations/        # External service adapters
    jira.rs            # JIRA REST API
    github.rs          # GitHub GraphQL/REST
    plugin.rs          # Plugin architecture
  janitor/             # Repository analysis
    analyzer.rs        # Issue discovery
    rules.rs           # Analysis rules
  mail/                # Agent Mail protocol
    address.rs         # Agent addressing
    postmaster.rs      # Message routing
    server.rs          # HTTP server
    locks.rs           # Resource locking
    message.rs         # Message types
  manifest/            # XML manifest parsing
    parser.rs          # Manifest parser
    schema.rs          # Manifest data structures
  sheriff/             # Sheriff daemon
    daemon.rs          # Event loop
    sync.rs            # State synchronization
    external_sync.rs   # JIRA/GitHub sync
  storage/             # Data persistence
    jsonl.rs           # JSONL format handling
  tui/                 # Terminal UI
    kanban.rs          # Kanban board view
    mail.rs            # Mail view
```

## Key Rust Crates Used

### Core Functionality
- `tokio`: Async runtime for Sheriff daemon
- `clap`: CLI argument parsing
- `serde`: Serialization/deserialization
- `serde_json`: JSON handling for beads
- `serde_yaml`: YAML config parsing

### Git Operations
- `git2`: libgit2 bindings for Rust

### TUI
- `ratatui`: Terminal UI framework
- `crossterm`: Terminal manipulation

### Data Storage
- `rusqlite`: SQLite bindings

### External Integrations
- `reqwest`: HTTP client for REST APIs
- `serde_xml_rs`: XML parsing for manifests

### Utilities
- `anyhow`: Error handling
- `thiserror`: Custom error types
- `tracing`: Structured logging
- `chrono`: Date/time handling
- `async-trait`: Async traits

## Beads Issue Tracking

This repository uses `bd` (beads) for issue tracking.

### Essential Beads Commands

```bash
# Create issues
bd create --title="Implement feature X" --type=feature --priority=1

# List and filter
bd list --status=open
bd ready                    # Show unblocked work

# Update work
bd update <id> --status=in_progress
bd close <id>

# Dependencies
bd dep add <issue> <depends-on>
bd graph                    # Visualize dependencies
```

Use beads for tracking multi-session work and complex features with dependencies. For simple tasks within a single session, TodoWrite is sufficient.

## Development Workflow

### Starting Development

1. **Read the PRD**: `specs/PRD-00.md` is the authoritative specification
2. **Check for work**: `bd ready` to see prioritized issues
3. **Run tests**: `cargo test` to ensure clean baseline
4. **Implement iteratively**: Start with tests, then implementation
5. **Verify**: Run `cargo clippy` and `cargo fmt` before committing

### Code Organization Principles

- Keep modules focused and cohesive
- Separate concerns (data structures, business logic, I/O)
- Use type system to enforce invariants
- Prefer composition over inheritance
- Make illegal states unrepresentable

### Testing Strategy

- **Unit tests**: Test individual functions and methods
- **Integration tests**: Test component interactions (in `tests/` directory)
- **Doc tests**: Include examples in documentation comments
- Currently 160+ tests covering core functionality

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for library-level custom errors
- Avoid panicking in library code
- Use `?` operator for error propagation

## Key Architectural Concepts

### The Federated Graph

The core innovation is treating multiple repositories' beads as a unified graph:
- Each "Rig" (member repository) maintains its own `.beads/` directory
- The Sheriff creates "Shadow Beads" in the Boss repo for Epic-level items
- Cross-repo dependencies are represented as `bead://repo-name/bead-id` URIs
- The federated graph is stored in Boss repo's `.boss/graph/` directory

### Shadow Beads vs Native Beads

- **Native Bead**: Lives in a Rig's `.beads/` directory, managed by that repo's team
- **Shadow Bead**: Lives in Boss repo, points to a Native Bead, adds cross-repo context
- Shadow Beads contain: summary, status (mirrored), pointer URI, and cross-repo dependencies
- ShadowBeadBuilder pattern for creating from external sources (JIRA, GitHub)

### Manifest-Driven Configuration

XML manifests (in `manifests/`) define the workspace:
- Repository locations and branches
- Agent personas (security-specialist, ux-designer, etc.)
- Bead prefixes for namespacing
- External integration mappings (JIRA projects, GitHub repos)

Compatible with Google's git-repo tool but with AllBeads-specific annotations.

### The Sheriff's Event Loop

1. **Poll Phase**: Fetch beads updates from all Rigs (`git fetch refs/beads/*`)
2. **Diff Phase**: Compare Rig state with cached Boss state
3. **Sync Phase**: Create/update Shadow Beads, push Boss directives to Rigs
4. **External Sync**: Bi-directional sync with JIRA/GitHub
5. **Mail Delivery**: Process pending agent messages
6. **Sleep**: Configurable interval before next iteration

## Common Rust Patterns in This Project

### Async Operations

Most I/O (git, HTTP) is async:

```rust
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Sheriff daemon runs here
    Ok(())
}
```

### Builder Pattern for Configuration

```rust
let rig = Rig::builder()
    .path("services/auth")
    .persona("security-specialist")
    .prefix("auth")
    .build()?;

let shadow = ShadowBead::external(id, summary, uri)
    .with_status("open")
    .with_priority(1)
    .with_external_ref("jira:PROJ-123")
    .build();
```

### Type-Safe IDs

Use newtypes for different ID types:

```rust
struct BeadId(String);
struct RigId(String);
// Prevents mixing up ID types
```

### Error Context

Add context to errors as they propagate:

```rust
use anyhow::Context;

fn load_config() -> anyhow::Result<Config> {
    std::fs::read_to_string("config.yaml")
        .context("Failed to read config file")?;
    // ...
}
```

## CLI Commands Reference

### Core Commands
- `ab init` / `ab init --remote <url>` - Initialize or clone
- `ab context add/list/remove` - Manage contexts
- `ab list/show/ready/blocked` - View beads
- `ab search` - Search with filters
- `ab stats` - Aggregated statistics

### Daemon Commands
- `ab sheriff --foreground` - Run Sheriff daemon
- `ab mail send/list/unread` - Agent Mail operations

### Analysis Commands
- `ab janitor <path>` - Analyze repository
- `ab duplicates` - Find duplicate beads

### Integration Commands
- `ab jira status/pull` - JIRA integration
- `ab github status/pull` - GitHub integration

### TUI Commands
- `ab tui` - Launch dashboard (Tab switches Kanban/Mail)

## References

- **PRD**: `specs/PRD-00.md` - Complete architectural specification
- **DEMO.md**: Usage examples and command reference
- **Beads Project**: https://github.com/steveyegge/beads
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Docs**: https://tokio.rs/

## Project-Specific Notes

- The PRD references Go and bubbletea, but we use Rust and ratatui
- The beads integration is critical - deep familiarity with beads' JSONL format is essential
- All async code uses tokio runtime
- SQLite (rusqlite) is used for caching, mail storage, and locks
- JIRA uses REST API v3, GitHub uses GraphQL for search + REST for mutations
