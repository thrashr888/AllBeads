---
description: Show aggregated statistics across all contexts
---

Display statistics aggregated across all AllBeads contexts.

Run `allbeads stats` to retrieve project metrics and present them clearly:
- Total beads by status (open, in_progress, blocked, closed)
- Breakdown by context (repository)
- Ready-to-work count
- Cache status

Suggest actions based on the stats:
- High number of blocked issues? Run `/allbeads:blocked` to investigate
- No in_progress work? Run `/allbeads:ready` to find tasks
- Stale cache? Run `allbeads sync --all` to refresh
