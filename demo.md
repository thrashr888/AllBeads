# AllBeads CLI Demo

This guide demonstrates AllBeads CLI commands for multi-repository bead aggregation.

## Configuration

Config file location: `~/.config/allbeads/config.yaml`

## Setup

Setup alias for convenient testing:
```bash
alias ab='cargo run --quiet -- --cached'
```

The `--cached` flag uses cached data without fetching from remotes (faster for testing).

## Quick Command Reference

```bash
# Context management
ab context add <name> <url>      # Add a Boss repository
ab context list                   # List all contexts
ab context remove <name>          # Remove a context

# Viewing beads
ab kanban                         # Launch Kanban board (Terminal UI)
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
ab search "query" --context work  # Search in specific context
ab duplicates                     # Find duplicate beads
ab duplicates --threshold 0.6     # Adjust similarity threshold

# Cache management
ab clear-cache                    # Clear the cache
```

## Context Management

AllBeads aggregates beads from multiple "Boss" repositories (contexts).

### Add a Boss repository

```bash
# Using SSH URL (recommended with ssh_agent)
ab context add work git@github.com:org/boss-work.git

# With custom path
ab context add personal git@github.com:you/boss.git --path ~/repos/boss

# With HTTPS and personal access token
ab context add enterprise https://github.company.com/boss.git --auth personal_access_token
```

**Note:** If you use an HTTPS URL with `ssh_agent` (default), you'll see a warning:

```
⚠️  Warning: Using HTTPS URL with ssh_agent authentication may fail.
   Suggestion: Use SSH URL instead:
   git@github.com:org/boss-work.git

   To add with SSH URL:
   allbeads context add work git@github.com:org/boss-work.git
```

**Best practices:**
- Use SSH URLs (`git@github.com:...`) with `ssh_agent` (default)
- Use HTTPS URLs only with `--auth personal_access_token` or `--auth gh_enterprise_token`

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

  ethertext
    URL:  https://github.com/thrashr888/ethertext.git
    Path: /Users/thrashr888/workspace/ethertext
    Auth: SshAgent
```

### Remove a context

```bash
ab context remove ethertext
```

## Kanban Board

Launch the interactive Kanban dashboard:

```bash
ab kanban
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
- Use `ab kanban` (with `--cached`) for fastest startup
- Navigate between columns to see different workflow stages
- Press Enter on any bead to see full details, dependencies, and description
- Text selection now works - mouse capture disabled

## Viewing Beads

### Show aggregated statistics

View summary across all contexts:

```bash
ab stats
```

Example output:
```
AllBeads Statistics:

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

Full-text search across title, description, notes, and ID:

```bash
# Search all beads
ab search "TUI"

# Search within a specific context
ab search "agent" --context rookery

# Search is case-insensitive
ab search "authentication"
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

### Example: Aggregating three Boss repositories

```bash
# Add multiple contexts
ab context add allbeads https://github.com/thrashr888/AllBeads.git
ab context add qdos https://github.com/thrashr888/QDOS.git
ab context add work https://github.com/org/boss-work.git

# Clear cache and fetch fresh data
ab clear-cache
cargo run --quiet -- stats

# View work from specific context
ab list --context work --status open

# Find all P1 beads across all repos
ab list --priority P1 --status open

# See what's ready to work on
ab ready
```

## Tips

- **Use the alias**: `ab` with `--cached` is fast for repeated commands
- **Fresh data**: Remove `--cached` when you need the latest from remotes
- **Context tags**: All beads are tagged with `@<context-name>` for filtering
- **Cache expiry**: Default is 5 minutes, configured in config.yaml
- **Parallel contexts**: Add as many Boss repos as you need to aggregate
- **Context breakdown**: Use `ab stats` to see bead distribution across contexts
- **SSH vs HTTPS**: Use SSH URLs (`git@github.com:...`) with default `ssh_agent` auth
