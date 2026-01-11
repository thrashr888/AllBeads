# SPEC: AllBeads Web Platform

**Status:** Draft / Ultrathink
**Author:** Claude Opus 4.5 + thrashr888
**Date:** 2026-01-11
**Epic:** ab-???

---

## Executive Summary

AllBeads Web is a hosted platform for teams to manage beads collaboratively - similar to how GitHub hosts Git repositories. It provides a web UI for the AllBeads ecosystem: organizations, projects, teams, governance dashboards, integrations, and **milestones/releases** layered on top of beads.

**Vision:** AllBeads Web becomes the "GitHub for AI-assisted work tracking" - where distributed beads sync to a central hub for visibility, collaboration, and governance.

---

## The Problem

### Current State

AllBeads today is:
- CLI-only (`ab`, `bd` commands)
- Distributed (each repo has its own `.beads/`)
- Individual (no team collaboration features)
- Local-first (TUI dashboard, no web)

### Pain Points

1. **No Team Visibility**
   - Each developer sees only their local view
   - No shared dashboard for team leads
   - Can't see org-wide bead status

2. **No Collaboration Features**
   - Comments on beads are local
   - No @mentions or notifications
   - No shared epics across teams

3. **No Release Management**
   - Epics exist but no milestones/releases
   - Can't group beads by version/sprint
   - No release burndown or tracking

4. **Integration Management is Manual**
   - JIRA/GitHub sync configured per-repo
   - No central integration dashboard
   - No org-wide sync status

5. **Governance is CLI-Only**
   - Reports generated locally
   - No visual compliance dashboard
   - Policy management is YAML editing

---

## Solution: AllBeads Web Platform

### Core Concept

AllBeads Web is a **distribution** of AllBeads, similar to how GitHub is a distribution of Git:

```
┌────────────────────────────────────────────────────────────────┐
│                      AllBeads Ecosystem                         │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────────────┐    ┌──────────────────┐                 │
│   │  AllBeads CLI    │    │  AllBeads Web    │                 │
│   │  (distributed)   │←──→│  (centralized)   │                 │
│   │                  │sync│                  │                 │
│   │  • ab commands   │    │  • Web UI        │                 │
│   │  • bd commands   │    │  • Team collab   │                 │
│   │  • TUI dashboard │    │  • Governance    │                 │
│   │  • Local-first   │    │  • Integrations  │                 │
│   └──────────────────┘    └──────────────────┘                 │
│            ↑                        ↑                          │
│            │                        │                          │
│            ↓                        ↓                          │
│   ┌──────────────────────────────────────────┐                 │
│   │              Git Repositories             │                 │
│   │   .beads/ directories synced via git     │                 │
│   └──────────────────────────────────────────┘                 │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

**Key Principle:** Beads remain git-native (stored in `.beads/`). The web platform syncs and presents them, but source of truth is the repo.

---

## Feature Set

### 1. Organizations & Teams

```
Organization: Acme Corp
├── Projects
│   ├── Platform (repos: api, frontend, infra)
│   ├── Mobile (repos: ios-app, android-app)
│   └── Data (repos: pipeline, ml-models)
│
├── Teams
│   ├── Platform Team (8 members)
│   ├── Mobile Team (5 members)
│   └── Data Team (4 members)
│
└── Settings
    ├── Integrations (JIRA, GitHub, Slack)
    ├── Governance Policies
    └── Billing
