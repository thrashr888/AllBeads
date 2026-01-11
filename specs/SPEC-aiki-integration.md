# SPEC: AllBeads + Aiki Integration

**Status:** Draft / Ultrathink
**Author:** Claude Opus 4.5 + thrashr888
**Date:** 2026-01-11

---

## Executive Summary

AllBeads and Aiki solve complementary problems in AI-assisted development:

| System | Focus | Core Innovation |
|--------|-------|-----------------|
| **AllBeads** | *What* work is being done | Multi-repo issue orchestration with dependencies |
| **Aiki** | *How* work gets done | Edit-level provenance with autonomous review |

Together, they could provide **complete traceability from intent to implementation**: an issue (bead) tracks the goal, Aiki tracks every AI edit made toward that goal, and the integration links them with cryptographic proof.

---

## The Gap Each Solves

### What AllBeads Provides (That Aiki Lacks)

1. **Cross-Repository Coordination**
   - Beads span multiple repos with dependency graphs
   - Sheriff daemon synchronizes state across repos
   - Contexts aggregate work from distributed teams

2. **Work Intent Tracking**
   - Issues capture *why* work is happening
   - Dependencies track *what must happen first*
   - Priorities and status track *what matters now*

3. **Agent Messaging Infrastructure**
   - Agent Mail protocol (LOCK, UNLOCK, NOTIFY, REQUEST, BROADCAST)
   - Postmaster daemon for message routing
   - Cross-session coordination between AI agents

### What Aiki Provides (That AllBeads Lacks)

1. **Edit-Level Provenance**
   - Which agent made which change, when
   - Iteration history (attempt 1 failed, attempt 2 passed)
   - Confidence levels and detection methods

2. **Autonomous Review Loop**
   - Pre-commit quality gates (2-5 seconds)
   - AI self-correction without human intervention
   - Structured feedback that agents can read

3. **Cryptographic Verification**
   - GPG/SSH signing of AI-attributed changes
   - Tamper-proof audit trails
   - Enterprise compliance (SOX, PCI-DSS)

4. **Jujutsu Foundation**
   - Change-centric model (stable change IDs)
   - Working-copy-as-commit enables rapid iteration
   - Operation log captures full creative process

---

## Integration Architecture

### Option A: Loose Coupling (Recommended for Phase 1)

```
┌─────────────────────────────────────────────────────────────┐
│                     Developer Workflow                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   bd update <id> --status=in_progress                        │
│              ↓                                               │
│   ┌─────────────────┐                                        │
│   │   AllBeads CLI  │  ← Tracks active bead                  │
│   └────────┬────────┘                                        │
│            │ Sets AB_ACTIVE_BEAD env var                     │
│            ↓                                                 │
│   ┌─────────────────┐                                        │
│   │   AI Agent      │  ← Claude Code / Cursor                │
│   │   (editing)     │                                        │
│   └────────┬────────┘                                        │
│            │ PostToolUse hook                                │
│            ↓                                                 │
│   ┌─────────────────┐                                        │
│   │   Aiki Hook     │  ← Records provenance + bead_id        │
│   │   Handler       │                                        │
│   └────────┬────────┘                                        │
│            │ [aiki] block includes bead_id                   │
│            ↓                                                 │
│   ┌─────────────────┐                                        │
│   │   JJ Change     │  ← Provenance stored in description    │
│   │   Description   │                                        │
│   └─────────────────┘                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**How it works:**

1. Developer runs `bd update bead-123 --status=in_progress`
2. AllBeads sets `AB_ACTIVE_BEAD=bead-123` environment variable
3. AI agent makes edits (Claude Code, Cursor)
4. Aiki's hook handler reads `AB_ACTIVE_BEAD` from environment
5. Provenance block includes `bead_id=bead-123`
6. Later queries can find all changes associated with a bead

**Metadata Format:**
```
[aiki]
agent=claude-code
session=claude-session-abc123
tool=Edit
confidence=High
method=Hook
bead_id=bead-123
[/aiki]
```

**Benefits:**
- Minimal coupling between systems
- Works with existing hook infrastructure
- No daemon-to-daemon communication needed
- Each system remains independently useful

### Option B: Deep Integration (Future Phase)

```
┌───────────────────────────────────────────────────────────────┐
│                    Unified Agent Platform                      │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│   ┌──────────────┐         ┌──────────────┐                    │
│   │   AllBeads   │ ←─────→ │    Aiki      │                    │
│   │   Sheriff    │ Agent   │   Daemon     │                    │
│   │   Daemon     │ Mail    │              │                    │
│   └──────┬───────┘         └──────┬───────┘                    │
│          │                        │                            │
│          │   Shared SQLite        │                            │
│          │   + Agent Mail         │                            │
│          ↓                        ↓                            │
│   ┌─────────────────────────────────────────┐                  │
│   │         Unified Agent Coordinator        │                  │
│   │                                          │                  │
│   │  • Bead assignment + provenance linking  │                  │
│   │  • Autonomous review → bead status       │                  │
│   │  • Cross-repo change coordination        │                  │
│   │  • Quality gates per bead/epic           │                  │
│   └─────────────────────────────────────────┘                  │
│                                                                │
└───────────────────────────────────────────────────────────────┘
```

**Additional Capabilities:**
- Aiki publishes review results to Agent Mail
- AllBeads updates bead status based on review outcomes
- Sheriff coordinates Aiki provenance across repos
- Unified TUI showing both issues and provenance

---

## Use Cases

### Use Case 1: Automatic Bead-to-Change Linking

**Scenario:** Developer works on a feature tracked by bead `rk-456`

```bash
# Start work
$ ab update rk-456 --status=in_progress
Activated bead: rk-456 (RFC-049: Sync Package)

