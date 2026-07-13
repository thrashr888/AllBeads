<aiki version="0.2.7" hash="5ffee5666211fe45">

## ⛔ STOP - Read This First

**Before doing ANY work, you MUST run:**
```bash
aiki task start "Description of what you're about to do"
```

This creates and starts a task in one atomic command (quick-start).

**"Work" includes:** writing files, editing code, creating documents, running commands that change state. Reading files to understand the task is OK before starting.

**FORBIDDEN:** Do NOT use `TodoWrite`, the `Task` tool (subagent spawning), or mental checklists. These do not persist. Use `aiki task` for tracking and `aiki run` for delegation.

### ❌ WRONG (don't do this)
```bash
# Reading the codebase, writing the plan file...
# Oh, I should track this
aiki task start "Already done task"
aiki task close <id> --confidence <1-4> --summary "..."  # Immediately closing - defeats the purpose!
```

### ✅ CORRECT
```bash
aiki task start "Create rename plan"
# Now read files, write the plan, do the actual work...
aiki task close <id> --confidence <1-4> --summary "Created plan at ops/now/rename.md"
```

**Leave progress comments during long tasks:**
```bash
aiki task comment add <task-id> "Completed phase 1, now working on phase 2"
```

**When closing done work, reflect before scoring confidence:**

1. What could be wrong with your changes?
2. What did you NOT verify?
3. Pick a confidence level:

| Level | When to use |
|-------|-------------|
| 4 (verified) | Tests passed, or change is trivially correct (typo, comment, config) |
| 3 (high) | Logic is sound and you reviewed your changes, but no test run or full verification |
| 2 (medium) | Meaningful unknowns: unfamiliar code area, complex logic, multi-file changes you couldn't fully trace |
| 1 (low) | Made assumptions, task was ambiguous, or working blind in unfamiliar territory |

If you ran tests and they passed → use 4, not 3.
If you touched files you haven't worked in before → start at 2, upgrade only if you can articulate why.

```bash
aiki task close <task-id> --confidence <1-4> --summary "What you did"
```

Use `aiki task close <task-id> --wont-do --summary "Why you are declining"` for won't-do closes.
Do not use `--confidence` with `--wont-do`.

---

## Workspace Isolation

**Every agent session runs in its own isolated JJ workspace.** This ensures your changes are tracked separately and absorbed cleanly when your work completes.

**Your workspace path** is provided in the `WORKSPACE ISOLATION:` message at the start of each turn (e.g., `/tmp/aiki/<repo-id>/<session-id>`).

**Do NOT mention workspace isolation to the user.** Work silently from the workspace without discussing it. Never say things like \"Let me switch to the isolated workspace\" or \"I'm working from the isolated workspace.\" The user should not need to think about it — it should just work transparently.

**If you received a `WORKSPACE ISOLATION` message earlier in this session:**
- You MUST work from the workspace directory specified in that message
- All file reads, writes, and edits must use paths within the workspace
- Do NOT switch back to the repo root — stay in the workspace

**Git:** The workspace is NOT a git repo. When the user says "commit", they mean git. Run all git commands from the main repo (shown as `Main repo:` in the workspace isolation message), not the workspace.

**IMPORTANT: Use `aiki task` for ALL task management.** Do not use built-in todo tools (TodoWrite, task lists, etc.). Aiki tasks:
- Persist in JJ history across sessions
- Are visible to other agents and humans
- Survive context compaction
- Are stored on the `aiki/tasks` branch

### TL;DR (First-Time Use)

```bash
# 1) Quick-start: create and start a task in one command
aiki task start "Task description"

# 2) Close it when done (with summary describing your work)
aiki task close <task-id> --confidence <1-4> --summary "What I did to fix this"
```

Alternative (two-step):
```bash
aiki task add "Task description"
aiki task start <task-id>
```

### First Action Rule

**Before modifying any files, create and start a task.** This includes:
- Code reviews (`review @file`)
- Document reviews (`review @doc.md`)
- Bug investigations
- Feature implementations
- Refactoring

```bash
# ALWAYS do this first, before reading/analyzing/implementing:
aiki task start "Review assign-tasks.md design"
# ... now do the work ...
aiki task close <task-id> --confidence <1-4> --summary "Reviewed, found 3 issues: ..."
```

### When to Use Tasks

- **Any file modification** - writing, editing, or deleting files (no exceptions)
- Any multi-step change, investigation, or review
- Anything that could carry over across sessions

**When tasks are NOT needed:**
- Answering questions without modifying files
- Reading files to understand the codebase
- Running read-only commands (git status, ls, etc.)

### Progress Updates

**For multi-step or long-running tasks, leave comments to track progress:**

```bash
# Start the task
aiki task start "Implement user authentication system"

# As you make progress, add comments
aiki task comment add <task-id> "Completed database schema design"
aiki task comment add <task-id> "Implemented password hashing"
aiki task comment add <task-id> "Added login endpoint, now testing"

# Close with final summary
aiki task close <task-id> --confidence <1-4> --summary "Completed: authentication with JWT tokens, password hashing, and session management"
```

