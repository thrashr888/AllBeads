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
