# The AllBeads Protocol: A Unified Architecture for Multi-Project AI Agent Orchestration and the "Boss" Meta-Repository

## 1. Introduction: The Crisis of Context in Autonomous Engineering

The paradigm of software engineering is undergoing a seismic shift, moving from a human-centric model of solitary creation to an agent-centric model of orchestration. In this emerging era, the primary bottleneck to productivity is no longer the generation of syntax or the algorithmic reasoning capabilities of Large Language Models (LLMs) like Claude 3.5 Sonnet or GPT-4o. Rather, the binding constraint is **context management**—specifically, the persistence of state, intent, and dependency across disjointed compute sessions and distributed repositories.

For decades, the Integrated Development Environment (IDE) served as the cockpit for the human developer, who maintained the high-level architecture, the dependency graph, and the execution plan within their own biological working memory. As we delegate increasing autonomy to AI agents, we confront the "Catastrophic Amnesia" problem.1 An AI agent, upon restarting a session, is effectively a new entity, tabula rasa, devoid of the historical nuance that guided previous decisions. Without a persistent external memory that is structured, queryable, and tightly coupled to the codebase, agents are condemned to relive the same discovery processes repeatedly, akin to the protagonist in the film *50 First Dates*.1

This report serves as a comprehensive Product Requirements Document (PRD) and Request for Comments (RFC) for the "AllBeads" initiative. It analyzes the current landscape of agent orchestration—specifically the unstructured "Prose" paradigm exemplified by prose.md and the parallelized execution environment of conductor.build. It contrasts these with the structured, graph-theoretic approach of the "Beads" ecosystem pioneered by Steve Yegge. Finally, it proposes the novel **"Boss" Repository Architecture**: a meta-orchestration layer designed to aggregate distributed work graphs, visualize cross-repository dependencies via a terminal-based interface, and synchronize state with legacy enterprise systems like JIRA and GitHub Issues.

The objective is to define a system where the "Manager" agent (The Mayor) and the "Executive" agent (The Boss) can collaborate across fifty distinct microservices without losing the strategic thread, transforming the AI from a mere coding assistant into a coherent, self-driving engineering organization.

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

## 4. The AllBeads PRD: The "Boss" Repository Architecture

We have established the limitations of Conductor (repo-bound) and the strengths of Beads (structured, git-backed). We now define the **AllBeads** architecture: a system to scale Beads from a single repository to an enterprise-grade "Boss" environment.

### 4.1 Problem Statement: The Polyrepo Coordination Failure

Consider a modern microservices architecture comprising 50 distinct repositories: auth-service, payment-gateway, frontend-web, frontend-mobile, analytics-worker, etc..7  
In the current Gas Town model, a human "Overseer" must manually install each Rig, instantiate a Mayor for each, and mentally track that "Task A in Auth" blocks "Task B in Frontend." There is no "God View."  
The requirement is a **Meta-Repository**—a "Boss" repo—that aggregates the state of all member Rigs, creates a unified dependency graph, and synchronizes this state with the human management layer (JIRA/GitHub).

### 4.2 The "Boss" Concept: A Strategic Meta-Kernel

The "Boss" is not a Monorepo containing all code. It is a **Control Plane**. It functions similarly to the Google repo tool's manifest repository 11 or the myrepos (mr) configuration 12, but with active, agentic intelligence layered on top.

#### 4.2.1 Directory Structure of the Boss Repo

The Boss repository must adhere to a strict schema to enable automated tooling and agent discovery.

| Path | Description |
| :---- | :---- |
| /.boss/config.yaml | Global configuration, API keys (encrypted), and policy definitions. |
| /.boss/graph/ | The aggregated SQLite/JSONL database of the federated graph. |
| /manifests/default.xml | The definition of member Rigs (URLs, branches, paths). |
| /beads/shadow/ | A directory containing "Shadow Beads"—pointers to external Rig beads. |
| /agents/executive.md | The high-level "Constitution" for the Executive Agent. |
| /dashboard/ | Source code for the TUI visualization tool. |

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

The **Sheriff** is a long-running background process, written in Go, that enforces consistency across the federated graph. It is the "glue" of the AllBeads architecture.

**Technical Specification:**

