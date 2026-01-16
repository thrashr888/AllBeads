# AllBeads CLI Demo

This guide demonstrates AllBeads CLI commands for multi-repository bead aggregation, enterprise integration, and agent coordination.

## Configuration

Config file location: `~/.config/allbeads/config.yaml`

## Setup

### Initialize AllBeads

First, initialize the configuration:

```bash
# Initialize AllBeads (creates ~/.config/allbeads/config.yaml)
ab init

# Or clone an existing Boss repository
ab init --remote git@github.com:org/boss-repo.git
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
ab init --remote <url>            # Clone existing Boss repo

# Context management
ab context add <path>             # Add repo (infers name/URL from git)
ab context add . --url <url>      # Add current dir with explicit URL
ab context list                   # List all contexts
ab context list --local           # Only contexts with local paths
ab context list --beads           # Only contexts with beads initialized
ab context list --names           # One name per line (for scripting)
ab context onboarding             # Show onboarding status
ab context onboarding --format=csv  # Export as CSV
ab context remove <name>          # Remove a context
ab open owner/repo#123            # Open GitHub issue in browser
ab open PROJ-123                  # Open JIRA issue in browser

# Viewing beads
ab tui                            # Launch TUI (Kanban + Mail)
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

# Sheriff daemon
ab sheriff --foreground           # Run sync daemon
ab sheriff -f -p 10               # Custom poll interval

# Agent mail
ab mail send --to <addr> --subject "..." --body "..."
ab mail list                      # List messages
ab mail unread                    # Check unread count

# Janitor analysis
ab janitor <path>                 # Analyze repo for issues
ab janitor <path> --dry-run       # Preview without creating

# Enterprise integration
ab jira status                    # Check JIRA config
ab jira pull -p PROJ -u <url>     # Pull JIRA issues
ab github status                  # Check GitHub config
ab github pull -o <owner>         # Pull GitHub issues

# Cache management
ab clear-cache                    # Clear the cache

# Plugin system (v0.2)
ab plugin list                    # List plugins
ab plugin recommend               # Get recommendations
ab plugin marketplace-list        # Show marketplaces

# Coding agents (v0.2)
ab agent list                     # List configured agents
ab agent detect                   # Detect agents in project
ab agent init <agent>             # Initialize agent config
ab agent sync                     # Sync context to agents

# Sync (v0.2)
ab sync                           # Sync config
ab sync --all                     # Sync all context beads
ab sync --status                  # Check sync status
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

# Filter to contexts with local paths
ab context list --local

# Filter to contexts with beads initialized
ab context list --beads

# Output just names (for scripting)
ab context list --names
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

### Check onboarding status

```bash
# Show onboarding status for all contexts
ab context onboarding

# Filter to repos with beads initialized
ab context onboarding --beads

# Export as CSV for reporting
ab context onboarding --format=csv

# Export as JSON
ab context onboarding --format=json
```

## TUI Dashboard

Launch the interactive dashboard with Kanban, Mail, Graph, and Swarm views:

```bash
ab tui
```

**Features:**

- **Kanban Board**: Three columns (Open, In Progress, Closed)
- **Mail View**: Agent message inbox
- **Graph View**: Dependency chain visualization with cross-context analysis
- **Swarm View**: Real-time agent status monitoring (when agents are active)
- **Color-Coded Priorities**: P0 (red) through P4 (gray)
- **Context Tags**: Shows which repo each bead is from (@allbeads, @qdos, etc.)
- **Vim Navigation**: j/k for up/down, h/l for column switching

**Keybindings:**

```
Tab        Switch between views (Kanban -> Mail -> Graph -> Swarm)
j / Down   Move down
k / Up     Move up
h / Left   Previous column (Kanban only)
l / Right  Next column (Kanban only)
Enter      Toggle detail view
Esc        Close detail view
f          Cycle filter (Graph view: All -> Blocked -> Cross-Context)
p          Pause agent (Swarm view)
r          Resume agent (Swarm view) / Mark read (Mail view)
x          Kill agent (Swarm view)
q          Quit
Ctrl+C     Quit
```

### Graph View

The Graph view visualizes dependency chains across all contexts:

- **⟳ Cycle detected**: Red indicator for circular dependencies
- **⬡ Cross-context**: Magenta indicator for dependencies spanning multiple repos
- **⊘ Blocked**: Yellow indicator for beads with blockers
- **○ Normal**: Green indicator for healthy dependency chains

Use `f` to cycle through filter modes:
- **All**: Show all dependency chains
- **Blocked Only**: Show only chains with active blockers
- **Cross-Context**: Show only chains that span multiple contexts

## Sheriff Daemon

The Sheriff daemon synchronizes beads across repositories and external systems.

### Run in foreground

```bash
# Basic foreground mode (recommended for development)
ab sheriff --foreground

# With custom poll interval (seconds)
ab sheriff --foreground --poll-interval 10

