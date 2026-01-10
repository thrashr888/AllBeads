# AllBeads CLI Demo

This guide demonstrates AllBeads CLI commands for multi-repository bead aggregation.

## Configuration

Config file location: `~/.config/allbeads/config.yaml`

## Setup

### Initialize AllBeads

First, initialize the configuration:

```bash
# Initialize AllBeads (creates ~/.config/allbeads/config.yaml)
ab init
```

### Setup alias

Setup alias for convenient testing:

```bash
alias ab='cargo run --quiet -- --cached'
# Or for release build:
alias ab='./target/release/allbeads --cached'
```

The `--cached` flag uses cached data without fetching from remotes (faster for testing).

## Quick Command Reference

```bash
# Initialization
ab init                           # Initialize config file

# Context management
ab context add <path>             # Add repo (infers name/URL from git)
ab context add . --url <url>      # Add current dir with explicit URL
ab context list                   # List all contexts
ab context remove <name>          # Remove a context

# Viewing beads
ab tui                               # Launch TUI (Kanban + Mail)
ab stats                          # Show aggregated statistics
ab list                           # List all beads
ab list --status open             # Filter by status
ab list --priority P1             # Filter by priority
ab list --context work            # Filter by context
ab ready                          # Show ready-to-work beads
ab blocked                        # Show blocked beads
ab show <id>                      # Show bead details

# Searching and analysis
ab search "query"                 # Full-text search
ab search --status open           # Filter by status
ab search --type epic             # Filter by type
ab search --priority-min P0       # Filter by priority
ab search --sort updated -r       # Sort by updated, reverse
ab duplicates                     # Find duplicate beads
ab duplicates --threshold 0.6     # Adjust similarity threshold

# Cache management
ab clear-cache                    # Clear the cache
```

## Context Management

AllBeads aggregates beads from multiple "Boss" repositories (contexts).

### Add a Boss repository

The easiest way to add a repository is from within the repo directory:

```bash
# Add current directory (auto-detects name from folder, URL from git remote)
cd ~/workspace/my-project
ab context add .

# Add with explicit URL (useful if remote isn't set)
ab context add . --url git@github.com:org/my-project.git

# Add with explicit name
ab context add . --name work

# Add a different directory
ab context add ~/workspace/another-project
```

**Authentication is auto-detected:**

- SSH URLs (`git@github.com:...`) use `ssh_agent`
- HTTPS URLs use `personal_access_token` (reads from `GITHUB_TOKEN` or `gh auth token`)

**Manual authentication override:**

```bash
# Force SSH agent
ab context add . --auth ssh_agent

# Force personal access token
ab context add . --auth personal_access_token
```

**Token resolution order for HTTPS:**

1. `<CONTEXT_NAME>_TOKEN` environment variable (e.g., `WORK_TOKEN`)
2. `GITHUB_TOKEN` environment variable
3. `gh auth token` (GitHub CLI)

### List all configured contexts

```bash
ab context list
```

Example output:

```
Configured contexts (3):

  allbeads
    URL:  https://github.com/thrashr888/AllBeads.git
    Path: /Users/thrashr888/workspace/AllBeads
    Auth: SshAgent

  qdos
    URL:  https://github.com/thrashr888/QDOS.git
    Path: /Users/thrashr888/workspace/QDOS
    Auth: SshAgent
```

### Remove a context

```bash
ab context remove ethertext
```

## Kanban Board (TUI)

Launch the interactive Kanban dashboard:

```bash
ab tui
```

**Features:**

- **Kanban Board**: Three columns (Open, In Progress, Closed)
- **Color-Coded Priorities**: P0 (red) through P4 (gray)
- **Context Tags**: Shows which repo each bead is from (@allbeads, @qdos, etc.)
- **Vim Navigation**: j/k for up/down, h/l for column switching
- **Detail View**: Press Enter to see full bead information
- **Read-Only**: View and navigate beads (editing requires Phase 2)

**Keybindings:**

```
j / ↓      Move down
k / ↑      Move up
h / ←      Previous column
l / →      Next column
Enter      Toggle detail view
Esc        Close detail view
q          Quit
Ctrl+C     Quit
```

**Help Footer:**
The bottom of the screen shows available keybindings and indicates `[READ-ONLY]` mode.

**Tips:**

- Use `ab tui` (with `--cached`) for fastest startup
- Navigate between columns to see different workflow stages
- Press Enter on any bead to see full details, dependencies, and description
- Text selection now works - mouse capture disabled

## Viewing Beads

### Show aggregated statistics

View summary across all contexts:

```bash
ab stats
```

Example output (colors shown in terminal):

```
📊 AllBeads Statistics:

  Total beads:      374
  Total shadows:    0
  Total rigs:       4

  Open:             84
  In Progress:      1
  Blocked:          0
  Closed:           285

Contexts:
  allbeads        47 beads (23 open)
  ethertext       27 beads (13 open)
  qdos            252 beads (25 open)
  rookery         48 beads (23 open)

Cache:
  Beads cached:     374
  Rigs cached:      4
  Cache age:        222.2s
  Expired:          false
```

### List all beads

```bash
# List all beads from all contexts
ab list

# List without using cache (fetch fresh data)
cargo run --quiet -- list
```

### Filter by status

```bash
# Show open beads
ab list --status open

# Show in-progress work
ab list --status in_progress

# Show blocked beads
ab list --status blocked

# Show closed beads
ab list --status closed
```

Available statuses: `open`, `in_progress`, `blocked`, `deferred`, `closed`, `tombstone`

