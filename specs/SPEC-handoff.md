# SPEC: Agent Handoff

**Status:** Draft
**Author:** Claude Opus 4.5 + thrashr888
**Date:** 2026-01-15
**Feature:** `ab handoff` - Hand off beads to AI agents

---

## Executive Summary

The `ab handoff` command ("valet launch queue") enables **handing off** beads to AI agents. This is fire-and-forget delegation, not orchestration - we launch agents with context and let them work independently.

**Philosophy: Hand-off, Not Ownership**
- We prepare context and launch agents
- We don't wait, poll, or manage their lifecycle
- For web agents, we record the task URL and move on
- Agents are responsible for their own completion

**Key Capabilities:**
- Launch agents (CLI or web-based) for specific beads
- Auto-detect available agents, remember user preferences per repo
- Auto-mark beads as `in_progress` on launch
- Record web agent task URLs in bead metadata
- Lightweight Agent Mail for local agent coordination (not a product)

---

## Motivation / Background

### The Problem

Currently, developers must manually:
1. Find work to do (`bd ready`)
2. Mark issue as in progress (`bd update <id> --status=in_progress`)
3. Switch to the correct repo context
4. Launch an AI agent (Claude Code, Cursor, etc.)
5. Provide context about the issue to the agent
6. Coordinate between multiple agents working in parallel
7. Track which agents are working on what

This manual overhead limits parallel productivity and introduces coordination errors.

### The Opportunity

AllBeads already has:
- Multi-context support with git worktrees
- Agent detection (`src/governance/agents.rs`)
- Agent Mail protocol (LOCK, UNLOCK, NOTIFY, etc.)
- Cross-repo dependency tracking

We can leverage these to automate agent lifecycle management.

### Goals

1. **Zero-friction agent launch**: `ab handoff ab-xyz` spins up an agent with full context
2. **Parallel execution**: Manage N agents across N repos simultaneously
3. **Automatic coordination**: Agents communicate via Agent Mail
4. **Multi-agent support**: Work with any CLI or web-based agent
5. **Hierarchical agents**: Parent agents spawn sub-agents for subtasks

### Non-Goals

- **Building orchestration**: We don't compete with Conductor.build, OpenAI Swarm, or other orchestration platforms
- **Agent lifecycle management**: We hand off work, we don't babysit agents
- **Polling/waiting on web agents**: Fire and forget - record the task URL and move on
- **Agent Mail as a product**: It's lightweight glue for local coordination, not a feature to market
- **Supporting every agent day one**: Start with Claude Code, add others incrementally

---

## Agent Landscape

### CLI Agents (Terminal-Native)

