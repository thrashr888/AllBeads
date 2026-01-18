---
description: Create a bead in a specific context (cross-repo task handoff)
---

Create a new bead (issue/task) in any AllBeads context. This enables cross-repository task handoff - creating work items in other repos for other agents to pick up.

## Usage

```bash
# Create in a specific context
ab create --context=<context-name> --title="<title>" --type=<type> --priority=<priority>

# Examples
ab create --context=AllBeadsWeb --title="Add new API endpoint" --type=feature --priority=2
ab create --context=AllBeadsApp --title="Fix menu bar icon" --type=bug --priority=1
ab create --context=QDOS --title="Implement parser" --type=task
```

## Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `--context` | Yes | Target context name (e.g., AllBeadsWeb, AllBeadsApp) |
| `--title` | Yes | Title of the bead |
| `--type` | No | Type: bug, feature, task, epic, chore (default: task) |
| `--priority` | No | Priority: P0-P4 or 0-4 (default: P2) |

## Cross-Repo Task Handoff

This is the primary mechanism for distributing work across the AllBeads ecosystem:

1. **Identify work for another repo**: While working, discover something that belongs elsewhere
2. **Create the bead**: `ab create --context=AllBeadsWeb --title="..." --type=feature`
3. **Agent in target repo picks it up**: They run `bd ready` and see the task
4. **Work flows naturally**: Each repo's agent handles its own work

## Common Handoff Targets

| Context | What goes there |
|---------|-----------------|
| `AllBeadsWeb` | Web UI features, API endpoints, dashboard work |
| `AllBeadsApp` | macOS app features, native UI, menu bar items |
| `AllBeads` | CLI features, core library, Rust code |

## Workflow Example

```bash
# Working in AllBeads CLI, realize we need a web endpoint
ab create --context=AllBeadsWeb \
  --title="Add /api/beads/import endpoint" \
  --type=feature \
  --priority=2

# The bead appears in AllBeadsWeb
ab list -C AllBeadsWeb

# Agent in AllBeadsWeb picks it up
cd ../AllBeadsWeb
bd ready
bd update abw-xxx --status=in_progress
# ... implement ...
bd close abw-xxx
```

## See Also

- `/allbeads:ready` - Find work across all contexts
- `/allbeads:list` - List beads with context filtering
- `/allbeads:handoff` - Hand off bead to an agent
