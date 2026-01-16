# AllBeads Architecture

This document describes the technical architecture of AllBeads, a multi-repository beads aggregator.

## Overview

AllBeads federates issue tracking (beads) across multiple git repositories into a unified view. It implements the "Boss Repository" pattern for coordinating AI agent workflows across distributed codebases.

```
┌─────────────────────────────────────────────────────────────┐
│                        AllBeads                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │   CLI    │  │   TUI    │  │  Cache   │  │   Mail   │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │             │             │             │           │
│       └─────────────┼─────────────┼─────────────┘           │
│                     │             │                         │
│              ┌──────┴──────┐  ┌───┴────┐                   │
│              │ Aggregator  │  │ SQLite │                   │
│              └──────┬──────┘  └────────┘                   │
│                     │                                       │
│         ┌───────────┼───────────┐                          │
│         │           │           │                          │
│    ┌────┴────┐ ┌────┴────┐ ┌────┴────┐                    │
│    │  Git    │ │  Git    │ │  Git    │                    │
│    │ Repo 1  │ │ Repo 2  │ │ Repo N  │                    │
│    └─────────┘ └─────────┘ └─────────┘                    │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

### Core Modules

#### `src/main.rs` - CLI Entry Point
The command-line interface built with `clap`. Handles:
- Argument parsing and command dispatch
- Configuration loading
- Error formatting and colored output

Key commands:
- `init` - Initialize configuration
- `context add/list/remove` - Manage repositories
- `list/show/ready/blocked/stats` - View beads
- `tui` - Launch Kanban board

#### `src/lib.rs` - Library Exports
Public API for the AllBeads library:
```rust
pub mod aggregator;
pub mod cache;
pub mod config;
pub mod git;
pub mod graph;
pub mod mail;
pub mod storage;
pub mod tui;
```

### Configuration (`src/config/`)

#### `allbeads_config.rs`
Main configuration structure loaded from `~/.config/allbeads/config.yaml`:
```rust
pub struct AllBeadsConfig {
    pub contexts: Vec<BossContext>,
    pub agent_mail: Option<AgentMailConfig>,
    pub visualization: Option<VisualizationConfig>,
}
```

#### `boss_context.rs`
Configuration for a managed repository:
```rust
pub struct BossContext {
    pub name: String,           // e.g., "work", "personal"
    pub context_type: String,   // "git"
    pub url: String,            // git remote URL
    pub path: PathBuf,          // local path
    pub auth_strategy: AuthStrategy,
}

pub enum AuthStrategy {
    SshAgent,
    PersonalAccessToken,
}
```

### Graph Data Structures (`src/graph/`)

#### `bead.rs`
Core bead representation:
```rust
pub struct Bead {
    pub id: BeadId,
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub issue_type: IssueType,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    pub assignee: Option<String>,
    pub labels: HashSet<String>,
    pub dependencies: Vec<BeadId>,
    pub blocks: Vec<BeadId>,
    pub notes: Option<String>,
}

pub enum Status {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
    Tombstone,
}

pub enum Priority {
    P0, P1, P2, P3, P4,
}
```

#### `federated_graph.rs`
Aggregated graph across all repositories:
```rust
pub struct FederatedGraph {
    pub beads: HashMap<BeadId, Bead>,
}

impl FederatedGraph {
    pub fn add_bead(&mut self, bead: Bead);
    pub fn get_bead(&self, id: &BeadId) -> Option<&Bead>;
    pub fn ready_beads(&self) -> Vec<&Bead>;
    pub fn blocked_beads(&self) -> Vec<&Bead>;
    pub fn stats(&self) -> GraphStats;
}
```

#### `ids.rs`
Type-safe identifiers:
```rust
pub struct BeadId(String);
pub struct RigId(String);

impl BeadId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
```

### Aggregator (`src/aggregator/`)

#### `boss_aggregator.rs`
Multi-repository aggregation logic:
```rust
pub struct Aggregator {
    config: AllBeadsConfig,
    cache: Option<Cache>,
}

impl Aggregator {
    pub fn new(config: AllBeadsConfig) -> Self;
    pub fn with_cache(self, cache: Cache) -> Self;
    pub fn aggregate(&self) -> Result<FederatedGraph>;
    pub fn aggregate_context(&self, name: &str) -> Result<FederatedGraph>;
}
```

The aggregation process:
1. Load cached graph if available and not expired
2. For each configured context:
   - Clone/fetch repository if needed
   - Parse `.beads/issues.jsonl`
   - Add context tag to each bead's labels
3. Merge all beads into `FederatedGraph`
4. Store in cache

### Git Operations (`src/git/`)

#### `operations.rs`
Git repository operations using `git2`:
```rust
pub fn clone_or_open(url: &str, path: &Path, auth: &AuthStrategy)
    -> Result<Repository>;
pub fn fetch_latest(repo: &Repository, auth: &AuthStrategy)
    -> Result<()>;
pub fn get_beads_content(repo: &Repository)
    -> Result<String>;
```

Authentication flow:
1. For SSH URLs: Use SSH agent
2. For HTTPS URLs:
   - Check `<CONTEXT>_TOKEN` environment variable
   - Check `GITHUB_TOKEN` environment variable
   - Try `gh auth token` (GitHub CLI)

### Storage (`src/storage/`)

#### `jsonl.rs`
Beads JSONL format parsing:
```rust
pub fn parse_jsonl(content: &str) -> Result<Vec<Bead>>;
```

JSONL format (one bead per line):
```json
{"id":"ab-123","title":"Task","status":"open","priority":2,...}
```

#### `conversions.rs`
Type conversions between internal and external formats.

### Cache (`src/cache/`)

#### `sqlite.rs`
SQLite-based cache with TTL:
```rust
pub struct Cache {
    conn: Connection,
    ttl: Duration,
}

