---
description: Create a new GitHub repository with AllBeads pre-configured
---

Create a brand new GitHub repository and set it up with AllBeads from the start.

This command provides a streamlined workflow similar to GitHub's "New Repository" form but adds:
- Automatic beads initialization
- AI agent configuration (CLAUDE.md, .cursorrules, etc.)
- Automatic addition to AllBeads contexts

Usage:
```bash
# Interactive mode (recommended for first-time use)
ab context new

# With arguments
ab context new myproject --private --description "My new project"

# Non-interactive mode
ab context new myproject --non-interactive --private

# Full specification
ab context new myproject \
  --private \
  --description "Project description" \
  --license MIT \
  --gitignore Rust \
  --init-agents claude,cursor
```

## Options:

| Option | Description |
|--------|-------------|
| `--description` | Repository description |
| `--private` | Create as private repository |
| `--org <ORG>` | Create in an organization |
| `--gitignore <TEMPLATE>` | .gitignore template (Rust, Node, Python, Go, etc.) |
| `--license <LICENSE>` | License (MIT, Apache-2.0, GPL-3.0, etc.) |
| `--init-agents <LIST>` | Comma-separated agents to configure (default: claude) |
| `--path <PATH>` | Custom local clone path |
| `--no-clone` | Create on GitHub only, don't clone locally |
| `--wizard` | Use interactive TUI wizard |
| `--non-interactive` | Use defaults without prompts |

## Workflow:

1. **Input Collection** - Name, visibility, description via prompts or CLI args
2. **GitHub Creation** - Creates repository using `gh` CLI
3. **Clone** - Clones repository to local workspace
4. **Beads Init** - Runs `bd init` to set up issue tracking
5. **Agent Config** - Creates CLAUDE.md and other agent files
6. **Git Push** - Commits and pushes configuration
7. **Context Add** - Adds to AllBeads config

## Requirements:

- GitHub CLI (`gh`) must be installed and authenticated
- Run `gh auth login` if not already authenticated
- Token needs `repo` scope for repository creation

## Example Session:

```bash
$ ab context new

Create a new repository

  Repository name: my-awesome-project
  Description (optional): A new project with AllBeads
  Private repository? [Y/n]: n
  Organization (leave empty for personal account):

Common .gitignore templates: Rust, Node, Python, Go, Java
  .gitignore template: Rust

Common licenses: MIT, Apache-2.0, GPL-3.0, BSD-3-Clause
  License: MIT

AllBeads Configuration
  Initialize beads? [Y/n]: y

Available agents: claude, cursor, copilot, aider
  Agents to configure (comma-separated) [claude]: claude,cursor

Summary
  Name:        my-awesome-project
  Visibility:  Public
  .gitignore:  Rust
  License:     MIT
  Beads:       Yes
  Agents:      claude, cursor

  Create repository? [Y/n]: y

Creating repository on GitHub...
✓ Created https://github.com/thrashr888/my-awesome-project
→ Cloning to /Users/me/Workspace/my-awesome-project...
✓ Cloned to /Users/me/Workspace/my-awesome-project
→ Initializing beads...
✓ Beads initialized
→ Configuring agents...
✓ Configured: claude, cursor
→ Committing configuration...
✓ Pushed to GitHub

✓ Added context 'my-awesome-project' to AllBeads config

Repository Created Successfully
  Name:   my-awesome-project
  URL:    https://github.com/thrashr888/my-awesome-project
  Path:   /Users/me/Workspace/my-awesome-project
  Beads:  initialized
  Agents: claude, cursor
```

## Compared to `ab onboard`:

| Feature | `ab context new` | `ab onboard` |
|---------|------------------|--------------|
| Creates GitHub repo | ✓ | ✗ |
| Clones existing repo | ✓ (after creating) | ✓ |
| Initializes beads | ✓ | ✓ |
| Configures agents | ✓ | ✓ |
| Adds to contexts | ✓ | ✓ |

Use `ab context new` for brand new projects.
Use `ab onboard` for existing repositories.

## See also:

- `/onboard` - Onboard an existing repository
- `/context` - Manage AllBeads contexts
- `/agents` - Detect and configure AI agents
