# SPEC: Milestones - Release Tracking for Beads

**Status:** Draft / Ultrathink
**Author:** Claude Opus 4.5 + thrashr888
**Date:** 2026-01-11
**Epic:** ab-???

---

## Executive Summary

This spec explores how to add **milestones/releases** to the beads ecosystem without modifying how `bd` works or creating features that only work with `ab`.

**The Core Tension:**
- Milestones are valuable for release planning
- We don't want to fork or modify `bd`
- We don't want `ab`-only features that break the distributed model

**Proposed Solution:** A **convention-based approach** using existing beads features (labels, epics) that `bd` already understands, with `ab` providing enhanced visualization and aggregation.

---

## The Problem

### What We Have

Beads has **epics** - a bead type that groups related work:

```bash
$ bd create --title="Authentication Overhaul" --type=epic
$ bd create --title="Add OAuth" --type=task
$ bd dep add auth-task auth-epic  # Task belongs to epic
```

### What's Missing

Epics group related work, but don't answer:
- "What's shipping in Q1?"
- "Are we on track for the March release?"
- "What's the burndown for v2.0?"

**Milestones** add a **time dimension** to epics:
- Target date
- Release version
- Progress tracking
- Cross-epic grouping

---

## Design Constraints

### Must Preserve

1. **bd works standalone** - No milestone features should break `bd` without `ab`
2. **Git-native storage** - Milestones stored in repo, not external DB
3. **Distributed model** - Each repo can have milestones, `ab` aggregates
4. **Simplicity** - Don't over-engineer for edge cases

### Must Avoid

1. **Forking bd** - We use upstream beads, not our fork
2. **ab-only data** - Anything stored should be visible to `bd` users
3. **Breaking changes** - Existing workflows continue working
4. **Magic behavior** - Transparent, predictable semantics

---

## Solution Options Analysis

### Option A: Propose Milestones to Beads Upstream

**How it works:**
- Submit PR to beads adding milestone support
- Milestones become first-class beads feature
- Both `bd` and `ab` use same implementation

**Pros:**
- Clean, native solution
- Community benefits
- No divergence

**Cons:**
- Depends on upstream acceptance
- Timeline uncertain
- May not match our vision

**Verdict:** Worth proposing, but need a fallback

---

### Option B: Convention-Based Labels

**How it works:**
Use labels with a naming convention that `bd` stores normally but `ab` interprets:

```bash
# Create milestone as a special epic
$ bd create --title="v2.0 Release" --type=epic --labels="milestone,target:2026-03-31"

# Assign beads to milestone via label
$ bd update auth-epic --labels="milestone:v2.0"
$ bd update payment-task --labels="milestone:v2.0"

# ab interprets these labels
$ ab milestones list
v2.0 Release (target: 2026-03-31)
  Progress: ████████░░░░░░░░ 45%
  Beads: 23 total, 10 complete, 3 blocked
```

**Storage in .beads/:**
```jsonl
{"id":"ms-v2","title":"v2.0 Release","type":"epic","labels":["milestone","target:2026-03-31"]}
{"id":"auth-epic","title":"Auth Overhaul","type":"epic","labels":["milestone:v2.0"]}
```

**Pros:**
- Works with existing `bd` - labels are already supported
- Stored in git via normal beads sync
- `bd` users see labels, `ab` users get enhanced view
- No upstream changes required

**Cons:**
- Convention requires documentation
- Label parsing is string-based
- No validation at `bd` level

**Verdict:** ⭐ Recommended approach

---

### Option C: Separate Milestone Files

**How it works:**
Store milestones in separate file that `bd` ignores:

```
.beads/
├── issues.jsonl      # Normal beads (bd manages)
└── milestones.jsonl  # Milestone definitions (ab manages)
```

**Pros:**
- Clean separation
- No label conventions needed
- Can have richer milestone metadata

**Cons:**
- `bd` doesn't see milestones at all
- Two sources of truth
- Sync complexity
- Feels like "ab-only" feature

