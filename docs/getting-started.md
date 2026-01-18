# Getting Started with AllBeads

AllBeads is a meta-orchestration system that federates issue tracking across multiple git repositories. This guide will help you install AllBeads, set up your first context, and learn the essential commands.

## Prerequisites

- **Git**: Required for repository operations
- **bd (beads CLI)**: Required for issue tracking - [Installation](https://github.com/steveyegge/beads)
- **GitHub CLI** (optional): Simplifies token management

## Installation

### Homebrew (macOS/Linux)

The easiest way to install AllBeads:

```bash
brew tap thrashr888/allbeads
brew install allbeads
```

### Pre-built Binaries

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -L https://github.com/thrashr888/AllBeads/releases/latest/download/allbeads-macos-aarch64 -o ab
chmod +x ab && sudo mv ab /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/thrashr888/AllBeads/releases/latest/download/allbeads-macos-x86_64 -o ab
chmod +x ab && sudo mv ab /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/thrashr888/AllBeads/releases/latest/download/allbeads-linux-x86_64 -o ab
chmod +x ab && sudo mv ab /usr/local/bin/
```

Verify the installation:

```bash
ab --version
```

### From Source

If you have the Rust toolchain installed:

```bash
git clone https://github.com/thrashr888/AllBeads.git
cd AllBeads && cargo build --release

# Add to PATH or create an alias
alias ab='./target/release/allbeads'
```

## Initial Setup

### 1. Initialize AllBeads

Create the configuration directory and default config file:

```bash
ab init
```

This creates `~/.config/allbeads/config.yaml` with default settings.

If you want to clone an existing Boss repository:

```bash
ab init --remote git@github.com:your-org/boss-repo.git
```

### 2. Add Your First Context

A "context" is a repository that AllBeads will track. Navigate to a repository with beads initialized:

```bash
cd /path/to/your-repo
ab context add .
```

AllBeads automatically detects:
- **Name**: From the folder name
- **URL**: From the git remote
- **Auth**: SSH agent for `git@` URLs, personal access token for `https://` URLs

You can also be explicit:

```bash
ab context add . --name myproject --url git@github.com:org/repo.git
```

### 3. Verify Your Setup

List your configured contexts:

```bash
ab context list
```

Output:
```
Configured contexts (1):

  myproject
    URL:  git@github.com:org/myproject.git
    Path: /Users/you/workspace/myproject
    Auth: SshAgent
```

## Basic Commands

### View Statistics

Get an overview of all beads across your contexts:

```bash
ab stats
```

Output:
```
AllBeads Statistics:

  Total beads:      47
  Open:             23
  In Progress:      3
  Blocked:          0
  Closed:           21

Contexts:
  myproject       47 beads (23 open)
```

### List Beads

View beads from all contexts:

```bash
ab list
```

Filter by status:

```bash
ab list --status open
ab list --status in_progress
ab list --status blocked
```

Filter by priority:

```bash
ab list --priority P0    # Critical
ab list --priority P1    # High
ab list --priority P2    # Medium (default)
```

### Find Ready Work

Show beads that have no blockers and are ready to work on:

```bash
ab ready
```

This is the recommended command for finding your next task.

### View Bead Details

Get full information about a specific bead:

```bash
ab show <bead-id>
```

Example:
```
ab-123: Implement OAuth flow
Status:       open
Priority:     P1
Type:         Task
Created:      2026-01-10 by thrashr888
Dependencies: ab-122 (open)

Description:
Add OAuth 2.0 authentication flow with support for
GitHub and Google providers.
```

### Search Beads

Find beads by text:

```bash
ab search "authentication"
ab search --status open --type feature
```

## Using the TUI Dashboard

Launch the interactive terminal UI:

```bash
ab tui
```

The TUI provides four views:
- **Kanban Board**: Three columns (Open, In Progress, Closed)
- **Mail View**: Agent message inbox
- **Graph View**: Dependency visualization
- **Swarm View**: Active agent monitoring

Key bindings:
- `Tab` - Switch between views
- `j`/`k` - Move up/down
- `h`/`l` - Switch columns (Kanban)
- `Enter` - View details
- `Esc` - Close detail view
- `q` - Quit

## Adding More Contexts

AllBeads shines when aggregating multiple repositories:

```bash
# Add more repos
cd ~/workspace/auth-service && ab context add .
cd ~/workspace/web-app && ab context add .
cd ~/workspace/api-gateway && ab context add .

# View aggregated stats
ab stats

# Find ready work across all repos
ab ready
```

## Authentication

AllBeads supports two authentication strategies:

### SSH Agent (Recommended)

For `git@github.com:...` URLs, AllBeads uses your SSH agent automatically.

### Personal Access Token

For `https://` URLs, AllBeads looks for tokens in this order:

1. `<CONTEXT_NAME>_TOKEN` environment variable (e.g., `MYPROJECT_TOKEN`)
2. `GITHUB_TOKEN` environment variable
3. `gh auth token` (GitHub CLI)

Example:
```bash
# Set globally
export GITHUB_TOKEN='ghp_xxxxxxxxxxxx'

# Or per-context
export MYPROJECT_TOKEN='ghp_xxxxxxxxxxxx'
```

## Next Steps

- Learn about [Core Concepts](./core-concepts.md) like Shadow Beads and the Federated Graph
- Explore the full [CLI Reference](./cli-reference.md)
- Follow [Tutorials](./tutorials.md) for common workflows
- Set up [Enterprise Integration](./integrations.md) with JIRA and GitHub Issues

## Getting Help

```bash
# Show all commands
ab --help

# Get help for a specific command
ab context --help
ab list --help
```

Report issues at: https://github.com/thrashr888/AllBeads/issues
