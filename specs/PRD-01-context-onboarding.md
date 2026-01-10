# PRD-01: Context Onboarding & Distributed Configuration

## Overview

This specification defines the enhanced context management system for AllBeads, enabling batch onboarding of folders, progressive status tracking ("Dry" to "Wet"), interactive setup wizards, and distributed configuration via git.

## Goals

1. **Batch Onboarding**: Enable users to onboard multiple folders at once (`ab context add ~/Workspace/*`)
2. **Progressive Status**: Track folders through stages from uninitialized ("Dry") to fully integrated ("Wet")
3. **Interactive Setup**: Guide users through configuration decisions with an intuitive CLI UX
4. **Separation of Concerns**: Individual beads repos remain unaware of AllBeads
5. **Distributed Config**: Share configuration across machines via git (repo or gist)

## Status Model: Dry to Wet

### Status Levels

| Level | Name | Description |
|-------|------|-------------|
| 0 | `dry` | Folder exists, no git or beads |
| 1 | `git` | Git repository initialized |
| 2 | `beads` | Beads initialized (`.beads/` exists) |
| 3 | `configured` | AllBeads config applied (prefix, persona, etc.) |
| 4 | `wet` | Fully integrated (syncing, hooks active) |

### Status Indicators

```
ab context list

Context: work (synced 2m ago)
┌────────────────────────────────────────────────────────────────┐
│ Folder                    │ Status     │ Prefix │ Issues │ Sync │
├───────────────────────────┼────────────┼────────┼────────┼──────┤
│ ~/Workspace/auth-service  │ 🌊 wet     │ auth   │ 12     │ ✓    │
│ ~/Workspace/api-gateway   │ 💧 config  │ api    │ 8      │ ✓    │
│ ~/Workspace/frontend      │ 🫧 beads   │ -      │ 3      │ -    │
│ ~/Workspace/docs          │ 📦 git     │ -      │ -      │ -    │
│ ~/Workspace/scripts       │ 🏜️ dry     │ -      │ -      │ -    │
└────────────────────────────────────────────────────────────────┘

Legend: 🏜️ dry → 📦 git → 🫧 beads → 💧 configured → 🌊 wet
```

## Commands

### `ab context add <paths...>`

Add folders to the context for tracking and potential onboarding.

```bash
# Add a single folder
ab context add ~/Workspace/my-project

# Add multiple folders via glob
ab context add ~/Workspace/*

# Add with initial configuration
ab context add ~/Workspace/auth --prefix=auth --persona=security

# Add and immediately start interactive setup
ab context add ~/Workspace/new-project --setup
```

**Behavior:**
- Scans each path to determine current status
- Adds to context configuration
- Does NOT automatically initialize git/beads
- Reports status summary after add

### `ab context list`

Display all tracked folders with their status.

```bash
# Basic list
ab context list

# Filter by status
ab context list --status=dry
ab context list --status=wet

# JSON output for scripting
ab context list --json

# Show detailed info
ab context list --verbose
```

### `ab context setup <path>`

Interactive setup wizard for a specific folder.

```bash
ab context setup ~/Workspace/my-project
```

**Interactive Flow:**

```
Setting up: ~/Workspace/my-project

Current Status: 🏜️ dry (no git repository)

Step 1/5: Git Repository
  This folder is not a git repository.
  ? Initialize git? [Y/n] y
  ? Default branch name: [main]
  ✓ Initialized git repository

Step 2/5: Beads Issue Tracker
  ? Initialize beads? [Y/n] y

  Beads Configuration:
  ? Issue prefix: [proj] auth
  ? Use SQLite database? [Y/n] n
  ? Use JSONL-only mode? [y/N] y
  ? Enable sync branch? [Y/n] y
  ? Sync branch name: [beads-sync]
  ✓ Initialized beads with prefix 'auth'

Step 3/5: Language & Project Type
  Detected: TypeScript, React
  ? Is this a monorepo? [y/N] n
  ? Primary language: [typescript]
  ✓ Configuration saved

Step 4/5: Agent Integration
  ? Agent persona:
    > General
      Security Specialist
      Frontend Developer
      Backend Developer
      DevOps Engineer
      Custom...
  ? Initialize Claude Code? [Y/n] y
  ✓ Created CLAUDE.md

Step 5/5: AllBeads Integration
  ? Enable automatic sync? [Y/n] y
  ? Sync interval: [5m]
  ✓ Configuration complete

Summary:
  Status: 🏜️ dry → 🌊 wet
  Prefix: auth
  Persona: security-specialist
  Sync: enabled (5m interval)
```

### `ab context remove <path>`

Remove a folder from tracking (does not delete files).

```bash
ab context remove ~/Workspace/old-project

# Remove and optionally clean up AllBeads config
ab context remove ~/Workspace/old-project --clean
```

### `ab context sync`

Synchronize all tracked folders.

```bash
# Sync all wet contexts
ab context sync

# Sync specific folder
ab context sync ~/Workspace/auth

# Dry run - show what would sync
ab context sync --dry-run
```