# AI agent makes changes (Aiki captures provenance)
# ... Claude Code edits files ...

# View what changed for this bead
$ aiki log --bead=rk-456
Changes for bead rk-456:

  abc12345 (Claude Code, 2 min ago)
    Modified: src/sync.rs (+47, -12)
    Review: PASSED (attempt 2)

  def67890 (Claude Code, 5 min ago)
    Modified: src/lib.rs (+3, -0)
    Review: PASSED (attempt 1)
```

**Value:** Complete traceability from issue to every edit made toward it

### Use Case 2: Quality Gates Per Epic

**Scenario:** High-priority epic requires stricter review

```yaml
# .aiki/config.yml (auto-detected from bead labels)
review:
  policies:
    - match:
        bead_labels: ["P0", "security"]
      require:
        autonomous_review: strict
        human_review: required
        test_coverage: ">80%"

    - match:
        bead_labels: ["P4", "chore"]
      require:
        autonomous_review: advisory
        human_review: optional
```

**Value:** Review strictness adapts to work priority automatically

### Use Case 3: Cross-Repo Provenance

**Scenario:** Change in `rookery` repo relates to bead in `ethertext` context

```bash
# AllBeads knows bead et-6tl depends on rk-ufu
$ ab show et-6tl
et-6tl: BLOCKED: Rookery Phase 1 must complete
  Blocked by: rk-ufu (in_progress)

# Developer works on rk-ufu in rookery repo
$ cd ~/Workspace/rookery
$ ab update rk-ufu --status=in_progress

# AI makes changes, Aiki records provenance with bead_id
# When rk-ufu closes, AllBeads can query Aiki for:
#   - Total changes made
#   - Review pass/fail ratio
#   - Agent attribution summary

$ ab show rk-ufu --provenance
rk-ufu: RFC-049/050: Sync Package
  Status: closed
  Provenance Summary (from Aiki):
    Total changes: 23
    Agents: Claude Code (21), Human (2)
    Reviews: 19 passed, 4 required iteration
    Time in review loop: 4m 32s (saved est. 2h)
```

**Value:** Cross-repo dependency tracking with per-bead provenance

### Use Case 4: Agent Mail for Review Events

**Scenario:** Aiki review failure notifies AllBeads to update status

```
Agent Mail Message:
  From: aiki-daemon@localhost
  To: allbeads-sheriff@localhost
  Type: NOTIFY
  Payload:
    event: review_failed
    bead_id: ab-123
    change_id: xyz789
    issues:
      - type: security
        severity: critical
        message: "API key hardcoded in auth.rs:45"
    attempts: 3
    recommendation: escalate_to_human

