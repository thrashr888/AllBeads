---
description: Plan a new project - create repo and beads, but don't code yet
---

Create a new GitHub repository with AllBeads and plan the work using beads issues. **Does not write code** - just sets up the project structure and creates the implementation plan.

## Workflow

1. **Gather Requirements** - Discuss the project idea with the user
2. **Create Spec** - Write a detailed specification document
3. **Create Repo** - Use `ab context new` to create GitHub repo
4. **Create README** - Add README.md with project overview and status
5. **Create Beads** - Create epic issues with specs embedded in descriptions
6. **STOP** - Do not write implementation code

The implementation will be handled later by:
- User manually starting work
- `/handoff` workflow (ab-vlq) triggering agent work
- Another session picking up from `bd ready`

## Why This Workflow?

- **Separation of Concerns** - Planning and implementation are distinct phases
- **Better Handoffs** - Beads contain all context needed for implementation
- **Interruptible** - Work can be picked up across sessions
- **Agent-Friendly** - Any agent can implement from the beads

## Usage

When the user describes a new project idea:

1. Ask clarifying questions about requirements
2. Write a spec document (can be in AllBeads/specs/ or in the new repo)
3. Create the repo:
   ```bash
   ab context new <project-name> --private --gitignore <lang> --license <license>
   ```
4. Create README.md with:
   - Project overview
   - Status: "Planning Phase"
   - Planned features table
   - Example usage (planned)
5. Create beads epics for each phase:
   ```bash
   bd create --title="[Phase 1] ..." --type=epic --priority=1
   ```
6. Add detailed descriptions to beads (specs, requirements, acceptance criteria)

## What NOT to Do

- Do NOT create source code files
- Do NOT set up build/test infrastructure
- Do NOT create CI/CD pipelines
- Do NOT implement any features

All implementation should wait for the handoff workflow or explicit user request.

## Example Session

```
User: I want to build a CLI tool that converts markdown to PDF

Agent: Great idea! Let me ask some questions:
- What language? (Go, Rust, Python?)
- Should it support themes/styles?
- Any specific PDF features (TOC, headers/footers)?

User: Go, yes to themes, and TOC would be nice

Agent: Perfect. Let me plan this project...

1. Creating spec document...
2. Creating repo with `ab context new md2pdf --private --gitignore Go --license MIT`
3. Adding README.md with project overview
4. Creating beads:
   - [Phase 1] Core CLI structure
   - [Phase 2] Markdown parsing
   - [Phase 3] PDF generation
   - [Phase 4] Theme support
   - [Phase 5] TOC generation

Project planned! The beads contain full specs. Implementation can begin via:
- `bd ready` to see available work
- `/handoff` to trigger agent implementation
- Or start manually: `bd update md2pdf-xxx --status=in_progress`
```

## See Also

- `/context-new` - Just create a repo (no planning)
- `/handoff` - Trigger agent to work on planned beads
- `bd ready` - See what's ready to implement