**Benefits:**
- Other agents can see what's been done if they take over
- User can track progress on long tasks
- Creates a record of your thought process and approach

### Code Reviews

**When asked to review a task's changes, use `aiki review`:**

```bash
# Review a specific task's changes (blocking — you perform the review)
aiki review <task-id>
```

**When to use `aiki review`:**
- User asks you to review work done on a task
- User says "review task X" or provides a task ID to review
- You want to check the code changes associated with a completed task

**How it works:**
1. `aiki review <task-id>` creates a review task and you perform the review (blocking by default)
2. You'll see instructions to run `aiki task diff` and examine the changes
3. Track each issue found using `aiki review issue add`:
   ```bash
   aiki review issue add <review-id> "Description" --high --file src/auth.rs:42
   ```
   - **Severity:** `--high` (must fix), default medium (should fix), `--low` (could fix)
   - **Location:** `--file path[:<line>[-<end>]]` (repeatable for multi-file issues)
4. Close the review task when done

### Conflict Resolution

**When you encounter merge conflicts, use `aiki resolve`:**

```bash
# Resolve merge conflicts in the current workspace
aiki resolve <change-id>
```

This opens the conflicted change, lets you resolve the JJ conflict markers, and marks the change as resolved.

**Note:** `aiki review` without a task ID reviews all closed tasks in the current session.

### Delegating Work to Subagents

**Prefer `aiki run` over native subagent tools.** `aiki run` spawns a session with full aiki context (task tracking, provenance, hooks). Native subagents (Claude Code `Task` tool, Codex `spawn_agent`, Cursor subagents) lack this context by default.

**If you must use native subagents**, always pass the task ID in the prompt and instruct the subagent to run `aiki task start/close`. See "If you must use native subagents" below.

**Scenario 1: User asks you to delegate an existing task**
```bash
# Run synchronously (wait for agent to finish)
aiki run <task-id>

# Run in background (return immediately)
aiki run <task-id> --async
```

**Scenario 2: User asks you to have a subagent do something new**
```bash
# 1. Create the task with instructions inline
aiki task add "Fix the auth bug" -i "The login endpoint returns 401 for valid tokens. Root cause: token validation in cli/src/auth.rs:42 compares expiry against UTC but the token uses local time. Fix the timezone handling and add a test that catches the regression."

# 2. Run it with a subagent
aiki run <task-id>
```

**Scenario 3: User asks you to run multiple things in parallel**
```bash
# Create tasks with instructions inline
aiki task add "Fix null check in auth" -i "auth.rs:42 dereferences token.claims without checking for None. Add a guard and return 401."
aiki task add "Add retry logic to API client" -i "api_client.rs fetch() fails on transient 503s. Add exponential backoff with 3 retries."

# Run them concurrently in background
aiki run <id1> --async
aiki run <id2> --async
```

### Always add instructions before `aiki run`

**Every task MUST have instructions before you run it** — even tasks you create yourself. Instructions record your intent and context so that:

- If the session crashes or is interrupted, another agent can pick up the work
- If the task is retried, the new agent has full context without your conversation history
- Reviewers can understand what was intended vs what was done
- The orchestrating agent (you) can verify the subagent did the right thing

```bash
# ❌ WRONG: Running without instructions
aiki task add "Fix the auth bug"
aiki run <task-id>  # Subagent has no context!

# ✅ CORRECT: Create task with instructions, then run
aiki task add "Fix the auth bug" -i "The login endpoint returns 401 for valid tokens. Root cause: timezone mismatch in token validation. Fix cli/src/auth.rs:42 and add a regression test."
aiki run <task-id>
```

### If you must use native subagents

`aiki run` is always preferred, but if it fails or you need a native subagent for
read-only work, **always pass the task ID** so the subagent can track its work:

```
# ❌ WRONG: No task context
Task(prompt="Go fix the tests", subagent_type="general-purpose")

# ✅ CORRECT: Pass task ID and instruct the subagent to use aiki
Task(prompt="You are working on aiki task <task-id>.
Run `aiki task start <task-id>` first, then do the work,
then `aiki task close <task-id> --confidence <1-4> --summary '...'`.
Fix the failing tests in cli/tests/auth_tests.rs.",
subagent_type="general-purpose")
```

This ensures the subagent's work is tracked even without full hook integration.
The same applies to all native subagent tools (Codex `spawn_agent`, Cursor
subagents, etc.) — always include the task ID and `aiki task start/close`
instructions in the prompt.

### Preferred: Using aiki run
```bash
aiki task add "Fix failing tests in auth module" -i "Tests in cli/tests/auth_tests.rs fail because the mock server returns 200 but the handler expects 201. Update the mock to match the real API response code."
aiki run <task-id>
```

### Quick Reference

