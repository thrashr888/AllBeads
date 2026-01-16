# External Integration Strategy: GitHub & JIRA

## Overview

AllBeads supports two integration modes with external issue trackers (GitHub Issues, JIRA):

1. **Link Mode** (Default): Reference external issues without syncing
2. **Sync Mode**: Bi-directional synchronization of issues

## Link Mode (Recommended for Most Users)

Link mode maintains references to external issues without creating or modifying beads.

### Use Cases
- View linked issues: `ab show <bead-id>` shows external references
- Open linked issues: `ab open <bead-id>` opens the external issue in browser
- Cross-reference: Beads can reference GH/JIRA issues in descriptions

### Configuration
```yaml
# No additional config needed - linking works by convention
# Reference issues in bead descriptions using:
#   - GitHub: owner/repo#123 or https://github.com/owner/repo/issues/123
#   - JIRA: PROJ-123 or https://company.atlassian.net/browse/PROJ-123
```

### Commands
```bash
ab open PROJ-123              # Open JIRA issue in browser
ab open owner/repo#123        # Open GitHub issue in browser
ab open <bead-id>             # Open linked external issue
```

## Sync Mode (Enterprise)

Sync mode creates Shadow Beads from external issues and syncs status changes bidirectionally.

### Use Cases
- Enterprise teams using JIRA as source of truth
- Cross-system visibility (see all work in AllBeads TUI)
- Agent coordination across external issues

### Ingress (External -> AllBeads)

**Pull Issues:**
```bash
ab jira pull --project PROJ --url https://company.atlassian.net
ab github pull --owner myorg --repo myrepo
```

**What Happens:**
1. Fetch issues from external system
2. Create Shadow Beads with `external_ref` field
3. Map status: Open -> open, In Progress -> in_progress, Done -> closed
4. Map priority: JIRA (Highest=P0, High=P1, Medium=P2, Low=P3, Lowest=P4)

### Egress (AllBeads -> External)

**Push Changes:**
When a bead with `external_ref` is updated, the Sheriff daemon:
1. Detects the state change
2. Maps bead status to external transition
3. Adds comment to external issue with change summary

### Configuration
```yaml
contexts:
  - name: work
    integrations:
      jira:
        url: https://company.atlassian.net
        project: PROJ
        sync_mode: pull_only  # or bidirectional
      github:
        owner: myorg
        sync_mode: pull_only  # or bidirectional
```

### Sync Modes
- `pull_only`: Only import issues, no write-back (safe)
- `bidirectional`: Full sync with external system (requires permissions)

## Decision: When to Use Each Mode

| Scenario | Recommended Mode |
|----------|------------------|
| Personal projects | Link Mode |
| Small teams using beads | Link Mode |
| Enterprise with JIRA | Sync Mode (pull_only) |
| Cross-team coordination | Sync Mode (bidirectional) |
| Compliance/audit requirements | Sync Mode (bidirectional) |

## Implementation Status

### Implemented
- [x] `ab jira status` - Check JIRA configuration
- [x] `ab jira pull` - Pull issues from JIRA
- [x] `ab github status` - Check GitHub configuration
- [x] `ab github pull` - Pull issues from GitHub
- [x] Shadow Bead creation with external_ref
- [x] Status mapping (external -> beads)

### Planned
- [ ] `ab open` command for opening linked issues
- [ ] Bidirectional sync (beads -> external)
- [ ] Comment sync
- [ ] Attachment handling
- [ ] Webhook support for real-time sync

## See Also

- PRD-00.md Section 5: Enterprise Integration
- DEMO.md: Enterprise Integration section