```

**Organization Features:**
- SSO/SAML authentication
- Role-based access (Admin, Member, Viewer)
- Audit logs for all actions
- Usage analytics

**Team Features:**
- Team-scoped dashboards
- Team assignments on beads
- Team velocity metrics
- Team notifications

### 2. Project Dashboards

**Kanban View** (similar to TUI but web):

```
┌─────────────────────────────────────────────────────────────────┐
│  Platform Project                                    [+ New Bead]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Ready (12)        │  In Progress (5)  │  Blocked (3)  │ Done   │
│  ────────────────  │  ────────────────  │ ────────────  │ ────── │
│  ┌──────────────┐  │  ┌──────────────┐  │ ┌──────────┐  │        │
│  │ api-123      │  │  │ api-456 P0   │  │ │ fe-789   │  │        │
│  │ Add caching  │  │  │ Fix auth bug │  │ │ Blocked  │  │        │
│  │ @alice P2    │  │  │ @bob         │  │ │ by api-  │  │        │
│  └──────────────┘  │  └──────────────┘  │ │ 456      │  │        │
│  ┌──────────────┐  │  ┌──────────────┐  │ └──────────┘  │        │
│  │ fe-234       │  │  │ infra-567    │  │               │        │
│  │ Dark mode    │  │  │ K8s upgrade  │  │               │        │
│  │ @carol P3    │  │  │ @dave P1     │  │               │        │
│  └──────────────┘  │  └──────────────┘  │               │        │
│                    │                    │               │        │
└─────────────────────────────────────────────────────────────────┘
```

**Views Available:**
- Kanban (status columns)
- List (sortable/filterable table)
- Timeline (Gantt-style with dependencies)
- Graph (dependency visualization)

**Filters:**
- By repo/context
- By assignee/team
- By priority
- By label
- By milestone/release
- By epic

### 3. Milestones & Releases

**This is the killer feature beads lacks.** Layer release management on top of epics:

```yaml
# Milestone structure
milestone:
  id: m-2026-q1
  name: "Q1 2026 Release"
  target_date: 2026-03-31
  status: in_progress

  # Beads/epics assigned to this milestone
  beads:
    - api-epic-auth    # Epic
    - fe-epic-redesign # Epic
    - api-123          # Individual bead
    - fe-234           # Individual bead

  # Computed metrics
  metrics:
    total_beads: 47
    completed: 23
    in_progress: 12
    blocked: 4
    not_started: 8
    completion_percentage: 49%
```

**Milestone UI:**

```
┌─────────────────────────────────────────────────────────────────┐
│  Milestone: Q1 2026 Release                                      │
│  Target: March 31, 2026 (47 days remaining)                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Progress: ████████████████░░░░░░░░░░░░░░░░ 49%                 │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Burndown Chart                            ││
│  │                                                              ││
│  │  50 ┤ ·                                                      ││
│  │     │  ·  ·                                                  ││
│  │  40 ┤     ·  ·                                               ││
│  │     │         ·  ·  ← Ideal                                  ││
│  │  30 ┤    ────────·──·──────────                              ││
│  │     │              ·                                         ││
│  │  20 ┤               ·  ← Actual                              ││
│  │     │                                                        ││
│  │  10 ┤                                                        ││
│  │     │                                                        ││
│  │   0 ┼────────────────────────────────────────────────────────││
│  │     Jan 1        Feb 1        Mar 1        Mar 31            ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Epics in this Milestone:                                        │
│  ────────────────────────                                        │
│  ● api-epic-auth (75% complete) ████████████████░░░░             │
│  ● fe-epic-redesign (30% complete) ██████░░░░░░░░░░░░            │
│  ● data-epic-pipeline (100% complete) ████████████████████ ✓     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Release Features:**
- Milestone CRUD
- Bead/epic assignment to milestones
- Burndown charts
- Velocity tracking
- Release notes generation
- Version tagging integration

### 4. Collaboration Features

**Comments & Activity:**