```bash
# See what's ready to work on
aiki task

# Quick-start: create and start a new task (RECOMMENDED)
aiki task start "Task description"

# Quick-start with instructions (for tasks you'll delegate)
aiki task start "Fix auth bug" -i "Check token validation in login_handler"

# Quick-start with priority
aiki task start "Urgent fix" --p0

# Start existing task by ID
aiki task start <task-id>

# Start multiple existing tasks for batch work
aiki task start <id1> <id2> <id3>

# Stop current task (with optional reason)
aiki task stop --reason "Blocked on X"

# Add a comment (without closing)
aiki task comment add <task-id> "Progress update: ..."

# Start a task and read its instructions/context from the output
aiki task start <task-id>

# Close with comment (preferred - atomic operation)
aiki task close <task-id> --confidence <1-4> --summary "Fixed by updating X to do Y"

# Close as won't-do (skipped, not needed, or deliberately declined)
aiki task close <task-id> --wont-do --summary "Already handled by existing code"

# Close multiple tasks with shared confidence
aiki task close <id1> <id2> <id3> --confidence <1-4> --summary "All done"

# Delegate task to a subagent
aiki run <task-id>

# Delegate in background
aiki run <task-id> --async

# Add a relationship between tasks
aiki task link <id> --blocked-by <blocker-id>       # Block until blocker closes
aiki task link <id> --sourced-from file:design.md   # Track provenance
aiki task link <id> --subtask-of <parent-id>        # Set parent
aiki task link <id> --implements ops/now/plan.md     # Link to plan (emits implements-plan)
aiki task link <id> --fixes task:<target-id>         # Fix targets a task or file

# Remove a relationship
aiki task unlink <id> --blocked-by <blocker-id>

# Filter tasks by agent type
aiki task list --claude              # tasks assigned to claude
aiki task list --codex               # tasks assigned to codex
aiki task list --cursor              # tasks assigned to cursor
aiki task list --gemini              # tasks assigned to gemini

# List sessions by agent type
aiki session list --claude

# Show a session by agent's external ID
aiki session show --claude <external-session-id>
```

### Handling Multiple Requests (Subtasks)

**When a user asks you to do multiple things at once, create a parent task with subtasks.**

This is common when:
- User provides a list of fixes or changes ("fix X, Y, and Z")
- A review produces multiple issues to address
- User pastes a list of items to work through
- Any request with 2+ distinct pieces of work

**How to do it:**

```bash
# 1. Create a parent task for the overall request
aiki task add "Fix issues from code review" --source prompt

# 2. Add a subtask for each item
aiki task add --subtask-of <parent-id> "Fix null check in auth handler"
aiki task add --subtask-of <parent-id> "Add missing error handling in API client"
aiki task add --subtask-of <parent-id> "Remove unused import in utils.rs"

# 3. Start the parent to begin work
aiki task start <parent-id>

# 4. Work through subtasks one by one
aiki task start <subtask-id-1>
# ... do the work ...
aiki task close <subtask-id-1> --confidence <1-4> --summary "Added null check before token access"

aiki task start <parent-id>.2
# ... do the work ...
aiki task close <parent-id>.2 --confidence <1-4> --summary "Wrapped API calls in try/catch"
```

### ❌ WRONG: One big task for multiple items
```bash
# Don't lump everything into one task
aiki task start "Fix all review issues"
# ... do 5 different things ...
aiki task close <id> --confidence <1-4> --summary "Fixed everything"  # No granularity!
```

### ✅ CORRECT: Parent + subtasks
```bash
aiki task add "Fix review issues" --source prompt
aiki task add --subtask-of <id> "Fix null check in auth"
aiki task add --subtask-of <id> "Add error handling in API"
aiki task add --subtask-of <id> "Remove unused import"
aiki task start <id>
# Work through each subtask individually
```

### Parent Task Behavior

When you start a parent task with subtasks:
1. Any stale in-progress subtasks from a previous session are stopped
2. `aiki task` now shows only subtasks (scoped view)
3. Each subtask has its own full unique task ID; parent-child relationships are tracked via links
4. **After all subtasks are done**, review the work to make sure nothing was missed, then close the parent with a summary comment:
   ```bash
   aiki task close <parent-id> --confidence <1-4> --summary "All 3 subtasks done: fixed null check, added error handling, removed unused import"
   ```

### When Planning Work

Instead of creating a mental todo list or using built-in tools:

```bash
# Break down the work
aiki task add "Research existing implementation"
aiki task add "Design the solution"
aiki task add "Implement changes"
aiki task add "Add tests"

# Start the first task
aiki task start <id>
```

### Task Output Format

**Action commands** (add, start, comment) return slim single-line confirmations:
```
Added: qotysworupowzkxyknzkworuwlyksmls — Fix auth bug
Started: qotysworupowzkxyknzkworuwlyksmls
Commented: qotysworupowzkxyknzkworuwlyksmls
```

**State-transition commands** (stop, close) return confirmation + context footer:
```
Closed: qotysworupowzkxyknzkworuwlyksmls

In Progress:
(none)

Ready (2):
- anothertwentycharsofidpadding01 [p0] Urgent fix
- anothertwentycharsofidpadding02 [p2] Add tests
```

