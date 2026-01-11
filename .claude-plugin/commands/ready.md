---
description: Find ready-to-work tasks across all contexts
---

Find beads that are ready to work on (no blocking dependencies) across all AllBeads contexts.

Run `allbeads ready` to get a list of unblocked issues from all repositories. Present them showing:
- Bead ID
- Title
- Priority
- Context (repository)

If there are ready tasks, ask the user which one they'd like to work on.

If there are no ready tasks, suggest:
- Check `/allbeads:blocked` to see what's blocking work
- Check individual repos with `bd ready`