### Filter by priority

```bash
# Show P1 beads (using priority name)
ab list --priority P1

# Show priority 2 beads (using number)
ab list --priority 2

# Show critical P0 beads
ab list --priority P0
```

Priority levels: `P0` (critical) through `P4` (backlog), or use numbers `0-4`.

### Filter by context

When you have multiple Boss repositories, filter by context:

```bash
# Show beads from allbeads context
ab list --context allbeads

# Show beads from qdos context
ab list --context qdos

# Show beads from work context
ab list --context work
```

Beads are automatically tagged with `@<context-name>` labels.

### Combine filters

```bash
# Show open P1 beads in the work context
ab list --status open --priority P1 --context work

# Show all open beads across contexts
ab list --status open
```

### Show ready-to-work beads

Find beads that are ready to work on (open status, no blockers):

```bash
ab ready
```

### Show blocked beads

View all beads that are blocked by dependencies:

```bash
ab blocked
```

Example output shows what each bead is blocked by:

```
Blocked beads: 47

[P0] [open] et-9ku: Create RookeryClient.swift with auth flow (@ethertext)
  → Blocked by: et-6tl
[P1] [open] ab-oqy: Implement basic TUI with ratatui (@allbeads)
  → Blocked by: ab-8ik
```

### Search beads

Powerful search with filters across all contexts:

```bash
# Basic text search
ab search "TUI"

# Search within a specific context
ab search "agent" --context rookery

# Filter by status
ab search --status open
ab search "bug" --status in_progress

# Filter by priority range
ab search --priority-min P0 --priority-max P2
ab search "critical" --priority-min 0 --priority-max 1

# Filter by type
ab search --type epic
ab search --type feature --status open

# Filter by label
ab search --label backend --label urgent

# Filter by assignee
ab search --assignee alice

# Sort results
ab search --status open --sort priority
ab search --sort created --reverse
ab search --sort title -n 20

# Combine filters for powerful queries
ab search "database" --context work --status open --priority-min P1 --sort updated

# Available sort fields: priority, created, updated, status, id, title, type
# Available statuses: open, in_progress, blocked, deferred, closed
# Available types: bug, feature, task, epic, chore
```

### Find duplicate beads

Detect potential duplicates across all contexts:

```bash
# Use default threshold (80% similarity)
ab duplicates

# Lower threshold to find more potential matches
ab duplicates --threshold 0.6

# Higher threshold for stricter matching
ab duplicates --threshold 0.9
```

Example output:

```
Potential duplicates (threshold: 80%): 5 pairs

Similarity: 100%
  QDOS-pcm: Refactor Beads integration as plugin
  QDOS-duu: Refactor Beads integration as plugin

Similarity: 75%
  AllBeads-06z: Implement core data structures
  ab-0lo: Core Data Structures
```

### Show bead details

View full information about a specific bead:

```bash
ab show ab-oqy
```

Example output:

```
ab-oqy: Implement basic TUI with ratatui
Status:       open
Priority:     P1
Type:         Task
Created:      2026-01-09T13:45:06.638459-08:00 by thrashr888
Updated:      2026-01-10T00:45:22.390809+00:00
Labels:       @allbeads
Depends on:   ab-8ik

Description:
Basic terminal UI showing: Kanban board with lanes (Open/InProgress/Closed),
Bead detail view, Simple filtering by context
```

## Cache Management

### Clear cache

Force a fresh aggregation on the next command:

```bash
ab clear-cache
```

### View cache status

The `stats` command shows cache age and expiration status.

### Use cached data

The alias includes `--cached` by default for faster commands. To fetch fresh data:

```bash
# Without cache
cargo run --quiet -- stats

# With cache (faster)
ab stats
```

Cache expires after 5 minutes by default.

## Debugging

### Enable logging

To see INFO/DEBUG logs, set `RUST_LOG`:

```bash
# Show INFO level logs
RUST_LOG=info cargo run --quiet -- stats

# Show DEBUG level logs
RUST_LOG=debug cargo run --quiet -- list

# Show only AllBeads logs at debug level
RUST_LOG=allbeads=debug cargo run --quiet -- stats
```

### Verbose git operations

```bash
# See git clone/fetch operations
RUST_LOG=allbeads::aggregator=debug cargo run -- stats
```

## Multi-Repository Workflow

### Example: Aggregating multiple Boss repositories

```bash
# Initialize AllBeads first
ab init

# Add repositories from their directories
cd ~/workspace/AllBeads
ab context add .

cd ~/workspace/QDOS
ab context add .

cd ~/workspace/work-project
ab context add . --name work

# Or add from anywhere with explicit paths
ab context add ~/workspace/AllBeads
ab context add ~/workspace/QDOS
ab context add ~/workspace/work-project --name work

# View aggregated stats across all repos
ab stats

# View work from specific context
ab list --context work --status open

# Find all P1 beads across all repos
ab list --priority P1 --status open

# See what's ready to work on
ab ready

# Launch Kanban board
ab tui
```

## Tips

- **Use the alias**: `ab` with `--cached` is fast for repeated commands
- **Fresh data**: Remove `--cached` when you need the latest from remotes
- **Context tags**: All beads are tagged with `@<context-name>` for filtering
- **Cache expiry**: Default is 5 minutes, configured in config.yaml
- **Parallel contexts**: Add as many Boss repos as you need to aggregate
- **Context breakdown**: Use `ab stats` to see bead distribution across contexts
- **SSH vs HTTPS**: Use SSH URLs (`git@github.com:...`) with default `ssh_agent` auth