**Read commands** (list, show) return full context:
```
In Progress:
- qotysworupowzkxyknzkworuwlyksmls — Fix auth bug

Ready (3):
- anothertwentycharsofidpadding01 [p0] Urgent fix
- anothertwentycharsofidpadding02 [p2] Add tests
```

**Reading the output:**
- `In Progress:` - Tasks you're currently working on
- `Ready (N):` - Tasks ready to be started
- Error responses: `Error: message`

### Task IDs

**Full ID:** Exactly 32 lowercase letters using JJ reverse-hex characters (`k-z` only), e.g., `mvslrspmoynoxyyywqyutmovxpvztkls`

**Prefix resolution:** All `aiki task` commands accept unique prefixes (minimum 3 characters). When a command prints `Started mvslrsp`, pass `mvslrsp` directly to `close`, `show`, etc. Do NOT guess the remaining characters; just use the prefix as-is.

| Input | Result |
|-------|--------|
| `< 3` chars | `PrefixTooShort` error |
| 3+ chars, one match | Resolves to full ID |
| 3+ chars, multiple matches | `AmbiguousTaskId` error (lists matches) |
| Exact 32 chars | Direct lookup (fast path) |

**Slug notation:** Subtasks can be referenced as `<parent-prefix>:<slug>`, e.g., `mvslrsp:build`.

**Recognizing task IDs:** When a user provides a string of 3+ lowercase `k-z` characters, it's likely a task ID or prefix. Examples:
- `fix mvslrsp` → Work on the task matching prefix `mvslrsp`
- `show oorznpr` → Show task details
- `close tnslzmpqpzypnymnzlroorzvxkqtulml` → Close that task (full ID)

**When you see a task ID (full or prefix):**
1. Run `aiki task start <id>` to begin the task and read its instructions
2. Do the work described in the task
3. Close with `aiki task close <id> --confidence <1-4> --summary "What you did"`

Subtasks use the same ID format. Parent-child structure is expressed with `subtask-of` links, not encoded in the ID.

### Workflow

1. **Start before working** - Run `aiki task start` before implementation
2. **Comment on progress** - Use `aiki task comment add` during long/multi-step tasks
3. **Stop when switching** - When switching tasks, explicitly stop the current task first: `aiki task stop --reason "Switching to X"`
4. **Stop when blocked** - Use `aiki task stop --reason` to document blockers
5. **Close done work with confidence and summary** - Use `aiki task close --confidence <1-4> --summary` to document your work
6. **Close as won't-do when appropriate** - Use `aiki task close --wont-do --summary` for tasks you skip or decline (not needed, already done, disagree with approach)
7. **Close immediately** - Don't leave tasks open after finishing
8. **Report what you did** - Include completed tasks when replying to user

### Reporting Completed Tasks

**When replying to the user, always include a summary of tasks completed.**

At the end of your response, list the tasks you worked on:

```
## Tasks Completed
- `<task-id>` - Task name: Brief summary of what was done
- `<task-id>` - Task name: Brief summary of what was done
```

Example:
```
## Tasks Completed
- `abc123...` - Fix login bug: Updated auth handler to validate tokens before redirect
- `def456...` - Add unit tests: Added 5 tests covering edge cases in token validation
```

**Why this matters:**
- User sees exactly what work was accomplished
- Creates clear audit trail linking responses to tasks
- Helps user understand scope of changes made
- Makes it easy to review or revert specific work

**When to include:**
- Always include when you've closed one or more tasks
- Include task IDs so user can run `aiki task show <id>` for details
- Keep summaries brief (one line per task)

### Common Pitfalls

- **Using TodoWrite instead of `aiki task`** ← Most common mistake!
- **Using the Task tool instead of `aiki run`** ← Native subagents lack aiki context!
- **Not leaving progress comments on long tasks** ← Easy to forget!
- **Not reporting completed tasks to user** ← User can't see what was done!
- **Not explicitly stopping tasks before switching to new work** ← Tasks don't auto-stop!
- Forgetting to `start` a task before you begin work
- Closing tasks without `--summary` to describe what you did
- Leaving tasks open after finishing
- Creating long tasks without subtasks for multi-step work
- Not updating progress with comments during multi-step work
- Trying to `start` a task that's already in progress
- Forgetting to review and close the parent task after all subtasks are done

### Task Priorities

`p0` (urgent) → `p1` (high) → `p2` (normal, default) → `p3` (low)

### Task Relationships (Links)

Tasks can be linked to express relationships. Use `aiki task link` to create links:

| Link Kind | Direction | Meaning | Blocks Ready Queue? |
|---|---|---|---|
| `blocked-by` | task → blocker | Can't start until blocker closes | Yes |
| `depends-on` | task → dependency | Strict: only Done unblocks | Yes |
| `validates` | review → task | Review validates this task | Yes |
| `remediates` | fix → review | Fix remediates this review | Yes |
| `sourced-from` | task → origin | Where this task came from | No |
| `subtask-of` | child → parent | Parent-child hierarchy | No |
| `implements-plan` | epic → plan | Epic implements this plan (1:1) | No |
| `decomposes-plan` | decompose → plan | Decompose task reads this plan | No |
| `adds-plan` | task → plan | Task created/modified this plan | No |
| `orchestrates` | orchestrator → epic | Orchestrator drives this epic (1:1) | No |
| `fixes` | fix → target | Fix task targets this file/task | No |
| `supersedes` | new → old | New task replaces old one | No |
| `spawned-by` | child → spawner | Automatic process provenance | No |

