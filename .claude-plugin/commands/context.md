---
description: Manage AllBeads contexts (repositories)
---

Manage the repositories (contexts) that AllBeads tracks.

Subcommands:
- `allbeads context list` - Show all configured contexts
- `allbeads context add <path>` - Add a repository as a context
- `allbeads context add <path> --name <name>` - Add with explicit name
- `allbeads context remove <name>` - Remove a context

When adding a context, AllBeads will:
1. Detect the repository name from the folder
2. Find the git remote URL
3. Determine the authentication strategy (SSH or PAT)

After adding, run `allbeads sync <name>` to fetch beads from the remote.
