# Core Concepts

This document explains the fundamental concepts behind AllBeads: the Boss Repository pattern, how beads are federated across repositories, and how agents coordinate through the mail system.

## The Boss Repository Pattern

### The Problem

Organizations typically choose between two approaches:

**Monorepo**: All code in one repository
- Pros: Easy coordination, single source of truth
- Cons: Doesn't scale, long CI times, permission complexity

**Polyrepo**: Independent repositories per service
- Pros: Team autonomy, focused scope, clear ownership
- Cons: Coordination becomes difficult, dependencies are invisible

### The AllBeads Solution

AllBeads introduces a third approach: the **Boss Repository**.

A Boss repo is a lightweight control plane that:
- Federates state across polyrepos without merging code
- Provides a unified view of work spanning multiple repositories
- Maintains dependency graphs that cross repository boundaries
- Enables AI agents to coordinate strategic work

```
                 ┌─────────────────┐
                 │   Boss Repo     │
                 │  (AllBeads)     │
                 │                 │
                 │  Shadow Beads   │
                 │  Federated Graph│
                 └────────┬────────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
         ▼                ▼                ▼
   ┌───────────┐   ┌───────────┐   ┌───────────┐
   │  Rig A    │   │  Rig B    │   │  Rig C    │
   │ auth-svc  │   │ web-app   │   │ api-gw    │
   │           │   │           │   │           │
   │  Beads    │   │  Beads    │   │  Beads    │
   └───────────┘   └───────────┘   └───────────┘
```

## Beads

A **Bead** is a git-native issue. Beads are stored in a `.beads/` directory at the repository root as JSONL files.

### Bead Structure

Each bead contains:

| Field | Description |
|-------|-------------|
| `id` | Unique identifier (e.g., `ab-123`) |
| `title` | Short summary of the work |
| `description` | Detailed explanation |
| `status` | Current state (open, in_progress, blocked, closed) |
| `priority` | P0 (critical) through P4 (backlog) |
| `type` | epic, feature, task, bug, chore |
| `created_by` | Author |
| `assignee` | Current owner |
| `labels` | Categorical tags |
| `dependencies` | IDs of beads this depends on |

### Bead Lifecycle

```
┌────────┐    claim     ┌─────────────┐    complete    ┌────────┐
│  open  │ ──────────►  │ in_progress │ ─────────────► │ closed │
└────────┘              └─────────────┘                └────────┘
     │                        │
     │                        │ blocker found
     │                        ▼
     │                  ┌─────────┐
     └────────────────► │ blocked │
       or direct block  └─────────┘
```

### Native Beads vs Shadow Beads

- **Native Bead**: Lives in a repository's `.beads/` directory. Managed by that team.
- **Shadow Bead**: Lives in the Boss repo. Points to work in other repositories. Used for Epic-level coordination.

## Shadow Beads

Shadow Beads enable the Boss repository to track Epic-level work that spans multiple repositories while letting each repo maintain autonomy.

### When Shadow Beads Are Created

- When an Epic needs to reference work across multiple Rigs
- When syncing with external systems (JIRA, GitHub Issues)
- When the Sheriff daemon detects cross-repo dependencies

### Shadow Bead Structure

A Shadow Bead contains:

| Field | Description |
|-------|-------------|
| `id` | Unique identifier in Boss repo |
| `summary` | Descriptive title |
| `status` | Mirrored from the source |
| `pointer_uri` | Reference to original (e.g., `bead://auth-svc/ab-456`) |
| `external_ref` | Optional link to JIRA/GitHub (e.g., `jira:PROJ-123`) |
| `cross_repo_deps` | Dependencies that span repositories |

### URI Format

Shadow Beads use URIs to reference native beads:

```
bead://rig-name/bead-id
bead://auth-service/ab-456
bead://web-app/wa-789
```

## Rigs

A **Rig** is a member repository managed by the Boss. The term comes from the concept of "rigging" - the infrastructure that supports work.

### Rig Configuration

Each Rig is defined with:

| Field | Description |
|-------|-------------|
| `name` | Identifier for the rig |
| `path` | Repository path (e.g., `services/auth`) |
| `url` | Git remote URL |
| `persona` | Agent type (security-specialist, frontend-expert) |
| `prefix` | Bead ID prefix for namespacing |

### Rig Personas

Personas help AI agents understand the domain context of a repository:

- `security-specialist` - Focus on auth, crypto, access control
- `frontend-expert` - UI/UX, components, accessibility
- `backend-architect` - APIs, databases, performance
- `devops-engineer` - CI/CD, infrastructure, monitoring
- `data-scientist` - ML, analytics, pipelines

## Federated Graph

The **Federated Graph** is the core data structure that aggregates beads from all Rigs into a unified dependency graph.

### Graph Properties