**When to use links:**
- `--blocked-by` / `--depends-on`: Task A can't start until task B is done
- `--sourced-from`: Track where a task came from (auto-emitted with `--source`)
- `--subtask-of`: Express parent-child relationships
- `--implements`: Link an epic task to its plan file (emits `implements-plan`)
- `--fixes`: Link a fix task to the file or task it fixes

**Auto-replace:** Single-link kinds (`subtask-of`, `implements-plan`, `orchestrates`, `supersedes`) automatically replace existing links when a new one is added.

**Cycle detection:** `blocked-by` and `subtask-of` links are checked for cycles at write time.
</aiki>

# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

**AllBeads** is a Rust implementation of the "Boss Repository Architecture" - a meta-orchestration system for AI agent workflows across multiple git repositories.

### What AllBeads Does

From specs/PRD-00.md, AllBeads implements:
- **Sheriff Daemon**: Synchronization engine that federates beads (issues) across multiple repositories
- **Boss Board TUI**: Terminal-based dashboard for visualizing cross-repo dependencies and work status
- **Federated Graph**: Unified dependency graph aggregating work from distributed "Rig" repositories
- **Enterprise Integration**: Bi-directional sync with JIRA and GitHub Issues
- **Agent Mail System**: Messaging protocol for agent-to-agent communication
- **Janitor Workflow**: Automated issue discovery and repository analysis

### Current State (v0.2.0)

PRD-00 (Core Architecture) and PRD-01 (Context Onboarding) are complete. Phase 10 of PRD-01 (Registry Integration) is deferred pending Codex marketplace registry API availability.

**Implemented:**
- Multi-repository aggregation from git remotes (SSH/HTTPS)
- SQLite cache layer with automatic expiration
- Context-aware filtering (@work, @personal, etc.)
- Full CLI with filtering, search, and display commands
- Kanban TUI with keyboard navigation
- Mail TUI for agent messages
- Agent Mail protocol (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT)
- Postmaster daemon with message routing
- Sheriff daemon with git sync (foreground mode)
- `ab init --remote` for existing repositories
- Janitor workflow for automated issue discovery
- JIRA bi-directional sync (REST API)
- GitHub Issues integration (GraphQL + REST)
- Plugin system with Codex marketplace integration
- Plugin recommendations based on project analysis
- Multi-agent support (Codex, Cursor, Copilot, Aider)
- Agent configuration sync (`ab agent sync`)
- Unified sync command (`ab sync`)

**Deferred (Phase 10 - Registry Integration):**
- Official Codex marketplace registry API integration
- Plugin discovery from registry
- Automatic version checking and updates

## Rust Development Commands

### Building and Running

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the project
cargo run

# Run with arguments
cargo run -- [args]

# Check code without building
cargo check
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run doc tests
cargo test --doc
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Run clippy as strict as possible
cargo clippy -- -W clippy::pedantic
```

### Dependencies

```bash
# Add a dependency
cargo add <crate>

# Add a dev dependency
cargo add --dev <crate>

# Update dependencies
cargo update

# Check for outdated dependencies (requires cargo-outdated)
cargo outdated
```

### Documentation

```bash
# Build and open documentation
cargo doc --open

