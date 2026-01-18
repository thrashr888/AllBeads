# Tutorials

Step-by-step guides for common AllBeads workflows.

## Tutorial 1: Onboarding an Existing Repository

This tutorial walks through adding an existing git repository to AllBeads with full beads support.

### Prerequisites

- A git repository with a remote configured
- `bd` (beads CLI) installed

### Step 1: Initialize Beads in the Repository

Navigate to your repository:

```bash
cd /path/to/my-project
```

Initialize beads:

```bash
bd init
```

This creates the `.beads/` directory with:
- `issues.jsonl` - Issue database
- `config.yaml` - Beads configuration

### Step 2: Add the Repository to AllBeads

Add as a context:

```bash
ab context add .
```

AllBeads auto-detects:
- Name from folder (`my-project`)
- URL from git remote
- Authentication strategy

Verify it was added:

```bash
ab context list
```

### Step 3: Create Your First Beads

Create an onboarding epic:

```bash
bd create --title "Onboard my-project to AllBeads" --type epic --priority 1
```

Add some tasks:

```bash
bd create --title "Configure CLAUDE.md" --type task --priority 2
bd create --title "Set up CI/CD integration" --type task --priority 3
bd create --title "Write initial documentation" --type task --priority 3
```

### Step 4: View in AllBeads

Check your statistics:

```bash
ab stats
```

View your beads:

```bash
ab list --context my-project
```

### Step 5: Sync and Commit

Sync your beads:

```bash
bd sync
```

This commits the `.beads/` changes and pushes to the remote.

---

## Tutorial 2: Setting Up JIRA Integration

Connect AllBeads to JIRA for bi-directional issue sync.

### Step 1: Create JIRA API Token

1. Go to https://id.atlassian.com/manage/api-tokens
2. Click "Create API token"
3. Give it a name like "AllBeads Integration"
4. Copy the token

### Step 2: Configure the Token

Set the environment variable:

```bash
export JIRA_API_TOKEN='your-token-here'
```

Add to your shell profile (`~/.bashrc`, `~/.zshrc`) for persistence:

```bash
echo 'export JIRA_API_TOKEN="your-token-here"' >> ~/.zshrc
```

### Step 3: Verify Configuration

Check the integration status:

```bash
ab jira status
```

Expected output:
```
JIRA Integration Status

  API Token: Set (from JIRA_API_TOKEN)
```

### Step 4: Pull Issues from JIRA

Pull issues with a specific label:

```bash
ab jira pull --project PROJ --url https://your-company.atlassian.net --label ai-agent
```

Options:
- `--project` - JIRA project key (e.g., PROJ, ENG, PLATFORM)
- `--url` - Your Atlassian instance URL
- `--label` - Filter to issues with this label

### Step 5: View Imported Issues

The imported issues appear as Shadow Beads:

```bash
ab search --status open
```

Each imported issue has an `external_ref` pointing to the original JIRA issue.

---

## Tutorial 3: Setting Up GitHub Integration

Connect AllBeads to GitHub Issues.

### Step 1: Create GitHub Token

1. Go to https://github.com/settings/tokens
2. Click "Generate new token" (classic)
3. Select scopes:
   - `repo` for private repositories
   - `public_repo` for public only
4. Copy the token

### Step 2: Configure the Token

Set the environment variable:

```bash
export GITHUB_TOKEN='ghp_xxxxxxxxxxxx'
```

Or use GitHub CLI (automatically detected):

```bash
gh auth login
```

### Step 3: Verify Configuration

Check the integration status:

```bash
ab github status
```

Expected output:
```
GitHub Integration Status

  API Token: Set (from GITHUB_TOKEN)
```

### Step 4: Pull Issues from GitHub

Pull from an organization:

```bash
ab github pull --owner my-org
```

Or from a specific repository:

```bash
ab github pull --owner my-org --repo my-repo --label ai-agent
```

### Step 5: View Imported Issues

```bash
ab search --status open
```

---

## Tutorial 4: Using Agent Handoff

Hand off work to AI agents for autonomous completion.

### Step 1: Find Ready Work

List beads that are ready for work:

