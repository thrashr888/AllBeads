# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**AllBeads** is a Rust implementation of the "Boss Repository Architecture" - a meta-orchestration system for AI agent workflows across multiple git repositories. This is a greenfield project with comprehensive specifications but no implementation yet.

### What This Will Be

From specs/PRD-00.md, AllBeads implements:
- **Sheriff Daemon**: Synchronization engine that federates beads (issues) across multiple repositories
- **Boss Board TUI**: Terminal-based dashboard for visualizing cross-repo dependencies and work status
- **Federated Graph**: Unified dependency graph aggregating work from distributed "Rig" repositories
- **Enterprise Integration**: Bi-directional sync with JIRA and GitHub Issues
- **Manifest System**: XML-based configuration for managing member repositories

### Current State

This is a brand new Rust project initialized with `cargo init`. The codebase currently contains:
- `specs/PRD-00.md`: 20,000+ word comprehensive architecture specification
- `Cargo.toml`: Rust project configuration (edition 2024)
- `src/main.rs`: Empty starter file
- `.beads/`: Beads issue tracker (available for project management)

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

### Core Components (To Be Implemented)

Based on the PRD, the system consists of:

#### 1. Sheriff Daemon
The synchronization engine written in Rust with:
- Concurrent repository polling using tokio async runtime
- Git operations for fetching beads from member repositories
- Diff and merge logic for Shadow Beads
- External API integration (JIRA via REST, GitHub via GraphQL)
- Event loop with configurable poll intervals

#### 2. Boss Board TUI
Terminal interface for visualization using Rust TUI libraries (ratatui or similar):
- Multi-view dashboard (Kanban, Dependency Graph, Agent Status)
- Real-time updates from Sheriff daemon
- Interactive navigation and filtering
- ASCII/Unicode graph rendering for dependency visualization

#### 3. Data Structures

Key entities:
- **Shadow Bead**: Pointer to beads in member repositories with metadata
- **Rig**: Configuration for a member repository (path, remote, persona)
- **Manifest**: XML definition of all managed repositories
- **Federated Graph**: Aggregated dependency graph with cross-repo links

#### 4. Integration Adapters
- **JIRA Adapter**: Bi-directional sync using Atlassian REST API
- **GitHub Adapter**: GraphQL-based issue sync
- **Git Adapter**: Native git operations for beads federation

### Directory Structure (Planned)

```
src/
  main.rs              # CLI entry point
  lib.rs              # Library root
  sheriff/            # Sheriff daemon implementation
    daemon.rs         # Main event loop
    poll.rs           # Repository polling
    sync.rs           # State synchronization
  boss_board/         # TUI implementation
    app.rs            # Main application state
    views/            # Different view components
  graph/              # Federated graph data structures
    shadow_bead.rs    # Shadow bead representation
    rig.rs            # Rig configuration
    graph.rs          # Graph operations
  manifest/           # Manifest parsing
    parser.rs         # XML manifest parser
    schema.rs         # Manifest data structures
  integrations/       # External service adapters
    jira.rs
    github.rs
  storage/            # Data persistence
    sqlite.rs         # SQLite operations
    jsonl.rs          # JSONL format handling
```

## Key Rust Crates to Consider

Based on the architecture requirements:

### Core Functionality
- `tokio`: Async runtime for Sheriff daemon
- `clap`: CLI argument parsing
- `serde`: Serialization/deserialization
- `serde_json`: JSON handling for beads
- `serde_yaml`: YAML config parsing

### Git Operations
- `git2`: libgit2 bindings for Rust
- `gix`: Pure Rust git implementation (alternative)

### TUI
- `ratatui`: Terminal UI framework (successor to tui-rs)
- `crossterm`: Terminal manipulation

### Data Storage
- `rusqlite`: SQLite bindings
- `sqlx`: Async SQL toolkit (if using async storage)

### External Integrations
- `reqwest`: HTTP client for REST APIs
- `graphql_client`: GraphQL client for GitHub
- `serde_xml_rs`: XML parsing for manifests

### Utilities
- `anyhow`: Error handling
- `thiserror`: Custom error types
- `tracing`: Structured logging
- `config`: Configuration management

## Beads Issue Tracking

This repository uses `bd` (beads) for issue tracking. Beads is available but not required for development.

### Essential Beads Commands

```bash
# Create issues
bd create --title="Implement Sheriff daemon" --type=feature --priority=1

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
3. **Plan the feature**: Consider creating a design doc for complex components
4. **Implement iteratively**: Start with core data structures, then build outward
5. **Test as you go**: Write unit tests alongside implementation

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
- **Property tests**: Consider `proptest` for complex data structures

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
5. **Sleep**: Configurable interval before next iteration

## Common Rust Patterns for This Project

### Async Operations

Most I/O (git, HTTP) should be async:

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

## References

- **PRD**: `specs/PRD-00.md` - Complete architectural specification
- **Beads Project**: https://github.com/steveyegge/beads
- **Gas Town**: https://github.com/steveyegge/gastown (inspiration for Rig/Mayor concepts)
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Docs**: https://tokio.rs/

## Project-Specific Notes

- This is implementing a concept from the PRD - the PRD mentions Go, but we're using Rust
- The PRD references bubbletea (Go TUI), we'll use ratatui (Rust equivalent)
- Focus on implementing the Sheriff daemon first, then the TUI
- The beads integration is critical - this tool manages beads, so deep familiarity with beads' JSONL format and SQLite schema will be essential