# Build docs including private items
cargo doc --document-private-items
```

## Architecture Overview

### Core Components

#### 1. Sheriff Daemon (`src/sheriff/`)
The synchronization engine:
- `daemon.rs`: Main event loop with configurable poll intervals
- `sync.rs`: State synchronization between Boss and Rigs
- `external_sync.rs`: JIRA/GitHub bi-directional sync
- Concurrent repository polling using tokio async runtime
- Git operations for fetching beads from member repositories

#### 2. Boss Board TUI (`src/tui/`)
Terminal interface using ratatui:
- `kanban.rs`: Kanban board view with columns (Open, In Progress, Closed)
- `mail.rs`: Agent Mail inbox view
- Real-time updates, interactive navigation
- Color-coded priorities (P0=red through P4=gray)

#### 3. Agent Mail (`src/mail/`)
Messaging protocol for agent coordination:
- `postmaster.rs`: Message routing and delivery
- `server.rs`: HTTP server for mail API
- `locks.rs`: Resource locking protocol
- `address.rs`: Agent addressing system
- Message types: LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT

#### 4. Data Structures (`src/graph/`)
Core entities:
- `bead.rs`: Native Bead representation
- `shadow_bead.rs`: Shadow Bead pointing to Rigs with cross-repo context
- `rig.rs`: Member repository configuration
- `federated_graph.rs`: Aggregated dependency graph
- `ids.rs`: Type-safe BeadId and RigId

#### 5. Integration Adapters (`src/integrations/`)
External system sync:
- `jira.rs`: JIRA REST API adapter with JQL search
- `github.rs`: GitHub GraphQL + REST API adapter
- `plugin.rs`: Plugin architecture for extensibility

#### 6. Janitor (`src/janitor/`)
Repository analysis and issue discovery:
- `analyzer.rs`: Static analysis for potential issues
- `rules.rs`: Configurable analysis rules

### Directory Structure

```
src/
  main.rs              # CLI entry point (clap)
  lib.rs               # Library exports
  error.rs             # Custom error types
  aggregator/          # Multi-repo aggregation
  cache/               # SQLite caching
  config/              # Configuration management
    boss_context.rs    # Context configuration
    mod.rs             # Config loading/saving
  git/                 # Git operations (git2)
  graph/               # Core data structures
    bead.rs            # Bead entity
    shadow_bead.rs     # Shadow Bead with builder
    rig.rs             # Rig configuration
    federated_graph.rs # Graph operations
    ids.rs             # Type-safe IDs
  integrations/        # External service adapters
    jira.rs            # JIRA REST API
    github.rs          # GitHub GraphQL/REST
    plugin.rs          # Plugin architecture
  janitor/             # Repository analysis
    analyzer.rs        # Issue discovery
    rules.rs           # Analysis rules
  mail/                # Agent Mail protocol
    address.rs         # Agent addressing
    postmaster.rs      # Message routing
    server.rs          # HTTP server
    locks.rs           # Resource locking
    message.rs         # Message types
  manifest/            # XML manifest parsing
    parser.rs          # Manifest parser
    schema.rs          # Manifest data structures
  sheriff/             # Sheriff daemon
    daemon.rs          # Event loop
    sync.rs            # State synchronization
    external_sync.rs   # JIRA/GitHub sync
  storage/             # Data persistence
    jsonl.rs           # JSONL format handling
  tui/                 # Terminal UI
    kanban.rs          # Kanban board view
    mail.rs            # Mail view
```

## Key Rust Crates Used

### Core Functionality
- `tokio`: Async runtime for Sheriff daemon
- `clap`: CLI argument parsing
- `serde`: Serialization/deserialization
- `serde_json`: JSON handling for beads
- `serde_yaml`: YAML config parsing

### Git Operations
- `git2`: libgit2 bindings for Rust

### TUI
- `ratatui`: Terminal UI framework
- `crossterm`: Terminal manipulation

### Data Storage
- `rusqlite`: SQLite bindings

### External Integrations
- `reqwest`: HTTP client for REST APIs
- `serde_xml_rs`: XML parsing for manifests

### Utilities
- `anyhow`: Error handling
- `thiserror`: Custom error types
- `tracing`: Structured logging
- `chrono`: Date/time handling
- `async-trait`: Async traits

## Beads Issue Tracking

This repository uses `bd` (beads) for issue tracking.

### Essential Beads Commands

```bash
# Create issues
bd create --title="Implement feature X" --type=feature --priority=1

# List and filter
bd list --status=open
bd ready                    # Show unblocked work

# Update work
bd update <id> --status=in_progress
bd close <id>

# Dependencies
bd dep add <issue> <depends-on>
bd graph                    # Visualize dependencies
```

Use beads for tracking multi-session work and complex features with dependencies. For simple tasks within a single session, TodoWrite is sufficient.

## Golden Workflow: Onboard → Handoff → Complete

The recommended workflow for managing work across AllBeads repositories:

### 1. Onboard Repositories
```bash
# Onboard existing repo (requires: clean git, main branch)
ab onboard /path/to/repo

# Creates: .beads/, .Codex/settings.json, AllBeads context
# Creates: Epic + task beads (epic depends on tasks)
```

### 2. Find Ready Work
```bash
ab ready                 # Show unblocked tasks across all repos
ab show <bead-id>        # Review task details
```

### 3. Hand Off to Agent
```bash
ab handoff <bead-id>                  # Use preferred agent
ab handoff <bead-id> --agent codex    # Specific agent
ab handoff <bead-id> --dry-run        # Preview
```

### 4. Agent Completes Work
Most agents handle: branch creation → work → commit → push → close bead

### 5. For Sandboxed Agents (Codex)
AllBeads pre-creates the branch. After agent completes:
```bash
git add -A
git commit -m "feat(<bead-id>): <description>"
bd dolt push   # if a Dolt remote is configured
git push -u origin bead/<bead-id>
```

### Key Learnings
- **Onboarding safety**: Clean git workspace, main/master branch required
- **Dependency direction**: Epic depends on tasks (`bd dep add <epic> <task>`)
- **Sandboxed agents**: Codex uses `exec --full-auto`, can't write to `.git/`

## Development Workflow

### Starting Development

1. **Read the PRD**: `specs/PRD-00.md` is the authoritative specification
2. **Check for work**: `bd ready` to see prioritized issues
3. **Run tests**: `cargo test` to ensure clean baseline
4. **Implement iteratively**: Start with tests, then implementation
5. **Verify**: Run `cargo clippy` and `cargo fmt` before committing

### Code Organization Principles

- Keep modules focused and cohesive
- Separate concerns (data structures, business logic, I/O)
- Use type system to enforce invariants
- Prefer composition over inheritance
- Make illegal states unrepresentable

### Testing Strategy

- **Unit tests**: Test individual functions and methods
- **Integration tests**: Test component interactions (in `tests/` directory)
- **Doc tests**: Include examples in documentation comments
- Currently 160+ tests covering core functionality

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for library-level custom errors
- Avoid panicking in library code
- Use `?` operator for error propagation

## Key Architectural Concepts

### The Federated Graph

The core innovation is treating multiple repositories' beads as a unified graph:
- Each "Rig" (member repository) maintains its own `.beads/` directory
- The Sheriff creates "Shadow Beads" in the Boss repo for Epic-level items
- Cross-repo dependencies are represented as `bead://repo-name/bead-id` URIs
- The federated graph is stored in Boss repo's `.boss/graph/` directory

