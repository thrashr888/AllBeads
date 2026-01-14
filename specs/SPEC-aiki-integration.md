# SPEC: AllBeads + Aiki Integration

**Status:** Updated / Ready for Implementation
**Author:** Claude Sonnet 4.5 + thrashr888
**Date:** 2026-01-13
**Last Updated:** 2026-01-13

---

## Executive Summary

AllBeads and Aiki solve complementary problems in AI-assisted development:

| System | Focus | Core Innovation |
|--------|-------|-----------------|
| **AllBeads** | *Cross-repo orchestration* | Multi-repo issue tracking with dependencies, Sheriff daemon, Agent Mail protocol |
| **Aiki** | *Single-repo AI workflow* | Task management, JJ-based event sourcing, edit-level provenance tracking |

**Key Insight from Latest Aiki Analysis (2026-01-13)**:
- Aiki has a **fully implemented task system** (not review system)
- Tasks stored on `aiki/tasks` JJ branch as event-sourced data
- Hierarchical tasks with parent/child relationships
- XML output optimized for AI agent consumption
- **No code review system implemented** (contrary to older specs)

Together, they could provide **complete traceability from strategic intent to tactical execution**: AllBeads tracks cross-repo epics and dependencies, Aiki tracks single-repo tasks and code changes.

---

## Current State: What Each System Actually Has

### AllBeads (v0.4.0) - IMPLEMENTED

**Core Features**:
- Multi-context aggregation (Boss repository pattern)
- Sheriff daemon for cross-repo synchronization
- Agent Mail protocol (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT)
- TUI with Kanban, Mail, Graph, Timeline, Governance, Stats views
- Governance policies with git hook enforcement
- JIRA and GitHub Issues bi-directional sync
- SQLite cache layer with automatic expiration

**Storage**: JSONL files in `.beads/` directory, Git-backed

**Data Model**:
```rust
struct Bead {
    id: BeadId,                    // ab-xyz format
    title: String,
    description: Option<String>,
    status: Status,                // Open, InProgress, Blocked, Closed
    priority: Priority,            // P0-P4
    issue_type: IssueType,         // Bug, Feature, Task, Epic
    dependencies: Vec<BeadId>,     // What this depends on
    blocks: Vec<BeadId>,           // What this blocks
    labels: HashSet<String>,
}
```

### Aiki (Latest) - IMPLEMENTED

**Core Features**:
- Task management system for AI agents
- JJ-based event sourcing on `aiki/tasks` branch
- Hierarchical tasks (parent.child.grandchild)
- Scope-based ready queue filtering
- XML output for AI consumption
- Edit-level provenance tracking ([aiki] blocks in JJ descriptions)
- Batch operations (start/stop/close multiple tasks)

**Storage**: Fileless JJ changes on `aiki/tasks` branch

**Data Model**:
```rust
struct Task {
    id: String,                    // JJ-style: zkmqwyx (32 char) or parent.N for children
    name: String,
    status: TaskStatus,            // Open, InProgress, Stopped, Closed
    priority: TaskPriority,        // P0-P3
    assignee: Option<String>,
    stopped_reason: Option<String>,
    closed_outcome: Option<TaskOutcome>,  // Done, WontDo
}
```

**Event Types**:
- Created, Started (batch), Stopped (batch), Closed (batch), Reopened, CommentAdded, Updated

**CLI Commands**:
```bash
aiki task add <name> [--parent parent_id] [--p0/p1/p2/p3]
aiki task list [--all | --open | --in-progress | --stopped | --closed]
aiki task start [id...] [--reopen --reason <text>]
aiki task stop [id] [--reason <text>] [--blocked <reason>]
aiki task close [id...] [--wont-do] [--duplicate <id>]
aiki task show [id]
aiki task update [id] [--name <text>] [--p0/p1/p2/p3]
aiki task comment [id] <text>
```

**What Aiki Does NOT Have**:
- ❌ Code review system (not implemented)
- ❌ Multi-repository support
- ❌ Cross-repo dependencies
- ❌ External integrations (JIRA, GitHub)
- ❌ Agent Mail protocol