**Verdict:** Not recommended - violates "bd works standalone"

---

### Option D: Hybrid - Labels + Metadata File

**How it works:**
- Use labels for bead→milestone assignment (Option B)
- Store milestone metadata in optional file `ab` reads

```
.beads/
├── issues.jsonl       # Beads with milestone:X labels
└── .milestones.yaml   # Milestone metadata (optional, ab-only)
```

```yaml
# .beads/.milestones.yaml
milestones:
  v2.0:
    title: "v2.0 Release"
    target_date: 2026-03-31
    description: "Major platform redesign"
  v2.1:
    title: "v2.1 Hotfix"
    target_date: 2026-04-15
```

**Pros:**
- Rich metadata without label gymnastics
- `bd` still works (ignores .milestones.yaml)
- Labels link beads to milestones

**Cons:**
- Two files to manage
- Potential drift between label and metadata

**Verdict:** Good fallback if pure labels insufficient

---

## Recommended Approach: Option B (Convention-Based Labels)

### Milestone Definition

A milestone is an **epic with special labels**:

```bash
# Create milestone
$ bd create \
    --title="v2.0 Release" \
    --type=epic \
    --labels="milestone,target:2026-03-31,version:2.0"
```

**Label conventions:**
| Label | Meaning |
|-------|---------|
| `milestone` | Marks this epic as a milestone |
| `target:YYYY-MM-DD` | Target release date |
| `version:X.Y.Z` | Version number |
| `released:YYYY-MM-DD` | Actual release date (when complete) |

### Bead Assignment

Assign beads to milestones via label:

```bash
# Assign to milestone
$ bd update auth-epic --labels="milestone:v2.0"
$ bd update payment-task --labels="milestone:v2.0"

# Or during creation
$ bd create --title="Fix login bug" --type=bug --labels="milestone:v2.0"
```

### AB Commands

`ab` provides milestone-aware commands:

```bash
# List milestones across all contexts
$ ab milestones list

Milestones:

@AllBeads:
  v0.3.0 (target: 2026-02-15) - 12 days remaining
    Progress: ████████████░░░░ 67%
    Beads: 18 total, 12 complete, 2 in_progress, 4 open

@rookery:
  v1.0 (target: 2026-03-01) - 49 days remaining
    Progress: ████░░░░░░░░░░░░ 23%
    Beads: 45 total, 10 complete, 8 in_progress, 27 open

# Show milestone details
$ ab milestones show v0.3.0

v0.3.0 Release
Target: 2026-02-15 (12 days remaining)
Context: @AllBeads

Progress: ████████████░░░░ 67% (12/18 complete)

Burndown:
  18 ┤ ·
     │  ·  ·
  12 ┤     ·  · ← Ideal
     │        ────·───────
   6 ┤              ·  ← Actual
     │                 ·
   0 ┼────────────────────────
     Jan 1    Jan 15    Feb 1    Feb 15

Included Beads:
  ✓ ab-123: Sheriff governance [closed]
  ✓ ab-124: TUI enhancements [closed]
  → ab-125: Milestone support [in_progress]
  ○ ab-126: Web platform MVP [open]
  ...

# Create milestone (wrapper for bd create)
$ ab milestones create --title="v0.4.0" --target=2026-04-01

# Assign bead to milestone
$ ab milestones assign ab-126 v0.4.0

# Generate release notes
$ ab milestones notes v0.3.0
```

### BD Compatibility

Users with only `bd` see milestones as regular epics with labels:

```bash
$ bd list --type=epic
ab-ms-v3: v0.3.0 Release [milestone, target:2026-02-15]
ab-ms-v4: v0.4.0 Release [milestone, target:2026-04-01]

$ bd show ab-ms-v3
ab-ms-v3: v0.3.0 Release
Status: open
Type: epic
Labels: milestone, target:2026-02-15, version:0.3.0

Blocks: (beads with milestone:v0.3.0 label)
  ← ab-123: Sheriff governance
  ← ab-124: TUI enhancements
  ...
```

