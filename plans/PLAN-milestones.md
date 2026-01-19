# PLAN: Unified Milestones Across Beads Ecosystem

**Date:** 2026-01-18
**Epic:** ab-4fa
**Status:** Exploration / Planning
**Author:** Claude Opus 4.5 + thrashr888

---

## Overview

This plan explores how to add milestones to the beads ecosystem in a way that:
1. Works with the distributed git-native architecture
2. Integrates with GitHub Milestones, Jira Versions, and Linear Cycles
3. Supports AllBeads CLI, AllBeadsWeb, and AllBeadsApp
4. Doesn't break or fork the upstream `bd` CLI

---

## Integration Data Model Analysis

### GitHub Milestones

```json
{
  "id": 1,
  "node_id": "MDk6TWlsZXN0b25lMQ==",
  "number": 1,
  "title": "v1.0",
  "description": "First major release",
  "state": "open",           // "open" | "closed"
  "due_on": "2026-03-31T07:00:00Z",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-15T00:00:00Z",
  "closed_at": null,
  "open_issues": 10,
  "closed_issues": 5
}
```

**Key fields:** title, description, due_on, state, open/closed counts

### Jira Versions (Fix Versions)

```json
{
  "id": "10001",
  "name": "2.0",
  "description": "Q1 2026 Release",
  "archived": false,
  "released": false,
  "releaseDate": "2026-03-31",
  "startDate": "2026-01-01",
  "projectId": 10000,
  "userReleaseDate": "31/Mar/26"
}
```

**Key fields:** name, description, releaseDate, startDate, released, archived

### Linear Cycles

```json
{
  "id": "cycle-123",
  "number": 5,
  "name": "Cycle 5",
  "description": "Sprint focus on auth",
  "startsAt": "2026-01-15T00:00:00Z",
  "endsAt": "2026-01-29T00:00:00Z",
  "completedAt": null,
  "progress": 0.45,
  "scopeProgress": 0.67
}
```

**Key fields:** name, startsAt, endsAt, progress (Linear also has Projects which are goal-based, not time-boxed)

### Linear Projects (alternative to Cycles)

```json
{
  "id": "project-456",
  "name": "Authentication Overhaul",
  "description": "Complete auth system rewrite",
  "state": "started",        // "backlog" | "planned" | "started" | "paused" | "completed" | "canceled"
  "targetDate": "2026-03-31",
  "startedAt": "2026-01-01T00:00:00Z",
  "completedAt": null,
  "progress": 0.33
}
```

---

## Unified Milestone Model

### Core Fields (Common Across All)

| Field | Beads | GitHub | Jira | Linear |
|-------|-------|--------|------|--------|
| **id** | bead ID | node_id | id | id |
| **title** | title | title | name | name |
| **description** | description | description | description | description |
| **target_date** | label | due_on | releaseDate | targetDate/endsAt |
| **start_date** | label | - | startDate | startsAt |
| **state** | status | state | released | state/completedAt |

### Proposed Beads Milestone Schema

```jsonl
{
  "id": "ab-ms-v2",
  "title": "v2.0 Release",
  "description": "Major platform update",
  "status": "open",
  "issue_type": "milestone",
  "priority": 2,
  "labels": [
    "target:2026-03-31",
    "start:2026-01-15",
    "version:2.0.0"
  ],
  "external_ref": "github:milestone:1",
  "external_url": "https://github.com/org/repo/milestone/1",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-18T00:00:00Z"
}
```

### Bead-to-Milestone Assignment

Option A: **Label-based** (current spec approach)
```jsonl
{"id": "ab-123", "title": "Add OAuth", "labels": ["milestone:v2.0"]}
```

Option B: **Field-based** (requires beads schema change)
```jsonl
{"id": "ab-123", "title": "Add OAuth", "milestone_id": "ab-ms-v2"}
```

Option C: **Dependency-based** (use existing depends_on)
```jsonl
{"id": "ab-123", "title": "Add OAuth", "depends_on": ["ab-ms-v2"]}
```

