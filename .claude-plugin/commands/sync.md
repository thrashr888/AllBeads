---
description: Synchronize contexts with git remotes
---

Synchronize AllBeads contexts with their git remotes.

Commands:
- `allbeads sync` - Sync the AllBeads config (if in git)
- `allbeads sync --all` - Sync all context beads
- `allbeads sync <context>` - Sync a specific context
- `allbeads sync --status` - Check sync status without syncing

This fetches the latest beads from each repository's remote and updates the local cache.

Run this:
- At the start of a session to get latest state
- After making changes in multiple repos
- Before generating cross-repo reports