Sheriff Response:
  - Updates bead ab-123 status to "blocked"
  - Adds comment: "Aiki review failed: security issue detected"
  - Notifies assigned developer via preferred channel
```

**Value:** Automated status updates based on code quality

### Use Case 5: Enterprise Compliance Audit

**Scenario:** Auditor needs to verify all changes to payment code

```bash
# AllBeads: Find all beads touching payment
$ ab search --label=payment --status=closed
pay-001: Add Stripe integration
pay-002: Refactor checkout flow
pay-003: Fix currency rounding

# Aiki: Get signed provenance for each bead
$ aiki audit --bead=pay-001 --verify
Audit Report for bead pay-001

Changes: 47
  ✓ All changes cryptographically signed
  ✓ All signatures valid (GPG: user@company.com)

Attribution:
  Claude Code: 38 changes (81%)
  Human: 9 changes (19%)

Reviews:
  Passed on first attempt: 31
  Required iteration: 16
  Maximum iterations: 3

Compliance:
  ✓ All auth/* changes had human review
  ✓ No hardcoded secrets detected
  ✓ Test coverage >80% for all changes
```

**Value:** Regulatory-grade audit trail linking issues to verified changes

---

## Implementation Phases

### Phase 1: Environment Variable Bridge (2-3 weeks)

**AllBeads Changes:**
1. `bd update --status=in_progress` sets `AB_ACTIVE_BEAD` env var
2. `bd close` unsets the variable
3. Shell hook maintains variable across terminal sessions

**Aiki Changes:**
1. Hook handler reads `AB_ACTIVE_BEAD` if present
2. Includes `bead_id=<value>` in provenance block
3. `aiki log --bead=<id>` filters by bead

**Integration Test:**
```bash
$ ab update ab-123 --status=in_progress
$ echo $AB_ACTIVE_BEAD
ab-123
$ # ... AI makes changes ...
$ aiki log --bead=ab-123
# Shows all changes tagged with ab-123
```

### Phase 2: Provenance Queries (4-6 weeks)

**AllBeads Changes:**
1. `ab show <id> --provenance` queries Aiki for change summary
2. Provenance summary in TUI kanban view
3. Cross-repo aggregation of per-bead provenance

**Aiki Changes:**
1. `aiki summary --bead=<id>` returns structured summary
2. JSON output for programmatic consumption
3. Handles repos where Aiki is not initialized (graceful fallback)

### Phase 3: Agent Mail Integration (6-8 weeks)

**AllBeads Changes:**
1. Sheriff subscribes to Aiki events via Agent Mail
2. Auto-updates bead status based on review outcomes
3. Blocked status when review fails repeatedly

**Aiki Changes:**
1. Publishes review events to Agent Mail
2. Configurable event types (review_passed, review_failed, escalated)
3. Message format compatible with AllBeads protocol

### Phase 4: Unified Daemon (Future)

**Considerations:**
- Shared SQLite for provenance + issues
- Single daemon managing both JJ sync and bead sync
- Unified TUI with kanban + provenance views
- Common configuration format

---

## Technical Considerations

### JJ + Git Coexistence

Both systems use Git, but differently:
- **AllBeads:** Pure Git (`.git/`), beads stored in `.beads/` directory
- **Aiki:** JJ with internal Git backend (`.jj/repo/store/git`), non-colocated

**Resolution:** They don't conflict. Aiki's JJ is separate from the working Git repo. AllBeads continues using standard Git. The repos can have both `.beads/` (AllBeads) and `.jj/` (Aiki) directories.

### Session vs Bead Granularity

- **Aiki Session:** Single Claude Code session, may touch multiple beads
- **Bead:** Single issue, may span multiple sessions

**Resolution:** Bead ID is additional metadata, not replacement for session. A session tagged with `bead_id=X` means "this session was working on bead X." Multiple sessions can work on same bead.

### Multi-Repo Aggregation

AllBeads aggregates beads from multiple repos. Aiki's provenance is per-repo.

**Resolution:**
1. Each repo has its own Aiki installation
2. AllBeads queries each repo's Aiki independently
3. Sheriff aggregates provenance summaries into unified view
4. Cross-repo change correlation via shared bead IDs

### Performance

Both systems run daemons. Resource usage considerations:
- **AllBeads Sheriff:** Polls repos on interval (configurable, default 30s)
- **Aiki Hooks:** Triggered on each file edit (~7-8ms per edit)

**Resolution:**
- Hooks are lightweight, no performance concern
- Daemons could share process if deeply integrated
- Initial phases keep them separate (simpler)

---

## Open Questions

### Q1: Bidirectional or Unidirectional?

**Option A:** AllBeads → Aiki (AllBeads provides context, Aiki consumes)
- Simpler
- Aiki doesn't need to know about AllBeads
- Bead ID is just another metadata field

**Option B:** Bidirectional (Both systems share state)
- Richer integration
- Aiki can update bead status
- Requires Agent Mail or similar protocol

**Recommendation:** Start with Option A, evolve to B if valuable

### Q2: Single Binary or Separate Tools?

**Option A:** Separate binaries (`ab`, `aiki`)
- Each tool remains independently useful
- Simpler development and deployment
- Users install what they need

**Option B:** Single binary (`ab` subsumes `aiki` or vice versa)
- Unified experience
- Single install
- Tighter integration

**Recommendation:** Keep separate. Integration via environment variables and protocols, not binary merger.

### Q3: Who Owns the Daemon?

If we have Agent Mail integration, who runs the daemon?

**Option A:** Separate daemons communicating via Agent Mail
- Clean separation of concerns
- Either can run without the other
- More processes to manage

**Option B:** AllBeads Sheriff hosts Aiki as plugin
- Single daemon
- Aiki becomes AllBeads component
- Tighter coupling

**Recommendation:** Option A for now. Keep systems independent.

### Q4: Shared Configuration?

Should config files be shared or separate?

```yaml
# Option A: Separate configs
# ~/.config/allbeads/config.yaml
# ~/.config/aiki/config.yaml

# Option B: Nested config
# ~/.config/allbeads/config.yaml
aiki:
  enabled: true
  autonomous_review: strict

# Option C: Reference config
# ~/.config/allbeads/config.yaml
integrations:
  aiki:
    config_path: ~/.config/aiki/config.yaml
```

**Recommendation:** Option C - reference, don't duplicate

---

## Success Metrics

### Phase 1 (Environment Bridge)
- [ ] Bead ID appears in Aiki provenance for 100% of tracked edits
- [ ] `aiki log --bead=X` returns correct filtered results
- [ ] No performance regression in either tool

### Phase 2 (Provenance Queries)
- [ ] `ab show --provenance` works for all contexts
- [ ] Provenance summary accurate vs raw Aiki data
- [ ] <100ms latency for provenance queries

### Phase 3 (Agent Mail)
- [ ] Review events delivered reliably to Sheriff
- [ ] Bead status updates correctly on review outcomes
- [ ] No message loss under normal conditions

### Long-term
- [ ] Users report improved traceability
- [ ] Enterprise customers cite integration for compliance
- [ ] Reduced time debugging "who changed what and why"

---

## Appendix: Related Work

### Similar Integrations

**GitHub + Jira:** Bidirectional sync of issues ↔ PRs
- Lesson: Keep sync simple, handle conflicts gracefully

**Linear + GitHub:** Issue tracking + code review
- Lesson: Tight integration valued by teams

**Datadog + PagerDuty:** Monitoring → Alerting
- Lesson: Event-driven integration scales well

### Why Not Just Use Git Commit Messages?

Git commits already have messages. Why add Aiki provenance?

1. **Granularity:** Git commits are batched; Aiki tracks every edit
2. **Attribution:** Git author is human; Aiki tracks which AI agent
3. **Iteration:** Git shows final; Aiki shows attempts and corrections
4. **Verification:** Git commits can be rewritten; Aiki signs cryptographically

AllBeads linking to Aiki provenance is richer than linking to Git commits.

---

## Next Steps

1. **Spike (1 week):** Implement env var bridge in both tools
2. **Test (1 week):** Validate with real multi-session workflow
3. **Document (ongoing):** Update CLAUDE.md files in both repos
4. **Iterate:** Based on actual usage patterns

---

*This spec is exploratory. Implementation should validate assumptions with minimal investment before committing to deep integration.*
