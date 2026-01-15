# Changelog

All notable changes to AllBeads will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-01-14

### Added

- **Aiki Integration (Phases 1-3)**: Foundation for linking AllBeads issues to Aiki provenance
  - Phase 1: Environment Variable Bridge
    - `ab aiki activate <bead-id>` sets `AB_ACTIVE_BEAD` environment variable
    - `ab aiki deactivate` clears the active bead
    - `ab aiki status` shows currently active bead
    - `ab aiki hook-init` outputs shell code for automatic env var loading
    - Shell integration via `eval "$(ab aiki hook-init)"`
  - Phase 2: Provenance Queries
    - `ab show <id> --provenance` queries Aiki for change summary
    - Displays total changes, agents, review statistics
    - Graceful fallback when Aiki is not installed
  - Phase 3: Agent Mail Integration
    - New `AikiEvent` message type for review outcomes
    - Support for review event types: `review_passed`, `review_failed`, `escalated`, `review_completed`
    - `ReviewIssue` structure with severity levels (info, warning, error, critical)
    - Aiki events displayed in TUI mail view with full detail

- **Governance Epic**: Complete governance framework for AI agent adoption
  - **Agent Detection**: Detect 14 AI agent types in repositories
    - Claude Code, GitHub Copilot, Cursor, Aider, Kiro, OpenAI Codex
    - Google Gemini, Amazon CodeWhisperer, Tabnine, Codeium
    - Sourcegraph Cody, Replit AI, JetBrains AI, Windsurf
    - Detection via config files, directories, and marker files
    - Confidence levels (High, Medium, Low) based on evidence
  - **Repository Policy Framework**: HCP Terraform-inspired enforcement
    - Enforcement levels: Advisory, SoftMandatory, HardMandatory
    - Policy checks: FileExists, FileExistsAny, OnboardingScore, AgentAllowlist, PatternAbsent
    - Exemption system with reason tracking
    - `ab governance check/status/violations/exempt/unexempt` commands
  - **GitHub Scanner**: Scan user/org repos for onboarding opportunities
    - GitHub Search API for efficient cross-repo detection (~100x faster)
    - Parallel batch processing with configurable concurrency
    - Real-time progress output during scanning
    - Onboarding priority scoring (High/Medium/Low/Skip)
    - Compare scanned repos against managed contexts
    - `ab scan user/org` commands
  - **Usage Tracking**: SQLite-based adoption metrics
    - Record agent detection history over time
    - Usage stats with adoption rate calculations
    - Daily trend aggregation
    - `ab agents track/stats` commands

- **TUI Contexts View**: Repository onboarding status dashboard
  - Multi-organization support
  - Real-time onboarding status per repository
  - GitHub Actions workflow tracking

- **Onboarding Workflow**: `ab onboard` command for guided setup
  - Multi-stage workflow (detect → configure → verify)
  - Agent-specific configuration generation
  - Beads integration setup

### Changed

- **TUI Navigation**: Stats view moved to last position in tab order
  - New order: Kanban → Mail → Graph → Timeline → Governance → Stats → Swarm

## [0.4.0] - 2026-01-12

### Added

- **TUI Stats View**: Dashboard showing project statistics with bar charts
  - Status breakdown (Open, In Progress, Blocked, Closed)
  - Priority distribution (P0-P4)
  - Beads by context with horizontal bar visualization
  - Ready beads count

- **TUI Timeline View**: Gantt-style visualization of beads
  - Chronological display sorted by creation date
  - ASCII bars showing days open per bead
  - Zoom controls (+/-) for timeline range
  - Detail view with full bead information

- **TUI Governance View**: Policy management dashboard
  - Two-panel layout (policies + check results)
  - Navigation between sections (h/l keys)
  - Detail view for policy and result inspection

- **Governance Module**: Policy Engine for compliance checking
  - Type-safe `PolicyType` enum with built-in rules
  - Built-in rules: `require-description`, `require-labels`, `max-in-progress`, `dependency-cycle-check`, `require-priority`, `require-assignee`
  - `PolicyChecker` for running policies against `FederatedGraph`
  - SQLite storage for policy configs and check results
  - 25 unit tests covering all policy rules

- **Sheriff Governance Integration**: Policy checks in daemon poll cycle
  - `SheriffEvent::PolicyChecked` for policy check notifications
  - `SheriffCommand::ReloadPolicies` and `CheckPolicies` commands
  - Policy checks run automatically at end of each poll cycle

