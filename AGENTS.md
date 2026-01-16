# AGENTS.md

Quick reference for AI agents working in this repository. For full details, see `CLAUDE.md`.

## Key Commands

```bash
# Find work across all contexts
ab ready              # Show unblocked beads ready to work on
ab list --status=open # List all open beads
ab show <id>          # Get full bead details

# Update work status
ab update <id> --status=in_progress  # Claim work
ab close <id> --reason="..."         # Complete work

# Sync and context
ab sync               # Sync beads with git remote
ab context list       # Show managed repositories
ab stats              # Aggregated statistics
```

## Visual Design Rules

Use small Unicode symbols with semantic colors, NOT emoji:

| Status | Symbol | Color |
|--------|--------|-------|
| Open | `○` | default |
| In Progress | `◐` | yellow |
| Blocked | `●` | red |
| Closed | `✓` | green |
| Frozen | `❄` | cyan |

**Anti-pattern:** Never use colored circle emojis (🔴🟡🟢). They cause cognitive overload.

## Interactive Command Restrictions

**DO NOT use these commands** (they require interactive input):
- `bd edit` - Opens $EDITOR
- `ab tui` - Opens TUI
- Any command requiring stdin

**Instead use:**
- `bd update <id> --title="..." --description="..."`
- `bd comments add <id> "comment text"`

## Session Completion Protocol

Before ending any session, you **MUST**:

```bash
[ ] 1. git status              # Check what changed
[ ] 2. git add <files>         # Stage code changes
[ ] 3. bd sync                 # Commit beads changes
[ ] 4. git commit -m "..."     # Commit code
[ ] 5. bd sync                 # Commit any new beads
[ ] 6. git push                # Push to remote
```

**CRITICAL:** Work is NOT complete until `git push` succeeds. Never leave work uncommitted locally.

## Multi-Context Workflow

AllBeads aggregates beads across multiple repositories:

```bash
# Work across contexts
ab list -C AllBeads,rookery    # Filter to specific contexts
ab search "auth" --context=@work  # Search in @work contexts

# Create in specific context
ab create --context=AllBeads --title="Fix bug" --type=bug

# Onboard new repos
ab context new myproject --private  # Create new GitHub repo
ab onboard /path/to/repo            # Onboard existing repo
```

## Golden Workflow: Onboard → Handoff → Complete

The recommended workflow for managing work across repositories:

```bash
# 1. Onboard a repository
ab onboard /path/to/repo

# 2. Find ready work
ab ready

# 3. Hand off to an agent
ab handoff <bead-id>
ab handoff <bead-id> --agent codex  # Specific agent

# 4. Agent completes work and closes bead
# (For sandboxed agents like Codex, you commit/push after)

# 5. Repeat
ab ready && ab handoff <next-bead>
```

## Handoff Workflow

### For Most Agents (Claude, Gemini, Aider)
Agent handles everything: branch creation, work, commit, push, close.

### For Sandboxed Agents (Codex)
AllBeads pre-creates the branch. After agent completes:
```bash
git add -A
git commit -m "feat(<bead-id>): <description>"
bd sync && git push -u origin bead/<bead-id>
```

## Agent Types

### Task Agent
Autonomous agent that finds and completes ready work:
1. `ab ready` - Find unblocked tasks
2. `ab update <id> --status=in_progress` - Claim
3. Complete the work
4. `ab close <id>` - Complete
5. Repeat

### Governance Agent
Enforces policies across managed repositories:
```bash
ab governance check        # Check all repos
ab agents list            # List detected agents
ab scan github <user>     # Scan for unmanaged repos
```

### Planning Agent
Plans new projects without writing code:
```bash
ab context new <name>     # Create repo
bd create --type=epic     # Create planning beads
# STOP - don't implement yet
```

## Discovery Pattern

When you find bugs, TODOs, or related work while implementing:
```bash
bd create --title="Found: ..." --type=bug
bd dep add <new-id> <current-id>  # Link as discovered-from
```

This maintains context for future work.

## Quality Gates

Before closing any task:
- [ ] Tests pass: `cargo test`
- [ ] Linter clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`
- [ ] No secrets committed
- [ ] Changes pushed to remote

## See Also

- `CLAUDE.md` - Full project guidance
- `specs/PRD-00.md` - Architecture specification
- `.claude-plugin/` - Commands and skills
