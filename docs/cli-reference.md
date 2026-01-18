# CLI Reference

Complete reference for all AllBeads commands, organized by category.

> **Note**: Examples use `ab` (the recommended alias). The full binary name is `allbeads`.

## Global Options

These options apply to all commands:

| Option | Description |
|--------|-------------|
| `--cached` | Use cached data only (faster, may be stale) |
| `--help` | Show help for any command |
| `--version` | Show version information |

## Initialization

### `ab init`

Initialize AllBeads configuration.

```bash
# Create default config
ab init

# Clone existing Boss repository
ab init --remote git@github.com:org/boss-repo.git
ab init --remote https://github.com/org/boss-repo.git
```

| Option | Description |
|--------|-------------|
| `--remote <url>` | Clone existing Boss repo instead of creating new config |

## Context Management

### `ab context add`

Add a repository as a context.

```bash
# Add current directory (auto-detects name, URL, auth)
ab context add .

# Add with explicit options
ab context add /path/to/repo --name myproject --url git@github.com:org/repo.git

# Specify authentication strategy
ab context add . --auth ssh_agent
ab context add . --auth personal_access_token
```

| Option | Description |
|--------|-------------|
| `--name <name>` | Override the context name |
| `--url <url>` | Override the git remote URL |
| `--auth <strategy>` | Authentication: `ssh_agent` or `personal_access_token` |

### `ab context list`

List configured contexts.

```bash
# List all contexts
ab context list

# Filter options
ab context list --local      # Only contexts with local paths
ab context list --beads      # Only contexts with beads initialized
ab context list --names      # One name per line (for scripting)
```

| Option | Description |
|--------|-------------|
| `--local` | Show only contexts with local paths |
| `--beads` | Show only contexts with beads initialized |
| `--names` | Output names only, one per line |

### `ab context remove`

Remove a context from configuration.

```bash
ab context remove myproject
```

### `ab context onboarding`

Show onboarding status for all contexts.

```bash
ab context onboarding
ab context onboarding --beads           # Filter to repos with beads
ab context onboarding --format=csv      # Export as CSV
ab context onboarding --format=json     # Export as JSON
```

| Option | Description |
|--------|-------------|
| `--beads` | Filter to repos with beads initialized |
| `--format <format>` | Output format: `table` (default), `csv`, `json` |

## Viewing Beads

### `ab stats`

Show aggregated statistics across all contexts.

```bash
ab stats
ab stats --remote    # Fetch from web API instead of local
```

| Option | Description |
|--------|-------------|
| `--remote` | Fetch stats from AllBeads Web API |

### `ab list`

List beads from all contexts.

```bash
# Basic listing
ab list

# Filter by status
ab list --status open
ab list --status in_progress
ab list --status blocked
ab list --status closed

# Filter by priority
ab list --priority P0    # Critical
ab list --priority P1    # High
ab list --priority P2    # Medium
ab list --priority P3    # Low
ab list --priority P4    # Backlog

# Filter by type
ab list --type epic
ab list --type feature
ab list --type task
ab list --type bug
ab list --type chore

# Filter by context
ab list --context myproject
ab list -C myproject,otherproject    # Multiple contexts

# Filter by assignee
ab list --assignee thrashr888

# Additional filters
ab list --local     # Current directory only (skip aggregation)
ab list --ready     # Only unblocked beads
ab list --all       # Include closed beads

# Limit results
ab list --limit 10
ab list -n 0        # Unlimited

# Combine filters
ab list --status open --priority P1 --type feature -n 20
```

| Option | Short | Description |
|--------|-------|-------------|
| `--status <status>` | | Filter by status |
| `--priority <priority>` | | Filter by priority (P0-P4) |
| `--type <type>` | | Filter by type |
| `--context <contexts>` | `-C` | Filter by context(s), comma-separated |
| `--assignee <name>` | | Filter by assignee |
| `--local` | | Current directory only |
| `--ready` | | Only unblocked beads |
| `--all` | `-a` | Include closed beads |
| `--limit <n>` | `-n` | Limit results (default: 50, 0 = unlimited) |

### `ab ready`

Show beads that are ready to work (no blockers).

```bash
ab ready
```

### `ab blocked`

Show blocked beads with their blockers.

```bash
ab blocked
```

### `ab show`

Show detailed information about a bead.

```bash
ab show ab-123
```

### `ab search`

Search beads by text and filters.

```bash
# Text search
ab search "authentication"
ab search "OAuth flow"

# Filter by attributes
ab search --status open
ab search --type feature
ab search --priority-min P0 --priority-max P2

# Negate filters with ^ prefix
ab search --status=^closed

# Sort results
ab search --sort priority
ab search --sort created --reverse
ab search --sort updated -r

# Combine all
ab search "database" --context work --status open --priority-min P1 --sort updated
```