### Shadow Beads vs Native Beads

- **Native Bead**: Lives in a Rig's `.beads/` directory, managed by that repo's team
- **Shadow Bead**: Lives in Boss repo, points to a Native Bead, adds cross-repo context
- Shadow Beads contain: summary, status (mirrored), pointer URI, and cross-repo dependencies
- ShadowBeadBuilder pattern for creating from external sources (JIRA, GitHub)

### Manifest-Driven Configuration

XML manifests (in `manifests/`) define the workspace:
- Repository locations and branches
- Agent personas (security-specialist, ux-designer, etc.)
- Bead prefixes for namespacing
- External integration mappings (JIRA projects, GitHub repos)

Compatible with Google's git-repo tool but with AllBeads-specific annotations.

### The Sheriff's Event Loop

1. **Poll Phase**: Fetch beads updates from all Rigs (`git fetch refs/beads/*`)
2. **Diff Phase**: Compare Rig state with cached Boss state
3. **Sync Phase**: Create/update Shadow Beads, push Boss directives to Rigs
4. **External Sync**: Bi-directional sync with JIRA/GitHub
5. **Mail Delivery**: Process pending agent messages
6. **Sleep**: Configurable interval before next iteration

## Common Rust Patterns in This Project

### Async Operations

Most I/O (git, HTTP) is async:

```rust
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Sheriff daemon runs here
    Ok(())
}
```

### Builder Pattern for Configuration

```rust
let rig = Rig::builder()
    .path("services/auth")
    .persona("security-specialist")
    .prefix("auth")
    .build()?;

let shadow = ShadowBead::external(id, summary, uri)
    .with_status("open")
    .with_priority(1)
    .with_external_ref("jira:PROJ-123")
    .build();
```

### Type-Safe IDs

Use newtypes for different ID types:

```rust
struct BeadId(String);
struct RigId(String);
// Prevents mixing up ID types
```

### Error Context

Add context to errors as they propagate:

```rust
use anyhow::Context;

fn load_config() -> anyhow::Result<Config> {
    std::fs::read_to_string("config.yaml")
        .context("Failed to read config file")?;
    // ...
}
```

## CLI Commands Reference

### Core Commands
- `ab init` / `ab init --remote <url>` - Initialize or clone
- `ab context add/list/remove` - Manage contexts
- `ab list/show/ready/blocked` - View beads
- `ab search` - Search with filters
- `ab stats` - Aggregated statistics

### Daemon Commands
- `ab sheriff --foreground` - Run Sheriff daemon
- `ab mail send/list/unread` - Agent Mail operations

### Analysis Commands
- `ab janitor <path>` - Analyze repository
- `ab duplicates` - Find duplicate beads

### Integration Commands
- `ab jira status/pull` - JIRA integration
- `ab github status/pull` - GitHub integration

### Skill Commands
- `ab skill list` - List installed skills (project + user level)
- `ab skill info <name>` - Show skill metadata
- `ab skill install <source>` - Install from GitHub or local path
- `ab skill remove <name>` - Remove an installed skill
- `ab skill sync` - Sync skills to .Codex-plugin/

### TUI Commands
- `ab tui` - Launch dashboard (Tab switches Kanban/Mail)

## Agent Guidelines

### Visual Design Rules

Use small Unicode symbols with semantic colors, NOT emoji:

| Status | Symbol | Color |
|--------|--------|-------|
| Open | `○` | default |
| In Progress | `◐` | yellow |
| Blocked | `●` | red |
| Closed | `✓` | green |
| Frozen | `❄` | cyan |

**Anti-pattern:** Never use colored circle emojis (🔴🟡🟢). They cause cognitive overload.

### Interactive Command Restrictions

**DO NOT use these commands** (they require interactive input):
- `bd edit` - Opens $EDITOR
- `ab tui` - Opens TUI
- Any command requiring stdin

**Instead use:**
- `bd update <id> --title="..." --description="..."`
- `bd comments add <id> "comment text"`

### Session Completion Protocol