* **Language:** Go (Golang) 1.22+.  
* **Concurrency:** Heavy usage of goroutines for parallel polling of Rigs and external APIs.  
* **Event Loop:**  
  1. **Poll Phase:** Iterate through manifests/default.xml. For each Rig, run git fetch origin refs/beads/*.  
  2. **Diff Phase:** Compare the local .beads state of the Rig with the cached state in the Boss Graph.  
  3. **Sync Phase:** If a Rig has new Epics, create Shadow Beads in Boss. If Boss has new directives (e.g., a "Global Mandate"), push new Beads to the Rig's .beads directory.  
  4. **External Sync Phase:** Push/Pull changes to JIRA and GitHub Issues (detailed in Section 5).

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

## 5. Enterprise Integration: Bridging the Gap with JIRA and GitHub

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

## 6. Visualizing the Swarm: The "Boss Board" TUI

A command-line tool (bd list) is insufficient for visualizing a graph spanning 50 repositories. We require a high-density information display. We propose the **Boss Board**, a Terminal User Interface (TUI) built with the Go bubbletea framework.15

### 6.1 Why TUI?

A TUI is preferred over a Web UI for this persona because:

* It runs over SSH, allowing management of remote development boxes.  
* It is "close to the metal," integrating directly with the local shell and git commands.  
* It requires no browser context switching.

### 6.2 Component Architecture with Bubble Tea

The bubbletea architecture follows The Elm Architecture (Model, View, Update), which is ideal for managing the complex state of a multi-repo dashboard.

#### 6.2.1 The Data Model

```go
type Model struct {  
    Graph       *beads.Graph      // The aggregated federated graph  
    Rigs       RigStatus       // List of rigs and their health (clean/dirty/ahead/behind)  
    Selection   CursorPosition    // User's focus  
    Viewport    viewport.Model    // For scrolling long logs  
    ActiveTab   int               // Kanban vs Graph vs Log  
}
```

#### 6.2.2 Visualization Views (Lipgloss Styles)

View 1: The Strategic Kanban  
Using lipgloss 17 for layout, we create a multi-column view representing the aggregation of all Rigs.

| To Do (Global) | In Progress (By Rig) | Blocked (Dependencies) | Done |
| :---- | :---- | :---- | :---- |
| **Auth:** JWT Refactor | **Auth:** Update Token (Mayor-1) | **Frontend:** Profile Page | **DB:** Schema Mig |
| **Pay:** Add Stripe | **Pay:** Webhook List (Mayor-2) | *(Blocked by Auth: Token)* |  |

View 2: The Dependency Web  
This is the most critical view for the Boss. We render a node-link diagram using ASCII/Braille characters.  

```
── blocks ──▶  
▲ │  
│ │  
related blocks  
│ ▼  
◀── blocks ──  
```

This visualization allows the human architect to instantly identify bottlenecks (e.g., "The Legacy-DB refactor is blocking Analytics, which is blocking the Q3 Report").  

View 3: The Agent Pulse  
A sidebar component showing the real-time status of the swarm.

* 🟢 **Mayor-Auth (PID 8821):** "Generating unit tests for user_model.go"  
* 🟡 **Mayor-Frontend (PID 8845):** "Waiting for lint check..."  
* 🔴 **Mayor-Pay (PID 8890):** "Error: API Rate Limit Exceeded"

### 6.3 Implementation Details

The TUI will run as a client connected to the Sheriff daemon via a local gRPC socket or by watching the SQLite file (using WAL mode for concurrent access). This separation ensures the UI remains responsive even if the Sheriff is performing heavy git network operations.

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

## 8. Conclusion and Strategic Roadmap

The transition from human-centric to agent-centric software engineering requires a fundamental re-evaluation of our tooling infrastructure. Tools like conductor.build have successfully demonstrated the power of parallelized, worktree-isolated agents, but they remain constrained by the lack of a persistent, cross-repository memory layer.

The **AllBeads** architecture, anchored by the **Boss Repository**, fills this void. By treating the organization's entire codebase not as a collection of text files but as a **Federated Graph of Intent**, we enable AI agents to operate with the strategic context of a Staff Engineer.

### 8.1 Comparative Summary

| Feature | Prose.md (Current) | Conductor (Current) | AllBeads "Boss" (Proposed) |
| :---- | :---- | :---- | :---- |
| **Primary Context** | Unstructured Text | Ephemeral Worktree | Git-Backed Graph (JSONL) |
| **Scope** | Single Session | Single Repo | Multi-Repo / Enterprise |
| **State Persistence** | Low (Lost on Reset) | Medium (Repo-Bound) | High (Federated & Synced) |
| **Conflict Resolution** | Manual Merge | Manual Review | Hash-Based / CRDT |
| **Visualization** | Text Editor | Diff Viewer | Strategic TUI / Kanban |
| **JIRA Integration** | Manual Copy-Paste | None | Bi-Directional Daemon |
| **Agent Autonomy** | Junior Developer | Individual Contributor | Engineering Manager |

### 8.2 Recommendations for Implementation

1. **Standardize the Manifest:** Adopt the XML schema defined in Section 4.4 immediately to allow for Rig discovery.  
2. **Build the Sheriff First:** The go-jira and githubv4 synchronization daemon is the critical path. Without it, the system remains an island.  
3. **Deploy the TUI:** Trust is the currency of autonomy. The "Boss Board" is essential for humans to feel comfortable delegating control to the swarm.

By implementing the AllBeads architecture, organizations can move beyond the "Chatbot" phase of AI and enter the era of the **Self-Driving Organization**, where the software builds, tests, and repairs itself under the high-level strategic guidance of its human creators.

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