---

## The Integration Gap

### Overlap: Both Have Task/Issue Systems

**Problem**: Two different task systems that could conflict:
- AllBeads beads (cross-repo, strategic)
- Aiki tasks (single-repo, tactical)

**Solution**: Don't compete - complement. Use each for what it's best at.

### What Each System Needs From The Other

**AllBeads Needs From Aiki**:
1. Fine-grained task decomposition within a repo
2. JJ change provenance linking
3. Agent-friendly XML output for task querying
4. Session-scoped task management

**Aiki Needs From AllBeads**:
1. Cross-repo orchestration (Aiki is single-repo only)
2. Strategic epic tracking that spans repositories
3. Agent Mail for cross-repo coordination
4. Integration with external systems (JIRA, GitHub)

---

## Integration Architecture: Hierarchical Decomposition

### Design Philosophy: Strategic vs Tactical

```
┌─────────────────────────────────────────────────────────────────┐
│                        AllBeads (Strategic)                      │
│  Cross-Repo Epics, Dependencies, External Integrations          │
└─────────────────────────────────────────────────────────────────┘
                           │
                           │ decomposes into
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    AllBeads Beads (Tactical)                     │
│  Repo-Specific Tasks, Implementation Work                        │
└─────────────────────────────────────────────────────────────────┘
                           │
                           │ decomposes into (optional)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Aiki Tasks (Micro)                          │
│  Session-Level Subtasks, Code-Level Changes                      │
└─────────────────────────────────────────────────────────────────┘
```

**Example Hierarchy**:
```
AllBeads Epic: ab-2xf "Implement Multi-Tenant Auth" (@allbeads context)
  ├─ AllBeads Bead: ab-3gh "Auth Service Changes" (@auth-service repo)
  │   └─ Aiki Task: zkmqwyx "Fix JWT validation bug" (local to auth-service)
  │       ├─ Aiki Task: zkmqwyx.1 "Add null check" (subtask)
  │       └─ Aiki Task: zkmqwyx.2 "Update tests" (subtask)
  └─ AllBeads Bead: ab-4jk "Frontend Login Flow" (@frontend repo)
      └─ Aiki Task: plwmrst "Implement token refresh" (local to frontend)
```

### Integration Strategy: Linked Not Merged

**Key Principle**: Keep systems separate, link via metadata.

**AllBeads → Aiki**: Beads can reference Aiki tasks
```yaml
# .beads/issues/ab-3gh.jsonl
{
  "id": "ab-3gh",
  "title": "Auth Service Changes",
  "aiki_tasks": ["zkmqwyx", "plwmrst"],  # Optional field
  "workdir": "/path/to/auth-service"
}
```

**Aiki → AllBeads**: Tasks can reference beads
```
# JJ change description with [aiki] block
[aiki]
author=claude-code
session=claude-session-abc123
task=zkmqwyx
bead=ab-3gh         # Link back to AllBeads bead
[/aiki]

# Also in task event metadata
[aiki-task]
event=created
task_id=zkmqwyx
name=Fix JWT validation bug
bead=ab-3gh         # Link to strategic bead
[/aiki-task]
```

---

## Integration Phases

### Phase 1: Metadata Linking (Already Partially Complete)

**Status**: ✅ AllBeads side complete, ⏳ Aiki side needs work

**AllBeads Changes (Complete)**:
- ✅ `ab aiki activate <bead-id>` sets `AB_ACTIVE_BEAD` env var
- ✅ `ab aiki status` shows active bead
- ✅ `ab show --provenance` queries Aiki (graceful fallback)

**Aiki Changes (Needed)**:
- ❌ Read `AB_ACTIVE_BEAD` from environment in flow hooks
- ❌ Include `bead=<id>` in JJ change [aiki] blocks
- ❌ Include `bead=<id>` in task event metadata
- ❌ Query support: `aiki task list --bead=<id>`