The convention is **self-documenting** - `bd` users understand what's happening even without `ab`.

---

## Implementation Details

### Label Parsing

```rust
struct MilestoneInfo {
    id: String,
    title: String,
    target_date: Option<NaiveDate>,
    version: Option<String>,
    released_date: Option<NaiveDate>,
}

impl MilestoneInfo {
    fn from_bead(bead: &Bead) -> Option<Self> {
        // Check if bead has "milestone" label
        if !bead.labels.contains(&"milestone".to_string()) {
            return None;
        }

        let target = bead.labels.iter()
            .find(|l| l.starts_with("target:"))
            .and_then(|l| NaiveDate::parse_from_str(&l[7..], "%Y-%m-%d").ok());

        let version = bead.labels.iter()
            .find(|l| l.starts_with("version:"))
            .map(|l| l[8..].to_string());

        Some(Self {
            id: bead.id.clone(),
            title: bead.title.clone(),
            target_date: target,
            version,
            released_date: None,
        })
    }
}
```

### Milestone Aggregation

```rust
fn collect_milestone_beads(graph: &FederatedGraph, milestone_id: &str) -> Vec<&Bead> {
    let milestone_label = format!("milestone:{}", milestone_id);

    graph.beads.values()
        .filter(|b| b.labels.contains(&milestone_label))
        .collect()
}

fn calculate_progress(beads: &[&Bead]) -> f64 {
    let total = beads.len() as f64;
    let closed = beads.iter().filter(|b| b.status == Status::Closed).count() as f64;
    closed / total
}
```

### CLI Commands

```rust
#[derive(Subcommand)]
enum MilestoneCommands {
    /// List all milestones
    List {
        #[arg(long)]
        context: Option<String>,
    },

    /// Show milestone details
    Show {
        /// Milestone ID or version
        id: String,
    },

    /// Create new milestone
    Create {
        #[arg(long)]
        title: String,

        #[arg(long)]
        target: String,  // YYYY-MM-DD

        #[arg(long)]
        version: Option<String>,

        #[arg(long)]
        context: Option<String>,
    },

    /// Assign bead to milestone
    Assign {
        /// Bead ID
        bead: String,

        /// Milestone ID or version
        milestone: String,
    },

    /// Generate release notes
    Notes {
        /// Milestone ID or version
        id: String,

        #[arg(long, default_value = "markdown")]
        format: String,
    },
}
```

---

## Upstream Proposal Strategy

While implementing the convention-based approach, we can propose native milestone support to beads:

### What to Propose

1. **Milestone type** (alternative to epic with labels):
   ```bash
   bd create --title="v2.0" --type=milestone --target=2026-03-31
   ```

2. **Milestone assignment flag**:
   ```bash
   bd update task-123 --milestone=v2.0
   ```

3. **Milestone commands**:
   ```bash
   bd milestone list
   bd milestone show v2.0
   bd milestone assign task-123 v2.0
   ```

### Proposal Timeline

1. **Now:** Implement convention-based approach in `ab`
2. **After validation:** Write RFC for beads upstream
3. **If accepted:** Migrate from labels to native support
4. **If rejected:** Continue with labels (still works!)

---

## Migration Path

### From Labels to Native (if upstream accepts)

```bash
# ab could auto-migrate
$ ab milestones migrate

Migrating milestones from labels to native format...

Found 3 milestones:
  v0.3.0 (18 beads)
  v0.4.0 (12 beads)
  v1.0 (45 beads)

Migration will:
  1. Create native milestone beads
  2. Update bead→milestone references
  3. Remove old label conventions

Proceed? [y/N]
```

### Backwards Compatibility

If beads adds native milestones, `ab` can support both:
- Detect native milestones (new format)
- Detect label-based milestones (old format)
- Display unified view