- **Git Hooks for Proactive Policy Enforcement**:
  - `ab check` command - Run policy checks on-demand
    - `--strict` mode exits non-zero on violations
    - `--pre-commit` mode optimized for git hooks (quiet if passing)
    - `--policy=NAME` to check specific policy only
    - `--fix` to show resolution suggestions
    - `--format=json|yaml` for scripting
  - `ab hooks` command - Manage git hooks
    - `install` - Create pre-commit/commit-msg/post-commit/pre-push hooks
    - `uninstall` - Remove hooks
    - `list` - Show installed hooks
    - `test` - Test hooks without committing
    - `status` - Check installation status
  - Smart hook templates detect dev vs production mode
  - Pre-commit hook blocks commits with policy violations
  - YAML policy configuration (`.beads/policies.yaml`)
  - Example policies for AllBeads, QDOS, ethertext, rookery contexts

## [0.3.1] - 2026-01-12

### Fixed

- **P0: `ab close` not working for newly created beads** - The close command relied on finding beads in the federated graph to determine their context, but newly created beads aren't in the graph until `bd sync` exports them to `issues.jsonl`. Now extracts the prefix from the bead ID and finds the matching context by reading each context's `.beads/config.yaml` issue-prefix setting. This allows `ab close` to work immediately after `ab create` without requiring a sync.

## [0.3.0] - 2026-01-12

### Added

- **Wrapper Commands**: Full suite of bd-compatible wrapper commands that delegate to the appropriate context
  - `ab create` - Create beads in any context
  - `ab update` - Update beads across contexts
  - `ab close` - Close beads with optional reason
  - `ab dep` - Manage dependencies (add/remove)
  - `ab reopen` - Reopen closed issues
  - `ab label` - Manage labels (add/remove/list)
  - `ab comments` - View and add comments
  - `ab q` - Quick capture for fast issue creation
  - `ab epic` - Epic management (list/create/show)
  - `ab edit` - Edit issues in $EDITOR
  - `ab delete` - Delete issues
  - `ab duplicate` - Mark issues as duplicates

- **bd-Compatible Global Flags**: All flags from bd now work with ab
  - Output control: `--json`, `--quiet`, `--verbose`
  - Database/storage: `--db`, `--no-db`, `--readonly`
  - Sync behavior: `--no-auto-flush`, `--no-auto-import`, `--no-daemon`, `--sandbox`, `--allow-stale`
  - Other: `--actor`, `--lock-timeout`, `--profile`

- **CLI Integration Tests**: 26 tests that verify all commands can parse arguments correctly, catching short flag conflicts early

- **Custom Help Output**: Organized help display matching bd's grouped format with sections for Aggregator, Wrapper, Daemon, TUI, and Admin commands

- **Beads Crate Enhancements** (v0.2.0):
  - `with_workdir_and_flags()` for passing global flags
  - New methods: `reopen_multiple()`, `delete()`, `delete_multiple()`, `duplicate()`, `quick_create()`, `quick_create_full()`, `label_list()`, `epic_list()`, `epic_list_open()`, `epic_show()`, `edit()`

### Fixed

- Context filter (`-C`/`--contexts`) now works correctly when loading from cache
- Context filter strips `@` prefix (both `-C rookery` and `-C @rookery` work)
- Context filter validates against available contexts with helpful error message
- `ab ready` now sorts by priority to match `bd ready` behavior
- Short flag conflicts between global `--config` and command-specific `--context` arguments

### Changed

- Refactored all wrapper commands to use the beads crate instead of ad-hoc implementations
- Case-insensitive context name comparison

## [0.2.0] - 2026-01-10

### Added

- AllBeads Claude Code plugin with hooks for session management
- Janitor workflow for automated issue discovery from codebase analysis
- SECURITY.md with security policy and vulnerability reporting guidelines
- Beads crate (`crates/beads`) - Rust wrapper for bd CLI

### Fixed

- Janitor self-detection to avoid analyzing its own output

## [0.1.0] - 2026-01-08

### Added

- Initial release
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
- FederatedGraph for cross-repo dependency tracking

[0.5.0]: https://github.com/thrashr888/AllBeads/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/thrashr888/AllBeads/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/thrashr888/AllBeads/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/thrashr888/AllBeads/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/thrashr888/AllBeads/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/thrashr888/AllBeads/releases/tag/v0.1.0
