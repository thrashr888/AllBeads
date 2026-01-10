# PRD-01: Context Onboarding & Distributed Configuration

## Overview

This specification defines the enhanced context management system for AllBeads, enabling batch onboarding of folders, progressive status tracking ("Dry" to "Wet"), interactive setup wizards, and distributed configuration via git.

## Goals

1. **Batch Onboarding**: Enable users to onboard multiple folders at once (`ab context add ~/Workspace/*`)
2. **Progressive Status**: Track folders through stages from uninitialized ("Dry") to fully integrated ("Wet")
3. **Interactive Setup**: Guide users through configuration decisions with an intuitive CLI UX
4. **Separation of Concerns**: Individual beads repos remain unaware of AllBeads
5. **Distributed Config**: Share configuration across machines via git (repo or gist)
6. **Plugin Ecosystem**: Extensible onboarding via Claude marketplace plugins

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

## Plugin System

AllBeads extends beyond git and beads to support a plugin ecosystem that integrates with the Claude marketplace. This enables onboarding of any tool that provides a plugin definition.

### Core Concept: Meta-Onboarder

AllBeads acts as a **meta-onboarder** that orchestrates the onboarding of multiple tools/plugins into a project. Each tool (git, beads, claude, prose, etc.) has its own onboarding process, and AllBeads knows how to invoke them through a standardized protocol.

```
┌─────────────────────────────────────────────────────────────────┐
│                         AllBeads                                 │
│                    (Meta-Onboarder)                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│   │   Git   │  │  Beads  │  │  Claude │  │  Prose  │  ...      │
│   │ (core)  │  │ (core)  │  │(plugin) │  │(plugin) │           │
│   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
│        │            │            │            │                  │
│        ▼            ▼            ▼            ▼                  │
│   ┌─────────────────────────────────────────────────┐           │
│   │           Unified Onboarding Flow               │           │
│   └─────────────────────────────────────────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Claude Marketplace Integration

AllBeads leverages the Claude plugin marketplace system for plugin discovery and metadata.

```bash
# List available marketplaces
ab marketplace list

Registered Marketplaces:
┌────────────────────────────────────────────────────────────────┐
│ Name              │ URL                                │ Plugins │
├───────────────────┼────────────────────────────────────┼─────────┤
│ claude-official   │ github.com/anthropics/marketplace  │ 12      │
│ openprose         │ github.com/openprose/prose         │ 1       │
│ local-dev         │ ~/.config/allbeads/marketplace     │ 3       │
└────────────────────────────────────────────────────────────────┘

# Add a marketplace
ab marketplace add https://github.com/openprose/prose

# Sync marketplace metadata
ab marketplace sync
```

### Plugin Discovery

Plugins are discovered through multiple mechanisms:

1. **Registered Marketplaces**: Queried via `claude plugin marketplace list`
2. **Direct URLs**: Add plugins by repository URL
3. **Project Detection**: Scan project for existing `.claude-plugin/` directories
4. **Dependency Analysis**: Detect plugins from package.json, Cargo.toml, etc.

```bash
# Search for plugins
ab plugin search "writing"

Found 3 plugins matching "writing":
  open-prose      (openprose)     - Programming language for AI sessions
  markdown-lint   (official)      - Markdown linting and formatting
  docs-generator  (community)     - Auto-generate documentation

# Show plugin details
ab plugin info open-prose

