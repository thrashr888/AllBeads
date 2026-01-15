---
description: Manage AllBeads contexts (repositories)
---

Manage the repositories (contexts) that AllBeads tracks.

Subcommands:
- `allbeads context list` - Show all configured contexts
- `allbeads context add <path>` - Add a repository as a context
- `allbeads context add <path> --name <name>` - Add with explicit name
- `allbeads context remove <name>` - Remove a context
- `allbeads context new <name>` - Create a new GitHub repository with AllBeads pre-configured
- `allbeads context onboarding` - Show onboarding status for all contexts

When adding a context, AllBeads will:
1. Detect the repository name from the folder
2. Find the git remote URL
3. Determine the authentication strategy (SSH or PAT)

After adding, run `allbeads sync <name>` to fetch beads from the remote.

## Creating New Repositories

Use `allbeads context new` to create a brand new GitHub repository:

```bash
# Interactive mode
ab context new

# With arguments
ab context new myproject --private --description "My project"
```

This creates the GitHub repo, clones it locally, initializes beads, configures AI agents, and adds it to AllBeads contexts.

See `/context-new` for detailed usage.