**Implementation**:
```rust
// cli/src/flows/bundled.yaml
change.completed:
  - let: active_bead = $env.AB_ACTIVE_BEAD
  - if: $active_bead
    then:
      - let: metadata = |
          [aiki]
          author=$event.agent
          session=$event.session_id
          task=$event.task_id
          bead=$active_bead
          [/aiki]
      - jj: describe --message "$metadata"
```

**Benefits**:
- Loose coupling - systems remain independent
- Graceful degradation - works without Aiki installed
- Simple to implement - just metadata fields

---

### Phase 2: Bidirectional Queries

**Goal**: Query tasks/beads across the boundary

**AllBeads Queries**:
```bash
# Show Aiki tasks linked to a bead
ab show ab-3gh --tasks

# Output:
ab-3gh: Auth Service Changes
  Status: in_progress
  Aiki Tasks:
    zkmqwyx - Fix JWT validation bug [in_progress]
    zkmqwyx.1 - Add null check [closed]
    zkmqwyx.2 - Update tests [open]
```

**Aiki Queries**:
```bash
# Show tasks for a bead
aiki task list --bead=ab-3gh

# Output (XML):
<aiki_task cmd="list" status="ok">
  <list total="3" bead="ab-3gh">
    <task id="zkmqwyx" name="Fix JWT validation bug" status="in_progress"/>
    <task id="zkmqwyx.1" name="Add null check" status="closed"/>
    <task id="zkmqwyx.2" name="Update tests" status="open"/>
  </list>
</aiki_task>
```

**Implementation Requirements**:

AllBeads side (`src/main.rs`):
```rust
// Enhance existing query_aiki_provenance function
fn query_aiki_tasks(bead_id: &str, repo_path: &Path) -> Result<Vec<AikiTaskSummary>> {
    let output = Command::new("aiki")
        .args(&["task", "list", &format!("--bead={}", bead_id), "--format=xml"])
        .current_dir(repo_path)
        .output()?;

    // Parse XML and return task list
}
```

Aiki side (`cli/src/tasks/manager.rs`):
```rust
// Add bead filtering to ready queue calculation
pub fn ready_queue_for_bead(&self, bead_id: &str) -> Vec<Task> {
    self.tasks.values()
        .filter(|t| t.status == TaskStatus::Open)
        .filter(|t| t.bead.as_ref() == Some(bead_id))
        .sorted_by_priority()
        .collect()
}
```

---

### Phase 3: Agent Mail Integration for Status Sync

**Goal**: Aiki task completions can update AllBeads bead status

**Flow**:
```
Aiki Task Closed
    ↓
Aiki sends Agent Mail message
    ↓
AllBeads Sheriff receives message
    ↓
Sheriff checks if all Aiki tasks for bead are closed
    ↓
If yes: Auto-close AllBeads bead
```

**Aiki Changes**:
```rust
// cli/src/tasks/manager.rs - When closing task
pub fn close_task(&mut self, task_id: &str) -> Result<()> {
    // ... existing close logic ...

    // If task has bead link, notify via Agent Mail
    if let Some(bead_id) = task.bead {
        self.notify_bead_update(bead_id, task_id, "task_closed")?;
    }
}

fn notify_bead_update(&self, bead_id: &str, task_id: &str, event: &str) -> Result<()> {
    // Send HTTP POST to localhost:7878 (Postmaster)
    let client = reqwest::blocking::Client::new();
    client.post("http://localhost:7878/send")
        .json(&json!({
            "from": "aiki@localhost",
            "to": "sheriff@localhost",
            "type": "Notify",
            "payload": {
                "message": format!("Task {} closed for bead {}", task_id, bead_id),
                "bead_id": bead_id,
                "task_id": task_id,
                "event": event
            }
        }))
        .send()?;
    Ok(())
}
```

