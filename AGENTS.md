
# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

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

### Current State (v0.2.0)

PRD-00 (Core Architecture) and PRD-01 (Context Onboarding) are complete. Phase 10 of PRD-01 (Registry Integration) is deferred pending Codex marketplace registry API availability.

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
- `ab init --remote` for existing repositories
- Janitor workflow for automated issue discovery
- JIRA bi-directional sync (REST API)
- GitHub Issues integration (GraphQL + REST)
- Plugin system with Codex marketplace integration
- Plugin recommendations based on project analysis
- Multi-agent support (Codex, Cursor, Copilot, Aider)
- Agent configuration sync (`ab agent sync`)
- Unified sync command (`ab sync`)

**Deferred (Phase 10 - Registry Integration):**
- Official Codex marketplace registry API integration
- Plugin discovery from registry
- Automatic version checking and updates

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

## Golden Workflow: Onboard → Handoff → Complete

The recommended workflow for managing work across AllBeads repositories:

### 1. Onboard Repositories
```bash
# Onboard existing repo (requires: clean git, main branch)
ab onboard /path/to/repo

# Creates: .beads/, .Codex/settings.json, AllBeads context
# Creates: Epic + task beads (epic depends on tasks)
```

### 2. Find Ready Work
```bash
ab ready                 # Show unblocked tasks across all repos
ab show <bead-id>        # Review task details
```

### 3. Hand Off to Agent
```bash
ab handoff <bead-id>                  # Use preferred agent
ab handoff <bead-id> --agent codex    # Specific agent
ab handoff <bead-id> --dry-run        # Preview
```

### 4. Agent Completes Work
Most agents handle: branch creation → work → commit → push → close bead

### 5. For Sandboxed Agents (Codex)
AllBeads pre-creates the branch. After agent completes:
```bash
git add -A
git commit -m "feat(<bead-id>): <description>"
bd dolt push   # if a Dolt remote is configured
git push -u origin bead/<bead-id>
```

### Key Learnings
- **Onboarding safety**: Clean git workspace, main/master branch required
- **Dependency direction**: Epic depends on tasks (`bd dep add <epic> <task>`)
- **Sandboxed agents**: Codex uses `exec --full-auto`, can't write to `.git/`

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

### Skill Commands
- `ab skill list` - List installed skills (project + user level)
- `ab skill info <name>` - Show skill metadata
- `ab skill install <source>` - Install from GitHub or local path
- `ab skill remove <name>` - Remove an installed skill
- `ab skill sync` - Sync skills to .Codex-plugin/

### TUI Commands
- `ab tui` - Launch dashboard (Tab switches Kanban/Mail)

## Agent Guidelines

### Visual Design Rules

Use small Unicode symbols with semantic colors, NOT emoji:

| Status | Symbol | Color |
|--------|--------|-------|
| Open | `○` | default |
| In Progress | `◐` | yellow |
| Blocked | `●` | red |
| Closed | `✓` | green |
| Frozen | `❄` | cyan |

**Anti-pattern:** Never use colored circle emojis (🔴🟡🟢). They cause cognitive overload.

### Interactive Command Restrictions

**DO NOT use these commands** (they require interactive input):
- `bd edit` - Opens $EDITOR
- `ab tui` - Opens TUI
- Any command requiring stdin

**Instead use:**
- `bd update <id> --title="..." --description="..."`
- `bd comments add <id> "comment text"`

### Session Completion Protocol

Before ending any session, you **MUST**:

```bash
[ ] 1. git status              # Check what changed
[ ] 2. git add <files>         # Stage code changes
[ ] 3. bd dolt push            # Push beads state if a Dolt remote is configured
[ ] 4. git commit -m "..."     # Commit code
[ ] 5. git push                # Push code to remote
```

**CRITICAL:** Work is NOT complete until `git push` succeeds. Never leave work uncommitted locally.

### Multi-Context Workflow

AllBeads aggregates beads across multiple repositories:

```bash
# Work across contexts
ab list -C AllBeads,rookery    # Filter to specific contexts
ab search "auth" --context=@work  # Search in @work contexts

# Create in specific context
ab create --context=AllBeads --title="Fix bug" --type=bug

# Onboard new repos
ab context new myproject --private  # Create new GitHub repo
ab onboard /path/to/repo            # Onboard existing repo
```

