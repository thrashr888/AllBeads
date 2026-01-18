---
description: Agent Mail for real-time coordination between agents
---

Agent Mail provides real-time messaging between agents. Use it alongside beads for coordinating work handoffs.

## When to Use Mail vs Beads

| Use Mail | Use Beads |
|----------|-----------|
| Real-time notifications | Persistent tasks |
| Request immediate input | Work that spans sessions |
| Announce status changes | Trackable deliverables |
| Coordinate handoffs | Dependencies between work |

**Best practice**: Create a bead for the work AND send mail to notify the receiving agent.

## Commands

```bash
# Check inbox
ab mail inbox

# Check unread count
ab mail unread

# Send test messages (for verification)
ab mail test "Your message here"
```

## Agent Handoff Workflow

When handing off work to another agent or repo:

### 1. Create the Bead (Persistent Task)
```bash
ab create --context=AllBeadsWeb --title="Implement /api/beads/import" --type=feature --priority=2
```

### 2. Send Mail Notification (Real-time Alert)
```bash
ab mail test "New task created: Implement /api/beads/import endpoint for CLI sync. See abw-079."
```

### 3. Receiving Agent Picks Up
The agent in the target repo:
```bash
ab mail inbox          # Sees the notification
bd ready               # Finds the task
bd update abw-079 --status=in_progress
```

## Viewing Mail

When logged in (`ab login`), mail syncs to the web dashboard:
- **CLI**: `ab mail inbox`
- **Web**: https://allbeads.co/dashboard/mail
- **TUI**: `ab tui` (Tab to Mail view)

## Message Types

| Type | Purpose | Example |
|------|---------|---------|
| NOTIFY | Inform about events | "Build completed successfully" |
| REQUEST | Ask for input/approval | "Approve deployment to production?" |
| BROADCAST | Announce to all agents | "API rate limit reached, pausing" |
| LOCK/UNLOCK | File coordination | "Locking src/auth.rs for refactor" |
| HEARTBEAT | Agent status | "Status: working on ab-123" |

## Best Practices

1. **Always create a bead** for trackable work - mail is ephemeral
2. **Reference bead IDs** in mail messages for context
3. **Use mail for urgency** - immediate attention needed
4. **Check inbox regularly** when starting work sessions
5. **Send completion notifications** when finishing handed-off work

## Integration with TUI

The TUI dashboard shows both Kanban (beads) and Mail views:
```bash
ab tui
# Tab - switch between Kanban and Mail
# j/k - navigate messages
# Enter - view details
# q - quit
```

## See Also

- `/allbeads:create` - Create beads for cross-repo handoff
- `/allbeads:handoff` - Full handoff workflow
- `/allbeads:tui` - Dashboard with mail view