**Recommendation:** Start with **Option A (labels)** for bd compatibility, consider Option B for upstream proposal.

---

## Distributed Architecture Considerations

### Per-Repository Milestones

Each repo maintains its own milestones in `.beads/issues.jsonl`:

```
repo-a/.beads/
  issues.jsonl  # Contains milestone ab-ms-v1 + beads referencing it

repo-b/.beads/
  issues.jsonl  # Contains milestone ab-ms-v1 (same version, different repo)
```

**Same version, different repos:** v1.0 in repo-a and v1.0 in repo-b are separate milestones.

### Cross-Repo Milestone Aggregation

AllBeads aggregates milestones across contexts:

```
ab milestones list

@repo-a:
  v1.0 (target: 2026-02-15) - 67% complete

@repo-b:
  v1.0 (target: 2026-02-15) - 45% complete
  v2.0 (target: 2026-04-01) - 12% complete

Cross-Repo Release "Q1-2026":
  @repo-a v1.0 + @repo-b v1.0 = 56% complete
```

### Meta-Milestones (Cross-Repo Releases)

For coordinated releases across repos, use a **meta-milestone** in a boss/orchestration repo:

```jsonl
// In boss-repo/.beads/issues.jsonl
{
  "id": "ab-release-q1",
  "title": "Q1 2026 Release",
  "issue_type": "milestone",
  "labels": ["meta-milestone", "target:2026-03-31"],
  "description": "Coordinated release across all services",
  "depends_on": [
    "bead://repo-a/ab-ms-v1",
    "bead://repo-b/ab-ms-v1"
  ]
}
```

This creates a **shadow milestone** that tracks child milestones across repos.

---

## Integration Sync Strategy

### GitHub Milestones Sync

**Pull (GitHub → Beads):**
```rust
// When pulling GitHub issues, also pull milestones
async fn pull_github_milestones(client: &GitHubClient, repo: &str) -> Vec<Milestone> {
    let milestones = client.list_milestones(repo).await?;
    milestones.into_iter().map(|m| Milestone {
        id: format!("gh-ms-{}", m.number),
        title: m.title,
        target_date: m.due_on,
        external_ref: format!("github:milestone:{}", m.number),
        // ...
    }).collect()
}
```

**Push (Beads → GitHub):**
```rust
// Create/update GitHub milestone from bead
async fn push_milestone_to_github(bead: &Bead, client: &GitHubClient) -> Result<()> {
    if bead.issue_type != "milestone" { return Ok(()); }

    let due_on = parse_target_label(&bead.labels);
    client.create_or_update_milestone(
        &bead.title,
        bead.description.as_deref(),
        due_on,
    ).await
}
```

**Issue Assignment Sync:**
When syncing issues, also sync milestone assignment:
```rust
// GitHub issue has milestone field
if let Some(gh_milestone) = github_issue.milestone {
    bead.labels.push(format!("milestone:{}", gh_milestone.title));
}
```

### Jira Versions Sync

**Pull (Jira → Beads):**
```rust
async fn pull_jira_versions(client: &JiraClient, project: &str) -> Vec<Milestone> {
    let versions = client.get_project_versions(project).await?;
    versions.into_iter().map(|v| Milestone {
        id: format!("jira-v-{}", v.id),
        title: v.name,
        target_date: v.release_date,
        start_date: v.start_date,
        status: if v.released { "closed" } else { "open" },
        external_ref: format!("jira:version:{}", v.id),
        // ...
    }).collect()
}
```

**Issue Assignment:**
Jira uses `fixVersions` field on issues:
```rust
// Map Jira fixVersions to milestone labels
for version in jira_issue.fields.fix_versions {
    bead.labels.push(format!("milestone:{}", version.name));
}
```

### Linear Cycles/Projects Sync

Linear has both Cycles (time-boxed sprints) and Projects (goal-based):