---

## TUI Integration

Add Milestones view to TUI (ties into ab-dmd):

```
┌─────────────────────────────────────────────────────────────────┐
│  Milestones                                           [Tab: 5/6]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  > v0.3.0 Release      Feb 15   ████████████░░░░ 67%  12 days   │
│    v0.4.0 Release      Apr 01   ████░░░░░░░░░░░░ 25%  77 days   │
│    v1.0 (rookery)      Mar 01   ██░░░░░░░░░░░░░░ 12%  49 days   │
│                                                                  │
│  ─────────────────────────────────────────────────────────────  │
│                                                                  │
│  v0.3.0 Release                                                  │
│  Target: 2026-02-15 (12 days remaining)                         │
│                                                                  │
│  Beads: 18 total                                                 │
│    ✓ Closed:      12                                             │
│    → In Progress:  2                                             │
│    ○ Open:         4                                             │
│    ✗ Blocked:      0                                             │
│                                                                  │
│  Risk: ⚠ Slightly behind ideal pace                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
│ j/k: Navigate  Enter: View beads  n: New  a: Assign  Tab: Views │
└─────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria

### Phase 1 (Convention Implementation)
- [ ] Milestone label conventions documented
- [ ] `ab milestones list/show/create/assign` commands
- [ ] Basic progress calculation
- [ ] Works with existing `bd` workflows

### Phase 2 (Enhanced Features)
- [ ] Burndown chart generation
- [ ] Release notes generation
- [ ] TUI Milestones view
- [ ] Cross-context milestone aggregation

### Phase 3 (Upstream Proposal)
- [ ] RFC written for beads project
- [ ] Submitted upstream
- [ ] Migration tooling (if accepted)

---

## Open Questions

### Q1: Milestone ID vs Version

Should we use bead ID or version for references?

```bash
# Option A: Bead ID
$ ab milestones assign task-123 ab-ms-1

# Option B: Version string
$ ab milestones assign task-123 v0.3.0
```

**Recommendation:** Support both. Version is more ergonomic, ID is unambiguous.

### Q2: Cross-Context Milestones

Can a milestone span multiple contexts?

**Option A:** Milestones are per-context
- Simple, clear ownership
- Can't track cross-repo releases

**Option B:** Milestones can reference beads from any context
- Requires milestone in one "primary" context
- Other contexts' beads reference via label

**Recommendation:** Start with per-context, add cross-context later

### Q3: Velocity/Estimation

Should milestones support story points or time estimates?

**Recommendation:** Not initially. Keep it simple - count beads. Add estimation later if needed.

---

## Appendix: Label Convention Reference

### Milestone Definition Labels

| Label | Format | Required | Example |
|-------|--------|----------|---------|
| `milestone` | literal | Yes | `milestone` |
| `target:DATE` | ISO date | Recommended | `target:2026-03-31` |
| `version:VER` | semver | Optional | `version:2.0.0` |
| `released:DATE` | ISO date | On completion | `released:2026-03-28` |

### Bead Assignment Labels

| Label | Format | Example |
|-------|--------|---------|
| `milestone:ID` | milestone ID or version | `milestone:v2.0` |

### Example Workflow

```bash
# 1. Create milestone
bd create --title="v2.0 Release" --type=epic \
  --labels="milestone,target:2026-03-31,version:2.0"

# 2. Create work items
bd create --title="Add OAuth" --type=feature \
  --labels="milestone:v2.0"

bd create --title="Refactor auth" --type=task \
  --labels="milestone:v2.0"

# 3. Track progress with ab
ab milestones show v2.0

# 4. On release, mark complete
bd close <milestone-id>
bd update <milestone-id> --labels="milestone,target:2026-03-31,version:2.0,released:2026-03-28"
```

---

*This spec proposes a convention-based approach that works with existing `bd` while enabling enhanced milestone features in `ab`. The approach is designed to be upward-compatible with potential native beads milestone support.*