impl Cache {
    pub fn new(config: CacheConfig) -> Result<Self>;
    pub fn store_graph(&self, graph: &FederatedGraph) -> Result<()>;
    pub fn load_graph(&self) -> Result<Option<FederatedGraph>>;
    pub fn clear(&self) -> Result<()>;
    pub fn stats(&self) -> Result<CacheStats>;
}
```

Cache location: `~/.cache/allbeads/cache.db`

### TUI (`src/tui/`)

#### `app.rs`
Application state:
```rust
pub struct App {
    pub graph: FederatedGraph,
    pub current_column: Column,
    pub list_state: ListState,
    pub show_detail: bool,
}

pub enum Column {
    Open,
    InProgress,
    Closed,
}
```

#### `ui.rs`
Rendering with `ratatui`:
- Kanban board with three columns
- Bead detail view
- Priority-based coloring
- Keyboard navigation (vim-style + arrows)

### Agent Mail (`src/mail/`)

#### `message.rs`
Message protocol for agent coordination:
```rust
pub struct Message {
    pub id: MessageId,
    pub from: String,           // e.g., "agent@project"
    pub to: String,             // e.g., "human@localhost"
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
}

pub enum MessageType {
    Lock(LockRequest),      // Request file lock
    Unlock(UnlockRequest),  // Release lock
    Notify(NotifyPayload),  // State change notification
    Request(RequestPayload), // Human approval request
    Broadcast(BroadcastPayload), // Announce to all
    Heartbeat(HeartbeatPayload), // Agent liveness
    Response(ResponsePayload),   // Reply to message
}
```

## Data Flow

### Aggregation Flow

```
1. CLI Command (ab list)
        │
        ▼
2. Load Config (~/.config/allbeads/config.yaml)
        │
        ▼
3. Check Cache
        │
   ┌────┴────┐
   │ Valid?  │
   └────┬────┘
    Yes │  No
   ┌────┘  └────┐
   ▼            ▼
4a. Load     4b. Fetch from Git
   from         │
   Cache        ▼
   │        5. Parse JSONL
   │            │
   │            ▼
   │        6. Build Graph
   │            │
   │            ▼
   │        7. Store in Cache
   │            │
   └────────────┤
                ▼
            8. Display Results
```

### TUI Event Loop

```
1. Initialize Terminal
        │
        ▼
2. Load Graph (cached)
        │
        ▼
   ┌─────────────┐
   │ Event Loop  │◄────────┐
   └──────┬──────┘         │
          │                │
          ▼                │
   ┌─────────────┐         │
   │ Wait Event  │         │
   └──────┬──────┘         │
          │                │
    ┌─────┼─────┐          │
    ▼     ▼     ▼          │
  Key   Quit  Resize       │
   │     │     │           │
   │     │     │           │
   ▼     │     ▼           │
Update   │  Redraw         │
State    │     │           │
   │     │     │           │
   └─────┼─────┴───────────┘
         │
         ▼
    Restore Terminal
```

## Error Handling

AllBeads uses `anyhow` for error propagation with context:
```rust
use anyhow::{Context, Result};

fn load_config() -> Result<AllBeadsConfig> {
    let path = config_path()?;
    let content = std::fs::read_to_string(&path)
        .context("Failed to read config file")?;
    // ...
}
```

Error display includes:
- Error chain (cause → context → message)
- Colored output (red for errors)
- Actionable suggestions where possible

## Testing

### Unit Tests
Located in each module with `#[cfg(test)]`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_creation() { ... }
}
```

### Integration Tests
Located in `tests/integration_test.rs`:
- Config creation and persistence
- Graph construction and queries
- Cache store/load cycles
- Message serialization

Run tests:
```bash
cargo test                    # All tests
cargo test --test integration # Integration only
cargo test bead               # Tests matching "bead"
```

## Configuration

### Config File
`~/.config/allbeads/config.yaml`:
```yaml
contexts:
  - name: project-a
    type: git
    url: git@github.com:org/project-a.git
    path: /path/to/project-a
    auth_strategy: ssh_agent

agent_mail:
  port: 8085
  storage: ~/.config/allbeads/mail.db

visualization:
  default_view: kanban
  theme: dark
  refresh_interval: 60
```

### Environment Variables
- `<CONTEXT>_TOKEN` - Token for specific context (e.g., `WORK_TOKEN`)
- `GITHUB_TOKEN` - Fallback GitHub token
- `ALLBEADS_CONFIG` - Custom config path (optional)

## Future Architecture

### Phase 2: The Mailroom
- Postmaster daemon for message routing
- Lock manager for file coordination
- Human inbox for approval requests

### Phase 3: The Sheriff
- Background synchronization daemon
- Shadow Bead creation
- External integration (JIRA, GitHub)

### Phase 4: Enterprise
- Multi-tenant support
- RBAC and audit logging
- High availability

## Dependencies

Key crates:
- `clap` - CLI argument parsing
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `git2` - Git operations
- `rusqlite` - SQLite cache
- `ratatui` - Terminal UI
- `crossterm` - Terminal handling
- `chrono` - Date/time
- `anyhow` / `thiserror` - Error handling
- `tokio` - Async runtime (future)