# Short form
ab sheriff -f -p 10
```

### Event output

When running in foreground, the Sheriff prints sync events:

```
[2026-01-10 12:00:00] Starting Sheriff daemon...
[2026-01-10 12:00:00] Poll cycle started
[2026-01-10 12:00:01] Synced rig 'auth-service': 3 shadows updated
[2026-01-10 12:00:02] External sync: 5 JIRA issues pulled
[2026-01-10 12:00:02] Poll cycle complete (2.1s)
```

## Agent Mail

The Agent Mail system enables messaging between agents.

### Send messages

```bash
# Send a notification
ab mail send --to agent-1 --subject "Task Complete" --body "Finished the auth refactor"

# Send to broadcast address (all agents)
ab mail send --to broadcast --subject "Announcement" --body "Deploying v2.0"
```

### List messages

```bash
# List messages for human inbox
ab mail list
```

Example output:

```
Messages for human:

[2026-01-10 11:30:00] From: agent-1
  Subject: Task Update
  Body: Completed auth-service refactor...

[2026-01-10 10:15:00] From: agent-2
  Subject: Help Request
  Body: Need clarification on API design...
```

### Check unread count

```bash
ab mail unread
```

## Janitor Analysis

The Janitor analyzes repositories and discovers potential issues.

### Analyze a repository

```bash
# Full analysis with issue creation
ab janitor /path/to/repo

# Dry run (preview what would be created)
ab janitor /path/to/repo --dry-run
```

### Analysis output

```
Analyzing repository: /path/to/repo

Found 5 potential issues:

[bug] Large file detected: data/backup.sql (150MB)
  Recommendation: Add to .gitignore or use Git LFS

[task] Missing README in src/utils/
  Recommendation: Add documentation

[chore] Outdated dependency: lodash@3.10.1
  Recommendation: Update to latest version

Would create 5 issues (--dry-run mode)
```

## Enterprise Integration

### JIRA Integration

```bash
# Check JIRA configuration status
ab jira status
```

Output:

```
JIRA Integration Status

  API Token: Not set

To configure JIRA integration:
  1. Create an API token at: https://id.atlassian.com/manage/api-tokens
  2. Set the environment variable:
     export JIRA_API_TOKEN='your-api-token'

Usage:
  ab jira pull --project PROJ --url https://company.atlassian.net
```

```bash
# Pull issues from JIRA
export JIRA_API_TOKEN='your-token'
ab jira pull --project PROJ --url https://company.atlassian.net --label ai-agent

# With verbose output
ab jira pull -p PROJ -u https://company.atlassian.net --verbose
```

Output:

```
Pulling issues from JIRA project PROJ with label 'ai-agent'...

Found 3 issues:

[High] [In Progress] PROJ-123: Implement OAuth flow
[Medium] [Open] PROJ-456: Add rate limiting
[Low] [Open] PROJ-789: Update documentation
```

### GitHub Integration

```bash
# Check GitHub configuration status
ab github status
```

Output:

```
GitHub Integration Status

  API Token: Set (GITHUB_TOKEN or GH_TOKEN)

To configure GitHub integration:
  1. Create a personal access token at: https://github.com/settings/tokens
     (requires 'repo' scope for private repos, 'public_repo' for public)
  2. Set the environment variable:
     export GITHUB_TOKEN='your-personal-access-token'

Usage:
  ab github pull --owner myorg
  ab github pull --owner myorg --repo myrepo
```

```bash
# Pull issues from GitHub organization
ab github pull --owner myorg

# Pull from specific repository
ab github pull --owner myorg --repo myrepo --label ai-agent

# With verbose output
ab github pull -o myorg --verbose
```

Output:

```
Pulling issues from GitHub myorg/all repositories with label 'ai-agent'...

Found 5 issues:

[O] myorg/api#123: Add authentication middleware [backend, ai-agent]
[O] myorg/web#456: Implement dark mode [frontend, ai-agent]
[C] myorg/api#789: Fix memory leak [bug, ai-agent]
```

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
```

Available statuses: `open`, `in_progress`, `blocked`, `deferred`, `closed`, `tombstone`

### Filter by priority

```bash
# Show P1 beads
ab list --priority P1

# Show critical P0 beads
ab list --priority P0
```

Priority levels: `P0` (critical) through `P4` (backlog)

### Combine filters

```bash
# Show open P1 beads in the work context
ab list --status open --priority P1 --context work
```

### Show ready-to-work beads

```bash
ab ready
```

### Show blocked beads

```bash
ab blocked
```

Example output:

```
Blocked beads: 47

[P0] [open] et-9ku: Create RookeryClient.swift with auth flow (@ethertext)
  Blocked by: et-6tl
[P1] [open] ab-oqy: Implement basic TUI with ratatui (@allbeads)
  Blocked by: ab-8ik
```

### Search beads

