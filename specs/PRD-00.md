# AllBeads: The Distributed Protocol for Agentic Orchestration and Communication

## 1. Introduction: The Crisis of Context in Autonomous Engineering

The paradigm of software engineering is undergoing a seismic shift, moving from a human-centric model of solitary creation to an agent-centric model of orchestration. In this emerging era, the primary bottleneck to productivity is no longer the generation of syntax or the algorithmic reasoning capabilities of Large Language Models (LLMs) like Claude 3.5 Sonnet or GPT-4o. Rather, the binding constraint is **context management**—specifically, the persistence of state, intent, and dependency across disjointed compute sessions and distributed repositories.

For decades, the Integrated Development Environment (IDE) served as the cockpit for the human developer, who maintained the high-level architecture, the dependency graph, and the execution plan within their own biological working memory. As we delegate increasing autonomy to AI agents, we confront the "Catastrophic Amnesia" problem.1 An AI agent, upon restarting a session, is effectively a new entity, tabula rasa, devoid of the historical nuance that guided previous decisions. Without a persistent external memory that is structured, queryable, and tightly coupled to the codebase, agents are condemned to relive the same discovery processes repeatedly, akin to the protagonist in the film *50 First Dates*.1

This document serves as a comprehensive Product Requirements Document (PRD) and Request for Comments (RFC) for **AllBeads**: an open-source CLI and TUI product designed to be the "Inter-Process Communication" (IPC) layer for the AI workforce. It analyzes the current landscape of agent orchestration—specifically the unstructured "Prose" paradigm exemplified by prose.md and the parallelized execution environment of conductor.build. It contrasts these with the structured, graph-theoretic approach of the "Beads" ecosystem pioneered by Steve Yegge.

AllBeads extends the Beads foundation with two critical innovations:

1. **Distributed Peer Model**: Unlike centralized orchestrators, AllBeads aggregates multiple independent "Boss" repositories—representing distinct contexts like Personal GitHub and Enterprise GitHub—into a unified "God Mode" dashboard without merging their data.

2. **Agent Mail Integration**: AllBeads incorporates messaging and file locking protocols directly into the workflow, allowing agents to negotiate conflicts and reserve resources, preventing the "Too Many Cooks" failure mode common in multi-agent systems.

The objective is to create an open-source (MIT licensed) tool that enables a human Architect to oversee a swarm of agents that not only track work (via Beads) but also communicate and coordinate (via Mail), transforming a collection of scripts into a self-organizing "Agent Village" that spans the entirety of a developer's digital life—from private hobby projects to massive enterprise codebases.

## 2. The Prose Paradigm and Conductor: Analysis of Current Orchestration Models

Before defining the target architecture of AllBeads, it is essential to dissect the prevailing methodologies for agent coordination. The industry has largely coalesced around two approaches: the unstructured narrative (Prose) and the parallelized workspace (Conductor).

### 2.1 The "Prose.md" Methodology: Narrative as Memory

The most immediate solution to agent amnesia has been the adoption of a persistent text file—often named prose.md, plan.md, or context.md—which serves as a shared scratchpad between the human operator and the AI agent.2

#### 2.1.1 Mechanics of Narrative Persistence

In the Prose paradigm, the agent is instructed to read this file at the start of every turn and append its progress at the end. The file typically contains a mixture of high-level goals ("Refactor the authentication middleware"), current status ("Debugging the JWT token expiry issue"), and fleeting thoughts or "scratchpad" notes. This mimics the human practice of maintaining a developer journal or a TODO file.

The primary advantage of this approach is its flexibility. Natural language allows for the capture of nuance—ambiguity, hesitation, and alternative strategies—that structured databases often strip away. For an LLM, which is natively a probabilistic engine for processing text, prose.md is an intuitive interface. It requires no schema validation, no API calls, and no external dependencies.

#### 2.1.2 The Scaling limitations of Unstructured Text

However, the "Prose.md" model inevitably fractures under the weight of complexity, particularly in multi-turn, long-horizon tasks.

* **Linearity vs. Graph Reality:** Software development is inherently non-linear. Tasks branch, block, and merge. A bug discovered in the frontend (Task A) might block the deployment of the backend (Task B) while being related to a legacy issue in the database (Task C). A flat text file struggles to represent this Directed Acyclic Graph (DAG) of dependencies. As the file grows, it becomes a chronological log rather than a structural map, forcing the agent to parse pages of obsolete text to find the current state.3  
* **Token Economics and Context Window Inflation:** Every token in prose.md consumes space in the agent's context window. As the history of the project expands, the "memory" crowds out the "working set" (the actual code being modified). This forces a destructive trade-off: either truncate the history and lose context, or retain the history and lose the ability to load the relevant source files.  
* **Concurrency and Race Conditions:** In a multi-agent environment, the prose file becomes a point of contention. If Agent A (working on the frontend) and Agent B (working on the API) both attempt to update prose.md simultaneously, standard file system locks or git merge conflicts ensue. LLMs are notoriously poor at resolving merge conflicts in natural language files, often resulting in duplicated text or hallucinated consolidations.

### 2.2 Conductor.build: Parallelizing the Workflow

conductor.build, developed by Melty Labs, represents the technological apex of the Prose paradigm.4 It acknowledges the limitations of a single, serial agent and attempts to solve the throughput problem via parallelization and rigorous environment isolation.

#### 2.2.1 The Conductor Architecture: Git Worktrees as Containers

Conductor operates on the premise that an AI engineering team should function like a human team: distributed, parallel, and review-centric. Its core innovation is the utilization of git worktree to manage agent workspaces.5

In a standard git workflow, switching branches updates the files in the current directory, effectively locking the repository to a single context. Conductor circumvents this by creating a separate directory (a worktree) for each active agent. This allows Agent Alpha to modify src/auth.ts on branch-feature-a while Agent Beta modifies src/user.ts on branch-feature-b simultaneously, on the same machine, without file system collisions.

This architecture provides a robust "sandbox" for agents. If an agent hallucinates destructive code, it is contained within its ephemeral worktree. The user supervises these agents via a "Mission Control" dashboard, reviewing diffs (changesets) rather than raw file generation.4 This aligns perfectly with the standard engineering practice of Pull Request (PR) reviews, treating the AI not as a magic box but as a junior engineer submitting code for approval.

#### 2.2.2 The "Repo-Bound" Constraint

Despite its sophistication in handling parallel execution, Conductor inherits the limitations of the Prose paradigm regarding state persistence.