Plugin: open-prose
Version: 0.3.1
Author: OpenProse (https://github.com/openprose)
Source: https://github.com/openprose/prose

Description:
  A programming language for AI sessions - structures English into
  unambiguous control flow for multi-agent orchestration

Keywords: ai, agents, orchestration, dsl, prose, workflow

Onboarding: ✓ AllBeads-compatible
  Status Levels: dry → installed → initialized → configured
  Prerequisites: prose CLI (cargo install prose)
```

### Plugin Onboarding Protocol

Plugins that want to integrate with AllBeads provide an `allbeads-onboarding.yaml` file in their `.claude-plugin/` directory. This declarative protocol defines how AllBeads should onboard the plugin.

```yaml
# .claude-plugin/allbeads-onboarding.yaml
schema_version: "1.0"
plugin: open-prose
version: 0.3.1

# When is this plugin relevant to a project?
relevance:
  # Match by project characteristics
  languages: ["*"]           # Any language (or ["rust", "typescript"])
  frameworks: []             # Specific frameworks
  files: []                  # Required files to suggest this plugin

  # Manual trigger conditions
  always_suggest: false      # Always show in suggestions
  user_requested: true       # Only when explicitly requested

# How to detect if plugin is already onboarded
detect:
  # Check for marker files
  files:
    - path: ".prose/"
      type: directory
    - path: "prose.yaml"
      type: file

  # Check for CLI availability
  commands:
    - command: "prose --version"
      success_pattern: "prose \\d+\\.\\d+"
      capture_version: true

# Status levels (plugin's own dry→wet progression)
status_levels:
  - id: dry
    name: "Not Installed"
    description: "Prose not available"
    icon: "🏜️"

  - id: installed
    name: "CLI Available"
    description: "Prose CLI installed but not initialized"
    icon: "📦"
    detect:
      commands: ["prose --version"]

  - id: initialized
    name: "Initialized"
    description: "Project initialized with .prose/"
    icon: "🫧"
    detect:
      files: [".prose/"]

  - id: configured
    name: "Configured"
    description: "Prose fully configured"
    icon: "🌊"
    detect:
      files: ["prose.yaml"]

# Prerequisites that must be satisfied first
prerequisites:
  - name: prose-cli
    description: "Prose command-line interface"
    check:
      command: "prose --version"
      success_pattern: "prose"
    install:
      # Multiple installation methods (user picks or auto-detect)
      cargo: "prose"
      brew: "prose"
      npm: "@openprose/cli"
      manual: "Visit https://prose.md/install"

# Onboarding steps (executed in order)
onboard:
  steps:
    - id: init
      name: "Initialize Prose"
      description: "Create .prose/ directory structure"
      type: command
      command: "prose init"
      cwd: "."
      skip_if:
        files: [".prose/"]

    - id: configure
      name: "Configure Prose"
      description: "Set up prose.yaml configuration"
      type: interactive
      prompts:
        - key: style
          question: "Default writing style?"
          type: select
          options:
            - value: formal
              label: "Formal"
              description: "Professional, precise language"
            - value: casual
              label: "Casual"
              description: "Conversational, friendly tone"
            - value: technical
              label: "Technical"
              description: "Documentation-focused, detailed"
          default: formal

        - key: audience
          question: "Target audience?"
          type: text
          default: "developers"
          validate: "^[a-zA-Z0-9\\s,]+$"

        - key: features
          question: "Enable features?"
          type: multiselect
          options:
            - value: spellcheck
              label: "Spell checking"
              default: true
            - value: grammar
              label: "Grammar suggestions"
              default: true
            - value: tone_analysis
              label: "Tone analysis"
              default: false

    - id: create_config
      name: "Create Configuration"
      description: "Generate prose.yaml from user choices"
      type: template
      template: |
        # Prose configuration
        # Generated by AllBeads onboarding

        style: {{ prompts.style }}
        audience: {{ prompts.audience }}

        features:
        {% for feature in prompts.features %}
          - {{ feature }}
        {% endfor %}

        # See https://prose.md/docs/config for more options
      dest: "prose.yaml"

    - id: claude_integration
      name: "Claude Integration"
      description: "Add prose commands to CLAUDE.md"
      type: append
      dest: "CLAUDE.md"
      content: |

        ## Prose Integration

        This project uses [Prose](https://prose.md) for structured AI communication.

        ### Commands
        - `prose run <file>` - Execute a prose script
        - `prose check` - Validate prose syntax
        - `prose fmt` - Format prose files

# Cleanup/uninstall steps (optional)
uninstall:
  steps:
    - type: confirm
      message: "Remove .prose/ directory and prose.yaml?"
    - type: delete
      paths:
        - ".prose/"
        - "prose.yaml"
    - type: edit
      file: "CLAUDE.md"
      remove_section: "## Prose Integration"
```

### Plugin Commands

```bash
# List installed plugins for current project
ab plugin list

Plugins in ~/Workspace/my-project:
┌────────────────────────────────────────────────────────────────┐
│ Plugin       │ Status      │ Version │ Source               │
├──────────────┼─────────────┼─────────┼──────────────────────┤
│ beads        │ 🌊 config   │ 0.5.0   │ (core)               │
│ claude-code  │ 🌊 config   │ 1.0.0   │ (core)               │
│ open-prose   │ 🫧 init     │ 0.3.1   │ openprose            │
│ cursor-sync  │ 📦 install  │ 0.1.0   │ community            │
└────────────────────────────────────────────────────────────────┘

# Install a plugin
ab plugin install open-prose

Installing open-prose v0.3.1...

Prerequisites:
  ✓ prose CLI (cargo install prose)
    Already installed: v0.3.1

Running onboarding steps:
  [1/4] ████████████████████ Initialize Prose
  [2/4] ████████████████████ Configure Prose (interactive)
  [3/4] ████████████████████ Create Configuration
  [4/4] ████████████████████ Claude Integration

✓ Plugin open-prose installed and configured

# Upgrade a plugin
ab plugin upgrade open-prose

# Remove a plugin
ab plugin remove open-prose --clean

# Check plugin status
ab plugin status open-prose

Plugin: open-prose
Status: 🫧 initialized (needs configuration)
Next step: Run 'ab plugin configure open-prose'
```

### Integrated Setup Flow

Plugins integrate seamlessly with the `ab context setup` wizard:

```
ab context setup ~/Workspace/my-project

Setting up: ~/Workspace/my-project

Current Status: 📦 git (Git initialized)

Step 1/4: Beads Issue Tracker
  ? Initialize beads? [Y/n] y
  ? Issue prefix: [proj]
  ✓ Initialized beads

Step 2/4: Claude Code Integration
  ? Initialize Claude Code? [Y/n] y
  ✓ Created CLAUDE.md

Step 3/4: Recommended Plugins
  Based on your project, these plugins might be useful:

  ◉ open-prose    - Programming language for AI sessions
                    Recommended: You have .md files
  ◯ docs-gen      - Auto-generate documentation
                    Recommended: Rust project detected
  ◯ test-runner   - Integrated test management

  ? Install selected plugins? [Y/n] y

  Installing open-prose...
    ? Default writing style? [Formal]
    ? Target audience: [developers]
    ✓ Installed open-prose

Step 4/4: AllBeads Integration
  ? Enable automatic sync? [Y/n] y
  ✓ Configuration complete

Summary:
  Project: ~/Workspace/my-project
  Status: 🏜️ dry → 🌊 wet
  Beads: ✓ (prefix: proj)
  Plugins: claude-code, open-prose
```

### Plugin Status in Context List

```
ab context list --verbose

Context: work (synced 2m ago)
┌──────────────────────────────────────────────────────────────────────────┐
│ Folder                   │ Status    │ Plugins                           │
├──────────────────────────┼───────────┼───────────────────────────────────┤
│ ~/Workspace/auth-service │ 🌊 wet    │ beads🌊 claude🌊 prose🌊          │
│ ~/Workspace/api-gateway  │ 💧 config │ beads🌊 claude🫧                  │
│ ~/Workspace/frontend     │ 🫧 beads  │ beads🫧                           │
│ ~/Workspace/docs         │ 📦 git    │ -                                 │
└──────────────────────────────────────────────────────────────────────────┘
```

### Creating AllBeads-Compatible Plugins

Plugin authors can make their Claude plugins AllBeads-compatible by adding the onboarding protocol:

```
my-plugin/
├── .claude-plugin/
│   ├── marketplace.json      # Claude marketplace registration
│   ├── plugin.json           # Plugin metadata
│   └── allbeads-onboarding.yaml  # ← Add this for AllBeads
├── src/
└── ...
```

**Minimal Example:**

```yaml
# .claude-plugin/allbeads-onboarding.yaml
schema_version: "1.0"
plugin: my-plugin
version: 1.0.0

detect:
  files:
    - path: ".my-plugin-config"

status_levels:
  - id: dry
    name: "Not Installed"
  - id: configured
    name: "Configured"
    detect:
      files: [".my-plugin-config"]

onboard:
  steps:
    - id: init
      name: "Initialize"
      type: command
      command: "my-plugin init"
```

### Security Model

Plugin onboarding can execute commands and modify files, so security is critical:

1. **Transparency**: All steps are shown before execution
2. **Consent**: User must approve each plugin installation
3. **Sandboxing**: Commands run in project directory only
4. **Dry Run**: Preview changes without executing

```bash
# Preview what a plugin will do
ab plugin install open-prose --dry-run

Plugin: open-prose v0.3.1

This plugin will:
  1. Run command: prose init
     In directory: ~/Workspace/my-project

  2. Create file: prose.yaml
     Content: (template, 15 lines)

  3. Append to: CLAUDE.md
     Content: (prose integration section, 12 lines)

No changes made. Run without --dry-run to install.
```

### Plugin Hooks

Plugins can define hooks that AllBeads calls at various lifecycle points:

```yaml
# In allbeads-onboarding.yaml
hooks:
  # Called when project is synced
  on_sync:
    command: "prose sync"

  # Called before commit
  pre_commit:
    command: "prose check"
    fail_on_error: true

  # Called when entering project directory
  on_enter:
    command: "prose status"
    silent: true
```

### Marketplace Protocol

For marketplace maintainers, AllBeads expects this structure:

```
marketplace-repo/
├── .claude-plugin/
│   └── marketplace.json     # Required: marketplace metadata
├── plugins/
│   ├── plugin-a/
│   │   ├── plugin.json
│   │   └── allbeads-onboarding.yaml
│   └── plugin-b/
│       ├── plugin.json
│       └── allbeads-onboarding.yaml
└── README.md
```

**marketplace.json:**

```json
{
  "name": "my-marketplace",
  "description": "Collection of AI workflow plugins",
  "owner": {
    "name": "My Org",
    "url": "https://github.com/myorg"
  },
  "plugins": [
    {
      "name": "plugin-a",
      "source": "./plugins/plugin-a",
      "description": "Does thing A",
      "allbeads_compatible": true
    },
    {
      "name": "plugin-b",
      "source": "./plugins/plugin-b",
      "description": "Does thing B",
      "allbeads_compatible": true
    }
  ]
}
```

### Plugin Recommendations Engine

AllBeads can intelligently recommend plugins based on project analysis:

```rust
pub struct PluginRecommendation {
    pub plugin: PluginInfo,
    pub reason: RecommendationReason,
    pub confidence: f32,  // 0.0 to 1.0
}

pub enum RecommendationReason {
    LanguageMatch(String),        // "Rust project detected"
    FrameworkMatch(String),       // "React project detected"
    FilePatternMatch(String),     // "Has .md files"
    DependencyMatch(String),      // "Uses tokio"
    UserHistory,                  // "You installed this in similar projects"
    Popular,                      // "Popular for this project type"
}
```

```bash
ab plugin recommend

Recommended plugins for ~/Workspace/my-project:

High confidence (>80%):
  ◉ open-prose (95%)
    Reason: Has markdown files, AI-focused project

  ◉ test-runner (85%)
    Reason: Rust project with tests/ directory

Medium confidence (50-80%):
  ◯ docs-gen (65%)
    Reason: Library crate without docs.rs setup

  ◯ ci-helper (55%)
    Reason: No CI configuration detected

? Install recommended plugins? [Select to customize]
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

### Phase 5: Plugin Foundation
- Plugin onboarding protocol parser (YAML)
- `ab plugin list/info/status` commands
- Plugin detection in projects

### Phase 6: Plugin Onboarding
- `ab plugin install/remove` with step execution
- Interactive prompts from plugin definitions
- Template rendering with Jinja-like syntax

### Phase 7: Marketplace Integration
- `ab marketplace add/list/sync` commands
- Claude marketplace discovery via `claude plugin marketplace list`
- Plugin metadata caching

### Phase 8: Plugin Recommendations
- Project analysis engine
- Recommendation scoring
- Integration with setup wizard

### Phase 9: Polish
- Rich CLI UX
- Error recovery
- Documentation
- Plugin SDK/starter template

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

### Plugin System

```rust
/// A registered marketplace source
pub struct Marketplace {
    pub name: String,
    pub url: String,
    pub plugins: Vec<PluginInfo>,
    pub last_synced: Option<DateTime<Utc>>,
}

/// Plugin metadata from marketplace
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Author,
    pub source_url: String,
    pub keywords: Vec<String>,
    pub allbeads_compatible: bool,
}

/// Plugin installation state in a project
pub struct InstalledPlugin {
    pub info: PluginInfo,
    pub status: PluginStatus,
    pub config: Option<serde_json::Value>,
    pub installed_at: DateTime<Utc>,
}

/// Plugin's own dry→wet progression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Dry,          // Not installed
    Installed,    // Prerequisites met, not initialized
    Initialized,  // Init command run
    Configured,   // Fully configured
}

/// Parsed plugin onboarding protocol
pub struct PluginOnboarding {
    pub schema_version: String,
    pub plugin: String,
    pub version: String,
    pub relevance: PluginRelevance,
    pub detect: DetectionConfig,
    pub status_levels: Vec<StatusLevel>,
    pub prerequisites: Vec<Prerequisite>,
    pub onboard: OnboardingSteps,
    pub uninstall: Option<UninstallSteps>,
    pub hooks: Option<PluginHooks>,
}

/// When should this plugin be suggested?
pub struct PluginRelevance {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub files: Vec<String>,
    pub always_suggest: bool,
    pub user_requested: bool,
}

/// How to detect if plugin is installed
pub struct DetectionConfig {
    pub files: Vec<FileDetection>,
    pub commands: Vec<CommandDetection>,
}

/// A prerequisite that must be installed
pub struct Prerequisite {
    pub name: String,
    pub description: String,
    pub check: CommandDetection,
    pub install: InstallMethods,
}

/// Multiple ways to install a prerequisite
pub struct InstallMethods {
    pub cargo: Option<String>,
    pub brew: Option<String>,
    pub npm: Option<String>,
    pub pip: Option<String>,
    pub manual: Option<String>,
}

/// Individual onboarding step
pub enum OnboardingStep {
    Command {
        id: String,
        name: String,
        description: String,
        command: String,
        cwd: Option<String>,
        skip_if: Option<DetectionConfig>,
    },
    Interactive {
        id: String,
        name: String,
        description: String,
        prompts: Vec<Prompt>,
    },
    Template {
        id: String,
        name: String,
        description: String,
        template: String,
        dest: String,
    },
    Append {
        id: String,
        name: String,
        description: String,
        dest: String,
        content: String,
    },
}

/// Interactive prompt configuration
pub struct Prompt {
    pub key: String,
    pub question: String,
    pub prompt_type: PromptType,
    pub default: Option<serde_json::Value>,
    pub validate: Option<String>,
}

pub enum PromptType {
    Text,
    Select { options: Vec<SelectOption> },
    MultiSelect { options: Vec<SelectOption> },
    Confirm,
}

/// Plugin lifecycle hooks
pub struct PluginHooks {
    pub on_sync: Option<HookConfig>,
    pub pre_commit: Option<HookConfig>,
    pub post_commit: Option<HookConfig>,
    pub on_enter: Option<HookConfig>,
    pub on_exit: Option<HookConfig>,
}

pub struct HookConfig {
    pub command: String,
    pub fail_on_error: bool,
    pub silent: bool,
}

/// Plugin recommendation with reasoning
pub struct PluginRecommendation {
    pub plugin: PluginInfo,
    pub reason: RecommendationReason,
    pub confidence: f32,
}

pub enum RecommendationReason {
    LanguageMatch(String),
    FrameworkMatch(String),
    FilePatternMatch(String),
    DependencyMatch(String),
    UserHistory,
    Popular,
    ExplicitlyRequested,
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

### Context Management
1. **Onboarding Time**: < 2 minutes for single project setup
2. **Batch Onboarding**: < 30 seconds per project in batch mode
3. **Config Sync**: < 5 seconds to sync config across machines
4. **Zero Lock-in**: Beads works identically with or without AllBeads

### Plugin System
5. **Plugin Install Time**: < 60 seconds for typical plugin (excluding prerequisite downloads)
6. **Marketplace Sync**: < 10 seconds to refresh plugin metadata
7. **Protocol Adoption**: Measure plugins with `allbeads-onboarding.yaml` in Claude marketplaces
8. **Plugin Discoverability**: Users find relevant plugins within 3 commands

## Open Questions

### Context Management
1. Should `ab` be a separate binary or subcommand of `allbeads`?
2. How to handle conflicts when syncing config from multiple machines?
3. Should templates support inheritance?
4. How to handle private vs public repos in config sync?

### Plugin System
5. Should AllBeads ship with a default marketplace, or start empty?
6. How to handle plugin version conflicts (plugin requires newer AllBeads)?
7. Should plugins be able to declare dependencies on other plugins?
8. How to sandbox plugin commands to prevent malicious operations?
9. Should there be a plugin "trust" system (verified publishers)?
10. How to handle plugins that modify the same files (e.g., CLAUDE.md)?
11. Should plugin onboarding be transactional (rollback on failure)?
12. How deep should Claude marketplace integration go? (read-only vs. bidirectional)

### Protocol Design
13. Should the onboarding protocol support conditional steps based on OS/platform?
14. Should there be a "headless" mode for CI/CD environments?
15. How to version the onboarding protocol itself?

## References

- [PRD-00: Boss Repository Architecture](./PRD-00.md)
- [Beads Issue Tracker](https://github.com/anthropics/beads)
- [Claude Code Marketplace](https://claude.ai/code/marketplace)
- [Prose - AI Session Language](https://prose.md) / [GitHub](https://github.com/openprose/prose)
- [Claude Plugin Marketplaces Documentation](https://code.claude.com/docs/en/plugin-marketplaces)
- Example Plugin Structures:
  - [Prose marketplace.json](https://github.com/openprose/prose/blob/main/.claude-plugin/marketplace.json)
  - [Prose plugin.json](https://github.com/openprose/prose/blob/main/.claude-plugin/plugin.json)
