---
description: Show blocked issues across all contexts
---

Show beads that are blocked by dependencies across all AllBeads contexts.

Run `allbeads blocked` to find issues that cannot proceed due to blockers.

For each blocked issue, show:
- Bead ID and title
- What it's blocked by
- Context (repository)

This helps identify:
- Cross-repo blocking relationships
- Bottlenecks in the workflow
- Issues that need attention to unblock downstream work
