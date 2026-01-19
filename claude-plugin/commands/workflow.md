---
description: Show the AllBeads workflow guide
---

# AllBeads Workflow Guide

## The Big Picture

```
[GitHub Repos] → [ab scan] → [ab onboard] → [AllBeads Contexts]
                                                    ↓
[ab context new] → [New Repo] → [bd init] → [Managed Context]
                                                    ↓
[Planning] → [bd create epics] → [Add specs] → [Ready for work]
                                                    ↓
[ab ready] → [Claim task] → [Implement] → [ab close] → [Push]
```

## Workflow Phases

### 1. Discovery
Find repos to manage:
```bash
ab scan github <username>      # Your repos
ab scan github <org>           # Org repos
ab context list               # Already managed
```

### 2. Onboarding
Bring repos into AllBeads:
```bash
# Existing repo
ab onboard <repo-url>

# New repo
ab context new <name> --private
```

### 3. Planning
Create work items with specs:
```bash
bd create --title="..." --type=epic
bd comments add <id> "<spec>"
```

### 4. Execution
Complete the work:
```bash
ab ready                      # Find work
ab update <id> --status=in_progress
# ... implement ...
ab close <id>
git push
```

### 5. Governance
Maintain quality:
```bash
ab governance check           # Policy compliance
ab agents list               # Agent coverage
ab stats                     # Health metrics
```

## Key Commands by Phase

| Phase | Command | Purpose |
|-------|---------|---------|
| Discovery | `ab scan github` | Find repos |
| Onboarding | `ab onboard` | Add to AllBeads |
| Planning | `ab context new` | Create new repo |
| Planning | `bd create` | Create work items |
| Execution | `ab ready` | Find work |
| Execution | `ab close` | Complete work |
| Governance | `ab governance check` | Verify policies |

## Session Protocol

Every session should:
1. `ab prime` - Load context
2. `ab ready` - Find work
3. Work on tasks
4. `bd sync` - Save beads
5. `git push` - Push changes

## See Also

- `AGENTS.md` - Quick agent reference
- `CLAUDE.md` - Full project guide
- `/prime` - Context priming
- `/ready` - Ready work