* **Ephemeral Planning:** Conductor's agents rely on the immediate chat context or local plan files. When a workspace is closed, the "thought process" of that agent is effectively lost unless manually serialized. There is no persistent database of "what has been tried" that survives the lifecycle of the worktree.6  
* **The Single-Repo Silo:** Conductor is designed primarily for single-repository workflows. While it allows creating workspaces from different branches, it lacks a native concept of "Cross-Repo Dependencies." Orchestrating a change that spans a microservice (Repo A) and a shared library (Repo B) requires the human user to manually spin up workspaces in both, mentally synchronize the objectives, and act as the bridge between the two agents. The agents themselves have no awareness of each other's existence or progress.

This "Repo-Bound" nature renders Conductor insufficient for the target persona of this report: the Architect managing a 50-microservice ecosystem.7 For such a scenario, we require a system that transcends the boundaries of a single git repository and establishes a unified, persistent memory graph.

## 3. The Beads Ecosystem: Structured Memory for the AI Age

The "Beads" project, led by Steve Yegge, offers a philosophical and architectural counterpoint to the Prose paradigm. It posits that for agents to function as reliable engineers, they require a memory structure that is deterministic, queryable, and resilient—a "memory upgrade" for the coding agent.3

### 3.1 The Philosophy of Graph-Based Memory

Beads (binary name bd) rejects the messy ambiguity of markdown in favor of a rigid, schema-driven issue tracker that lives inside the git repository.

#### 3.1.1 Git as the Database of Record

The defining characteristic of Beads is its storage mechanism. Issues are serialized as JSONL (JSON Lines) files within a hidden .beads/ directory in the repository root.3 This decision has profound implications for state management:

* **Atomic Versioning:** Because the issues are files in the repo, they are versioned alongside the code. If a developer checks out a feature branch from three weeks ago, the issue tracker state automatically reverts to exactly what it was three weeks ago. This eliminates the "Drift" problem common in JIRA, where the code on a branch might be weeks old, but the JIRA ticket status is "Closed," causing confusion for an agent trying to understand the current context.  
* **Offline Sovereignty:** Beads requires no external server, no API keys, and no network connection. An agent can perform complex query planning on an airplane or in a secure, air-gapped enclave. This aligns with the "Local-First" software movement.

#### 3.1.2 Hash-Based Identity and Conflict Avoidance

In distributed systems, naming things is one of the hardest problems. Traditional trackers use sequential integers (PROJ-1, PROJ-2). If two agents on disconnected branches both create "PROJ-3", a collision occurs upon merging.

Beads utilizes cryptographic hashes (e.g., bd-a3f8) for issue identification.3 This ensures that Agent A can create a dozen tasks on its branch, and Agent B can create a dozen on hers, and when they merge, the union of the two sets is perfectly preserved without ID conflict. This feature is critical for the "Boss" architecture, which relies on aggregating beads from disjoint sources.

### 3.2 Semantic Compaction: The Solution to Context Inflation

One of the most innovative features of Beads is "Compaction".8 As discussed, retaining the full history of a project bloats the context window. Beads solves this by treating completed tasks as compressible data.

When a task is closed, the system (or the agent) can perform a semantic summarization step. The verbose logs, the back-and-forth debugging steps, and the intermediate errors are compressed into a concise "resolution summary." The original detailed object is archived (moved to a different storage path or git history), and only the lightweight summary remains in the active graph. This allows an agent to query the history ("How did we fix the JWT bug last month?") and receive the high-level logic without being burdened by the megabytes of chat logs that produced it.

### 3.3 Gas Town: The Orchestration Layer

While Beads provides the storage layer (the "Hippocampus"), "Gas Town" provides the executive function (the "Frontal Cortex"). Gas Town is a workspace manager that wraps the Beads data structure in a runtime environment.9

#### 3.3.1 The Taxonomy of the Colony

Gas Town introduces a rigorous taxonomy for the components of an AI colony, utilizing a metaphorical "Mad Max" naming convention to distinguish roles 10:

* **The Rig:** A container for a specific project (a git repository) and its associated Beads graph.  
* **The Town:** The local directory housing the collection of Rigs and the global configuration.  
* **The Mayor:** The primary supervising agent (typically a high-intelligence model like Claude 3.5 Sonnet). The Mayor maintains the strategic plan and delegates execution.  
* **The Polecat:** A specialized, ephemeral worker agent spawned to execute a specific Bead. Polecats are "fire-and-forget"; they live only as long as the task requires.  
* **The Convoy:** A logical grouping of tasks (Beads) that travels between agents. This solves the handoff problem—when a Mayor delegates to a Polecat, it constructs a Convoy containing the necessary context and constraints.

#### 3.3.2 The Propulsion Principle

Gas Town operates on the "Propulsion Principle": the idea that the state of work should be propelled forward by git hooks and persistent storage, not by the volatile memory of a running process.9 Every significant action (starting a task, finishing a task) triggers a git commit in the Beads directory. This means that if the power fails, or the agent crashes, or the context window overflows, the state is preserved on disk. Upon restart, the Mayor simply reads the .beads directory to reconstruct the exact state of the world.

## 4. The AllBeads PRD: Distributed Orchestration with Agent Communication

> **Implementation Status (as of January 2026):**
> - ✅ **Core CLI**: 25+ commands implemented (`ab list`, `ab show`, `ab tui`, `ab mail`, `ab sheriff`, `ab janitor`, `ab swarm`, `ab info`, `ab prime`, `ab human`, etc.)
> - ✅ **Multi-Context**: Work/personal Boss repo aggregation fully functional
> - ✅ **TUI**: All 4 views complete (Kanban, Mail, Graph, Swarm)
> - ✅ **Agent Mail**: Postmaster server with all 7 message types, file locking, HTTP/IPC interfaces
> - ✅ **Sheriff**: Foreground mode with manifest parsing and shadow sync; background daemon mode planned
> - ✅ **Agent Integration**: Claude Code marketplace plugin, agent commands (info/prime/human/onboard/setup/quickstart)
> - ✅ **Release Infrastructure**: GitHub Actions for CI/CD, Homebrew formula, CONTRIBUTING.md
> - ✅ **Janitor**: Automated codebase analysis for legacy repo onboarding
> - ✅ **Enterprise**: JIRA/GitHub integration with REST/GraphQL adapters
> - ✅ **Swarm**: Agent lifecycle, cost tracking, budget management, TUI monitoring
> - ✅ **Graph**: Dependency visualization with cross-context analysis and cycle detection

We have established the limitations of Conductor (repo-bound) and the strengths of Beads (structured, git-backed). We now define the **AllBeads** product: an open-source CLI/TUI system that extends Beads from a single repository to a distributed, multi-context environment with inter-agent communication.

