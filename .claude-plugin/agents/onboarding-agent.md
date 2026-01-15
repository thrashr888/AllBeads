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
