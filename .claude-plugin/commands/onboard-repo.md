---
description: Interactive onboarding for a repository into AllBeads
---

Onboard the current repository (or a specified repository) into the AllBeads ecosystem.

This command runs an interactive workflow that:
1. Initializes beads tracking (`bd init`)
2. Creates a CLAUDE.md file with project guidance
3. Adds the repository to AllBeads contexts
4. Optionally runs janitor to discover existing issues
5. Sets up CI/CD recommendations

Usage:
```bash
# Onboard current directory
allbeads onboard-repo

# Onboard specific directory
allbeads onboard-repo /path/to/repo

# Non-interactive mode (use defaults)
allbeads onboard-repo --yes

# Skip specific steps
allbeads onboard-repo --skip-init      # Skip bd init
allbeads onboard-repo --skip-claude    # Skip CLAUDE.md creation
allbeads onboard-repo --skip-context   # Skip adding to contexts
```

## What gets set up:

### 1. Beads Tracking (.beads/ directory)
- Runs `bd init` to create the `.beads/` directory
- Initializes `issues.jsonl` for issue tracking
- Sets up local SQLite database

### 2. CLAUDE.md
- Creates a template CLAUDE.md file
- Includes project overview section
- Pre-fills common development commands
- Documents the project structure
- Provides guidance for AI agents

### 3. AllBeads Context
- Adds the repository to `~/.config/allbeads/config.yaml`
- Detects git remote URL automatically
- Auto-configures authentication strategy (SSH vs HTTPS)
- Makes the repo visible in `ab context list`

### 4. Optional: Janitor Analysis
- Scans codebase for potential issues
- Creates beads for TODO comments
- Identifies security concerns
- Detects code smells and technical debt

## After onboarding:

The repository will be ready for:
- Issue tracking with `bd create/list/update/close`
- Multi-context aggregation via `ab list/stats`
- TUI visualization with `ab tui`
- Agent coordination with AllBeads Mail
- Cross-repository dependency tracking

Check onboarding status:
```bash
ab context onboarding
```

## Example session:

```bash
$ cd ~/projects/my-app
$ allbeads onboard-repo

🚀 AllBeads Repository Onboarding

Repository: my-app
Path: /Users/me/projects/my-app
Remote: git@github.com:me/my-app.git

📦 Step 1: Initialize Beads Tracking
  Initialize beads tracking in this repository? [Y/n]: y
  Running: bd init
  ✓ Beads initialized successfully

📝 Step 2: Setup CLAUDE.md
  Create a starter CLAUDE.md file? [Y/n]: y
  ✓ Created CLAUDE.md
  ℹ  Edit CLAUDE.md to add project-specific guidance

🔗 Step 3: Add to AllBeads Contexts
  Add this repository to AllBeads contexts? [Y/n]: y
  ✓ Added to AllBeads contexts as 'my-app'

📚 Next Steps:

1. Create your first issue:
   cd /Users/me/projects/my-app
   bd create --title="Initial setup" --type=task --priority=2

2. View your issues in the TUI:
   ab tui

3. Check onboarding status:
   ab context onboarding

✅ Onboarding complete!
```

## See also:

- `/context` - Manage AllBeads contexts
- `/stats` - View aggregated statistics
- `/tui` - Launch the AllBeads dashboard