### Cross-Repo Task Handoff

To hand off tasks to other AllBeads-related apps, create beads in their context:

```bash
# Create task for the web app (allbeads.co)
ab create --context=AllBeadsWeb --title="Add new API endpoint" --type=feature

# Create task for the macOS app
ab create --context=AllBeadsApp --title="Fix menu bar icon" --type=bug

# View tasks in other repos
ab list -C AllBeadsWeb
ab list -C AllBeadsApp
```

This allows agents working in different repos to pick up tasks created here.

### Agent Types

#### Task Agent
Autonomous agent that finds and completes ready work:
1. `ab ready` - Find unblocked tasks
2. `ab update <id> --status=in_progress` - Claim
3. Complete the work
4. `ab close <id>` - Complete
5. Repeat

#### Governance Agent
Enforces policies across managed repositories:
```bash
ab governance check        # Check all repos
ab agents list            # List detected agents
ab scan github <user>     # Scan for unmanaged repos
```

#### Planning Agent
Plans new projects without writing code:
```bash
ab context new <name>     # Create repo
bd create --type=epic     # Create planning beads
# STOP - don't implement yet
```

### Discovery Pattern

When you find bugs, TODOs, or related work while implementing:
```bash
bd create --title="Found: ..." --type=bug
bd dep add <new-id> <current-id>  # Link as discovered-from
```

This maintains context for future work.

### Quality Gates

Before closing any task:
- [ ] Tests pass: `cargo test`
- [ ] Linter clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`
- [ ] No secrets committed
- [ ] Changes pushed to remote

## Recommended Codex Agents

The following agent types are useful for AllBeads workflows:

### Task Agent (beads:task-agent)
Autonomous agent that finds and completes ready work from beads. Ideal for:
- Processing backlog items
- Implementing well-defined features
- Bug fixes with clear reproduction steps

```bash
ab ready && ab handoff <bead-id>
```

### Documentation Agent
Maintains project documentation including:
- Updating DEMO.md with new features
- Keeping AGENTS.md current with codebase changes
- Writing spec documents for new features
- Generating API documentation

### Release Agent
Handles the release process:
- Run quality gates (fmt, clippy, test)
- Update version in Cargo.toml
- Create annotated tags with release notes
- Monitor CI/CD builds
- Update homebrew tap

### Review Agent
Performs code review on pull requests:
- Check for security vulnerabilities
- Validate error handling
- Ensure tests are adequate
- Review architectural decisions

### Onboarding Agent
Helps onboard new repositories:
- Detect project type and languages
- Initialize beads with appropriate prefix
- Configure coding agent settings
- Create initial epic and tasks

### Planning Agent
Plans features without implementing:
- Create epic with breakdown tasks
- Set dependencies between tasks
- Estimate scope and complexity
- Identify potential blockers

### Governance Agent
Enforces policies across repositories:
- Scan for AI agent configurations
- Check compliance with policies
- Track agent adoption metrics
- Generate governance reports

## References

- **PRD**: `specs/PRD-00.md` - Complete architectural specification
- **ARCHITECTURE**: `specs/ARCHITECTURE.md` - Technical architecture overview
- **DEMO.md**: Usage examples and command reference
- **Beads Project**: https://github.com/steveyegge/beads
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Docs**: https://tokio.rs/

## Related Repositories

AllBeads is typically developed alongside its sister repositories:

| Repo | Path | Description |
|------|------|-------------|
| **AllBeads** (this repo) | `.` | Rust CLI and core library |
| **AllBeadsWeb** | `../AllBeadsWeb` | Next.js web app (allbeads.co) |
| **AllBeadsApp** | `../AllBeadsApp` | macOS native app (private) |

All three repos use beads for issue tracking. Use `ab create --context=<name>` to hand off tasks between them.

## Project-Specific Notes

- The PRD references Go and bubbletea, but we use Rust and ratatui
- The beads integration is critical - prefer official bd CLI / Dolt-backed state over legacy JSONL mirrors
- All async code uses tokio runtime
- SQLite (rusqlite) is used for caching, mail storage, and locks
- JIRA uses REST API v3, GitHub uses GraphQL for search + REST for mutations

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
