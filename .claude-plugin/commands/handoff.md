---
description: Hand off a bead to an AI agent for implementation
---

Fire-and-forget delegation to AI agents. Launches the agent with full bead context (title, description, dependencies) and moves on.

## Usage

```bash
# Hand off a specific bead to Claude (default agent)
ab handoff <bead-id>

# Hand off to a specific agent
ab handoff <bead-id> --agent <agent>

# Hand off in an isolated worktree
ab handoff <bead-id> --worktree

# Show beads that have been handed off
ab handoff --list

# Show ready beads that could be handed off
ab handoff --ready

# Dry run - see what would happen without launching
ab handoff <bead-id> --dry-run

# Show available agents (detected on system)
ab handoff --agents
```

## Agent Preferences

On first use, you'll be prompted to select your preferred agent from the detected installed agents. Your choice is saved to `.beads/config.yaml` and used for subsequent handoffs.

To override the saved preference, use `--agent`:
```bash
ab handoff ab-xyz --agent gemini    # Use Gemini instead of preferred
```

## Supported Agents

### Terminal Agents (CLI)
- `claude` - Claude Code (default)
- `opencode` - OpenCode
- `codex` - OpenAI Codex
- `gemini` - Gemini CLI
- `aider` - Aider
- `cody` - Sourcegraph Cody

### IDE Agents
- `cursor` - Cursor
- `kiro` - Kiro (AWS)
- `antigravity` - Antigravity
- `copilot` - VS Code Copilot

### Web Agents
- `jules` - Jules (Google) - opens browser
- `chatgpt-codex` - ChatGPT Codex - opens browser

## What Happens

1. Bead context is loaded (title, description, dependencies, labels)
2. A prompt is generated from the bead content with workflow instructions
3. Bead status is updated to `in_progress`
4. Handoff is recorded (comment + `handed-off` label)
5. If `--worktree` is used, a git worktree is created in `.worktrees/`
6. Agent is launched with the prompt
   - CLI agents: launched with prompt argument (use `codex exec --full-auto` for non-interactive)
   - IDE agents: launched with chat command
   - Web agents: browser opened with deep-link URL

## Agent Workflow (What the Agent Does)

The handoff prompt instructs the agent to:

1. **Create a branch** for the work: `git checkout -b bead/<id>`
2. **Do the work** described in the bead
3. **Commit changes**: `git add -A && git commit -m "feat(<id>): <description>"`
4. **Close the bead**: `bd close <id>` (use `--force` if blocked by an epic)
5. **Sync and push**: `bd sync && git push -u origin bead/<id>`

This ensures clean git history and proper bead tracking.

### Sandboxed Agents (Codex)

Some agents run in sandboxes that prevent git operations (e.g., OpenAI Codex with `workspace-write` sandbox). AllBeads handles this automatically:

1. **Pre-creates the branch** before launching (`git checkout -b bead/<id>`)
2. **Provides simplified workflow prompt** (skip git operations)
3. **User handles commit/push** after agent completes

The sandboxed prompt tells the agent:
- Focus on the work, not git operations
- The branch already exists
- Just close the bead when done

After a sandboxed agent completes:
```bash
# Commit the agent's work
git add -A
git commit -m "feat(<bead-id>): <description>"

# Sync and push
bd sync
git push -u origin bead/<bead-id>
```

## Worktrees

Use `--worktree` for isolated development:
```bash
ab handoff ab-xyz --worktree
```

This creates a git worktree at `.worktrees/ab-xyz/` with a branch `ab/ab-xyz`. The agent runs in the worktree, keeping your main branch clean. Useful for:
- Parallel work on multiple beads
- Risky changes that might need to be discarded
- Keeping main stable while experimenting

## Environment

The `AB_ACTIVE_BEAD` environment variable is set to the bead ID when launching CLI agents, allowing the agent to know which bead it's working on.

## Workflow

### Starting Work
```bash
ab handoff --ready              # See what's available
ab handoff ab-xyz               # Hand off to Claude
ab handoff ab-xyz --agent gemini  # Hand off to Gemini
```

### Tracking Handed-off Work
```bash
ab handoff --list               # See what's in progress
ab show ab-xyz                  # View bead details + handoff info
bd show ab-xyz                  # View raw bead details and comments
```

Handoff info (agent, time, task URL) is automatically shown when viewing a handed-off bead with `ab show`.

### Completing Work
After the agent finishes:
```bash
bd close ab-xyz --reason="Implemented feature X"
bd sync
git push
```

## Philosophy

"Hand-off, not ownership" - We fire and forget. The agent takes over and works asynchronously. We don't poll for status. The work will get done.

## See Also

- `ab ready` - Show unblocked work
- `ab list --status=in_progress` - All in-progress work
- `/release` - Ship a new version
- `specs/SPEC-handoff.md` - Full specification
