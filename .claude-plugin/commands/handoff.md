---
description: Hand off planned work to implementation
---

Trigger implementation of planned work. Links to the handoff workflow (ab-vlq).

## Usage

```bash
# Start implementation of ready work
/handoff

# Handoff specific bead
/handoff <bead-id>

# Handoff in specific context
/handoff --context=<name>
```

## Workflow

1. **Find Ready Work**
   ```bash
   ab ready                # Or bd ready in specific repo
   ```

2. **Select Task**
   - User specifies or agent picks highest priority

3. **Claim Task**
   ```bash
   ab update <id> --status=in_progress
   ```

4. **Implement**
   - Follow spec in bead description/comments
   - Write code, tests
   - Follow CLAUDE.md guidelines

5. **Complete**
   ```bash
   git add -A && git commit -m "..."
   ab close <id> --reason="..."
   git push
   ```

6. **Continue**
   - Check for newly unblocked work
   - Repeat

## Handoff from Planning

After `/project-new` creates a repo with specs:

```bash
/handoff terraform-provider-registry-d6e  # Start Phase 1
```

The task agent takes over and implements based on specs.

## Important

- Handoff requires specs in bead comments
- Empty beads cannot be handed off
- Verify dependencies are resolved first
- Push changes before completing

## See Also

- `/project-new` - Plan a new project
- `/ready` - View ready work
- `ab-vlq` - Handoff workflow bead