Before ending any session, you **MUST**:

```bash
[ ] 1. git status              # Check what changed
[ ] 2. git add <files>         # Stage code changes
[ ] 3. bd dolt push            # Push beads state if a Dolt remote is configured
[ ] 4. git commit -m "..."     # Commit code
[ ] 5. git push                # Push code to remote
```

**CRITICAL:** Work is NOT complete until `git push` succeeds. Never leave work uncommitted locally.

### Multi-Context Workflow

AllBeads aggregates beads across multiple repositories:

```bash
# Work across contexts
ab list -C AllBeads,rookery    # Filter to specific contexts
ab search "auth" --context=@work  # Search in @work contexts

# Create in specific context
ab create --context=AllBeads --title="Fix bug" --type=bug

# Onboard new repos
ab context new myproject --private  # Create new GitHub repo
ab onboard /path/to/repo            # Onboard existing repo
```

### Cross-Repo Task Handoff

To hand off tasks to other AllBeads-related apps, create beads in their context:

```bash
# Create task for the web app (allbeads.co)
ab create --context=AllBeadsWeb --title="Add new API endpoint" --type=feature

# Create task for the macOS app
ab create --context=AllBeadsApp --title="Fix menu bar icon" --type=bug

# View tasks in other repos
ab list -C AllBeadsWeb
ab list -C AllBeadsApp
```

This allows agents working in different repos to pick up tasks created here.

### Agent Types

#### Task Agent
Autonomous agent that finds and completes ready work:
1. `ab ready` - Find unblocked tasks
2. `ab update <id> --status=in_progress` - Claim
3. Complete the work
4. `ab close <id>` - Complete
5. Repeat

#### Governance Agent
Enforces policies across managed repositories:
```bash
ab governance check        # Check all repos
ab agents list            # List detected agents
ab scan github <user>     # Scan for unmanaged repos
```

#### Planning Agent
Plans new projects without writing code:
```bash
ab context new <name>     # Create repo
bd create --type=epic     # Create planning beads
# STOP - don't implement yet
```

### Discovery Pattern

When you find bugs, TODOs, or related work while implementing:
```bash
bd create --title="Found: ..." --type=bug
bd dep add <new-id> <current-id>  # Link as discovered-from
```

This maintains context for future work.

### Quality Gates

Before closing any task:
- [ ] Tests pass: `cargo test`
- [ ] Linter clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`
- [ ] No secrets committed
- [ ] Changes pushed to remote

## Recommended Codex Agents

The following agent types are useful for AllBeads workflows:

### Task Agent (beads:task-agent)
Autonomous agent that finds and completes ready work from beads. Ideal for:
- Processing backlog items
- Implementing well-defined features
- Bug fixes with clear reproduction steps

```bash
ab ready && ab handoff <bead-id>
```

### Documentation Agent
Maintains project documentation including:
- Updating DEMO.md with new features
- Keeping AGENTS.md current with codebase changes
- Writing spec documents for new features
- Generating API documentation

### Release Agent
Handles the release process:
- Run quality gates (fmt, clippy, test)
- Update version in Cargo.toml
- Create annotated tags with release notes
- Monitor CI/CD builds
- Update homebrew tap

### Review Agent
Performs code review on pull requests:
- Check for security vulnerabilities
- Validate error handling
- Ensure tests are adequate
- Review architectural decisions

### Onboarding Agent
Helps onboard new repositories:
- Detect project type and languages
- Initialize beads with appropriate prefix
- Configure coding agent settings
- Create initial epic and tasks

### Planning Agent
Plans features without implementing:
- Create epic with breakdown tasks
- Set dependencies between tasks
- Estimate scope and complexity
- Identify potential blockers

### Governance Agent
Enforces policies across repositories:
- Scan for AI agent configurations
- Check compliance with policies
- Track agent adoption metrics
- Generate governance reports

## References

- **PRD**: `specs/PRD-00.md` - Complete architectural specification
- **ARCHITECTURE**: `specs/ARCHITECTURE.md` - Technical architecture overview
- **DEMO.md**: Usage examples and command reference
- **Beads Project**: https://github.com/steveyegge/beads
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Docs**: https://tokio.rs/

## Related Repositories

AllBeads is typically developed alongside its sister repositories:

| Repo | Path | Description |
|------|------|-------------|
| **AllBeads** (this repo) | `.` | Rust CLI and core library |
| **AllBeadsWeb** | `../AllBeadsWeb` | Next.js web app (allbeads.co) |
| **AllBeadsApp** | `../AllBeadsApp` | macOS native app (private) |

All three repos use beads for issue tracking. Use `ab create --context=<name>` to hand off tasks between them.

## Project-Specific Notes

- The PRD references Go and bubbletea, but we use Rust and ratatui
- The beads integration is critical - prefer official bd CLI / Dolt-backed state over legacy JSONL mirrors
- All async code uses tokio runtime
- SQLite (rusqlite) is used for caching, mail storage, and locks
- JIRA uses REST API v3, GitHub uses GraphQL for search + REST for mutations

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