```
┌─────────────────────────────────────────────────────────────────┐
│  api-456: Fix authentication timeout bug                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Status: In Progress    Priority: P0    Assignee: @bob          │
│  Milestone: Q1 2026     Epic: api-epic-auth                      │
│                                                                  │
│  ─────────────────────────────────────────────────────────────  │
│                                                                  │
│  Activity:                                                       │
│                                                                  │
│  @alice (2 hours ago):                                           │
│    This is blocking the release. Can we prioritize?             │
│                                                                  │
│  @bob (1 hour ago):                                              │
│    Working on it now. Root cause identified - JWT expiry        │
│    wasn't being refreshed. Fix incoming.                        │
│                                                                  │
│  [System] (45 min ago):                                          │
│    Claude Code pushed fix: src/auth/jwt.rs (+23, -5)            │
│    Aiki review: PASSED (1 iteration)                            │
│                                                                  │
│  @carol (30 min ago):                                            │
│    @bob tests passing locally, can you push to staging?         │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Add a comment...                               [Post]      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Collaboration Features:**
- Threaded comments on beads
- @mentions with notifications
- Reactions (👍 ❤️ 🎉 etc.)
- Activity feed per bead/project
- Email/Slack notifications
- Watch beads for updates

### 5. Governance Dashboard

Visual representation of Sheriff governance:

```
┌─────────────────────────────────────────────────────────────────┐
│  Governance Dashboard                              [Generate Report]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Overall Compliance: 94%                                         │
│  ████████████████████████████████████████░░░                    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Repository Status                                            ││
│  ├─────────────────────────────────────────────────────────────┤│
│  │ Repository      │ Compliance │ Issues │ Last Check          ││
│  │─────────────────┼────────────┼────────┼─────────────────────││
│  │ api             │ ✓ 100%     │ 0      │ 2 min ago           ││
│  │ frontend        │ ✓ 98%      │ 1 warn │ 2 min ago           ││
│  │ billing         │ ✓ 100%     │ 0      │ 2 min ago           ││
│  │ legacy-service  │ ⚠ 78%      │ 3 crit │ 5 min ago           ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌────────────────────────┐  ┌────────────────────────┐         │
│  │ Policy Violations      │  │ Agent Activity          │         │
│  │ (last 7 days)          │  │ (last 7 days)          │         │
│  │                        │  │                        │         │
│  │ Critical: 2 (fixed: 2) │  │ Claude Code: 234 edits │         │
│  │ Warning: 8 (fixed: 7)  │  │ Cursor: 89 edits       │         │
│  │ Info: 15 (fixed: 12)   │  │ Human: 156 edits       │         │
│  │                        │  │                        │         │
│  │ [View Details]         │  │ [View Audit Log]       │         │
│  └────────────────────────┘  └────────────────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 6. Integration Hub

Central management for all integrations:

```
┌─────────────────────────────────────────────────────────────────┐
│  Integrations                                     [+ Add Integration]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ GitHub                                          [Connected] │ │
│  │ Sync issues, PRs, and comments bidirectionally             │ │
│  │                                                            │ │
│  │ Repos synced: 12    Last sync: 30 sec ago    Status: ✓     │ │
│  │ [Configure] [Sync Now] [View Logs]                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ JIRA                                            [Connected] │ │
│  │ Import issues from JIRA projects                           │ │
│  │                                                            │ │
│  │ Projects: PLAT, MOBILE    Last sync: 5 min ago  Status: ✓  │ │
│  │ [Configure] [Sync Now] [View Logs]                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Slack                                           [Connected] │ │
│  │ Notifications for bead updates and governance alerts       │ │
│  │                                                            │ │
│  │ Channels: #eng, #alerts    Notifications: 234 this week    │ │
│  │ [Configure] [Test] [View Logs]                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  Available Integrations:                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
│  │ Linear      │ │ Asana       │ │ PagerDuty   │               │
│  │ [Install]   │ │ [Install]   │ │ [Install]   │               │
│  └─────────────┘ └─────────────┘ └─────────────┘               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 7. Onboarding Wizard

Guided setup for new organizations:

```
┌─────────────────────────────────────────────────────────────────┐
│  Welcome to AllBeads! Let's get you set up.                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Step 2 of 5: Connect Your Repositories                         │
│  ─────────────────────────────────────                          │
│                                                                  │
│  We found these repositories in your GitHub organization:        │
│                                                                  │
│  ☑ acme/api           Already has .beads/ ✓                     │
│  ☑ acme/frontend      Already has .beads/ ✓                     │
│  ☐ acme/mobile        No .beads/ (will initialize)              │
│  ☐ acme/docs          No .beads/ (will initialize)              │
│  ☑ acme/billing       Already has .beads/ ✓                     │
│                                                                  │
│  [Select All]  [Select None]                                     │
│                                                                  │
│  For repos without .beads/, we'll:                               │
│    1. Create .beads/ directory                                   │
│    2. Initialize with your governance policies                   │
│    3. Install CLI hooks (via PR)                                 │
│                                                                  │
│                                        [Back]  [Next: Teams →]   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                       AllBeads Web Platform                       │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                        Web Frontend                          │ │
│  │            (React/Next.js or Rust/Leptos)                    │ │
│  └───────────────────────────┬─────────────────────────────────┘ │
│                              │                                    │
│  ┌───────────────────────────┴─────────────────────────────────┐ │
│  │                         API Layer                            │ │
│  │              (REST + WebSocket for real-time)                │ │
│  └───────────────────────────┬─────────────────────────────────┘ │
│                              │                                    │
│  ┌───────────────────────────┴─────────────────────────────────┐ │
│  │                      Core Services                           │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │ │
│  │  │  Sync   │  │Governance│ │ Notify  │  │ Report  │        │ │
│  │  │ Service │  │ Service │  │ Service │  │ Service │        │ │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘        │ │
│  └───────────────────────────┬─────────────────────────────────┘ │
│                              │                                    │
│  ┌───────────────────────────┴─────────────────────────────────┐ │
│  │                       Data Layer                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │ │
│  │  │  PostgreSQL │  │    Redis    │  │ Blob Store  │         │ │
│  │  │  (primary)  │  │   (cache)   │  │  (reports)  │         │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Integration Workers                       │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │ │
│  │  │ GitHub  │  │  JIRA   │  │  Slack  │  │ Linear  │        │ │
│  │  │ Worker  │  │ Worker  │  │ Worker  │  │ Worker  │        │ │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘        │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
              │                                    │
              │ Git Push/Pull                      │ Webhooks
              ↓                                    ↓
