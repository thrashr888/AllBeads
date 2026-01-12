# Changelog

All notable changes to AllBeads will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.3.0]: https://github.com/thrashr888/AllBeads/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/thrashr888/AllBeads/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/thrashr888/AllBeads/releases/tag/v0.1.0