| Option | Description |
|--------|-------------|
| `--status <status>` | Filter by status (prefix with ^ to negate) |
| `--type <type>` | Filter by type |
| `--priority-min <p>` | Minimum priority |
| `--priority-max <p>` | Maximum priority |
| `--context <name>` | Filter by context |
| `--sort <field>` | Sort by: priority, created, updated |
| `--reverse`, `-r` | Reverse sort order |

### `ab duplicates`

Find potential duplicate beads.

```bash
ab duplicates
ab duplicates --threshold 0.6    # Lower similarity threshold (default: 0.8)
```

| Option | Description |
|--------|-------------|
| `--threshold <n>` | Similarity threshold 0.0-1.0 (default: 0.8) |

### `ab open`

Open an issue in the browser.

```bash
ab open owner/repo#123    # GitHub issue
ab open PROJ-123          # JIRA issue
```

## Comments

### `ab comments list`

List comments on a bead.

```bash
ab comments list ab-123
ab comments list ab-123 --remote    # From web API
```

### `ab comments add`

Add a comment to a bead.

```bash
ab comments add ab-123 "This is my comment"
ab comments add ab-123 "Comment text" --remote    # Via web API
```

## TUI Dashboard

### `ab tui`

Launch the interactive terminal UI.

```bash
ab tui
```

**Views:**
- **Kanban** - Three-column board (Open, In Progress, Closed)
- **Mail** - Agent message inbox
- **Graph** - Dependency chain visualization
- **Swarm** - Active agent monitoring

**Keybindings:**

| Key | Action |
|-----|--------|
| `Tab` | Switch between views |
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `h` / `Left` | Previous column (Kanban) |
| `l` / `Right` | Next column (Kanban) |
| `Enter` | Toggle detail view |
| `Esc` | Close detail view |
| `f` | Cycle filter (Graph: All/Blocked/Cross-Context) |
| `p` | Pause agent (Swarm) |
| `r` | Resume agent (Swarm) / Mark read (Mail) |
| `x` | Kill agent (Swarm) |
| `q` | Quit |

## Sheriff Daemon

### `ab sheriff`

Run the synchronization daemon.

```bash
# Foreground mode (recommended for development)
ab sheriff --foreground
ab sheriff -f

# Custom poll interval (seconds)
ab sheriff -f --poll-interval 10
ab sheriff -f -p 10

# With specific manifest
ab sheriff --manifest manifests/work.xml -f
```

| Option | Short | Description |
|--------|-------|-------------|
| `--foreground` | `-f` | Run in foreground (show events) |
| `--poll-interval <secs>` | `-p` | Poll interval in seconds |
| `--manifest <path>` | `-m` | Use specific manifest file |

## Agent Mail

### `ab mail send`

Send a message.

```bash
ab mail send --to agent-1 --subject "Task done" --body "Completed the auth work"
ab mail send --to broadcast --subject "Deploy" --body "Deploying v2.0"
```

| Option | Description |
|--------|-------------|
| `--to <address>` | Recipient address (required) |
| `--subject <text>` | Message subject (required) |
| `--body <text>` | Message body (required) |

### `ab mail list`

List messages in inbox.

```bash
ab mail list
```

### `ab mail unread`

Check unread message count.

```bash
ab mail unread
```

### `ab mail read`

Mark message(s) as read.

```bash
ab mail read <message-id>
ab mail read --all
```

### `ab mail archive`

Archive message(s).

```bash
ab mail archive <message-id>
ab mail archive --read    # Archive all read messages
```

### `ab mail delete`

Delete a message.

```bash
ab mail delete <message-id>
```

## Janitor Analysis

### `ab janitor`

Analyze a repository for potential issues.

```bash
ab janitor /path/to/repo
ab janitor /path/to/repo --dry-run    # Preview without creating issues
```

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be created without creating |

## Enterprise Integration

### `ab jira status`

Check JIRA integration configuration.

```bash
ab jira status
```

### `ab jira pull`

Pull issues from JIRA.

```bash
export JIRA_API_TOKEN='your-token'
ab jira pull --project PROJ --url https://company.atlassian.net
ab jira pull -p PROJ -u https://company.atlassian.net --label ai-agent
ab jira pull -p PROJ -u https://company.atlassian.net --verbose
```

| Option | Short | Description |
|--------|-------|-------------|
| `--project <key>` | `-p` | JIRA project key (required) |
| `--url <url>` | `-u` | JIRA instance URL (required) |
| `--label <label>` | | Filter by label |
| `--verbose` | | Show detailed output |

### `ab github status`

Check GitHub integration configuration.

```bash
ab github status
```

### `ab github pull`

Pull issues from GitHub.

```bash
ab github pull --owner myorg
ab github pull --owner myorg --repo myrepo
ab github pull -o myorg --label ai-agent
ab github pull -o myorg --verbose
```

| Option | Short | Description |
|--------|-------|-------------|
| `--owner <name>` | `-o` | GitHub organization/user (required) |
| `--repo <name>` | | Specific repository |
| `--label <label>` | | Filter by label |
| `--verbose` | | Show detailed output |

## Plugin System