### `ab context promote <path>`

Advance a folder to the next status level.

```bash
# Promote to next level with interactive prompts
ab context promote ~/Workspace/my-project

# Promote to specific level
ab context promote ~/Workspace/my-project --to=wet

# Non-interactive with defaults
ab context promote ~/Workspace/my-project --yes
```

## Beads Installation Flavors

AllBeads supports multiple beads configurations to accommodate different use cases:

### Standard (Default)
- SQLite database for queries
- JSONL files for git-sync
- Full feature set

```bash
ab context setup ~/project --beads-mode=standard
```

### JSONL-Only
- No SQLite database
- Lighter weight, simpler
- Good for small projects

```bash
ab context setup ~/project --beads-mode=jsonl
```

### Custom Prefix
- Non-standard prefix for namespacing
- Useful when aggregating across repos

```bash
ab context setup ~/project --prefix=auth
```

### Sync Branch
- Dedicated branch for beads data
- Keeps issues separate from code commits
- Cleaner git history

```bash
ab context setup ~/project --sync-branch=beads-sync
```

### Daemon Mode
- Background sync daemon
- Real-time updates
- Higher resource usage

```bash
ab context setup ~/project --daemon
```

## Language Detection

AllBeads automatically detects project languages and frameworks to provide contextual suggestions.

### Detection Sources

1. **File Extensions**: `.rs`, `.ts`, `.py`, `.go`, etc.
2. **Config Files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`
3. **Framework Markers**: `next.config.js`, `vite.config.ts`, `Dockerfile`

### Language-Specific Defaults

| Language | Default Prefix | Suggested Persona |
|----------|---------------|-------------------|
| Rust | `rs` | Backend Developer |
| TypeScript | `ts` | Frontend Developer |
| Python | `py` | General |
| Go | `go` | Backend Developer |
| Security-related | - | Security Specialist |

## Monorepo Support

AllBeads handles monorepos with multiple packages/services.

### Detection

```bash
ab context add ~/Workspace/monorepo

Detected monorepo structure:
├── packages/
│   ├── frontend/    (TypeScript, React)
│   ├── backend/     (TypeScript, Node)
│   └── shared/      (TypeScript)
└── services/
    ├── auth/        (Go)
    └── api/         (Go)

? How would you like to configure this?
  > Single context (one prefix for all)
    Multiple contexts (separate prefix per package)
    Skip sub-packages
```

### Multi-Context Mode

```yaml
# .allbeads/contexts.yaml
contexts:
  - path: packages/frontend
    prefix: fe
    persona: frontend-developer
  - path: packages/backend
    prefix: be
    persona: backend-developer
  - path: services/auth
    prefix: auth
    persona: security-specialist
```

## Git Worktree Support

AllBeads supports multiple worktrees for the same repository.

### Worktree Detection

```bash
ab context add ~/Workspace/myproject-feature

Detected: Git worktree
  Main worktree: ~/Workspace/myproject
  This worktree: ~/Workspace/myproject-feature (branch: feature/new-auth)

? Link to main worktree context? [Y/n] y
? Share beads database? [Y/n] y

✓ Linked to main worktree
  Beads will be shared across worktrees
```

### Worktree Configuration

```yaml
# In main worktree .allbeads/config.yaml
worktrees:
  shared_beads: true
  link_config: true
```

## Distributed Configuration

AllBeads configuration can be stored and synced via git.

### Configuration Repository

```bash
# Initialize config repo
ab config init --remote=git@github.com:user/allbeads-config.git

# Or use a gist
ab config init --gist=abc123def456

# Sync config across machines
ab config pull
ab config push
```

### Config Structure

```
~/.config/allbeads/
├── config.yaml           # Global settings
├── contexts/
│   ├── work.yaml         # Work context
│   └── personal.yaml     # Personal context
├── templates/
│   ├── rust-project.yaml # Template for Rust projects
│   └── ts-monorepo.yaml  # Template for TS monorepos
└── .git/                 # Git repo for sync
```

### Context Configuration File

```yaml
# contexts/work.yaml
name: work
description: "Work projects"

defaults:
  sync_interval: 5m
  daemon: true
  persona: general

folders:
  - path: ~/Workspace/auth-service
    prefix: auth
    persona: security-specialist
    status: wet
    beads_mode: standard

  - path: ~/Workspace/api-gateway
    prefix: api
    status: configured
    beads_mode: jsonl

  - path: ~/Workspace/frontend
    status: beads
    # Not yet configured, just tracking

integrations:
  jira:
    url: https://company.atlassian.net
    project: PROJ
  github:
    org: mycompany
```

### Template System

```bash
# Create template from existing project
ab template create rust-service --from=~/Workspace/auth-service

# Apply template to new project
ab context add ~/Workspace/new-service --template=rust-service
```

### Template File

```yaml
# templates/rust-service.yaml
name: rust-service
description: "Standard Rust microservice setup"

init:
  git: true
  beads: true
  claude: true

beads:
  mode: standard
  sync_branch: beads-sync

