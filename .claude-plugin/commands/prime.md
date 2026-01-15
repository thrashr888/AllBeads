---
description: Prime agent memory with project context
---

Load project context into agent memory for effective work.

## Usage

```bash
ab prime                    # Prime with all contexts
ab prime -C <context>       # Prime specific context
```

## What Gets Loaded

1. **Project Overview**
   - Active contexts and paths
   - Repository status (beads, agents)

2. **Ready Work**
   - Unblocked beads ready to implement
   - Priority-sorted task list

3. **Recent Activity**
   - Recently modified beads
   - Recent commits

4. **Statistics**
   - Open/closed/blocked counts
   - Work distribution

## When to Use

- Start of new session
- After conversation compaction
- Switching between contexts
- Before starting complex work

## Output

```
# AllBeads Context Priming

## Active Contexts
- AllBeads: /Users/me/Workspace/AllBeads (174 beads)
- rookery: /Users/me/Workspace/rookery (23 beads)

## Ready Work (5 tasks)
1. ab-xyz [P1] - Implement feature X
2. rk-123 [P2] - Fix bug in auth

## Recent Activity
- 3 beads closed today
- 2 beads created today

## Statistics
- Open: 45, In Progress: 3, Blocked: 2, Closed: 124
```

## See Also

- `/ready` - Just show ready work
- `/stats` - Just show statistics
- `/list` - Full bead listing