| Agent | Launch Command | Config | Notes |
|-------|---------------|--------|-------|
| Claude Code | `claude "prompt"` | `.claude/`, `CLAUDE.md` | Primary target, rich integration |
| OpenCode | `opencode --prompt "prompt"` | `opencode.json` | [Open source](https://opencode.ai/docs/cli/), multi-provider |
| Codex (OpenAI) | `codex "prompt"` | `.codex/` | OpenAI's terminal agent |
| Gemini CLI | `gemini -p "prompt"` | `.gemini/` | Google's terminal agent |
| Aider | `aider --message "prompt"` | `.aider.conf.yml` | Terminal-native |
| Cody | `cody chat "prompt"` | `.cody/` | Sourcegraph's agent |

### IDE-Based Agents (Launch IDE with prompt)

| Agent | Launch Command | Config | Notes |
|-------|---------------|--------|-------|
| Cursor | `cursor-agent chat "prompt"` | `.cursor/`, `.cursorrules` | [Cursor CLI](https://cursor.com/docs/cli/overview) |
| Kiro (AWS) | `kiro chat "prompt"` | `.kiro/` | AWS's VSCode-based agent |
| Antigravity | `antigravity chat "prompt"` | `.agent/` | Google's VSCode-based agent |
| VSCode | `code chat "prompt"` | `.vscode/`, `.agent/` | Microsoft Copilot agent mode |

**Your installed agents**: claude, gemini, codex, opencode, cursor-agent, kiro, antigravity, code

### Web Agents (Cloud Execution)

| Agent | URL | API | Notes |
|-------|-----|-----|-------|
| [Jules (Google)](https://jules.google.com/) | jules.google.com | Public API | Async, VM-based, auto-PRs |
| [Codex (ChatGPT)](https://chatgpt.com/features/codex/) | chatgpt.com/codex | Via ChatGPT API | Multi-agent, sandboxed |
| [Claude.ai Code](https://claude.ai/code) | claude.ai/code | Via Claude API? | Web-based Claude Code |
| Cursor Cloud | cursor.com | Proprietary | Cloud-hosted Cursor |

### Orchestration Platforms (Not Competing)

| Platform | Focus | Notes |
|----------|-------|-------|
| [Conductor.build](https://conductor.build/) | Multi-agent Claude Code | They do orchestration, we do hand-off |

**Our position**: We're a task launcher, not an orchestrator. Users who need sophisticated multi-agent orchestration should use Conductor or similar tools.

---

## Proposal

### Command: `ab handoff`

**VLQ = Valet Launch Queue** - manages the agent launch queue.

```bash
# Launch agent for a single bead
ab handoff ab-xyz

# Launch agents for all ready beads (parallel)
ab handoff --ready

# Launch with specific agent
ab handoff ab-xyz --agent claude
ab handoff ab-xyz --agent aider
ab handoff ab-xyz --agent jules  # Web agent

# Launch in background
ab handoff ab-xyz --background

# List running agents
ab handoff --list

# Stop an agent
ab handoff --stop ab-xyz

# Send message to agent
ab handoff --message ab-xyz "Consider using async/await"
```

### Core Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│                        ab handoff ab-xyz                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────────────┐
        │ 1. Load bead metadata                        │
        │    - Title, description, context             │
        │    - Dependencies (what's blocking/blocked)  │
        │    - Associated repo path                    │
        └─────────────────────────────────────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────────────┐
        │ 2. Update bead status                        │
        │    - Set status = in_progress                │
        │    - Set assigned_agent = <agent-id>         │
        │    - Record start_time                       │
        └─────────────────────────────────────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────────────┐
        │ 3. Prepare agent context                     │
        │    - Generate prompt with bead details       │
        │    - Include dependency context              │
        │    - Set environment variables               │
        │    - Create/update git worktree if needed    │
        └─────────────────────────────────────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────────────┐
        │ 4. Launch agent                              │
        │    - CLI: spawn process with prompt          │
        │    - Web: POST to API, track task ID         │
        │    - Register with Postmaster                │
        └─────────────────────────────────────────────┘
                              │
                              ▼
        ┌─────────────────────────────────────────────┐
        │ 5. Monitor and coordinate                    │
        │    - Watch for completion signals            │
        │    - Handle Agent Mail messages              │
        │    - Update bead on completion/failure       │
        └─────────────────────────────────────────────┘
```

---

## Technical Design

### Data Model Extensions

**Philosophy**: Store enough to link back to where work was handed off. Don't track lifecycle.

```rust
// New fields on Bead (in .beads/issues/<id>.jsonl)
struct Bead {
    // ... existing fields ...

    /// Agent handoff info (if handed off to an agent)
    handoff: Option<AgentHandoff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentHandoff {
    /// Agent type (claude, aider, jules, etc.)
    agent_type: AgentType,

    /// When handed off
    handed_off_at: DateTime<Utc>,

    /// For web agents: URL to check status (Jules task, Codex task, etc.)
    /// This is the key field - lets user click through to see progress
    task_url: Option<String>,

    /// For CLI agents: which repo/worktree was used
    workdir: Option<PathBuf>,

    /// Brief note about the handoff
    note: Option<String>,
}
```

**Example bead after handoff**:
```json
{
  "id": "ab-xyz",
  "title": "Fix authentication bug",
  "status": "in_progress",
  "handoff": {
    "agent_type": "gemini",
    "handed_off_at": "2026-01-15T10:30:00Z",
    "task_url": "https://jules.google.com/tasks/abc123",
    "note": "Handed off via ab handoff"
  }
}
```

### Agent Launchers

```rust
// src/vlq/launchers/mod.rs

pub trait AgentLauncher: Send + Sync {
    /// Agent type this launcher handles
    fn agent_type(&self) -> AgentType;

    /// Check if agent is available
    fn is_available(&self) -> bool;

    /// Launch agent for a bead
    async fn launch(&self, ctx: &LaunchContext) -> Result<AgentHandle>;

    /// Send message to running agent
    async fn send_message(&self, handle: &AgentHandle, message: &str) -> Result<()>;

    /// Check agent status
    async fn status(&self, handle: &AgentHandle) -> Result<AgentStatus>;

    /// Stop agent
    async fn stop(&self, handle: &AgentHandle) -> Result<()>;
}

#[derive(Debug)]
pub struct LaunchContext {
    /// The bead to work on
    pub bead: Bead,

    /// Working directory
    pub workdir: PathBuf,

    /// Initial prompt/context
    pub prompt: String,

    /// Environment variables to set
    pub env: HashMap<String, String>,

    /// Run in background
    pub background: bool,

    /// Allow sub-agent spawning
    pub allow_subagents: bool,
}

#[derive(Debug)]
pub struct AgentHandle {
    pub agent_type: AgentType,
    pub agent_id: String,
    pub pid: Option<u32>,
    pub task_id: Option<String>,
}

#[derive(Debug)]
pub enum AgentStatus {
    Running,
    Completed { result: String },
    Failed { error: String },
    WaitingForInput,
    Unknown,
}
```

### CLI Agent Launcher (Claude Code)

```rust
// src/vlq/launchers/claude.rs

pub struct ClaudeCodeLauncher;

impl AgentLauncher for ClaudeCodeLauncher {
    fn agent_type(&self) -> AgentType {
        AgentType::Claude
    }

    fn is_available(&self) -> bool {
        Command::new("claude").arg("--version").output().is_ok()
    }

    async fn launch(&self, ctx: &LaunchContext) -> Result<AgentHandle> {
        // Set environment
        let mut env = ctx.env.clone();
        env.insert("AB_ACTIVE_BEAD".to_string(), ctx.bead.id.to_string());
        env.insert("AB_AGENT_SESSION".to_string(), uuid::Uuid::new_v4().to_string());

        // Build prompt
        let prompt = format!(
            "You are working on bead {}.\n\n## Title\n{}\n\n## Description\n{}\n\n## Context\n{}",
            ctx.bead.id,
            ctx.bead.title,
            ctx.bead.description.as_deref().unwrap_or("No description"),
            ctx.prompt
        );

        // Launch claude with prompt
        let child = if ctx.background {
            Command::new("claude")
                .arg("--print")
                .arg(&prompt)
                .current_dir(&ctx.workdir)
                .envs(&env)
                .spawn()?
        } else {
            // Interactive mode - use exec to replace process
            Command::new("claude")
                .arg("--prompt")
                .arg(&prompt)
                .current_dir(&ctx.workdir)
                .envs(&env)
                .spawn()?
        };

        Ok(AgentHandle {
            agent_type: AgentType::Claude,
            agent_id: format!("claude-{}", child.id()),
            pid: Some(child.id()),
            task_id: None,
        })
    }

    // ... other methods
}
```

### Web Agent Launcher (Jules) - Hand-off Model

**Philosophy**: Launch the task, record the URL, move on. No polling or status tracking.

```rust
// src/vlq/launchers/jules.rs

pub struct JulesLauncher;

impl JulesLauncher {
    /// Check if Jules CLI is available (preferred over API)
    fn has_cli() -> bool {
        Command::new("jules").arg("--version").output().is_ok()
    }

    /// Launch via CLI (preferred)
    async fn launch_via_cli(&self, ctx: &LaunchContext) -> Result<String> {
        // Use jules CLI - it handles auth via browser
        let output = Command::new("jules")
            .args(["task", "create"])
            .arg("--repo").arg(get_github_remote(&ctx.workdir)?)
            .arg("--description").arg(&format!(
                "Work on bead {}: {}\n\n{}",
                ctx.bead.id, ctx.bead.title,
                ctx.bead.description.as_deref().unwrap_or("")
            ))
            .output()?;

        // Parse task URL from output
        let stdout = String::from_utf8_lossy(&output.stdout);
        extract_jules_task_url(&stdout)
    }

    /// Launch via deep-link URL (fallback)
    fn launch_via_url(&self, ctx: &LaunchContext) -> Result<String> {
        let repo_url = get_github_remote(&ctx.workdir)?;
        let description = urlencoding::encode(&format!(
            "Bead {}: {}", ctx.bead.id, ctx.bead.title
        ));

        // Open browser with pre-filled task
        let url = format!(
            "https://jules.google.com/new?repo={}&description={}",
            urlencoding::encode(&repo_url),
            description
        );

        open::that(&url)?;
        Ok(url) // Return the URL we opened
    }
}

impl AgentLauncher for JulesLauncher {
    async fn launch(&self, ctx: &LaunchContext) -> Result<HandoffResult> {
        let task_url = if Self::has_cli() {
            self.launch_via_cli(ctx).await?
        } else {
            self.launch_via_url(ctx)?
        };

        // That's it - we're done. Record the URL and move on.
        Ok(HandoffResult {
            agent_type: AgentType::Gemini,
            task_url: Some(task_url),
            message: "Task handed off to Jules. Check the URL for status.".to_string(),
        })
    }

    // No status() method - we don't track web agents
    // No stop() method - we don't manage web agents
}
```

**Key difference from CLI agents**: We return immediately after launch with just a URL. The user checks Jules directly for status.

### Agent Detection & Selection

On first use in a repo, detect available agents and prompt user to choose.

```rust
// src/vlq/detection.rs

/// Detect which CLI agents are installed (run checks in parallel)
pub async fn detect_installed_agents() -> Vec<AgentType> {
    let checks = vec![
        // Terminal-native
        tokio::spawn(async { check_agent("claude", AgentType::Claude) }),
        tokio::spawn(async { check_agent("opencode", AgentType::OpenCode) }),
        tokio::spawn(async { check_agent("codex", AgentType::Codex) }),
        tokio::spawn(async { check_agent("gemini", AgentType::Gemini) }),
        tokio::spawn(async { check_agent("aider", AgentType::Aider) }),
        tokio::spawn(async { check_agent("cody", AgentType::Cody) }),
        // IDE-based
        tokio::spawn(async { check_agent("cursor-agent", AgentType::Cursor) }),
        tokio::spawn(async { check_agent("kiro", AgentType::Kiro) }),
        tokio::spawn(async { check_agent("antigravity", AgentType::GenericAgent) }),
        tokio::spawn(async { check_agent("code", AgentType::Copilot) }),
    ];

    let results = futures::future::join_all(checks).await;
    results.into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect()
}

fn check_agent(cmd: &str, agent_type: AgentType) -> Option<AgentType> {
    Command::new(cmd).arg("--version").output().ok()
        .filter(|o| o.status.success())
        .map(|_| agent_type)
}

/// Prompt user to select preferred agent for this repo
pub fn prompt_agent_selection(available: &[AgentType]) -> Result<AgentType> {
    use dialoguer::Select;

    let items: Vec<String> = available.iter().map(|a| a.name().to_string()).collect();

    let selection = Select::new()
        .with_prompt("Which agent should handle tasks in this repo?")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(available[selection])
}

/// Save agent preference to repo config
pub fn save_agent_preference(repo_path: &Path, agent: AgentType) -> Result<()> {
    let config_path = repo_path.join(".beads/config.yaml");
    // ... save to config
}
```

### Hand-off Flow

```rust
// src/vlq/handoff.rs

/// Hand off a bead to an agent
pub async fn handoff_bead(
    bead_id: &BeadId,
    agent_type: Option<AgentType>,
) -> Result<HandoffResult> {
    // 1. Load bead
    let mut bead = load_bead(bead_id)?;
    let repo_path = bead.context_path()?;

    // 2. Determine agent (specified, saved preference, or prompt)
    let agent_type = match agent_type {
        Some(a) => a,
        None => get_or_prompt_agent_preference(&repo_path).await?,
    };

    // 3. Get appropriate launcher
    let launcher = get_launcher(agent_type)?;

    // 4. Build context for agent
    let ctx = LaunchContext {
        bead: bead.clone(),
        workdir: repo_path.clone(),
        prompt: build_agent_prompt(&bead),
        env: build_env_vars(&bead),
    };

    // 5. Launch agent
    let result = launcher.launch(&ctx).await?;

    // 6. Update bead with handoff info
    bead.status = Status::InProgress;
    bead.handoff = Some(AgentHandoff {
        agent_type,
        handed_off_at: Utc::now(),
        task_url: result.task_url.clone(),
        workdir: Some(repo_path),
        note: Some(format!("Handed off via ab handoff")),
    });
    save_bead(&bead)?;

    // 7. Done - we don't wait or track
    Ok(result)
}
```

### Worktree Management

```rust
// src/vlq/worktrees.rs

/// Manage git worktrees for parallel agent execution
pub struct WorktreeManager {
    /// Base repo path
    base_path: PathBuf,

    /// Worktree directory
    worktrees_dir: PathBuf,
}

impl WorktreeManager {
    /// Create or get worktree for a bead
    pub fn get_or_create_worktree(&self, bead: &Bead) -> Result<PathBuf> {
        let worktree_name = format!("ab-{}", bead.id);
        let worktree_path = self.worktrees_dir.join(&worktree_name);

        if worktree_path.exists() {
            return Ok(worktree_path);
        }

        // Create new worktree on a branch for this bead
        let branch_name = format!("ab/{}", bead.id);

        let repo = git2::Repository::open(&self.base_path)?;

        // Create branch if it doesn't exist
        if repo.find_branch(&branch_name, git2::BranchType::Local).is_err() {
            let head = repo.head()?.peel_to_commit()?;
            repo.branch(&branch_name, &head, false)?;
        }

        // Create worktree
        Command::new("git")
            .args(["worktree", "add", "-b", &branch_name, worktree_path.to_str().unwrap()])
            .current_dir(&self.base_path)
            .output()?;

        Ok(worktree_path)
    }

    /// Clean up worktree after agent completes
    pub fn cleanup_worktree(&self, bead: &Bead) -> Result<()> {
        let worktree_name = format!("ab-{}", bead.id);
        let worktree_path = self.worktrees_dir.join(&worktree_name);

        if worktree_path.exists() {
            Command::new("git")
                .args(["worktree", "remove", worktree_path.to_str().unwrap()])
                .current_dir(&self.base_path)
                .output()?;
        }

        Ok(())
    }
}
```

### Sub-Agent Support

```rust
// src/vlq/subagents.rs

/// Protocol for parent-child agent relationships
pub struct SubAgentManager {
    registry: Arc<Mutex<AgentRegistry>>,
    postmaster: Postmaster,
}

impl SubAgentManager {
    /// Called by parent agent to spawn sub-agent
    pub async fn spawn_subagent(
        &self,
        parent_handle: &AgentHandle,
        subtask: SubTask,
    ) -> Result<AgentHandle> {
        // Create child bead for subtask
        let child_bead = Bead {
            id: BeadId::new_child(&subtask.parent_bead_id),
            title: subtask.title,
            description: Some(subtask.description),
            status: Status::Open,
            priority: Priority::P2,
            parent: Some(subtask.parent_bead_id.clone()),
            // ... other fields
        };

        // Launch sub-agent
        let mut registry = self.registry.lock().await;
        let handle = registry.launch_for_bead(&child_bead, subtask.agent_type).await?;

        // Notify parent via Agent Mail
        self.postmaster.send(Message {
            from: format!("{}@localhost", handle.agent_id).parse()?,
            to: format!("{}@localhost", parent_handle.agent_id).parse()?,
            message_type: MessageType::Notify(Notification {
                message: format!("Sub-agent spawned for: {}", child_bead.title),
            }),
        }).await?;

        Ok(handle)
    }

    /// Wait for sub-agents to complete
    pub async fn wait_for_subagents(&self, parent_bead_id: &BeadId) -> Result<Vec<AgentResult>> {
        let registry = self.registry.lock().await;
        let mut results = Vec::new();

        // Find all child agents
        for (bead_id, handle) in registry.agents.iter() {
            if bead_id.is_child_of(parent_bead_id) {
                let launcher = registry.get_launcher(handle.agent_type)?;

                // Poll until complete
                loop {
                    match launcher.status(handle).await? {
                        AgentStatus::Completed { result } => {
                            results.push(AgentResult::Success(result));
                            break;
                        }
                        AgentStatus::Failed { error } => {
                            results.push(AgentResult::Failure(error));
                            break;
                        }
                        _ => tokio::time::sleep(Duration::from_secs(5)).await,
                    }
                }
            }
        }

        Ok(results)
    }
}
```

---

## Agent Mail Integration

### Message Types for VLQ

```rust
// Extend existing Agent Mail protocol

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VlqMessage {
    /// Agent started working on bead
    AgentStarted {
        bead_id: BeadId,
        agent_type: AgentType,
        agent_id: String,
    },

    /// Agent completed work
    AgentCompleted {
        bead_id: BeadId,
        agent_id: String,
        commits: Vec<String>,
        pr_url: Option<String>,
    },

    /// Agent needs input/decision
    AgentBlocked {
        bead_id: BeadId,
        agent_id: String,
        reason: String,
        options: Vec<String>,
    },

    /// Agent spawned sub-agent
    SubAgentSpawned {
        parent_bead_id: BeadId,
        child_bead_id: BeadId,
        child_agent_id: String,
    },

    /// Request to spawn sub-agent
    SpawnSubAgent {
        parent_bead_id: BeadId,
        subtask_title: String,
        subtask_description: String,
        preferred_agent: Option<AgentType>,
    },

    /// Cross-agent coordination
    CoordinationRequest {
        from_bead: BeadId,
        to_bead: BeadId,
        message: String,
    },
}
```

### Sheriff Integration

The Sheriff daemon monitors VLQ messages and updates bead states:

```rust
// src/sheriff/vlq_handler.rs

impl Sheriff {
    async fn handle_vlq_messages(&mut self) -> Result<()> {
        let messages = self.postmaster.list_vlq_messages().await?;

        for msg in messages {
            match msg {
                VlqMessage::AgentCompleted { bead_id, commits, pr_url, .. } => {
                    // Update bead status
                    self.update_bead(&bead_id, |bead| {
                        bead.status = Status::InReview; // or Closed if no PR
                        bead.pr_url = pr_url;
                        bead.commits.extend(commits);
                    }).await?;
                }

                VlqMessage::AgentBlocked { bead_id, reason, .. } => {
                    self.update_bead(&bead_id, |bead| {
                        bead.status = Status::Blocked;
                        bead.blocked_reason = Some(reason);
                    }).await?;
                }

                // ... handle other message types
            }
        }

        Ok(())
    }
}
```

---

## CLI Interface

### Primary Commands

```bash
# Launch agent for specific bead
ab handoff <bead-id> [OPTIONS]

Options:
  --agent <type>     Agent to use (claude, aider, jules, codex, gemini)
  --background       Run in background
  --worktree         Create dedicated git worktree
  --no-prompt        Skip interactive prompt generation
  --prompt <file>    Use custom prompt file
  --env <KEY=VALUE>  Set environment variable

# Launch agents for all ready beads
ab handoff --ready [OPTIONS]

Options:
  --max <N>          Maximum concurrent agents (default: 3)
  --agent <type>     Agent type for all
  --sequential       Run one at a time

# List running agents
ab handoff --list

Output:
BEAD       AGENT         STATUS      STARTED         WORKDIR
ab-xyz     claude-12345  running     5m ago          ~/project/.worktrees/ab-xyz
ab-abc     jules-task-1  completed   1h ago          (cloud)
ab-def     aider-67890   blocked     30m ago         ~/project/.worktrees/ab-def

# Check specific agent
ab handoff --status <bead-id>

# Stop agent
ab handoff --stop <bead-id>

# Send message to agent
ab handoff --message <bead-id> "Consider using the existing helper function"

# Spawn sub-agent (from within agent context)
ab handoff --spawn-child --title "Add unit tests" --parent ab-xyz
```

### Environment Variables

```bash
# Set by VLQ when launching agents
AB_ACTIVE_BEAD=ab-xyz           # Current bead ID
AB_AGENT_SESSION=<uuid>         # Session identifier
AB_PARENT_AGENT=claude-12345    # Parent agent (if sub-agent)
AB_ALLOW_SUBAGENTS=true         # Can spawn sub-agents
AB_POSTMASTER_URL=localhost:7878  # Agent Mail endpoint
```

---

## Integration Points

### With Existing AllBeads Components

1. **Beads CLI (`bd`)**: VLQ reads bead metadata, updates status
2. **Sheriff Daemon**: Monitors VLQ messages, syncs state
3. **Agent Mail**: Communication backbone for coordination
4. **TUI**: New "Agents" view showing active agents
5. **Governance**: Policies for which agents can work where

### With External Systems

1. **GitHub**: Auto-create PRs, link to beads
2. **JIRA**: Sync agent status to external tickets
3. **Conductor.build**: Potential orchestrator integration

### With Web Agents

1. **Jules (Google)**: REST API for task submission/monitoring
2. **Codex (OpenAI)**: Via ChatGPT API or dedicated endpoint
3. **Claude.ai/code**: If API becomes available

---

## Phased Implementation

### Phase 1: Claude Code Hand-off (MVP) ✅
- [x] `ab handoff <bead-id>` launches Claude Code with bead context
- [x] Auto-update bead status to `in_progress`
- [x] Set `AB_ACTIVE_BEAD` environment variable
- [x] AgentHandoff struct defined (agent type, timestamp, workdir, note)
- [x] Handoff module with AgentType enum (11+ agents)
- [x] `--dry-run` flag for testing
- [x] Store handoff info in bead (comment + label)

### Phase 2: Agent Selection ✅
- [x] Detect installed CLI agents (`ab handoff --agents`)
- [x] Allow `--agent` override per command
- [x] Support Aider, OpenCode, Codex, Gemini, Cursor, and more
- [x] Prompt user to select preferred agent (first use)
- [x] Save preference to `.beads/config.yaml`

### Phase 3: Web Agent Hand-off (Jules) ✅
- [x] Jules CLI integration (`jules new "prompt"`)
- [x] Jules URL deep-link fallback (if CLI not installed)
- [x] ChatGPT Codex URL deep-link fallback
- [x] Store task URL in bead (as comment)
- [x] No polling - fire and forget (design decision)

### Phase 4: Worktrees & Bulk Operations ✅
- [x] `ab handoff --ready` shows unblocked beads
- [x] `ab handoff --list` shows handed-off beads
- [x] Optional worktree creation for isolation (`--worktree`)
- [x] `ab show` displays handoff info and task URL

**Deferred / Out of Scope:**
- Agent lifecycle management (not doing this)
- Status polling for web agents (not doing this)
- Orchestration features (use Conductor.build instead)
- TUI agents view (maybe later, low priority)

---

## Security Considerations

1. **Credential Handling**: API keys via environment or first-use prompt, not stored in beads
2. **Worktree Isolation**: Optional isolated worktrees for parallel work
3. **No Sensitive Data in Prompts**: Bead descriptions should not contain secrets

---

## Success Metrics

1. **Hand-off latency**: < 3 seconds from `ab handoff` to agent prompt appearing (CLI) or browser opening (web)
2. **Context quality**: Agents receive complete bead context (title, description, dependencies)
3. **Status tracking**: Beads show handoff info including task URLs for web agents
4. **User friction**: First-time agent selection takes < 30 seconds
5. **Incremental adoption**: Users can start with Claude Code only, add agents later

---

## Design Decisions (Resolved)

### Q1: Default agent selection ✅

**Decision**: Detect installed agents in parallel, prompt user to choose, save preference to repo config.

**Implementation**:
1. On first `ab handoff` in a repo, detect all installed CLI agents in parallel
2. Present picker: "Which agent should handle tasks in this repo?"
3. Save selection to `.beads/config.yaml` or similar
4. Allow per-command override: `ab handoff ab-xyz --agent aider`
5. Different task types can use different agents (user's choice at runtime)

### Q2: Web agent authentication ✅

**Decision**: Flexible per-agent, prioritize simplicity.

**Approaches (in order of preference)**:
1. **URL with query params**: If agent supports deep-linking (check Jules docs)
2. **CLI equivalent**: Use `jules-cli` if available instead of raw API
3. **API key from env**: `JULES_API_KEY`, `CODEX_API_KEY`
4. **First-use prompt**: Ask once, save to AllBeads config

**References**:
- Jules CLI: https://jules.google/docs/cli/reference
- Jules API: https://jules.google/docs/api/reference/
- Jules Tasks: https://jules.google/docs/running-tasks/

**Note**: Each web agent may need different handling. Don't over-engineer - add support incrementally.

### Q3: Sub-agent limits ✅

**Decision**: Follow best practices per agent, default to max depth of 2.

**Rationale**: Different agents handle sub-agents differently. Claude Code has its own sub-agent patterns. We defer to each agent's conventions rather than imposing our own limits.

### Q4: Orchestration ✅

**Decision**: Do not build orchestration. Stay focused on core competency.

**Rationale**:
- We're a task launcher and issue tracker, not an orchestrator
- Conductor.build and others are well-funded, established players
- Agent Mail is lightweight glue, not a product to market
- Our value is in cross-repo context preparation, not agent lifecycle management

---

## Future Considerations

### AllBeads Web App Integration

When the AllBeads web app is available, it unlocks smoother integrations:

- **OAuth-based web agent auth**: Formal integrations with Jules, Codex, etc. - no API keys to manage
- **Hosted Agent Mail**: Cloud-hosted Postmaster for cross-agent coordination without local daemon
- **Web-to-web hand-offs**: Click a button in AllBeads web → task created in Jules/Codex
- **Credential management**: Secure storage of API keys and OAuth tokens
- **Callback URLs**: Web agents can notify AllBeads when tasks complete (webhook-style)

This makes the current CLI-first approach the foundation - we design for eventual web integration without depending on it.

### Plugin/Skill Distribution

**Question**: Should AllBeads plugins/skills be installed into other agents' ecosystems?

**Option A: Project-level config (current approach)**
- Beads config lives in `.beads/` in the repo
- Each agent reads its own config (`.cursorrules`, `CLAUDE.md`, etc.)
- AllBeads generates/syncs these configs
- **Pro**: Works everywhere, no marketplace dependency
- **Con**: Fragmented, need to maintain multiple formats

**Option B: Agent marketplaces**
- Publish AllBeads as a plugin/skill in each agent's marketplace
- Cursor extensions, Claude plugins, VSCode extensions, etc.
- **Pro**: Native integration, discoverable
- **Con**: Maintenance burden, approval processes, feature parity

**Option C: Hybrid**
- Core beads functionality via project config
- Enhanced features via marketplace plugins where valuable
- Example: VSCode extension for TUI, but CLI works standalone

**Recommendation**: Start with Option A, evaluate Option C for high-value integrations later.

### Other Ideas (Lower Priority)
- **Agent templates**: Pre-built prompts for common task types (bug fix, feature, refactor)
- **Cost tracking**: Monitor API usage for web agents
- **Team dashboards**: See who handed off what to which agent

---

## References

### CLI Agents
- [Claude Code](https://claude.ai/code) - Anthropic's terminal agent
- [Cursor Agent CLI](https://cursor.com/docs/cli/overview) - Cursor's terminal agent
- [OpenCode](https://opencode.ai/docs/cli/) - Open source multi-provider agent
- [OpenAI Codex CLI](https://github.com/openai/codex) - OpenAI's terminal agent
- [Gemini CLI](https://ai.google.dev/gemini-api/docs) - Google's terminal agent

### Web Agents
- [Google Jules](https://jules.google.com/) - Async AI coding agent ([CLI docs](https://jules.google/docs/cli/reference), [API docs](https://jules.google/docs/api/reference/))
- [ChatGPT Codex](https://chatgpt.com/features/codex/) - Cloud-based software engineering agent

### Orchestration (Not Competing)
- [Conductor.build](https://conductor.build/) - Multi-agent orchestration for Mac

### AllBeads
- Agent Detection: `src/governance/agents.rs`
- Agent Mail: `src/mail/`
- Related: SPEC-aiki-integration.md
