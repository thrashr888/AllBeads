---
name: allbeads
description: >
  Multi-repo orchestration for AI agent workflows. Aggregate beads across
  repositories, manage contexts, and coordinate distributed work across
  microservices and polyrepo architectures.
allowed-tools: "Read,Bash(allbeads:*)"
version: "0.2.0"
author: "Paul Thrasher <https://github.com/thrashr888>"
license: "MIT"
---

# AllBeads - Multi-Repo Agent Orchestration

AllBeads implements the "Boss Repository" pattern - a control plane that federates issue tracking (beads) across multiple git repositories, enabling AI agents to coordinate work across distributed microservices.

## AllBeads vs bd (beads)

| allbeads (multi-repo) | bd (single-repo) |
|----------------------|------------------|
| Aggregates across repos | Single repository |
| Context-based filtering | Local .beads only |
| Cross-repo dependencies | Local dependencies |
| Federated dashboard | Single project view |

**Decision test**: "Does my work span multiple repositories?" → YES = allbeads

**When to use allbeads**:
- Work spans multiple microservices/repos
- Need unified view across polyrepo architecture
- Cross-repository dependencies exist
- Managing work across team contexts (@work, @personal)
- Enterprise integration (JIRA, GitHub Issues)

**When to use bd**:
- Single repository work
- Local issue tracking
- No cross-repo dependencies needed

## Prerequisites

```bash
allbeads --version  # Requires v0.2.0+
```

- **allbeads CLI** installed (`brew install thrashr888/allbeads/allbeads`)
- **bd CLI** installed for underlying beads operations
- **Contexts configured**: `allbeads context add .`

## Session Start Checklist

When starting a work session, check for handoffs from other agents:

```bash
ab mail inbox        # Check for messages from other agents
ab ready             # See what work is available
bd ready             # Local repo ready tasks
```

**Why check mail?** Other agents may have completed work and handed off follow-up tasks. The mail often contains context about what was done and what's expected next.

## Core Commands

### Viewing Aggregated Work

```bash
# Show statistics across all contexts
allbeads stats

# List all beads across contexts
allbeads list

# Filter by status
allbeads list --status open

# Show ready-to-work beads (no blockers)
allbeads ready

# Show blocked beads
allbeads blocked

# Search across all contexts
allbeads search "query"
```

### Context Management

```bash
# Add current repo as context
allbeads context add .

# Add with explicit name
allbeads context add /path/to/repo --name myproject

# List contexts
allbeads context list

# Remove context
allbeads context remove myproject
```

### Synchronization

```bash
# Sync all contexts with remotes
allbeads sync --all

# Check sync status
allbeads sync --status

# Sync specific context
allbeads sync mycontext
```

### Interactive Dashboard

```bash
# Launch TUI (Kanban + Mail views)
allbeads tui

# Keyboard shortcuts:
#   Tab           - Switch views
#   j/k or Up/Down - Navigate
#   Enter         - View details
#   q             - Quit
```

## Key Concepts

### Contexts
A context is a git repository that AllBeads tracks. Each context has its own `.beads/` directory and can be filtered independently or viewed in aggregate.

### Shadow Beads
When working with cross-repo Epics, AllBeads creates "Shadow Beads" in the Boss repository that point to native beads in member repositories, enabling dependency tracking across repo boundaries.

### Federated Graph
AllBeads maintains a unified dependency graph across all contexts, allowing you to see how work in one repository blocks or enables work in another.

## Cross-Repo Task Handoff

AllBeads enables seamless task handoff between repositories using **beads** (persistent tasks) and **mail** (real-time notifications).

### Beads + Mail: The Complete Handoff

```bash
# 1. Create the persistent task (bead)
ab create --context=AllBeadsWeb --title="Add /api/beads/import endpoint" --type=feature

# 2. Send real-time notification (mail)
ab mail send --to AllBeadsWeb "New task: Add /api/beads/import endpoint. See bd ready."

# 3. Target agent picks up
# In AllBeadsWeb:
ab mail inbox    # Sees notification
bd ready         # Finds task
```

### When to Use Each

| Use Beads | Use Mail |
|-----------|----------|
| Trackable tasks | Real-time alerts |
| Work spanning sessions | Immediate attention needed |
| Dependencies | Status updates |
| Persistent record | Coordination messages |

**Best practice**: Create a bead AND send mail for important handoffs.

### Common Handoff Targets

| Context | Purpose |
|---------|---------|
| `AllBeadsWeb` | Web UI, API endpoints, dashboard |
| `AllBeadsApp` | macOS native app, menu bar |
| `AllBeads` | CLI, core Rust library |

## Agent Mail

Agent Mail enables real-time communication between agents across repositories.

```bash
# Send a message to another context/agent
ab mail send --to AllBeadsWeb "Feature ready for review"
ab mail send --to AllBeadsWeb --message-type request "Approve deployment?"

# Check your inbox
ab mail inbox

# Check unread count
ab mail unread

# Send test messages
ab mail test "Build completed for ab-123. Ready for review."
```

### Mail in the Dashboard

View mail in multiple ways:
- **CLI**: `ab mail inbox`
- **TUI**: `ab tui` then Tab to Mail view
- **Web**: https://allbeads.co/dashboard/mail (when logged in)

### Message Types

| Type | Use For |
|------|---------|
| **NOTIFY** | Status updates, completions, FYIs |
| **REQUEST** | Approval needed, input required |
| **BROADCAST** | Announcements to all agents |
| **LOCK/UNLOCK** | File coordination |
| **HEARTBEAT** | Agent liveness signals |