```rust
// Map Linear cycle to milestone
fn linear_cycle_to_milestone(cycle: LinearCycle) -> Milestone {
    Milestone {
        id: format!("linear-c-{}", cycle.id),
        title: cycle.name,
        target_date: Some(cycle.ends_at),
        start_date: Some(cycle.starts_at),
        labels: vec!["linear-cycle".to_string()],
        // ...
    }
}

// Map Linear project to milestone
fn linear_project_to_milestone(project: LinearProject) -> Milestone {
    Milestone {
        id: format!("linear-p-{}", project.id),
        title: project.name,
        target_date: project.target_date,
        labels: vec!["linear-project".to_string()],
        // ...
    }
}
```

---

## App Integration

### AllBeadsWeb (Next.js)

**Database Schema (Prisma):**
```prisma
model Milestone {
  id          String   @id
  repoId      String
  title       String
  description String?
  targetDate  DateTime?
  startDate   DateTime?
  status      MilestoneStatus @default(OPEN)
  version     String?
  externalRef String?
  externalUrl String?
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt

  repo        Repository @relation(fields: [repoId], references: [id])
  beads       Bead[]     @relation("BeadMilestone")
}

enum MilestoneStatus {
  OPEN
  CLOSED
  RELEASED
}

model Bead {
  // ... existing fields
  milestoneId String?
  milestone   Milestone? @relation("BeadMilestone", fields: [milestoneId], references: [id])
}
```

**API Endpoints:**
- `GET /api/milestones` - List milestones (with progress aggregation)
- `GET /api/milestones/:id` - Milestone details + burndown data
- `POST /api/milestones` - Create milestone
- `PATCH /api/milestones/:id` - Update milestone
- `POST /api/milestones/:id/assign` - Assign beads to milestone

**Import Enhancement:**
Update `/api/beads/import` to handle milestones:
```typescript
// Detect milestone beads by issue_type or labels
if (bead.issue_type === 'milestone' || bead.labels?.includes('milestone')) {
  await upsertMilestone(bead);
} else {
  await upsertBead(bead);
  // Also sync milestone assignment from labels
  const milestoneLabel = bead.labels?.find(l => l.startsWith('milestone:'));
  if (milestoneLabel) {
    await assignBeadToMilestone(bead.id, milestoneLabel.split(':')[1]);
  }
}
```

### AllBeadsApp (macOS)

**SwiftUI Views:**
- `MilestoneListView` - List milestones with progress bars
- `MilestoneDetailView` - Milestone details, burndown chart, bead list
- `MilestonePickerView` - Assign beads to milestones

**Local Storage:**
- Milestones stored in CoreData alongside beads
- Sync via same mechanism as beads (git-backed)

**Menu Bar Quick Actions:**
- Show current milestone progress
- Quick-assign active bead to milestone

---

## CLI Commands

### Core Commands (ab)

```bash
# List milestones across all contexts
ab milestones list
ab milestones list --context=AllBeads
ab milestones list --upcoming  # Sort by target date

# Show milestone details
ab milestones show v2.0
ab milestones show ab-ms-v2 --burndown  # ASCII burndown chart

# Create milestone
ab milestones create --title="v2.0" --target=2026-03-31
ab milestones create --title="v2.0" --target=2026-03-31 --start=2026-01-15

# Assign bead to milestone
ab milestones assign ab-123 v2.0
ab milestones unassign ab-123

# Generate release notes
ab milestones notes v2.0
ab milestones notes v2.0 --format=markdown
ab milestones notes v2.0 --format=changelog
```

### Wrapper for bd (no bd changes needed)

```bash
# ab milestones create wraps bd create
ab milestones create --title="v2.0" --target=2026-03-31

# Equivalent to:
bd create --title="v2.0" --type=epic --labels="milestone,target:2026-03-31,version:2.0"
```

---

## Migration Path

### Phase 1: Convention-Based (No upstream changes)
- Use labels for milestone definition and assignment
- `ab` interprets labels, `bd` sees them as normal labels
- Works today, no dependencies