┌─────────────────────────────┐    ┌─────────────────────────────┐
│    Developer Machines       │    │   GitHub/GitLab/etc.        │
│    (AllBeads CLI)           │    │   (Webhook events)          │
└─────────────────────────────┘    └─────────────────────────────┘
```

### Sync Protocol

**Problem:** Beads live in git repos. Web platform needs them.

**Solution:** Bidirectional sync via git:

```
Developer Machine                 AllBeads Web                 Git Remote
      │                               │                            │
      │  bd create "New task"         │                            │
      │  ─────────────────────→       │                            │
      │  (writes to .beads/)          │                            │
      │                               │                            │
      │  git push                     │                            │
      │  ──────────────────────────────────────────────────────→   │
      │                               │                            │
      │                               │    Webhook: push event     │
      │                               │  ←─────────────────────    │
      │                               │                            │
      │                               │  git pull .beads/*         │
      │                               │  ─────────────────────→    │
      │                               │  ←─────────────────────    │
      │                               │                            │
      │                               │  Update PostgreSQL         │
      │                               │  Broadcast via WebSocket   │
      │                               │                            │
      │                               │                            │
      │  Web User adds comment        │                            │
      │                               │  ←────────────────────     │
      │                               │                            │
      │                               │  Commit to .beads/         │
      │                               │  git push                  │
      │                               │  ─────────────────────→    │
      │                               │                            │
      │  git pull                     │                            │
      │  ←─────────────────────────────────────────────────────    │
      │  (gets new comment)           │                            │
```

**Conflict Resolution:**
- Web platform never overwrites CLI changes
- Last-write-wins with merge for comments
- Structured merge for bead metadata (status, priority)
- Conflicts flagged for manual resolution

### Data Model

```sql
-- Organizations
CREATE TABLE organizations (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  slug TEXT UNIQUE NOT NULL,
  settings JSONB,
  created_at TIMESTAMP
);

-- Projects (group of repos)
CREATE TABLE projects (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  settings JSONB
);

-- Repositories (linked to projects)
CREATE TABLE repositories (
  id UUID PRIMARY KEY,
  project_id UUID REFERENCES projects(id),
  remote_url TEXT NOT NULL,
  context_name TEXT,  -- AllBeads context
  last_synced_at TIMESTAMP,
  sync_status TEXT
);

-- Beads (cached from git)
CREATE TABLE beads (
  id TEXT PRIMARY KEY,  -- e.g., "api-123"
  repo_id UUID REFERENCES repositories(id),
  title TEXT NOT NULL,
  description TEXT,
  status TEXT,
  priority TEXT,
  bead_type TEXT,
  assignee TEXT,
  labels TEXT[],
  blocks TEXT[],       -- Bead IDs this blocks
  blocked_by TEXT[],   -- Bead IDs blocking this
  milestone_id UUID REFERENCES milestones(id),
  epic_id TEXT,
  created_at TIMESTAMP,
  updated_at TIMESTAMP,
  raw_jsonl JSONB      -- Original JSONL for sync
);

-- Milestones (web-only, not synced to git)
CREATE TABLE milestones (
  id UUID PRIMARY KEY,
  project_id UUID REFERENCES projects(id),
  name TEXT NOT NULL,
  description TEXT,
  target_date DATE,
  status TEXT,  -- planned, in_progress, completed
  created_at TIMESTAMP
);

-- Comments (synced to git as part of bead history)
CREATE TABLE comments (
  id UUID PRIMARY KEY,
  bead_id TEXT REFERENCES beads(id),
  author_id UUID REFERENCES users(id),
  content TEXT,
  created_at TIMESTAMP,
  synced_to_git BOOLEAN DEFAULT FALSE
);

-- Users
CREATE TABLE users (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  email TEXT UNIQUE,
  name TEXT,
  role TEXT,  -- admin, member, viewer
  settings JSONB
);

-- Teams
CREATE TABLE teams (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  name TEXT,
  members UUID[]  -- User IDs
);
```

---

## API Design

### REST Endpoints

```
# Organizations
GET    /api/orgs
POST   /api/orgs
GET    /api/orgs/:slug
PATCH  /api/orgs/:slug
DELETE /api/orgs/:slug

# Projects
GET    /api/orgs/:org/projects
POST   /api/orgs/:org/projects
GET    /api/orgs/:org/projects/:slug
PATCH  /api/orgs/:org/projects/:slug

# Repositories
GET    /api/projects/:id/repos
POST   /api/projects/:id/repos
POST   /api/repos/:id/sync  # Trigger sync

# Beads
GET    /api/repos/:id/beads
GET    /api/repos/:id/beads/:bead_id
PATCH  /api/repos/:id/beads/:bead_id
GET    /api/projects/:id/beads  # Aggregated
GET    /api/orgs/:org/beads     # Org-wide

# Milestones
GET    /api/projects/:id/milestones
POST   /api/projects/:id/milestones
GET    /api/milestones/:id
PATCH  /api/milestones/:id
DELETE /api/milestones/:id
POST   /api/milestones/:id/beads  # Assign beads

# Comments
GET    /api/beads/:id/comments
POST   /api/beads/:id/comments
DELETE /api/comments/:id

# Governance
GET    /api/orgs/:org/governance/status
GET    /api/orgs/:org/governance/reports
POST   /api/orgs/:org/governance/check
GET    /api/repos/:id/governance/status

# Integrations
GET    /api/orgs/:org/integrations
POST   /api/orgs/:org/integrations
DELETE /api/integrations/:id
POST   /api/integrations/:id/sync
```

### WebSocket Events

```typescript
// Real-time updates
interface WSEvent {
  type: 'bead.updated' | 'bead.created' | 'comment.added' | 'sync.completed';
  payload: {
    bead_id?: string;
    repo_id?: string;
    project_id?: string;
    data: any;
  };
}

// Client subscribes to channels
ws.subscribe('org:acme');
ws.subscribe('project:platform');
ws.subscribe('bead:api-123');
```

---

## Implementation Phases

### Phase 1: Core Platform (8-10 weeks)

**Goal:** Basic web UI with bead viewing

- [ ] User auth (email/password initially)
- [ ] Organization CRUD
- [ ] Project CRUD
- [ ] Repository linking (via git URL)
- [ ] Basic sync (pull beads from git)
- [ ] Bead list view
- [ ] Bead detail view
- [ ] Basic kanban board

**Tech Stack Decision:**
- Frontend: Next.js (React) or Leptos (Rust WASM)
- Backend: Rust (Axum) to share code with CLI
- Database: PostgreSQL
- Cache: Redis

### Phase 2: Collaboration (6-8 weeks)

- [ ] Comments on beads
- [ ] @mentions
- [ ] Notifications (email, in-app)
- [ ] Activity feed
- [ ] User profiles
- [ ] Team management
- [ ] Real-time updates (WebSocket)

### Phase 3: Milestones & Releases (4-6 weeks)

- [ ] Milestone CRUD
- [ ] Assign beads to milestones
- [ ] Burndown charts
- [ ] Release notes generation
- [ ] Velocity metrics
- [ ] Sprint planning view

### Phase 4: Governance Dashboard (4-6 weeks)

- [ ] Visual governance status
- [ ] Policy management UI
- [ ] Report viewing
- [ ] Compliance trends
- [ ] Agent activity dashboard
- [ ] Audit log viewer

### Phase 5: Integrations (6-8 weeks)

- [ ] GitHub App for webhooks
- [ ] JIRA Cloud integration
- [ ] Slack integration
- [ ] Linear integration
- [ ] Integration marketplace

### Phase 6: Enterprise (8-10 weeks)

- [ ] SSO/SAML
- [ ] SCIM provisioning
- [ ] Advanced RBAC
- [ ] Audit exports
- [ ] On-premise deployment option
- [ ] SLA dashboard

---

## Business Model

### Pricing Tiers

```
Free Tier:
- 1 organization
- 3 repositories
- 5 users
- Basic integrations
- Community support

Team ($15/user/month):
- Unlimited repositories
- Unlimited users
- All integrations
- Milestones & releases
- Priority support

Enterprise ($30/user/month):
- Everything in Team
- SSO/SAML
- Advanced governance
- Compliance reports
- On-premise option
- Dedicated support
```

### Revenue Projections

```
Year 1: Focus on adoption
- 100 free orgs, 20 paying teams
- ~$5K MRR

Year 2: Team growth
- 500 free orgs, 100 paying teams
- ~$50K MRR

Year 3: Enterprise
- 1000 orgs, 200 teams, 10 enterprise
- ~$150K MRR
```

---

## Competitive Landscape

| Feature | AllBeads Web | Linear | Jira | GitHub Issues |
|---------|--------------|--------|------|---------------|
| Git-native storage | ✓ | ✗ | ✗ | ✓ |
| Multi-repo | ✓ | ✓ | ✓ | ✗ |
| AI agent tracking | ✓ | ✗ | ✗ | ✗ |
| Governance | ✓ | ✗ | ✓ | ✗ |
| Offline-first | ✓ | ✗ | ✗ | ✗ |
| CLI-first | ✓ | ✗ | ✗ | ✗ |
| Milestones | ✓ | ✓ | ✓ | ✓ |
| Cross-repo deps | ✓ | ✗ | ✗ | ✗ |

**Our Differentiators:**
1. Git-native (beads live in repos, not SaaS database)
2. AI agent awareness (provenance, governance)
3. CLI-first with web as complement
4. Cross-repo dependencies built-in
5. Governance as first-class feature

---

## Open Questions

### Q1: Build or Buy Frontend?

**Option A:** Build with Leptos (Rust WASM)
- Pros: Code sharing with CLI, consistent stack
- Cons: Smaller ecosystem, learning curve

**Option B:** Build with Next.js/React
- Pros: Larger ecosystem, easier hiring
- Cons: Two languages (Rust + TypeScript)

**Recommendation:** Start with Next.js for speed, consider Leptos later

### Q2: How Much Syncs to Git?

**Option A:** Everything syncs (comments, milestones, etc.)
- Pros: True git-native, offline works
- Cons: Complex sync, merge conflicts

**Option B:** Beads sync, web features stay in DB
- Pros: Simpler, faster
- Cons: Not fully git-native, data split

**Recommendation:** Option B - core beads sync, web features (milestones, comments) can be web-only initially

### Q3: Self-Hosted vs SaaS Only?

**Option A:** SaaS only initially
- Pros: Simpler ops, faster iteration
- Cons: Loses enterprise deals

**Option B:** Both from start
- Pros: Enterprise ready
- Cons: 2x ops burden

**Recommendation:** SaaS first (Phase 1-4), self-hosted later (Phase 6)

### Q4: How to Handle Milestone→Epic Relationship?

If beads implements milestones directly:
- Adopt their implementation
- Web provides visualization

If beads doesn't implement:
- Web-only milestones
- Sync epic→milestone mapping via labels

**Recommendation:** Propose milestones to beads upstream, implement web-only as fallback

---

## Success Metrics

### Phase 1 (Core Platform)
- [ ] 50 organizations signed up
- [ ] 20 actively using (weekly)
- [ ] <500ms page load times
- [ ] Sync lag <30 seconds

### Phase 3 (Milestones)
- [ ] 50% of orgs using milestones
- [ ] Burndown charts accurate within 5%
- [ ] Release notes used by 30% of teams

### Year 1
- [ ] 500 organizations
- [ ] 100 paying teams
- [ ] NPS >40
- [ ] <1% churn

---

## Appendix: Milestone/Release Design Details

### Milestone Structure

```yaml
milestone:
  id: "m-2026-q1"
  name: "Q1 2026 Release"
  description: "Major platform redesign"

  # Date tracking
  target_date: 2026-03-31
  started_at: 2026-01-01
  completed_at: null  # Set when all beads done

  # Status
  status: in_progress  # planned, in_progress, completed, cancelled

  # Scope
  projects: [platform, mobile]  # Which projects included
  beads: []  # Assigned via bead.milestone_id

  # Computed (not stored, calculated)
  metrics:
    total_beads: 47
    by_status:
      open: 8
      in_progress: 12
      blocked: 4
      closed: 23
    by_priority:
      P0: 3
      P1: 12
      P2: 20
      P3: 10
      P4: 2
    completion_pct: 49%
    velocity:
      last_7_days: 8  # Beads closed
      avg_daily: 1.14
    projection:
      estimated_completion: 2026-03-15  # Based on velocity
      on_track: true
```

### Release Notes Generation

```bash
# CLI command
$ ab release notes m-2026-q1

# Generates markdown:

## Q1 2026 Release

**Released:** March 31, 2026

### Features
- **api-epic-auth:** Complete authentication overhaul
  - api-101: OAuth2 support
  - api-102: MFA implementation
  - api-103: Session management

- **fe-epic-redesign:** Frontend redesign
  - fe-201: New dashboard
  - fe-202: Dark mode

### Bug Fixes
- api-150: Fixed timeout in auth flow (P0)
- fe-210: Fixed mobile layout issues (P1)

### Contributors
- @alice (23 beads)
- @bob (18 beads)
- Claude Code (12 beads)

### Metrics
- Beads completed: 47
- P0 bugs fixed: 3
- Features delivered: 12
```

### Sprint Planning View

```
┌─────────────────────────────────────────────────────────────────┐
│  Sprint Planning: Sprint 23 (Jan 13 - Jan 27)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Capacity: 45 story points    Committed: 38 points              │
│                                                                  │
│  Backlog                      │  Sprint                         │
│  ─────────────────────────── │  ─────────────────────────────  │
│  ┌─────────────────────────┐  │  ┌─────────────────────────┐    │
│  │ api-301 [5pts] P1       │  │  │ api-201 [8pts] P0       │    │
│  │ Add rate limiting       │──┼─→│ Fix auth timeout        │    │
│  └─────────────────────────┘  │  └─────────────────────────┘    │
│  ┌─────────────────────────┐  │  ┌─────────────────────────┐    │
│  │ fe-401 [3pts] P2        │  │  │ fe-301 [5pts] P1        │    │
│  │ Improve loading         │  │  │ Dark mode toggle        │    │
│  └─────────────────────────┘  │  └─────────────────────────┘    │
│  ┌─────────────────────────┐  │  ┌─────────────────────────┐    │
│  │ api-302 [8pts] P2       │  │  │ api-202 [8pts] P1       │    │
│  │ Caching layer           │  │  │ OAuth2 integration      │    │
│  └─────────────────────────┘  │  └─────────────────────────┘    │
│                               │                                  │
│  [+ Add to Sprint]            │  Total: 38/45 pts (84%)         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

*This spec outlines the AllBeads Web Platform vision. Implementation should validate core sync and UI before adding advanced features like milestones and governance dashboards.*
