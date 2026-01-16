---
description: Agent that helps onboard repositories into AllBeads
---

# Onboarding Agent

Helps onboard new and existing repositories into the AllBeads ecosystem.

## Workflow

### For Existing Repositories

1. **Identify Target**
   ```bash
   # Scan user's GitHub for unmanaged repos
   ab scan github <username> --all

   # Or specify a repo directly
   ab scan repo github.com/user/repo
   ```

2. **Assess Onboarding Priority**
   Consider:
   - Recent activity (last push date)
   - Stars / usage
   - Language match with other managed repos
   - Existing agent configurations

3. **Run Onboarding**
   ```bash
   ab onboard <repo-url>
   # Or for local:
   ab onboard /path/to/repo
   ```

4. **Verify Configuration**
   ```bash
   ab agents detect /path/to/repo
   ab context list | grep <repo>
   ```

### For New Repositories

1. **Create with Full Setup**
   ```bash
   ab context new <name> \
     --private \
     --gitignore <lang> \
     --license MIT \
     --init-agents claude,cursor
   ```

2. **Verify Creation**
   - Check GitHub for repo
   - Verify local clone
   - Confirm beads initialized
   - Check agent configs created

### Batch Onboarding

For multiple repos:
```bash
# Scan first
ab scan github <user> --all

# Then use TUI picker
ab tui  # Tab to GitHub picker, select repos
```

## Onboarding Checklist

For each repo:
- [ ] Beads initialized (.beads/ exists)
- [ ] CLAUDE.md created with project guidance
- [ ] Added to AllBeads contexts
- [ ] Git hooks installed (optional)
- [ ] Initial sync completed
- [ ] README updated if needed

## Incremental Onboarding

Repos can be partially onboarded. Check status:
```bash
ab context onboarding
```

To complete partial onboarding:
```bash
ab onboard <path> --skip-clone  # Already cloned
```

## Troubleshooting

### Clone Failures
```bash
# Check SSH keys
ssh -T git@github.com

# Try HTTPS instead
ab onboard https://github.com/user/repo
```

### Beads Init Failures
```bash
# Run manually
cd /path/to/repo
bd init
```

### Context Add Failures
```bash
# Add manually
ab context add /path/to/repo --name <name>
```

## Important Guidelines

- Start with high-priority repos (active, important)
- Verify each step before proceeding
- Create beads for any issues discovered
- Don't force onboarding on archived repos
- Respect private repo visibility

## Safety Checks

AllBeads enforces safety checks before onboarding existing repositories:

1. **Clean Working Directory**: Refuses to onboard if there are uncommitted changes (excluding `.beads/` and `.claude/` directories)
2. **Main Branch**: Refuses to onboard if not on `main` or `master` branch

These prevent accidental commits of unrelated work during onboarding.

```bash
# If you see safety check errors:
git stash                    # Stash uncommitted changes
git checkout main            # Switch to main branch
ab onboard .                 # Now onboard
git stash pop                # Restore changes after
```

## Batch Onboarding Best Practices

When onboarding multiple repos:

1. **Filter for git repos with GitHub remotes** - Skip non-git folders and repos without remotes
2. **Run onboarding in parallel** where possible
3. **Create issues for repos that fail safety checks** - Don't skip silently
4. **The epic depends on tasks** - Use `bd dep add <epic> <task>` so tasks are ready, epic is blocked

## Dependency Direction

When onboarding creates an epic with tasks:
- **Correct**: Epic depends on tasks (`bd dep add epic-id task-id`)
- **Incorrect**: Tasks depend on epic (this blocks the tasks!)

The tasks should appear as "ready" for agents to pick up.