### Managing Your Inbox

```bash
# Mark messages as read
ab mail read <message-id>    # Mark specific message
ab mail read --all           # Mark all as read

# Archive old messages
ab mail archive <message-id> # Archive specific message
ab mail archive --all        # Archive all read messages

# Delete messages
ab mail delete <message-id>  # Permanently delete
```

**Workflow**: Check inbox → Process messages → Mark read → Archive when done.

### Mail Best Practices

1. **Reference bead IDs** in messages for context
2. **Check inbox** when starting work sessions
3. **Send completion notices** after finishing handed-off work
4. **Use mail for urgency**, beads for tracking
5. **Clean up regularly** with `ab mail read --all && ab mail archive --all`

## Common Workflows

**Starting cross-repo work:**
```bash
allbeads ready           # Find available work across all repos
allbeads show <id>       # Review issue details
bd update <id> --status=in_progress  # Claim it (in the local repo)
```

**Checking project health:**
```bash
allbeads stats           # Overview across all contexts
allbeads blocked         # Find blocked issues
allbeads sync --status   # Check sync state
```

**Adding a new repository:**
```bash
cd /path/to/new-repo
allbeads context add . --name new-project
allbeads sync new-project
```

**Creating a brand new project:**
```bash
ab context new myproject --private --gitignore Go --license MIT
```
This creates GitHub repo, clones locally, initializes beads, and adds to AllBeads.

**Planning a project (no code yet):**
```bash
ab context new myproject --private
bd create --title="[Phase 1] ..." --type=epic
bd comments add <id> "<spec details>"
# STOP - let handoff workflow implement
```

## Golden Workflow: Onboard → Handoff → Complete

The recommended workflow for managing work across repositories:

### 1. Onboard Repositories
```bash
# Onboard existing repo (safety checks: clean git, main branch)
ab onboard /path/to/repo

# Creates: .beads/, .claude/settings.json, adds to AllBeads config
# Creates: Epic + task beads for onboarding work
```

### 2. Find Ready Work
```bash
ab ready                 # Show unblocked tasks across all repos
ab show <bead-id>        # Review task details
```

### 3. Hand Off to Agent
```bash
# Hand off to your preferred agent
ab handoff <bead-id>

# Or specify agent explicitly
ab handoff <bead-id> --agent codex
ab handoff <bead-id> --agent gemini
```

### 4. Agent Completes Work
The agent:
1. Creates branch (or uses pre-created for sandboxed agents)
2. Does the work
3. Closes the bead: `bd close <bead-id>`

### 5. Commit and Push (if sandboxed agent)
For sandboxed agents like Codex that can't do git operations:
```bash
git add -A
git commit -m "feat(<bead-id>): <description>"
bd sync
git push -u origin bead/<bead-id>
```

### 6. Repeat
```bash
ab ready                 # Find next task
ab handoff <bead-id>     # Hand off
```

## Key Learnings

### Onboarding
- **Safety checks**: Clean git workspace, main/master branch required
- **Dependency direction**: Epic depends on tasks (tasks ready, epic blocked)
- **Plugins**: Only beads + allbeads auto-enabled

### Handoff
- **Sandboxed agents**: Codex can't write to `.git/` - branch pre-created
- **Codex command**: Uses `codex exec --full-auto` for non-interactive mode
- **After sandboxed agent**: User commits and pushes the work

## Agents

AllBeads provides specialized agents:

| Agent | Purpose |
|-------|---------|
| **task-agent** | Autonomous task completion across contexts |
| **governance-agent** | Policy enforcement and compliance |
| **planning-agent** | Project planning without implementation |
| **onboarding-agent** | Repository onboarding assistance |

## Commands Reference

| Command | Purpose |
|---------|---------|
| `/create` | Create bead in any context (cross-repo handoff) |
| `/mail` | Agent mail for real-time coordination |
| `/ready` | Show unblocked work |
| `/list` | List all beads |
| `/show` | Show bead details |
| `/stats` | Aggregated statistics |
| `/sync` | Sync with remotes |
| `/context` | Manage contexts |
| `/context-new` | Create new GitHub repo |
| `/project-new` | Plan new project (no code) |
| `/handoff` | Hand off to implementation |
| `/workflow` | Workflow guide |
| `/prime` | Prime agent context |
| `/scan` | Scan GitHub for repos |
| `/governance` | Check policies |
| `/agents` | Manage AI agents |
| `/onboard-repo` | Onboard repository |
| `/blocked` | Show blocked work |
| `/tui` | Launch dashboard |

## Quick Reference

```bash
# Discovery
ab scan github <user>         # Find repos

# Onboarding
ab onboard <url>              # Onboard existing
ab context new <name>         # Create new

# Work
ab ready                      # Find work
ab update <id> --status=...   # Update status
ab close <id>                 # Complete

# Cross-Repo Handoff
ab create --context=AllBeadsWeb --title="..." --type=feature
ab mail send --to AllBeadsWeb "New task: see bd ready"

# Mail
ab mail send --to <context> "message"  # Send to context/agent
ab mail inbox                 # Check messages
ab mail unread                # Unread count

# Sync
ab sync --all                 # Sync everything
bd sync                       # Sync current repo

# Governance
ab governance check           # Check policies
ab agents list               # List agents
```

## See Also

- `AGENTS.md` - Quick reference for agents
- `CLAUDE.md` - Full project guide
- `specs/PRD-00.md` - Architecture specification