```bash
# Basic text search
ab search "TUI"

# Filter by status
ab search --status open

# Negate filters with ^ prefix
ab search --status=^closed

# Filter by priority range
ab search --priority-min P0 --priority-max P2

# Sort results
ab search --status open --sort priority
ab search --sort created --reverse

# Combine filters
ab search "database" --context work --status open --priority-min P1 --sort updated
```

### Find duplicate beads

```bash
# Default threshold (80% similarity)
ab duplicates

# Lower threshold
ab duplicates --threshold 0.6
```

### Show bead details

```bash
ab show ab-oqy
```

Example output:

```
ab-oqy: Implement basic TUI with ratatui
Status:       open
Priority:     P1
Type:         Task
Created:      2026-01-09T13:45:06 by thrashr888
Updated:      2026-01-10T00:45:22
Labels:       @allbeads
Depends on:   ab-8ik

Description:
Basic terminal UI showing: Kanban board with lanes (Open/InProgress/Closed),
Bead detail view, Simple filtering by context
```

## Cache Management

### Clear cache

```bash
ab clear-cache
```

### Use cached vs fresh data

```bash
# With cache (faster)
ab --cached stats

# Without cache (fresh data)
ab stats
```

Cache expires after 5 minutes by default.

## Debugging

### Enable logging

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
RUST_LOG=allbeads::aggregator=debug cargo run -- stats
```

## Multi-Repository Workflow

### Example: Aggregating multiple Boss repositories

```bash
# Initialize AllBeads first
ab init

# Add repositories
cd ~/workspace/AllBeads && ab context add .
cd ~/workspace/QDOS && ab context add .
cd ~/workspace/work-project && ab context add . --name work

# View aggregated stats
ab stats

# View work from specific context
ab list --context work --status open

# Find all P1 beads across all repos
ab list --priority P1 --status open

# Launch TUI
ab tui
```

## Plugin System (v0.2)

AllBeads integrates with the Claude plugin ecosystem for extensibility.

### List plugins

```bash
# List installed plugins
ab plugin list

# List all available plugins (including not installed)
ab plugin list --all

# Filter by category
ab plugin list --category beads
```

### Get recommendations

```bash
# Analyze project and get plugin recommendations
ab plugin recommend

# Specify project path
ab plugin recommend /path/to/project
```

Example output:

```
Plugin Recommendations

  Project: /Users/you/workspace/my-project

  Project Analysis

  Languages: rust
  Type: Monorepo
  Git: ✓  Beads: ✓

  Recommended Plugins

  · ███ mcp-github - GitHub integration via MCP
      → Found config file: .github (35% confidence)
  · █░░ beads - Git-backed issue tracker with dependencies
      → Recommended for all projects (20% confidence)

  Legend: ✓ configured  ○ installed  · not installed
          ███ high  ██░ medium  █░░ low confidence
```

### Marketplace

```bash
# List registered marketplaces
ab plugin marketplace-list

# Sync marketplace metadata
ab plugin marketplace-sync
```

## Coding Agents (v0.2)

AllBeads supports multiple coding agents (Claude Code, Cursor, GitHub Copilot, Aider).

### Detect agents

```bash
# Detect which agents are configured
ab agent detect
```

Example output:

```
Agent Detection

  Project: /Users/you/workspace/my-project

  ✓ Claude Code (CLAUDE.md) [synced]
  · Cursor (not configured)
  · GitHub Copilot (not configured)
  · Aider (not configured)

  Tip: Use 'ab agent init <agent>' to configure an agent.
```

### List agents

```bash
# List all agents with status
ab agent list

# JSON output
ab agent list --json
```

### Initialize agents

```bash
# Initialize Claude Code configuration
ab agent init claude

# Initialize Cursor rules
ab agent init cursor

# Initialize GitHub Copilot instructions
ab agent init copilot

# Initialize Aider configuration
ab agent init aider
```

### Sync context

```bash
# Sync AllBeads context to all configured agents
ab agent sync

# Sync to specific agent only
ab agent sync --agent claude
```

### Preview configuration

```bash
# Preview what a config would look like
ab agent preview cursor
```

## Sync (v0.2)

Unified synchronization for AllBeads config and context beads.

```bash
# Sync AllBeads config directory (if tracked in git)
ab sync

# Sync all context beads (runs bd sync in each context)
ab sync --all

# Sync specific context
ab sync mycontext

# Check sync status without syncing
ab sync --status

# With commit message
ab sync --message "Updated config"
```

## Tips

- **Use the alias**: `ab` with `--cached` is fast for repeated commands
- **Fresh data**: Remove `--cached` when you need the latest from remotes
- **Context tags**: All beads are tagged with `@<context-name>` for filtering
- **Sheriff foreground**: Use `--foreground` during development to see sync events
- **Environment tokens**: Set `JIRA_API_TOKEN` and `GITHUB_TOKEN` for integrations
- **Dry run**: Use `--dry-run` with janitor to preview before creating issues
- **Plugin recommendations**: Run `ab plugin recommend` to discover useful plugins
- **Agent sync**: Use `ab agent sync` to keep agent configs updated with beads info