### `ab plugin list`

List plugins.

```bash
ab plugin list
ab plugin list --all         # Include not-installed
ab plugin list --category beads
```

| Option | Description |
|--------|-------------|
| `--all` | Include not-installed plugins |
| `--category <cat>` | Filter by category |

### `ab plugin recommend`

Get plugin recommendations for a project.

```bash
ab plugin recommend
ab plugin recommend /path/to/project
```

### `ab plugin info`

Show information about a plugin.

```bash
ab plugin info beads
```

### `ab plugin marketplace-list`

List registered marketplaces.

```bash
ab plugin marketplace-list
```

### `ab plugin marketplace-sync`

Sync marketplace metadata.

```bash
ab plugin marketplace-sync
```

## Coding Agents

### `ab agent list`

List configured agents with status.

```bash
ab agent list
ab agent list --json
```

### `ab agent detect`

Detect which agents are configured in a project.

```bash
ab agent detect
```

### `ab agent init`

Initialize agent configuration.

```bash
ab agent init claude     # CLAUDE.md
ab agent init cursor     # .cursorrules
ab agent init copilot    # GitHub Copilot
ab agent init aider      # Aider
```

### `ab agent sync`

Sync AllBeads context to agent configs.

```bash
ab agent sync
ab agent sync --agent claude    # Specific agent only
```

### `ab agent preview`

Preview what agent configuration would look like.

```bash
ab agent preview cursor
```

## Agent Handoff

### `ab handoff`

Hand off a bead to an AI agent.

```bash
# Use preferred agent (prompts on first use)
ab handoff ab-123

# Specify agent
ab handoff ab-123 --agent gemini
ab handoff ab-123 --agent cursor
ab handoff ab-123 --agent jules    # Opens browser

# Use isolated worktree
ab handoff ab-123 --worktree

# Preview
ab handoff ab-123 --dry-run
```

**Utility options:**

```bash
ab handoff --agents    # List available agents
ab handoff --list      # Show handed-off beads
ab handoff --ready     # Show ready beads for handoff
```

| Option | Description |
|--------|-------------|
| `--agent <name>` | Specific agent to use |
| `--worktree` | Use isolated git worktree |
| `--dry-run` | Show what would happen |
| `--agents` | List available agents |
| `--list` | Show handed-off beads |
| `--ready` | Show ready beads |

**Supported Agents:**
- **CLI**: claude, opencode, codex, gemini, aider, cody
- **IDE**: cursor, kiro, antigravity, copilot
- **Web**: jules (Google), chatgpt-codex

## Governance

### `ab governance check`

Check policies against current repository.

```bash
ab governance check
```

### `ab governance status`

View loaded policies and exemptions.

```bash
ab governance status
```

### `ab governance violations`

List policy violations.

```bash
ab governance violations
```

### `ab governance exempt`

Exempt a repository from a policy.

```bash
ab governance exempt my-repo policy-name --reason "Legacy codebase"
```

### `ab governance unexempt`

Remove an exemption.

```bash
ab governance unexempt my-repo policy-name
```

## GitHub Scanning

### `ab scan user`

Scan user's GitHub repositories.

```bash
ab scan user thrashr888
ab scan user thrashr888 --language rust --min-stars 10
```

### `ab scan org`

Scan organization repositories.

```bash
ab scan org my-company
```

### `ab scan compare`

Compare scanned repos with managed contexts.

```bash
ab scan compare
```

| Option | Description |
|--------|-------------|
| `--language <lang>` | Filter by language |
| `--min-stars <n>` | Minimum star count |
| `--include-archived` | Include archived repos |

## Onboarding

### `ab onboard`

Run guided onboarding workflow.

```bash
ab onboard
ab onboard /path/to/repo
```

## Sync

### `ab sync`

Unified synchronization for config and beads.

```bash
# Sync AllBeads config (if tracked in git)
ab sync

# Sync all context beads
ab sync --all

# Sync specific context
ab sync mycontext

# Check sync status
ab sync --status

# With commit message
ab sync --message "Updated config"

# Sync to web platform (experimental)
ab sync --web
```

| Option | Short | Description |
|--------|-------|-------------|
| `--all` | | Sync all context beads |
| `--status` | | Check status without syncing |
| `--message <msg>` | `-m` | Commit message |
| `--web` | | Push to web platform |

## Cache Management

### `ab clear-cache`

Clear the local cache.

```bash
ab clear-cache
```

## Debugging

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging level (e.g., `info`, `debug`, `allbeads=debug`) |
| `GITHUB_TOKEN` | GitHub API token |
| `JIRA_API_TOKEN` | JIRA API token |

### Examples

```bash
# Show INFO level logs
RUST_LOG=info ab stats

# Show DEBUG level logs
RUST_LOG=debug ab list

# Show only AllBeads logs at debug level
RUST_LOG=allbeads=debug ab stats

# Verbose git operations
RUST_LOG=allbeads::aggregator=debug ab stats
```