- **Directed**: Dependencies flow from dependent to dependency
- **Potentially Cyclic**: Cycles are detected and reported
- **Cross-Repo**: Can span multiple repositories

### Graph Operations

```rust
// Example: Finding blocked paths
graph.blocked_paths()

// Example: Cross-repo dependencies
graph.cross_repo_dependencies()

// Example: Cycle detection
graph.find_cycles()
```

### Visualization

The TUI Graph view shows dependency chains with indicators:

| Symbol | Meaning |
|--------|---------|
| `⟳` | Cycle detected (red) |
| `⬡` | Cross-context dependency (magenta) |
| `⊘` | Blocked (yellow) |
| `○` | Normal/healthy (green) |

## Agent Mail

The Agent Mail system enables messaging between AI agents and humans for coordination.

### Message Types

| Type | Purpose |
|------|---------|
| `NOTIFY` | One-way notification (task complete, status update) |
| `REQUEST` | Requires response (help needed, approval) |
| `BROADCAST` | Sent to all agents |
| `LOCK` | Request exclusive resource access |
| `UNLOCK` | Release resource lock |
| `HEARTBEAT` | Health check signal |

### Addressing

Messages are addressed using agent identifiers:

- `human` - The human operator
- `agent-1`, `agent-2` - Specific agents
- `broadcast` - All agents
- `claude@myproject` - Agent at specific context

### Message Flow

```
┌─────────┐         ┌────────────┐         ┌─────────┐
│ Agent A │ ──────► │ Postmaster │ ──────► │ Agent B │
└─────────┘  send   │   Daemon   │  route  └─────────┘
                    └────────────┘
                          │
                          │ store
                          ▼
                    ┌─────────────┐
                    │  SQLite DB  │
                    └─────────────┘
```

### Remote Mail

When authenticated with AllBeads Web, messages are stored centrally:

```bash
# Send message
ab mail send --to agent-1 --subject "Task done" --body "Completed auth refactor"

# Check inbox
ab mail list
ab mail unread

# Mark as read
ab mail read <message-id>
```

## The Sheriff Daemon

The Sheriff is a background synchronization service that keeps everything in sync.

### Event Loop

The Sheriff runs in a continuous loop:

1. **Poll Phase**: Fetch beads updates from all Rigs
2. **Diff Phase**: Compare Rig state with cached Boss state
3. **Sync Phase**: Create/update Shadow Beads, push directives
4. **External Sync**: Bi-directional sync with JIRA/GitHub
5. **Mail Delivery**: Process pending agent messages
6. **Sleep**: Wait for next poll interval

### Running the Sheriff

```bash
# Foreground mode (see all events)
ab sheriff --foreground

# With custom poll interval (seconds)
ab sheriff -f -p 10
```

### Event Output

```
[2026-01-10 12:00:00] Starting Sheriff daemon...
[2026-01-10 12:00:00] Poll cycle started
[2026-01-10 12:00:01] Synced rig 'auth-service': 3 shadows updated
[2026-01-10 12:00:02] External sync: 5 JIRA issues pulled
[2026-01-10 12:00:02] Poll cycle complete (2.1s)
```

## Health Checks

AllBeads tracks repository health across several dimensions:

| Check | Description |
|-------|-------------|
| Beads | Is `.beads/` initialized? |
| Skills | Are Claude plugins configured? |
| Integration | Is JIRA/GitHub connected? |
| CI/CD | Are workflows present? |
| Hooks | Are git hooks installed? |

View health status:

```bash
ab stats
```

Output includes:
```
Health Checks
  Beads initialized:    36/36
  Skills configured:    32/36
  CI/CD detected:       8/36
  Hooks installed:      36/36
  Overall Health:       62%
```

## Dependency Management

### Creating Dependencies

When a bead depends on another:

```bash
bd dep add ab-456 ab-123    # ab-456 depends on ab-123
```

### Blocking Semantics

- A bead with open dependencies is **blocked**
- When all dependencies close, the bead becomes **unblocked**
- Use `ab blocked` to see all blocked beads
- Use `ab ready` to see unblocked work

### Cross-Repository Dependencies

Dependencies can span repositories:

```
bead://auth-service/ab-123 → bead://web-app/wa-456
```

The Federated Graph tracks these and reports them in the Graph view.

## Summary

| Concept | Purpose |
|---------|---------|
| Boss Repo | Control plane federating polyrepos |
| Bead | Git-native issue in `.beads/` |
| Shadow Bead | Boss-level reference to work elsewhere |
| Rig | Member repository managed by Boss |
| Federated Graph | Unified cross-repo dependency graph |
| Agent Mail | Messaging protocol for coordination |
| Sheriff | Background sync daemon |
| Health Checks | Repository readiness tracking |

Understanding these concepts helps you leverage AllBeads for strategic coordination across distributed codebases.
