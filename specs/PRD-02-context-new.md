# PRD-02: GitHub Repository Creation (ab context new)

## Overview

Add the ability to create brand new GitHub repositories directly from AllBeads, following a guided flow similar to GitHub's new repository form. This streamlines the process of starting new projects with AllBeads configured from day one.

## Problem Statement

Currently, to start a new AllBeads-managed project:
1. Go to GitHub web UI and create repository
2. Clone the repository locally
3. Run `ab onboard .` to initialize beads and configure agents
4. Run `ab context add .` to add to AllBeads

This multi-step process requires context switching between browser and terminal. The `ab context new` command consolidates this into a single guided workflow.

## User Stories

1. As a developer, I want to create a new GitHub repository and have it fully configured for AllBeads in one command
2. As a team lead, I want to quickly scaffold new projects with consistent agent configurations
3. As an AI agent, I want to spawn new repositories for subtasks without human intervention

## Proposed Solution

### Command: `ab context new`

```bash
# Interactive mode (recommended)
ab context new

# With arguments
ab context new myproject --private --description "My new project"

# From template
ab context new myproject --template rust-cli

# Full specification
ab context new myproject \
  --private \
  --description "Description" \
  --template rust-cli \
  --license MIT \
  --gitignore Rust \
  --init-beads \
  --init-agents claude,cursor
```

### Interactive Prompt Form

When run without arguments, presents a GitHub-like form:

```
┌─────────────────────────────────────────────────────────────┐
│  Create a new repository                                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Repository name: [_________________]                        │
│                                                              │
│  Description (optional): [_________________]                 │
│                                                              │
│  Visibility:                                                 │
│    (●) Public  ○ Private                                     │
│                                                              │
│  Initialize with:                                            │
│    [x] README.md                                             │
│    [x] .gitignore: [Rust      ▾]                            │
│    [x] LICENSE: [MIT         ▾]                             │
│                                                              │
│  AllBeads Configuration:                                     │
│    [x] Initialize beads (.beads/)                            │
│    [x] Configure AI agents                                   │
│        [x] Claude Code (CLAUDE.md)                          │
│        [x] Cursor (.cursorrules)                            │
│        [ ] GitHub Copilot                                    │
│        [ ] Aider                                              │
│                                                              │
│  Template (optional): [None         ▾]                       │
│    Available: rust-cli, rust-lib, node-ts, python-cli       │
│                                                              │
│  Local path: ~/Workspace/[repo-name]                         │
│                                                              │
│  [Create Repository]  [Cancel]                               │
└─────────────────────────────────────────────────────────────┘
```

### Workflow Stages

1. **Input Collection** - Gather repository configuration via prompts or CLI args
2. **Validation** - Check name availability, permissions
3. **GitHub Creation** - Create repository via GitHub API
4. **Clone** - Clone to local workspace
5. **Template Application** - Apply project template if specified
6. **Beads Init** - Run `bd init` with appropriate prefix
7. **Agent Configuration** - Create CLAUDE.md, .cursorrules, etc.
8. **Git Setup** - Initial commit with all configurations
9. **Context Registration** - Add to AllBeads config

### API Requirements

Uses GitHub REST API v3:
- `POST /user/repos` - Create repository
- `GET /gitignore/templates` - List .gitignore templates
- `GET /licenses` - List available licenses

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | required | Repository name |
| `--description` | string | "" | Repository description |
| `--private` | flag | false | Create private repository |
| `--public` | flag | true | Create public repository |
| `--template` | string | none | AllBeads project template to apply |
| `--license` | string | none | License (MIT, Apache-2.0, GPL-3.0, etc.) |
| `--gitignore` | string | auto | .gitignore template (Rust, Node, Python, etc.) |
| `--readme` | flag | true | Initialize with README.md |
| `--init-beads` | flag | true | Run bd init |
| `--init-agents` | string | "claude" | Comma-separated agent list |
| `--path` | string | workspace/name | Local clone path |
| `--org` | string | none | Create in organization (requires permissions) |
| `--no-clone` | flag | false | Create repo but don't clone locally |
| `--wizard` | flag | false | Use interactive TUI wizard |
| `--non-interactive` | flag | false | Use defaults, no prompts |

### Error Handling

| Error | Resolution |
|-------|------------|
| Repository name taken | Suggest alternatives or prompt for new name |
| No GitHub token | Direct to `gh auth login` or token setup |
| Insufficient permissions | Show required scopes |
| Template not found | List available templates |
| Clone path exists | Ask to overwrite or choose new path |

### Integration with Existing Commands

- **Complements `ab onboard`**: `onboard` is for existing repos, `context new` is for brand new repos
- **Uses `ab folder template`**: Can apply existing templates during creation
- **Calls `bd init`**: Standard beads initialization
- **Adds to config**: Same as `ab context add`

## Implementation Plan

### Phase 1: Core Creation (ab-5pf1)
- Add `ContextCommands::New` variant
- Implement GitHub API repo creation
- Basic clone and bd init

### Phase 2: Prompt Form (ab-43b)
- Interactive prompts for repository options
- Validation and error messages
- Progress display during creation

### Phase 3: Template Integration
- Connect to `ab folder template` system
- Support custom templates
- Language-specific defaults

### Phase 4: Skills Command (ab-6x7)
- Create `/context-new` skill for Claude Code
- Enable AI agents to create repos programmatically

## Success Metrics

- Time to create new project reduced from ~5 minutes to <30 seconds
- 100% of new repos have beads configured from start
- Agent configuration consistent across all new projects

## Dependencies

- GitHub REST API access (token with repo scope)
- `bd` CLI for beads initialization
- Existing template system (`ab folder template`)

## Non-Goals

- GitLab/Bitbucket support (future enhancement)
- Repository migration (use `ab onboard` for existing repos)
- Team/organization management