### Phase 2: Upstream Proposal
- Propose `milestone` as first-class issue_type to beads
- Propose `milestone_id` field on beads
- If accepted, cleaner model; if rejected, labels still work

### Phase 3: Integration Sync
- Add milestone sync to GitHub/Jira/Linear integrations
- Bi-directional: pull milestones from external, push local milestones

### Phase 4: App Integration
- Add milestone views to AllBeadsWeb
- Add milestone views to AllBeadsApp
- Dashboard widgets, burndown charts

---

## Open Questions

### Q1: Milestone Identity Across Repos

If repo-a and repo-b both have "v1.0", are they the same milestone?

**Options:**
1. **Always separate** - Each repo's v1.0 is independent
2. **Linked by convention** - If same name + same target date, treat as related
3. **Explicit linking** - Use meta-milestone in boss repo to link

**Recommendation:** Option 1 (always separate) with Option 3 (explicit linking) for coordinated releases.

### Q2: Milestone vs Epic

What's the difference between a milestone and an epic?

| Aspect | Epic | Milestone |
|--------|------|-----------|
| **Focus** | Feature/goal grouping | Time-based release |
| **Target date** | Optional | Required |
| **Progress** | By dependency completion | By bead count |
| **Cross-repo** | Can span repos via shadows | Usually per-repo |

**Recommendation:** Milestone is a **time-targeted epic**. Could be modeled as epic + `milestone` label.

### Q3: Burndown Calculation

How to calculate burndown?

**Option A:** Count beads (simple)
- Progress = closed_beads / total_beads

**Option B:** Count by priority weight
- P0 = 8, P1 = 4, P2 = 2, P3 = 1
- Progress = weighted_closed / weighted_total

**Option C:** Story points (requires new field)
- Each bead has `points` field
- Progress = closed_points / total_points

**Recommendation:** Start with Option A, add Option B later.

### Q4: Historical Data

How to track milestone progress over time for burndown charts?

**Options:**
1. **Snapshot in comments** - Daily bot adds progress comment to milestone bead
2. **Separate log file** - `.beads/milestone-history.jsonl`
3. **Calculate from git history** - Replay JSONL changes to build history
4. **External tracking** - Store in AllBeadsWeb database only

**Recommendation:** Option 4 for web burndown charts, Option 3 for CLI if needed.

---

## Success Criteria

### Phase 1 (Convention Implementation)
- [ ] Milestone label conventions documented
- [ ] `ab milestones list/show/create/assign` commands work
- [ ] Progress calculation works
- [ ] Works with existing `bd` workflows
- [ ] No `bd` modifications required

### Phase 2 (Integration Sync)
- [ ] GitHub milestone sync (pull + push)
- [ ] Jira version sync (pull + push)
- [ ] Linear cycle/project sync (pull)
- [ ] Bead-to-milestone assignment syncs

### Phase 3 (App Integration)
- [ ] AllBeadsWeb milestone views
- [ ] AllBeadsWeb burndown charts
- [ ] AllBeadsApp milestone views
- [ ] Cross-platform milestone sync

---

## Next Steps

1. **Review this plan** - Ensure alignment on approach
2. **Create sub-tasks** - Break into implementable beads
3. **Start with CLI** - Implement `ab milestones` commands
4. **Add to web** - After CLI works, add web views
5. **Integration sync** - Add GitHub/Jira milestone sync
6. **Upstream proposal** - Once validated, propose to beads

---

## Appendix: Label Convention Summary

### Milestone Definition
```
labels: ["milestone", "target:YYYY-MM-DD", "version:X.Y.Z", "start:YYYY-MM-DD"]
```

### Bead Assignment
```
labels: ["milestone:v2.0"]  // or milestone:ab-ms-v2
```

### Integration Markers
```
labels: ["github-milestone:1", "jira-version:10001", "linear-cycle:abc"]
```