**AllBeads Changes**:
```rust
// src/sheriff/sync.rs - Sheriff poll cycle
fn handle_aiki_notifications(&mut self) -> Result<()> {
    // Check for Aiki task notifications
    let messages = self.postmaster.list_for("sheriff@localhost")?;

    for msg in messages.iter().filter(|m| m.from.user == "aiki") {
        if let MessageType::Notify(n) = &msg.message_type {
            if let Some(bead_id) = &n.bead_id {
                // Check if all Aiki tasks for this bead are complete
                if self.all_aiki_tasks_closed(bead_id)? {
                    // Auto-close the bead
                    self.close_bead(bead_id, "All Aiki tasks completed")?;
                }
            }
        }
    }

    Ok(())
}
```

**Benefits**:
- Automatic synchronization without polling
- Leverages existing Agent Mail infrastructure
- Optional - works even if Agent Mail not available

---

### Phase 4: Unified TUI (Future)

**Goal**: View both AllBeads beads and Aiki tasks in one interface

**New TUI Tab**: "Tasks" view
- Shows AllBeads beads in current context
- Expands to show linked Aiki tasks
- Hierarchical tree view:
  ```
  [Epic] ab-2xf: Implement Multi-Tenant Auth
    [Task] ab-3gh: Auth Service Changes [in_progress]
      └─ [Aiki] zkmqwyx: Fix JWT validation bug [in_progress]
         ├─ [Aiki] zkmqwyx.1: Add null check [closed]
         └─ [Aiki] zkmqwyx.2: Update tests [open]
    [Task] ab-4jk: Frontend Login Flow [open]
  ```

**Implementation**: New view in `src/tui/tasks_unified_view.rs`

---

## Use Cases

### Use Case 1: Epic Decomposition

**Scenario**: Large epic needs breakdown into repo-specific tasks, then code-level subtasks

```bash
# Create epic in AllBeads
ab create --title "Implement Multi-Tenant Auth" --type epic --priority 0

# Create repo-specific beads
ab create --title "Auth Service Changes" --type task --priority 1
ab dep add ab-3gh ab-2xf  # Auth service depends on epic

# Switch to auth-service repo
cd ~/auth-service

# Activate the bead for Aiki tracking
ab aiki activate ab-3gh

# Create Aiki tasks
aiki task add "Fix JWT validation bug"
aiki task add "Update middleware" --parent zkmqwyx
aiki task add "Add integration tests" --parent zkmqwyx

# Work on tasks
aiki task start zkmqwyx.1
# ... make code changes ...
aiki task close

# When all Aiki tasks done, Aiki notifies AllBeads
# AllBeads auto-closes ab-3gh
```

### Use Case 2: Cross-Repo Coordination

**Scenario**: Change requires updates in multiple repos

```bash
# AllBeads tracks the cross-repo epic
ab create --title "Migrate to New API" --type epic

# Create beads for each affected repo
ab create --title "Backend API Changes" --type task
ab create --title "Frontend Client Updates" --type task
ab create --title "Mobile App Updates" --type task

# Add dependencies
ab dep add ab-5mn ab-4kl  # Frontend depends on backend
ab dep add ab-6op ab-4kl  # Mobile depends on backend

# In backend repo
cd ~/backend
ab aiki activate ab-4kl
aiki task add "Update endpoint contracts"
aiki task add "Migrate database schema"

# AllBeads Sheriff shows cross-repo status
ab ready  # Shows what's unblocked across all repos
```

### Use Case 3: Agent Handoff

**Scenario**: One agent starts work, another agent continues

```bash
# Agent 1 in Claude Code
cd ~/auth-service
ab aiki activate ab-3gh
aiki task start zkmqwyx
# ... makes some changes ...
aiki task stop --reason "Need security review"

# Agent 2 in Cursor
cd ~/auth-service
aiki task list  # Sees stopped task
aiki task show zkmqwyx  # Reads context
aiki task start zkmqwyx  # Resumes work
# ... completes the task ...
aiki task close
```

---

## Implementation Plan

### Phase 1: Metadata Linking (2-3 days)

**Aiki Work**:
1. Add `bead` field to task events
2. Read `AB_ACTIVE_BEAD` from environment in flows
3. Include `bead=<id>` in JJ change [aiki] blocks
4. Add `--bead` filter to `aiki task list`

**AllBeads Work**:
1. ✅ Already complete