config:
  persona: backend-developer

files:
  - source: CLAUDE.md.template
    dest: CLAUDE.md
  - source: .cargo/config.toml.template
    dest: .cargo/config.toml
```

## Separation of Concerns

A key design principle is that individual beads repositories remain unaware of AllBeads.

### What Goes Where

| Component | Location | Knows About |
|-----------|----------|-------------|
| Beads issues | `.beads/` in each repo | Nothing (standalone) |
| AllBeads config | `~/.config/allbeads/` | All contexts, folders |
| Per-repo hints | `.allbeads.yaml` in repo | Optional, minimal |

### Per-Repo Hints File (Optional)

A minimal file that helps AllBeads but doesn't create dependency:

```yaml
# .allbeads.yaml (optional, in repo root)
# This file provides hints to AllBeads but beads works without it

prefix_hint: auth
persona_hint: security-specialist
monorepo: false
```

### No AllBeads Lock-in

- Beads works standalone without AllBeads
- Removing AllBeads leaves beads fully functional
- `.allbeads.yaml` is optional and ignorable

## CLI UX Guidelines

### Progress Indicators

```
ab context setup ~/Workspace/project

[1/5] ████████████████████░░░░░░░░░░ Initializing git...
```

### Confirmation Prompts

```
? This will initialize git in ~/Workspace/project. Continue? [Y/n]
```

### Multi-Select

```
? Select features to enable:
  ◉ Git repository
  ◉ Beads issue tracker
  ◯ Claude Code integration
  ◉ Automatic sync
```

### Error Handling

```
✗ Failed to initialize git: Permission denied

  Suggestions:
  • Check folder permissions: ls -la ~/Workspace/project
  • Try running with sudo (not recommended)
  • Contact your administrator

  [R]etry  [S]kip  [A]bort
```

## Implementation Phases

### Phase 1: Core Context Management
- `ab context add/remove/list`
- Status detection (dry/git/beads)
- Basic configuration storage

### Phase 2: Interactive Setup
- `ab context setup` wizard
- Language detection
- Beads initialization

### Phase 3: Advanced Configuration
- Monorepo support
- Worktree support
- Templates

### Phase 4: Distributed Config
- Git-based config sync
- Gist support
- Cross-machine synchronization

### Phase 5: Polish
- Rich CLI UX
- Error recovery
- Documentation

## Data Structures

### Context

```rust
pub struct Context {
    pub name: String,
    pub folders: Vec<TrackedFolder>,
    pub defaults: ContextDefaults,
    pub integrations: Integrations,
}

pub struct TrackedFolder {
    pub path: PathBuf,
    pub status: FolderStatus,
    pub config: Option<FolderConfig>,
    pub detected: DetectedInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderStatus {
    Dry,        // No git
    Git,        // Git initialized
    Beads,      // Beads initialized
    Configured, // AllBeads config applied
    Wet,        // Fully integrated
}

pub struct FolderConfig {
    pub prefix: Option<String>,
    pub persona: Option<AgentPersona>,
    pub beads_mode: BeadsMode,
    pub sync_enabled: bool,
    pub sync_interval: Duration,
}

pub struct DetectedInfo {
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub is_monorepo: bool,
    pub is_worktree: bool,
    pub main_worktree: Option<PathBuf>,
}
```

### BeadsMode

```rust
pub enum BeadsMode {
    Standard,     // SQLite + JSONL
    JsonlOnly,    // JSONL only, no SQLite
    SyncBranch {  // Dedicated sync branch
        branch: String,
    },
    Daemon {      // Background daemon
        interval: Duration,
    },
}
```

## Migration Path

For users with existing beads repos:

```bash
# Scan and import existing beads repos
ab context scan ~/Workspace

Found 5 repositories with beads:
  ~/Workspace/auth-service    (prefix: auth, 12 issues)
  ~/Workspace/api-gateway     (prefix: api, 8 issues)
  ~/Workspace/frontend        (prefix: fe, 15 issues)
  ~/Workspace/backend         (prefix: be, 23 issues)
  ~/Workspace/infra           (prefix: ops, 5 issues)

? Import all to context 'work'? [Y/n] y

✓ Imported 5 repositories
  Status: All at 'beads' level
  Run 'ab context promote --all' to configure and wet them
```

## Success Metrics

1. **Onboarding Time**: < 2 minutes for single project setup
2. **Batch Onboarding**: < 30 seconds per project in batch mode
3. **Config Sync**: < 5 seconds to sync config across machines
4. **Zero Lock-in**: Beads works identically with or without AllBeads

## Open Questions

1. Should `ab` be a separate binary or subcommand of `allbeads`?
2. How to handle conflicts when syncing config from multiple machines?
3. Should templates support inheritance?
4. How to handle private vs public repos in config sync?

## References

- [PRD-00: Boss Repository Architecture](./PRD-00.md)
- [Beads Issue Tracker](https://github.com/anthropics/beads)
- [Claude Code Marketplace](https://claude.ai/code/marketplace)