```bash
ab handoff --ready
```

Or use:

```bash
ab ready
```

### Step 2: Review the Bead

Before handing off, review the bead details:

```bash
ab show ab-123
```

Ensure:
- The description is clear
- Dependencies are resolved
- Scope is well-defined

### Step 3: Configure Your Preferred Agent

On first use, AllBeads prompts you to select an agent:

```bash
ab handoff ab-123
```

Output:
```
Select your preferred agent:
  1. claude (CLI)
  2. cursor (IDE)
  3. codex (CLI)
  4. gemini (CLI)
  5. aider (CLI)
```

Your preference is saved to `.beads/config.yaml`.

### Step 4: Hand Off the Bead

```bash
ab handoff ab-123
```

This:
1. Updates the bead status to `in_progress`
2. Records handoff metadata (agent, timestamp)
3. Launches the agent with bead context

### Step 5: Monitor Progress

Check the bead status:

```bash
ab show ab-123
```

The handoff information appears in the output:
```
Handoff:
  Agent:     claude
  Started:   2026-01-10 14:30:00
  Branch:    bead/ab-123
```

### Alternative: Specific Agent

To use a specific agent instead of your default:

```bash
ab handoff ab-123 --agent gemini
ab handoff ab-123 --agent cursor
```

### Alternative: Isolated Worktree

For parallel work, use an isolated git worktree:

```bash
ab handoff ab-123 --worktree
```

This creates `../<repo>-ab-123/` with the work isolated from main.

---

## Tutorial 5: Running the Sheriff Daemon

Keep beads synchronized across repositories.

### Step 1: Start in Foreground Mode

For development, run in foreground to see all events:

```bash
ab sheriff --foreground
```

Output:
```
[2026-01-10 12:00:00] Starting Sheriff daemon...
[2026-01-10 12:00:00] Loaded 3 rigs from config
[2026-01-10 12:00:00] Poll cycle started
```

### Step 2: Observe Sync Events

As the Sheriff runs, it logs events:

```
[2026-01-10 12:00:01] Synced rig 'auth-service': 3 shadows updated
[2026-01-10 12:00:02] Synced rig 'web-app': 1 new shadow created
[2026-01-10 12:00:02] External sync: 5 JIRA issues pulled
[2026-01-10 12:00:02] Poll cycle complete (2.1s)
```

### Step 3: Customize Poll Interval

For faster sync during active development:

```bash
ab sheriff -f -p 5    # Poll every 5 seconds
```

For production (less load):

```bash
ab sheriff -f -p 60   # Poll every minute
```

### Step 4: Understanding the Sync Cycle

Each poll cycle:

1. **Fetch**: Pull beads from all configured Rigs
2. **Diff**: Compare with cached state
3. **Sync**: Update Shadow Beads in Boss repo
4. **External**: Bi-directional sync with JIRA/GitHub
5. **Mail**: Process pending agent messages

### Step 5: Stop the Daemon

Press `Ctrl+C` to stop gracefully.

---

## Tutorial 6: Using the TUI Dashboard

Navigate work visually with the terminal UI.

### Step 1: Launch the TUI

```bash
ab tui
```

### Step 2: Kanban View (Default)

The Kanban board shows three columns:

```
┌─────────────┬─────────────┬─────────────┐
│    Open     │ In Progress │   Closed    │
├─────────────┼─────────────┼─────────────┤
│ ○ ab-123    │ ◐ ab-456    │ ✓ ab-789    │
│ P1 Auth     │ P2 API      │ P2 Docs     │
│ @auth-svc   │ @web-app    │ @auth-svc   │
│             │             │             │
│ ○ ab-124    │             │ ✓ ab-790    │
│ P2 Tests    │             │ P3 Cleanup  │
│ @web-app    │             │ @api-gw     │
└─────────────┴─────────────┴─────────────┘
```

**Navigation:**
- `j`/`k` - Move up/down within column
- `h`/`l` - Switch between columns
- `Enter` - View bead details

### Step 3: Mail View

Press `Tab` to switch to Mail view:

```
┌─────────────────────────────────────────┐
│              Agent Mail                 │
├─────────────────────────────────────────┤
│ ● [agent-1] Task Complete               │
│   Finished auth refactor                │
│   2026-01-10 14:30                      │
│                                         │
│ ○ [agent-2] Help Request                │
│   Need clarification on API...          │
│   2026-01-10 13:15                      │
└─────────────────────────────────────────┘
```

**Actions:**
- `r` - Mark message as read
- `Enter` - View full message

### Step 4: Graph View

Press `Tab` again to see the dependency graph:

```
┌─────────────────────────────────────────┐
│           Dependency Graph              │
├─────────────────────────────────────────┤
│ ○ ab-100: Epic - Auth System            │
│   └─○ ab-101: OAuth flow                │
│     └─⬡ ab-102: Web integration         │
│   └─○ ab-103: Token refresh             │
│                                         │
│ ⊘ ab-200: Epic - API Gateway            │
│   └─⟳ ab-201: Rate limiting (blocked)   │
└─────────────────────────────────────────┘
```

**Indicators:**
- `○` Normal (green)
- `⬡` Cross-context dependency (magenta)
- `⊘` Blocked (yellow)
- `⟳` Cycle detected (red)

**Filters:**
- `f` - Cycle through: All → Blocked Only → Cross-Context

### Step 5: Swarm View

Press `Tab` again for agent monitoring:

```
┌─────────────────────────────────────────┐
│            Agent Swarm                  │
├─────────────────────────────────────────┤
│ ● claude@auth-svc    ACTIVE  ab-123     │
│   Running for 5m, last heartbeat 2s ago │
│                                         │
│ ○ gemini@web-app     IDLE               │
│   Available for work                    │
│                                         │
│ ◐ cursor@api-gw      PAUSED  ab-456     │
│   Paused by user                        │
└─────────────────────────────────────────┘
```

**Actions:**
- `p` - Pause agent
- `r` - Resume agent
- `x` - Kill agent

### Step 6: Exit

Press `q` or `Ctrl+C` to quit.

---

## Tutorial 7: Multi-Repository Workflow

Coordinate work across multiple repositories.

### Step 1: Add Multiple Contexts

```bash
cd ~/workspace/auth-service && ab context add .
cd ~/workspace/web-app && ab context add .
cd ~/workspace/api-gateway && ab context add .
```

### Step 2: View Aggregated State

See statistics across all repos:

```bash
ab stats
```

Output:
```
AllBeads Statistics:

  Total beads:      156
  Open:             45
  In Progress:      8
  Blocked:          3
  Closed:           100

Contexts:
  auth-service    52 beads (15 open)
  web-app         68 beads (20 open)
  api-gateway     36 beads (10 open)
```

### Step 3: Find Cross-Repo Dependencies

Use the TUI Graph view or:

```bash
ab blocked
```

This shows beads blocked by dependencies in other repos:

```
Blocked beads: 3

[P1] wa-456: Implement OAuth login (@web-app)
  Blocked by: auth-123 (@auth-service)

[P2] gw-789: Add rate limiting (@api-gateway)
  Blocked by: wa-456 (@web-app)
```

### Step 4: Prioritize Ready Work

Find work that's unblocked:

```bash
ab ready
```

Filter to a specific context:

```bash
ab ready --context auth-service
```

### Step 5: Work Across Contexts

Switch between repos as needed:

```bash
# Work on auth-service
cd ~/workspace/auth-service
bd update auth-123 --status in_progress
# ... do work ...
bd close auth-123

# Now wa-456 is unblocked in web-app
cd ~/workspace/web-app
bd update wa-456 --status in_progress
```

### Step 6: Sync All

Sync beads in all contexts:

```bash
ab sync --all
```

---

## Summary

| Tutorial | Focus |
|----------|-------|
| 1 | Onboarding a new repository |
| 2 | JIRA integration setup |
| 3 | GitHub integration setup |
| 4 | AI agent handoff |
| 5 | Sheriff daemon operation |
| 6 | TUI dashboard navigation |
| 7 | Multi-repository coordination |

For command details, see the [CLI Reference](./cli-reference.md).