### 4.1 Problem Statement: The Identity and Coordination Failures

Modern developers face two critical challenges:

#### 4.1.1 The Split-Identity Problem

Engineers rarely have one identity. They have:
- **Work Identity**: Authenticated to enterprise GitHub (e.g., `ibm.github.com`) via VPN/SSO, bound by corporate policies and IP protection
- **Personal Identity**: Authenticated to public GitHub (`github.com`) for open-source and hobby projects

**The Problem**: Existing tools force a choice or awkward context switching. Centralizing everything in one repository is a security violation (leaking intellectual property); separating them destroys the "single pane of glass" productivity engineers need.

**The AllBeads Solution**: AllBeads acts as a **Meta-Client**. It reads from `~/work/beads-boss` and `~/personal/beads-boss` simultaneously, aggregating them into a unified view without ever merging their data or crossing security boundaries.

#### 4.1.2 The Agent Collision Problem

When multiple AI agents work on a codebase simultaneously, they lack coordination mechanisms:
- **File Clobbering**: Agent A refactors `auth.ts` while Agent B adds features to the same file, resulting in conflicting changes
- **Duplicate Work**: Multiple agents independently attempt to fix the same bug without awareness of each other
- **Resource Contention**: Agents consume API rate limits, compute resources, and context windows without coordination

**The Problem**: Beads (Git) is excellent for **State** (what needs to be done) but poor for **Signaling** (who is touching what right now?).

**The AllBeads Solution**: AllBeads integrates **Agent Mail** protocols, providing a "Post Office" where agents broadcast intent ("I am refactoring auth.ts") and acquire mutex locks on files, preventing the "Too Many Cooks" failure mode.

### 4.2 Design Principles

AllBeads is built on three foundational principles:

#### 4.2.1 Federated Authority

**There is no single "Master."** There are only Peers. Your "Work Boss" repository and "Personal Boss" repository are equals in the TUI. AllBeads never "crosses the streams"—it aggregates view-only, never pushing Personal beads to Work repos or vice versa.

This enables:
- **Security**: Enterprise IP stays in enterprise repos with enterprise authentication
- **Flexibility**: Personal projects remain fully independent
- **Unified UX**: Single dashboard for all work without compromising boundaries

#### 4.2.2 Surgical Invasion

AllBeads does not require codebases to be "re-platformed." It can surgically `init` a `.beads/` directory in a legacy 2015 repository, assign an agent to it, and track work without disrupting existing workflows, CI/CD pipelines, or team practices.

This enables:
- **Brownfield Adoption**: Add agents to existing codebases incrementally
- **Zero Migration Cost**: No need to move issues from JIRA or GitHub
- **Reversible**: `.beads/` can be removed with `git rm` if desired

#### 4.2.3 Open Standards

Built for the community with an **MIT License**, using standard protocols:
- **JSONL** for beads storage (human-readable, git-friendly)
- **MCP (Model Context Protocol)** for Agent Mail communication
- **XML manifests** compatible with Google's `git-repo` tool
- **Standard Git** for all state management

This enables:
- **Interoperability**: Works with any beads-compatible tool
- **Extensibility**: Community can build plugins and adapters
- **Transparency**: All state is inspectable, no lock-in

### 4.3 The Boss Repository: Multi-Context Aggregation

A "Boss" repository is a standard Git repository used to track high-level goals for a specific domain. Unlike traditional approaches, **AllBeads allows users to register multiple Boss repositories.**

**User Story:**
> "As a developer, I configure `~/.config/allbeads/config.yaml` to point to `github.com/me/life-boss` and `ibm.github.com/team-alpha/work-boss`. AllBeads pulls tasks from both. When I select a work task, it uses my IBM credentials. When I select a personal task, it uses my personal SSH key. The TUI shows both contexts unified, but they never intermix data."

#### 4.3.1 Directory Structure of a Boss Repo

Each Boss repository adheres to a standard schema:

| Path | Description |
| :---- | :---- |
| /.boss/config.yaml | Boss-specific configuration, integration settings, and policies |
| /.boss/graph/ | The aggregated SQLite/JSONL database of the federated graph |
| /manifests/default.xml | The definition of member Rigs (repositories, branches, paths) |
| /beads/shadow/ | Shadow Beads—pointers to beads in member repositories |
| /agents/personas/ | Agent persona definitions (security-specialist, ux-designer, etc.) |

#### 4.3.2 Multi-Boss Configuration

AllBeads configuration lives at `~/.config/allbeads/config.yaml`:

```yaml
contexts:
  - name: personal
    type: git
    url: git@github.com:thrashr888/beads-boss.git
    auth_strategy: ssh_agent

  - name: work
    type: git
    url: https://ibm.github.com/cloud-team/beads-boss.git
    auth_strategy: gh_enterprise_token
    env_vars:
      GITHUB_TOKEN: $IBM_GH_TOKEN
    integrations:
      jira:
        url: https://jira.ibm.com
        project: CLOUD

agent_mail:
  port: 8085
  storage: ~/.config/allbeads/mail.db

visualization:
  theme: dark
  default_view: kanban
```

**Security Note**: AllBeads maintains strict separation. Personal beads never leak into work contexts and vice versa. The aggregation happens only in the local TUI's in-memory state and local SQLite cache.

### 4.3 The Federated Graph Mechanism

The central technical challenge is linking a bead in boss-repo to a bead in child-repo. We introduce the concept of **Shadow Beads** and **Pointer IDs**.

#### 4.3.1 Shadow Beads

The Boss repository does not replicate every minute sub-task from every child repo. That would create unmanageable noise. Instead, it maintains a "Shadow Bead" for every **Epic-level** item in a member Rig.

When a Mayor in auth-service promotes a task to an Epic, the "Sheriff" daemon (described below) creates a corresponding Shadow Bead in the Boss repo. This Shadow Bead contains:

* **Summary:** "Refactor JWT Logic"  
* **Status:** Mirrored from child.  
* **Pointer:** uri: bead://auth-service/bd-a3f8  
* **Dependencies:** Cross-rig links (e.g., blocks: bead://frontend-web/bd-x9y2).

#### 4.3.2 The Sheriff Daemon: The Synchronization Engine

> **Implementation Status**: ✅ Foreground mode implemented (`ab sheriff --foreground`)
> - ✅ Tokio async runtime with configurable poll intervals
> - ✅ Manifest parsing (XML) with Rig configuration
> - ✅ Shadow Bead synchronization
> - ✅ Event stream for TUI communication
> - ❌ Background daemon mode (systemd/launchd integration planned)
> - ❌ External sync phase (JIRA/GitHub - Phase 4)

The **Sheriff** is a long-running background process that enforces consistency across the federated graph. It is the "glue" of the AllBeads architecture.

**Technical Specification:**

* **Language:** Rust with tokio async runtime for performance and safety ✅
* **Concurrency:** Async tasks for parallel polling of Rigs and external APIs ✅
* **Event Loop:**
  1. **Poll Phase:** Iterate through manifests/default.xml. For each Rig, run `git fetch origin refs/beads/*` ✅
  2. **Diff Phase:** Compare the local `.beads` state of the Rig with the cached state in the Boss Graph ✅
  3. **Sync Phase:** If a Rig has new Epics, create Shadow Beads in Boss. If Boss has new directives (e.g., a "Global Mandate"), push new Beads to the Rig's `.beads` directory ✅
  4. **External Sync Phase:** Push/Pull changes to JIRA and GitHub Issues (detailed in Section 5) ❌
  5. **Mail Delivery Phase:** Process Agent Mail queue, deliver messages, enforce file locks ✅

### 4.4 The Manifest Standard

To define the member Rigs, we adopt a schema compatible with the git-repo standard 11 but enhanced with AllBeads-specific annotations. This allows existing tools to clone the workspace while providing the Sheriff with necessary metadata.

**Example manifests/default.xml:**

```xml
<manifest>
  <remote name="origin" fetch=".." />
  <default revision="main" remote="origin" />

  <project path="services/auth" name="backend/auth-service">
    <annotation key="allbeads.persona" value="security-specialist" />
    <annotation key="allbeads.prefix" value="auth" />
    <annotation key="allbeads.jira-project" value="SEC" />
  </project>

  <project path="frontend/web" name="frontend/react-app">
    <annotation key="allbeads.persona" value="ux-designer" />
    <annotation key="allbeads.prefix" value="ui" />
    <annotation key="allbeads.jira-project" value="FE" />
  </project>
</manifest>
```

This XML allows the Sheriff to know not just *where* the code is, but *who* (which specialized agent persona) should be summoned to work on it, and *how* to namespace the beads (auth-xxx, ui-xxx) to ensure global uniqueness in the Boss graph.

### 4.5 Agent Mail: The Communication and Coordination Layer ✅ IMPLEMENTED

> **Implementation Status**: Full Agent Mail system implemented
> - ✅ Postmaster server (HTTP + IPC interfaces)
> - ✅ All 7 message types (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST, HEARTBEAT, RESPONSE)
> - ✅ File locking with TTL and lease management
> - ✅ SQLite-backed message persistence
> - ✅ Agent addressing (`agent_name@project_id`)
> - ✅ TUI Mail view with inbox

While Beads provides the memory layer (what needs to be done), **Agent Mail** provides the signaling layer (who is doing what right now). This prevents the coordination failures that plague multi-agent systems.

#### 4.5.1 The Post Office Architecture

AllBeads includes a lightweight message server (the "Postmaster") implementing the MCP Agent Mail protocol. This server runs locally as part of the Sheriff daemon and provides:

**Message Routing:**
- Agents send messages to `agent_name@project_id` (e.g., `refactor_bot@legacy-repo-1`)
- Human operator has an inbox: `human@localhost`
- Broadcast channels: `all@project_id` for announcements

**Delivery Guarantees:**
- At-least-once delivery for critical messages (file locks, blocking notifications)
- Best-effort for status updates (progress notifications, thoughts)
- Persistent storage in SQLite (`~/.config/allbeads/mail.db`)

#### 4.5.2 File Locking and Mutex Protocol

The most critical feature of Agent Mail is **file locking** to prevent concurrent modifications:

**Lock Request Flow:**

1. **Agent Intent**: Agent `refactor_bot` wants to modify `src/auth/parser.rs`
2. **Request Lock**: Sends message to Postmaster: `LOCK src/auth/parser.rs TTL=3600`
3. **Check Availability**: Postmaster queries lock table in SQLite
4. **Grant or Deny**:
   - If free: Grant lease, store in database with expiration timestamp
   - If locked: Deny with message indicating current holder and expiration time
5. **Work Execution**: Agent performs modifications
6. **Release**: Agent sends `UNLOCK src/auth/parser.rs` or lease expires

**Conflict Resolution:**

When an agent requests a locked file:
```
Agent B: LOCK src/auth/parser.rs
Postmaster: DENIED - Locked by refactor_bot until 2026-01-09 14:30 UTC
             Reason: "Refactoring authentication logic"

             Options:
             1. WAIT - Subscribe to unlock notification
             2. STEAL - Request human approval to break lock
             3. ABORT - Work on different task
```

#### 4.5.3 Message Types

The Agent Mail protocol supports several message categories:

| Type | Description | Example |
|------|-------------|---------|
| `LOCK` | Request exclusive file access | `LOCK src/db.rs TTL=1800` |
| `UNLOCK` | Release file lock | `UNLOCK src/db.rs` |
| `NOTIFY` | Inform about state changes | `NOTIFY "PR #402 ready for review"` |
| `REQUEST` | Ask for human input | `REQUEST "Approve scope change for Epic ab-15k?"` |
| `BROADCAST` | Announce to all agents | `BROADCAST "JIRA API rate limit exhausted, pausing"` |
| `HEARTBEAT` | Agent liveness signal | `HEARTBEAT agent=refactor_bot status=working` |

#### 4.5.4 Human Inbox Integration

The TUI includes a dedicated "Mail" tab showing:

**Incoming Messages:**
- `Agent smith@auth-service: Blocked by file lock on config.yaml held by neo@frontend`
- `Agent trinity@payment: PR #845 ready for review (3 files changed)`
- `Agent oracle@analytics: REQUEST - Database migration will require 2 hours downtime, approve?`

**Actions Available:**
- **Reply**: Send message back to specific agent
- **Approve/Deny**: For REQUEST messages
- **Break Lock**: Override file lock (with confirmation)
- **Kill Agent**: Terminate agent process

#### 4.5.5 The "Janitor" Workflow ✅ IMPLEMENTED

> **Implementation Status**: `ab janitor` command fully implemented with:
> - ✅ Missing documentation detection (README, LICENSE, CONTRIBUTING)
> - ✅ Missing configuration detection (.gitignore, SECURITY.md)
> - ✅ Test coverage analysis per language
> - ✅ TODO/FIXME/HACK comment scanning
> - ✅ Security pattern detection (hardcoded secrets, eval usage)
> - ✅ `--dry-run` and `--verbose` flags

A specialized use case for Agent Mail is the "Janitor" mode for legacy repository onboarding:

```bash
$ allbeads init --remote https://github.com/org/legacy-repo --janitor
```

This command:
1. Clones the repository (sparse checkout for `.beads/` only initially)
2. Injects a `.beads/` directory and initializes the database
3. Creates an "Analysis" bead assigned to a Janitor agent
4. The Janitor agent:
   - Scans the codebase for issues (using static analysis, grep patterns, etc.)
   - Creates new beads for bugs, tech debt, missing tests
   - Broadcasts findings via Agent Mail: `NOTIFY "Created 47 tech debt issues in backlog"`
   - Populates the Boss backlog without human intervention

This enables "fire and forget" adoption of brownfield codebases.

## 5. Enterprise Integration: Bridging the Gap with JIRA and GitHub

> **Implementation Status**: ❌ Phase 4 - Not yet implemented
> - Placeholder modules exist in `src/integrations/` (jira.rs, github.rs)
> - Architecture defined but no functional implementation
> - Planned for Q4 2026

The greatest barrier to AI adoption in large organizations is the disconnect between the "System of Execution" (Git/Code) and the "System of Record" (JIRA/GitHub Issues). Managers live in JIRA; Agents live in Git. The "Boss" architecture bridges this gap.

### 5.1 The Integration Paradox

AI agents struggle with JIRA. The JIRA interface is slow, complex, and prone to DOM changes that break scrapers. The API is vast and often rate-limited. Conversely, forcing human managers to use the bd CLI is a non-starter.  
The solution is Bi-Directional Mirroring, handled exclusively by the Sheriff daemon. The Agent never talks to JIRA directly; it talks to Beads. The Sheriff talks to JIRA.

### 5.2 JIRA Integration Architecture via go-jira

We leverage the go-jira library 13 to build a robust bridge.

#### 5.2.1 Ingress Strategy (JIRA -> Boss)

* **Polling/Webhook:** The Sheriff listens for JIRA issues matching a JQL filter (e.g., labels = "ai-agent" AND status = "To Do").  
* **Translation:** When an issue is found (e.g., PROJ-101), the Sheriff:  
  1. Determines the target Rig based on the JIRA Project Key (mapped in manifests/default.xml).  
  2. Creates a P0 Bead in that Rig's .beads/ directory.  
  3. Populates the Bead with the JIRA description and attachments.  
  4. Adds a metadata field: external_ref: "jira:PROJ-101".

#### 5.2.2 Egress Strategy (Boss -> JIRA)

* **Status Mapping:** When the Agent closes the Bead (bd close), the Sheriff detects the state change. It maps the Bead status (closed/fixed) to the JIRA transition ID (e.g., "Move to QA").  
* **Comment Sync:** The Sheriff appends a comment to the JIRA ticket: *"Work completed by Gas Town Agent. PRs created: #45, #46. Summary of changes:."*

#### 5.2.3 Conflict Resolution (CRDT-Lite)

A critical edge case is "Split-Brain": What if the Manager updates the JIRA ticket to "Blocked" while the Agent updates the Bead to "In Progress"?  
The Sheriff implements a "Last-Write-Wins" policy based on timestamps, but with Human Authority Override. If the JIRA update is from a human user, it trumps the Agent's update. The Sheriff effectively "reverts" the Bead state to match JIRA, ensuring the Agent receives the signal to stop working.

### 5.3 GitHub Issues Integration via GraphQL

For GitHub Issues, which are often more technical and code-adjacent than JIRA, we use the shurcooL/githubv4 library 14 to leverage the GraphQL API.

#### 5.3.1 The "Discussion Thread" Sync

Unlike JIRA, where the description is static, GitHub Issues are often conversation-driven.

* **Mechanism:** The Sheriff subscribes to issue comments. When a human comments "Please also check the regex in parsing.go", the Sheriff appends this as a "Note" to the active Bead.  
* **Discovery Link:** If an Agent discovers a bug and creates a discovered-from bead, the Sheriff automatically opens a GitHub Issue, tags the relevant human owners (based on CODEOWNERS), and links it back to the parent PR. This creates a seamless audit trail from "AI Discovery" to "Human Triage".

## 6. Visualizing the Swarm: The Unified TUI ("All-Seeing Eye")

> **Implementation Status**: ✅ COMPLETE
> - ✅ Kanban view with multi-context aggregation
> - ✅ Mail view with inbox, compose, and reply
> - ✅ Graph view (dependency visualization with ASCII rendering)
> - ✅ Swarm view (agent status monitor)
> - ✅ ratatui + crossterm architecture
> - ✅ Vim-style keyboard navigation

A command-line tool (`bd list`) is insufficient for visualizing work spanning multiple contexts and repositories. The **AllBeads TUI** serves as the command center, aggregating all Boss repositories into a single, coherent interface.

### 6.1 Why TUI?

A TUI is preferred over a Web UI because:

* **SSH-Friendly**: Runs over SSH, allowing management of remote development boxes
* **Low Latency**: Direct integration with local git commands and file system
* **No Context Switching**: Stays in the terminal alongside editor and shell
* **Lightweight**: Minimal resource footprint compared to Electron/web apps

### 6.2 Component Architecture

The TUI is built with **ratatui** (Rust TUI framework), following a message-passing architecture similar to The Elm Architecture.

#### 6.2.1 The Data Model

```rust
struct AppState {
    contexts: Vec<BossContext>,           // Multiple Boss repos (work, personal)
    aggregated_graph: FederatedGraph,     // Unified view of all beads
    active_agents: Vec<AgentStatus>,      // Running agent processes
    mail_inbox: Vec<Message>,             // Agent Mail messages
    file_locks: HashMap<PathBuf, Lock>,   // Current file locks
    active_view: ViewMode,                // Kanban, Graph, Mail, Swarm
    filter: Filter,                       // @work, @personal, #bug, etc.
}

enum ViewMode {
    Kanban,      // Task board aggregated across all contexts
    Graph,       // Dependency visualization
    Mail,        // Agent communication inbox
    Swarm,       // Real-time agent status
}
```

#### 6.2.2 The Four Primary Views

> **Implementation Status**: Kanban ✅ | Mail ✅ | Graph ✅ | Swarm ✅

**View 1: The Strategic Kanban** ✅ IMPLEMENTED

Aggregates beads from **all Boss contexts** into a unified board with context indicators:

```
┌─ Backlog ────────────┬─ In Progress ─────────┬─ Blocked ─────────────┬─ Done ──────────┐
│ @work [SEC-101]      │ @work [SEC-99]        │ @personal [ab-5]      │ @work [SEC-88]  │
│ JWT Refactor         │ Token Update          │ Blog redesign         │ Schema migration│
│ Priority: P1         │ Agent: refactor_bot   │ (Blocked by ab-3)     │ ✓ Merged        │
│                      │ 🔒 auth.rs locked     │                       │                 │
├──────────────────────┼───────────────────────┼───────────────────────┼─────────────────┤
│ @personal [ab-8]     │ @work [PAY-45]        │ @work [FE-123]        │                 │
│ Implement RSS        │ Stripe webhook        │ Profile page update   │                 │
│ Priority: P3         │ Agent: payment_agent  │ (Blocked by SEC-99)   │                 │
└──────────────────────┴───────────────────────┴───────────────────────┴─────────────────┘
```

Key features:
- **Context Tags**: `@work`, `@personal` color-coded
- **Lock Indicators**: 🔒 shows active file locks
- **Blocking Relationships**: Clearly marked with bead IDs

**View 2: The Dependency Graph** ✅ IMPLEMENTED

Renders cross-repository dependencies using ASCII/Unicode:

```
       @work                               @personal
   ┌────────────┐                      ┌──────────────┐
   │ SEC-99     │──blocks──▶           │ FE-123       │
   │ JWT Token  │                      │ Profile Page │
   └────────────┘                      └──────────────┘
         │                                      │
    related-to                             depends-on
         │                                      │
         ▼                                      ▼
   ┌────────────┐                      ┌──────────────┐
   │ SEC-101    │                      │ ab-3         │
   │ Refactor   │                      │ CSS Framework│
   └────────────┘                      └──────────────┘
```

Allows instant identification of cross-context bottlenecks.

**View 3: Agent Mail Inbox** ✅ IMPLEMENTED

The communication hub for agent-human interaction:

```
┌─ Inbox (5 unread) ──────────────────────────────────────────────────────────────┐
│ ● refactor_bot@auth-service                                        2 mins ago   │
│   Subject: File lock conflict                                                   │
│   Message: Requested auth.rs but locked by payment_agent. Should I wait or     │
│            work on different file?                                              │
│   [Reply] [Approve Wait] [Break Lock]                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│ ● payment_agent@payment                                            5 mins ago   │
│   Subject: PR #845 ready for review                                            │
│   Message: Completed webhook implementation. 3 files changed, 12 tests pass.   │
│   [View PR] [Approve Merge] [Request Changes]                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│   oracle@analytics                                                 10 mins ago  │
│   Subject: Migration requires downtime                                          │
│   Message: Database migration will take ~2 hours. Approve deployment window?   │
│   [Replied: "Approved for Saturday 3am"]                                       │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**View 4: The Swarm Monitor** ✅ IMPLEMENTED

Real-time status of all active agents across all contexts:

```
┌─ Active Agents ──────────────────────────────────────────────────────────────────┐
│ @work Context (3 active)                                                        │
│   🟢 refactor_bot (PID 8821) - auth-service                                     │
│      Status: Generating unit tests for auth.rs                                  │
│      Runtime: 15m 32s | Cost: $0.42 | Locked: [auth.rs, auth_test.rs]         │
│                                                                                  │
│   🟡 payment_agent (PID 8845) - payment-gateway                                 │
│      Status: Waiting for CI checks...                                           │
│      Runtime: 45m 12s | Cost: $1.23 | No locks                                 │
│                                                                                  │
│   🔴 frontend_agent (PID 8890) - frontend-web [ERROR]                           │
│      Status: API Rate Limit Exceeded (retry in 3m)                              │
│      Runtime: 1h 05m | Cost: $2.15                                             │
│                                                                                  │
│ @personal Context (1 active)                                                    │
│   🟢 blog_agent (PID 9012) - personal-blog                                      │
│      Status: Refactoring CSS grid layout                                        │
│      Runtime: 8m 15s | Cost: $0.08 | Locked: [styles.css]                     │
│                                                                                  │
│ [Kill Agent] [Pause All] [Resume All] [View Logs]                              │
└──────────────────────────────────────────────────────────────────────────────────┘
```

#### 6.2.3 Filtering and Navigation

Vim-style filtering allows quick context slicing:

- `/` - Enter filter mode
- `@work` - Show only work context
- `@personal` - Show only personal context
- `#bug` - Filter by tag
- `@claude` - Filter by agent name
- `/auth` - Text search across beads
- Filters can be combined: `@work #p1 /authentication`

**Keyboard Shortcuts:**

| Key | Action |
|-----|--------|
| `Tab` | Cycle through views (Kanban → Graph → Mail → Swarm) |
| `j/k` | Navigate up/down |
| `Enter` | Open/focus selected item |
| `/` | Enter filter mode |
| `Esc` | Clear filters / go back |
| `r` | Refresh from remotes |
| `m` | Open mail composer |
| `q` | Quit |

### 6.3 Implementation Details

The TUI is built with the following architecture:

**Components:**
- **ratatui**: Terminal rendering framework
- **crossterm**: Terminal manipulation and event handling
- **tokio**: Async runtime for real-time updates

**Communication:**
The TUI runs as a client connected to the Sheriff daemon via:
- **IPC Socket**: Unix domain socket for low-latency commands
- **Shared SQLite**: WAL mode for concurrent read access to state
- **Event Stream**: Sheriff pushes updates (new beads, agent status changes, mail)

This separation ensures the UI remains responsive even during heavy git network operations.

## 7. Operational Workflow: The "Migration" Scenario

To demonstrate the efficacy of the AllBeads architecture, we detail a complex operational workflow: **"Migrating the User Database from SQL to NoSQL"**. This task spans three repositories: backend-api (logic), migration-scripts (data), and frontend-web (UI changes).

### Phase 1: Inception and Strategic Planning

1. **Human Origin:** The Architect creates a JIRA Epic: "Migrate Users to DynamoDB".  
2. **Ingress:** The Sheriff detects the Epic and creates a Boss Bead: boss-mig-1.  
3. **Executive Agent (Boss):** The Boss Agent (Claude 3.5) wakes up. It reads boss-mig-1 and the manifests/default.xml. It reasons: "To migrate the DB, I need to update the API, write a migration script, and update the UI to handle the new ID format."  
4. **Decomposition:** The Executive Agent creates three sub-beads in the Boss Repo:  
   * boss-mig-1.1 -> Assigned to backend-api  
   * boss-mig-1.2 -> Assigned to migration-scripts  
   * boss-mig-1.3 -> Assigned to frontend-web  
   * **Dependency Linking:** It links 1.3 (UI) as blocked by 1.1 (API).

### Phase 2: Dispatch and Execution

1. **Sheriff Push:** The Sheriff pushes boss-mig-1.1 to the backend-api Rig as bd-api-50.  
2. **Mayor Wake-Up:** The Mayor agent in the backend-api Rig sees the new P0 bead. It spawns a Polecat to execute the code changes.  
3. **Cross-Repo Discovery:** While working, the Polecat realizes the new ID format breaks the analytics service. It creates a discovered-from bead: bd-api-51: Analytics Service needs update.  
4. **Bubble Up:** The Sheriff sees bd-api-51. It checks the manifest, realizes Analytics is a separate repo. It promotes this bead to the Boss level, creates a new Epic boss-mig-1.4, and assigns it to the analytics-worker Rig.

### Phase 3: Review and Completion

1. **TUI Visualization:** The Architect checks the Boss Board. They see the migration is "In Progress" but now has a new branch blocking it (Analytics). They approve the scope creep via the TUI.  
2. **Integration:** As the Backend and Analytics Rigs complete their work (PRs merged), the Sheriff updates the dependencies. The Frontend Rig (previously blocked) is now unlocked.  
3. **Finalize:** The Frontend Mayor completes the UI updates. The Sheriff sees all sub-beads are closed. It marks the parent boss-mig-1 as Closed and transitions the JIRA Epic to "Done".

## 8. Conclusion: From Beads Boss to Beads Society

The transition from human-centric to agent-centric software engineering requires a fundamental re-evaluation of our tooling infrastructure. Tools like conductor.build have demonstrated the power of parallelized, worktree-isolated agents, but they remain constrained by single-repository scope and lack of persistent state. The beads ecosystem provides the memory layer, but lacks coordination primitives for multi-agent and multi-context scenarios.

**AllBeads bridges these gaps** by combining:
- **Distributed Authority**: Multi-context aggregation respecting security boundaries (work/personal)
- **Agent Communication**: Mail-based coordination preventing file clobbering and duplicate work
- **Federated Memory**: Cross-repository dependency graphs with Shadow Beads
- **Enterprise Integration**: Bi-directional sync with JIRA and GitHub Issues

This moves beyond the concept of a single "Beads Boss" to a **"Beads Society"**—a networked ecosystem where agents communicate, negotiate resources, and coordinate across the entirety of a developer's digital life.

### 8.1 Comparative Summary

| Feature | Prose.md | Conductor | Beads Alone | **AllBeads** |
| :---- | :---- | :---- | :---- | :---- |
| **Primary Context** | Unstructured Text | Ephemeral Worktree | Git-Backed JSONL | Multi-Boss Federation |
| **Scope** | Single Session | Single Repo | Single Repo | Multi-Context / Multi-Repo |
| **State Persistence** | Low (Volatile) | Medium (Repo-Bound) | High (Git-Native) | **Highest (Federated & Distributed)** |
| **Agent Communication** | None | None | None | **Agent Mail + File Locks** |
| **Identity Management** | Single | Single | Single | **Multi-Context (Work/Personal)** |
| **Conflict Resolution** | Manual Merge | Manual Review | Hash-Based | **Hash-Based + Mutex Protocol** |
| **Visualization** | Text Editor | Diff Viewer | CLI Only | **Unified TUI (4 views)** |
| **JIRA Integration** | Manual | None | Plugin Possible | **Built-In Bi-Directional** |
| **Agent Autonomy** | Junior Developer | Individual Contributor | Senior Developer | **Staff Engineer + Coordinator** |
| **License** | N/A | Proprietary | MIT | **MIT (Open Source)** |

### 8.2 Open Source Strategy and Roadmap

To ensure broad adoption and community contribution, AllBeads will be developed as an **MIT-licensed open-source project** with the following roadmap:

**Phase 1: The Reader (Read-Only Aggregation)** - Q1 2026 ✅ COMPLETE
- ✅ CLI that reads multiple local `.beads` repositories
- ✅ Basic TUI showing unified Kanban view
- ✅ Multi-context configuration support
- **Deliverable**: `allbeads list` shows work and personal tasks together ✅

**Phase 2: The Mailroom (Agent Communication)** - Q2 2026 ✅ COMPLETE
- ✅ Implement MCP Agent Mail server (Postmaster)
- ✅ File locking/mutex protocol
- ✅ Mail tab in TUI with human inbox
- **Deliverable**: Agents can coordinate and prevent conflicts ✅

**Phase 3: The Writer (Distributed Boss)** - Q3 2026 ✅ COMPLETE
- ✅ Support `allbeads init --remote` for legacy repos
- ✅ Janitor workflow for automated issue discovery
- ✅ Sheriff daemon with git sync (foreground mode)
- **Deliverable**: Full write-back to Boss repos ✅

**Phase 4: Enterprise Integration** - Q4 2026 ✅ COMPLETE
- ✅ JIRA bi-directional sync (REST API adapter)
- ✅ GitHub Issues integration (GraphQL + REST API adapter)
- ✅ Plugin architecture for other systems (extensible structs)
- ✅ External sync in Sheriff daemon
- ✅ CLI commands: `ab jira`, `ab github`
- **Deliverable**: Enterprise-ready orchestration ✅

**Phase 5: The Swarm (Advanced Agents)** - Q1 2027 ✅ COMPLETE
- ✅ Agent lifecycle management (spawn, monitor, kill)
- ✅ Cost tracking and budget management
- ✅ Swarm TUI view with real-time status
- ✅ CLI commands: `ab swarm list`, `ab swarm stats`, `ab swarm budget`, etc.
- ✅ Context-based budget limits with warnings
- **Deliverable**: Self-managing agent workforce ✅

**Phase 6: Graph Visualization** - Q1 2027 ✅ COMPLETE
- ✅ Dependency Graph TUI view with ASCII visualization
- ✅ Cross-context dependency chain analysis
- ✅ Cycle detection in dependency graphs
- ✅ Filter modes (All, Blocked, Cross-Context)
- ✅ Graph detail view with dependency trees
- **Deliverable**: Visual understanding of work dependencies ✅

**Phase 7: Agent Integration** - Q1 2027 ✅ COMPLETE
- ✅ Claude Code marketplace skills plugin (`.claude-plugin/marketplace.json`)
- ✅ Agent onboarding commands (`ab human`, `ab info`, `ab prime`, `ab setup`, `ab quickstart`, `ab onboard`)
- ✅ Context recovery commands for new sessions
- ✅ CLI commands organized into logical sections
- ✅ AGENTS.md agent integration guide
- ✅ Workflow guides via `ab onboard --full` and AGENTS.md
- ✅ Integration with beads via `ab prime` and `ab info` commands
- **Deliverable**: First-class AI agent support ✅

**Phase 8: Release & Community** - Q1 2027 ✅ COMPLETE
- ✅ GitHub Actions for automated releases (`.github/workflows/release.yml`)
- ✅ GitHub Actions for CI (`.github/workflows/ci.yml`)
- ✅ Homebrew formula template (`packaging/homebrew/allbeads.rb`)
- ✅ `CONTRIBUTING.md` with development guide
- ✅ Cross-platform binaries (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64)
- ✅ AGENTS.md for AI agent integration
- ❌ Comprehensive documentation and tutorials
- ❌ Community support channels
- **Deliverable**: Production-ready open source release ✅

### 8.3 Community and Contribution

**Dogfooding Requirement**: The AllBeads repository itself **must be managed by AllBeads**. All issues, PRs, and work coordination will use the tool being built. The `CONTRIBUTING.md` will explain how to:
1. Install AllBeads locally
2. Point it at the AllBeads Boss repository
3. Pick up a "Good First Issue" tracked as a bead
4. Have an agent assist with implementation

**Plugin Ecosystem**: The Sheriff daemon will support plugins written in Rust, Python, or WASM, allowing community contributions for:
- Additional integrations (Trello, Monday.com, Notion)
- Custom agent personas
- Alternative visualization themes
- Domain-specific workflows (data science, devops, etc.)

### 8.4 Impact and Vision

By implementing AllBeads, the software development community can move beyond the "Chatbot" phase of AI and enter the era of the **Agent Society**—where software builds, tests, and repairs itself under strategic human guidance, across all contexts and repositories, with agents that communicate and coordinate like a real engineering team.

The vision is not a single monolithic AI, but a **distributed swarm of specialized agents** that:
- Respect security boundaries (enterprise/personal)
- Coordinate via standardized protocols (Beads + Mail)
- Operate transparently (all state in git)
- Remain under human oversight (via the unified TUI)

AllBeads transforms agents from isolated scripts into a **coherent, self-organizing workforce** that spans the polyrepo frontier.

#### Works cited

1. Beads: Memory for Your Coding Agents - Emergent Minds | paddo.dev, accessed January 9, 2026, [https://paddo.dev/blog/beads-memory-for-coding-agents/](https://paddo.dev/blog/beads-memory-for-coding-agents/)  
2. Henry's Clinical Diagnosis and Management by Laboratory Methods. 22nd Edition. ISBN 1437709745, 978-1437709742 | PDF | Pathology | Doctor Of Medicine - Scribd, accessed January 9, 2026, [https://www.scribd.com/document/757028318/Henry-s-Clinical-Diagnosis-and-Management-by-Laboratory-Methods-22nd-Edition-ISBN-1437709745-978-1437709742](https://www.scribd.com/document/757028318/Henry-s-Clinical-Diagnosis-and-Management-by-Laboratory-Methods-22nd-Edition-ISBN-1437709745-978-1437709742)  
3. steveyegge/beads - A memory upgrade for your coding agent - GitHub, accessed January 9, 2026, [https://github.com/steveyegge/beads](https://github.com/steveyegge/beads)  
4. Conductor - Today on Mac, accessed January 9, 2026, [https://todayonmac.com/conductor/](https://todayonmac.com/conductor/)  
5. Conductor: Welcome, accessed January 9, 2026, [https://docs.conductor.build/](https://docs.conductor.build/)  
6. Your first workspace - Conductor, accessed January 9, 2026, [https://docs.conductor.build/first-workspace](https://docs.conductor.build/first-workspace)  
7. How do you handle your deployments in a multi-repo architecture? : r/devops - Reddit, accessed January 9, 2026, [https://www.reddit.com/r/devops/comments/1icd9ol/how_do_you_handle_your_deployments_in_a_multirepo/](https://www.reddit.com/r/devops/comments/1icd9ol/how_do_you_handle_your_deployments_in_a_multirepo/)  
8. The Beads Revolution: How I Built The TODO System That AI Agents Actually Want to Use, accessed January 9, 2026, [https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9](https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9)  
9. steveyegge/gastown: Gas Town - multi-agent workspace ... - GitHub, accessed January 9, 2026, [https://github.com/steveyegge/gastown](https://github.com/steveyegge/gastown)  
10. README.md - steveyegge/gastown - GitHub, accessed January 9, 2026, [https://github.com/steveyegge/gastown/blob/main/README.md](https://github.com/steveyegge/gastown/blob/main/README.md)  
11. accessed January 9, 2026, [https://gerrit.googlesource.com/git-repo/+/HEAD/docs/manifest-format.md#:~:text=A%20repo%20manifest%20describes%20the,in%20the%20top%20level%20directory.](https://gerrit.googlesource.com/git-repo/+/HEAD/docs/manifest-format.md#:~:text=A%20repo%20manifest%20describes%20the,in%20the%20top%20level%20directory.)  
12. How to manage multiple repositories using myrepos tool - Github-Gist, accessed January 9, 2026, [https://gist.github.com/rmi1974/9e06453f1db1b9327933ea5510a97522](https://gist.github.com/rmi1974/9e06453f1db1b9327933ea5510a97522)  
13. andygrunwald/go-jira: Go client library for Atlassian Jira - GitHub, accessed January 9, 2026, [https://github.com/andygrunwald/go-jira](https://github.com/andygrunwald/go-jira)  
14. shurcooL/githubv4: Package githubv4 is a client library for accessing GitHub GraphQL API v4 (https://docs.github.com/en/graphql). - GitHub, accessed January 9, 2026, [https://github.com/shurcooL/githubv4](https://github.com/shurcooL/githubv4)  
15. charmbracelet/bubbletea: A powerful little TUI framework - GitHub, accessed January 9, 2026, [https://github.com/charmbracelet/bubbletea](https://github.com/charmbracelet/bubbletea)  
16. mintoolkit/kubecon-eu-2024-terminal-ui - GitHub, accessed January 9, 2026, [https://github.com/mintoolkit/kubecon-eu-2024-terminal-ui](https://github.com/mintoolkit/kubecon-eu-2024-terminal-ui)  
17. charmbracelet/lipgloss: Style definitions for nice terminal layouts - GitHub, accessed January 9, 2026, [https://github.com/charmbracelet/lipgloss](https://github.com/charmbracelet/lipgloss)