**Testing**:
- Create bead, activate it, create Aiki tasks, verify linking

### Phase 2: Bidirectional Queries (3-5 days)

**Aiki Work**:
1. Implement `aiki task list --bead=<id>` with XML output
2. Add task summary to `aiki show` output

**AllBeads Work**:
1. Enhance `ab show --tasks` to query Aiki
2. Display Aiki tasks in bead details

**Testing**:
- Query tasks from beads, verify accurate results
- Test graceful fallback when Aiki not installed

### Phase 3: Agent Mail Integration (5-7 days)

**Aiki Work**:
1. Add Agent Mail client to Aiki
2. Send notifications on task state changes
3. Configure Postmaster endpoint

**AllBeads Work**:
1. Handle Aiki notifications in Sheriff
2. Auto-update bead status based on task completions
3. Add policy: "close bead when all tasks done"

**Testing**:
- Create bead, create tasks, close tasks, verify bead auto-closes
- Test with Postmaster unavailable (graceful degradation)

### Phase 4: Unified TUI (10-14 days)

**AllBeads Work**:
1. Create new `tasks_unified_view.rs`
2. Hierarchical tree widget for beads + tasks
3. Keyboard navigation (expand/collapse)
4. Add to TUI tab bar

**Testing**:
- Navigate tree, verify task details
- Test with/without Aiki installation

---

## Success Metrics

### Phase 1
- ✅ Beads linked to Aiki tasks via metadata
- ✅ JJ changes include bead ID in [aiki] blocks
- ✅ Query tasks by bead ID

### Phase 2
- ✅ `ab show <id> --tasks` displays Aiki tasks
- ✅ `aiki task list --bead=<id>` filters correctly
- ✅ Cross-query works in both directions

### Phase 3
- ✅ Closing all Aiki tasks auto-closes AllBeads bead
- ✅ Sheriff receives and processes Aiki notifications
- ✅ No manual sync required

### Phase 4
- ✅ Unified TUI shows both beads and tasks
- ✅ Tree navigation works smoothly
- ✅ Agents use unified view for context

---

## Open Questions

### Q1: Should AllBeads beads auto-create Aiki tasks?

**Option A**: Manual - User creates tasks explicitly
**Option B**: Auto - Starting a bead auto-creates Aiki task

**Recommendation**: Manual for Phase 1, auto in Phase 2+ with flag

### Q2: What happens when Aiki tasks exist but bead is closed?

**Option A**: Orphaned - Aiki tasks remain open
**Option B**: Auto-close - AllBeads closing bead closes Aiki tasks
**Option C**: Warning - Prevent closing bead with open tasks

**Recommendation**: Option C for safety, Option B as advanced feature

### Q3: Should Aiki support multiple beads per task?

**Current**: One task → one bead (optional)
**Future**: One task → multiple beads?

**Recommendation**: Keep single-bead for simplicity

---

## Comparison to Old Spec

**Major Changes from Original 2026-01-11 Spec**:

1. **Removed Review System References**: Aiki doesn't have review system
2. **Changed from provenance focus to task integration**: More accurate to what Aiki provides
3. **Hierarchical decomposition model**: Strategic (AllBeads) → Tactical (Aiki)
4. **Realistic implementation phases**: Based on actual codebases
5. **Agent Mail for sync**: Leverages existing AllBeads infrastructure

**Kept from Original**:
- Loose coupling philosophy
- Environment variable bridge (Phase 1)
- Graceful fallback when Aiki unavailable
- Both systems remain independently useful

---

## Summary

This integration connects AllBeads' cross-repo orchestration with Aiki's single-repo task management:

- **Phase 1**: Link via metadata (bead ID in Aiki tasks)
- **Phase 2**: Bidirectional queries (see tasks from beads, vice versa)
- **Phase 3**: Auto-sync via Agent Mail (task completion → bead updates)
- **Phase 4**: Unified TUI (single view for strategic + tactical work)

The key is **hierarchical decomposition**: AllBeads tracks "what work spans repos", Aiki tracks "how work happens in this repo".
