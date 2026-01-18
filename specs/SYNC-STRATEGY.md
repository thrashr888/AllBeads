# AllBeads Local/Remote Data Sync Strategy

## Overview

AllBeads uses a **hybrid sync model** where different data types follow different sync strategies based on their nature and usage patterns.

## Data Classification

### Local-Only Data (Git-Native)

These data types are managed entirely in git and should NOT sync to the web database:

| Data Type | Location | Sync Method | Rationale |
|-----------|----------|-------------|-----------|
| **Beads/Issues** | `.beads/*.jsonl` | Git push/pull | Git is source of truth; web imports for read-only dashboard |
| **File Locks** | `mail.db` (local) | Real-time via mail | Locks are ephemeral, time-sensitive |
| **SQLite Cache** | `cache.db` | None | Performance cache only |

**Why git-native for beads?**
- Beads are tightly coupled to code changes
- Dependencies reference specific commits
- Team sync via git branches works well
- No conflict with external systems (JIRA, GitHub Issues)

### Cloud-Synced Data

These data types sync between CLI and web:

| Data Type | CLI Location | Web Location | Sync Direction | Implementation |
|-----------|--------------|--------------|----------------|----------------|
| **Contexts** | `config.yaml` | `Repository` | Bi-directional | `ab context sync push/pull` |
| **Agent Mail** | `mail.db` | `AgentMail` | CLI → Web | `POST /api/mail` |
| **User Prefs** | `config.yaml` | `User.settings` | Future | Not implemented |

### Web-Only Data

These exist only in the web platform:

| Data Type | Purpose |
|-----------|---------|
| **Organizations** | Team management, billing |
| **Projects** | Grouping repos, milestones |
| **Milestones** | Release planning |
| **Integrations** | GitHub App, JIRA connections |
| **Governance Policies** | Compliance rules |

## Sync Implementation Status

### Implemented

1. **Context Sync** (`ab context sync`)
   - `push`: Upload local contexts to web
   - `pull`: Download contexts from web (preserves local paths)
   - Preserves local-only fields (paths, integrations)

2. **Agent Mail** (`ab mail`)
   - When authenticated, mail sends to remote `/api/mail`
   - Inbox/unread queries remote when logged in
   - Falls back to local when offline

### Planned

3. **Beads Dashboard Import** (read-only)
   - Sheriff daemon pushes bead summaries to web
   - Web displays aggregated view across repos
   - No write-back to git (prevents conflicts)

4. **Statistics Sync**
   - Aggregate stats from CLI → web
   - Powers web dashboard analytics

## Conflict Resolution

### Contexts
- Last-write-wins with timestamp
- Local paths always preserved (not synced)
- `localPath` field ignored in web → CLI sync

### Beads
- NO sync to web database for writes
- Git merge handles conflicts
- Web import is read-only snapshot

### Mail
- CLI → Web only (no pull)
- Web is append-only log
- Read status synced back eventually

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/cli/contexts` | GET/POST | Sync contexts |
| `/api/mail` | GET/POST | Agent mail |
| `/api/orgs` | GET | List user's orgs |
| `/api/beads/import` | POST | Future: Import beads snapshot |

## Authentication

All CLI → Web sync requires:
1. `ab login` to authenticate via device code flow
2. Bearer token stored in `config.yaml` under `web_auth`
3. Token validated on each request

## Offline Behavior

When not authenticated or offline:
- Contexts: Local-only operation
- Mail: Local postmaster (SQLite)
- Beads: Normal git-based workflow

## Future Considerations

### Real-time Sync (Not Planned)
- WebSocket connection for instant updates
- Would require daemon always running
- Complexity not justified for current use case

### Selective Bead Sync
- Option to sync specific beads to web
- For cross-team visibility
- Would need conflict handling

### Comment Sync
- Web comments → git `.beads/comments.jsonl`
- Requires careful merge strategy

## Summary

| Data | Source of Truth | Sync Model |
|------|-----------------|------------|
| Beads | Git | None (git push/pull) |
| Contexts | Hybrid | Bi-directional |
| Mail | Web (when logged in) | CLI → Web |
| Config | Local | None |
| Orgs/Projects | Web | Read-only |
