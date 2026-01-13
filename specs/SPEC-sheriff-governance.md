# SPEC: Sheriff Governance - Cross-Repo Policy Enforcement

**Status:** Draft / Ultrathink
**Author:** Claude Opus 4.5 + thrashr888
**Date:** 2026-01-11
**Epic:** ab-???

---

## Executive Summary

The Sheriff daemon already synchronizes beads across repositories. This spec explores extending Sheriff into a **governance engine** that enforces policies, runs checks, and ensures compliance across an organization's entire repository fleet.

**Vision:** Sheriff becomes the "compliance backbone" for multi-repo AI development - ensuring every repo follows org standards, every bead meets quality gates, and every agent operates within defined boundaries.

---

## The Problem

### Current State

Organizations using AllBeads have:
- Multiple repositories (10-100+) with beads
- Different teams with varying standards
- No centralized policy enforcement
- Manual compliance checking
- No visibility into cross-repo health

### Pain Points

1. **Inconsistent Standards**
   - Repo A requires tests for all features
   - Repo B has no such requirement
   - No way to enforce org-wide policies

2. **Compliance Gaps**
   - Security-sensitive repos need stricter review
   - No automated enforcement
   - Audits require manual repo-by-repo inspection

3. **Visibility Deficit**
   - Can't see overall org health
   - No cross-repo metrics
   - Issues discovered late

4. **Agent Governance**
   - AI agents operate without guardrails
   - No policy on what agents can modify
   - No audit trail of agent actions across repos

---

## Solution: Sheriff as Governance Engine

### Core Concept

Sheriff evolves from "sync daemon" to "governance daemon":

```
┌────────────────────────────────────────────────────────────────┐
│                     Sheriff Governance Engine                   │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│   │   Policy    │  │   Checks    │  │  Reporting  │            │
│   │   Engine    │  │   Runner    │  │   Engine    │            │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
│          │                │                │                    │
│          └────────────────┼────────────────┘                    │
│                           │                                     │
│                    ┌──────┴──────┐                              │
│                    │   Sheriff   │                              │
│                    │   Daemon    │                              │
│                    └──────┬──────┘                              │
│                           │                                     │
│          ┌────────────────┼────────────────┐                    │
│          ↓                ↓                ↓                    │
│   ┌──────────┐     ┌──────────┐     ┌──────────┐               │
│   │  Repo A  │     │  Repo B  │     │  Repo C  │               │
│   │ .beads/  │     │ .beads/  │     │ .beads/  │               │
│   └──────────┘     └──────────┘     └──────────┘               │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

## Feature Set

### 1. Policy Engine

Define organizational policies in a central config:

```yaml
# ~/.config/allbeads/governance.yaml

organization: acme-corp

policies:
  # Global policies (apply to all repos)
  global:
    beads:
      require_priority: true
      require_type: true
      require_description_min_length: 50
      max_open_p0: 5  # Alert if >5 P0 beads open

    agents:
      allowed_agents: [claude-code, cursor, copilot]
      require_human_review_for:
        - paths: ["auth/*", "security/*", "payment/*"]
        - priority: P0

    workflow:
      require_blocking_reason: true
      max_wip_per_author: 3  # Work-in-progress limit
      stale_threshold_days: 14

  # Repo-specific overrides
  repos:
    "github.com/acme/billing":
      classification: critical
      inherit: global
      override:
        agents:
          require_human_review_for:
            - paths: ["*"]  # All paths need review
        compliance:
          require_sign_off: [security-team, billing-team]
          audit_retention_days: 2555  # 7 years for SOX

    "github.com/acme/docs":
      classification: low
      inherit: global
      override:
        beads:
          require_description_min_length: 0  # Relaxed for docs
        agents:
          require_human_review_for: []  # No review needed
```

### 2. Checks Runner

Automated checks that run on Sheriff sync:

```yaml
# Check definitions
checks:
  # Built-in checks
  builtin:
    - id: stale-beads
      description: "Find beads with no activity"
      threshold: 14 days
      severity: warning

    - id: orphan-beads
      description: "Find beads with no assignee"
      severity: info

    - id: blocked-chain
      description: "Find chains of blocked beads >3 deep"
      severity: warning

    - id: p0-count
      description: "Alert if P0 count exceeds limit"
      threshold: 5
      severity: critical

    - id: wip-limit
      description: "Check work-in-progress limits"
      per_author: 3
      severity: warning

  # Custom checks (scripts)
  custom:
    - id: security-review
      command: "./scripts/check-security-beads.sh"
      repos: ["billing", "auth"]
      severity: critical

    - id: test-coverage
      command: "./scripts/check-coverage.sh"
      args: ["--threshold", "80"]
      severity: warning
```

**Check execution:**

```bash
# Run all checks across all repos
$ ab sheriff check --all

Running governance checks...

✓ acme/api: 8 checks passed
✓ acme/frontend: 8 checks passed
⚠ acme/billing: 1 warning
    → stale-beads: 3 beads with no activity >14 days
✗ acme/auth: 1 critical
    → p0-count: 7 P0 beads open (limit: 5)

Summary: 32 passed, 1 warning, 1 critical
```

**Additional Built-in Check Types:**

```yaml
checks:
  builtin:
    # Sheriff sync health
    - id: sheriff-sync-lag
      description: "Repos not synced recently"
      threshold: 1 hour
      severity: warning

    - id: sheriff-sync-failures
      description: "Repos with sync errors"
      severity: critical

    # Janitor integration
    - id: janitor-security
      description: "Security issues found by janitor"
      severity: critical
      action: block-commits  # Optional enforcement

    - id: janitor-todos
      description: "Untracked TODOs in code"
      severity: info
      auto_create_beads: true  # Convert to beads

    - id: janitor-duplicates
      description: "Duplicate beads detected"
      severity: warning

    # Agent/Tool governance
    - id: mcp-allowlist
      description: "Only approved MCP servers allowed"
      allowed:
        - filesystem
        - github
        - slack
      severity: critical

    - id: skills-allowlist
      description: "Only approved Claude Code skills"
      allowed:
        - beads
        - allbeads
        - commit
      severity: warning

    - id: agent-allowlist
      description: "Only approved AI agents"
      allowed:
        - claude-code
        - cursor
      severity: critical

    # Repository health
    - id: dry-repos
      description: "Repos with no bead activity"
      threshold: 30 days  # No new beads in 30 days
      severity: info
      action: notify-owner

    - id: abandoned-repos
      description: "Repos with no commits or bead activity"
      threshold: 90 days
      severity: warning
      action: flag-for-archive
```

### 3. Compliance Reports

Generate compliance reports for auditors:

```bash
$ ab sheriff report --format=pdf --period=Q4-2025

Generating Q4 2025 Compliance Report...

┌─────────────────────────────────────────────────────────────┐
│           AllBeads Governance Report - Q4 2025              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Organization: acme-corp                                      │
│ Period: Oct 1, 2025 - Dec 31, 2025                          │
│ Repositories: 47                                             │
│ Total Beads: 1,234                                          │
│                                                              │
│ COMPLIANCE STATUS                                            │
│ ─────────────────                                            │
│ Policy Violations: 12 (resolved: 11, open: 1)               │
│ Check Failures: 89 (resolved: 87, open: 2)                  │
│ Audit Events: 4,567                                         │
│                                                              │
│ CRITICAL REPOSITORIES                                        │
│ ─────────────────────                                        │
│ billing: 100% compliant                                     │
│ auth: 98% compliant (2 stale beads)                         │
│ payment: 100% compliant                                     │
│                                                              │
│ AGENT ACTIVITY                                               │
│ ──────────────                                               │
│ Claude Code: 2,345 changes (approved: 2,301, rejected: 44)  │
│ Cursor: 1,234 changes (approved: 1,220, rejected: 14)       │
│ Human: 3,456 changes                                         │
│                                                              │
│ POLICY ENFORCEMENT                                           │
│ ──────────────────                                           │
│ Human review required: 234 changes                          │
│ Human review completed: 234 changes                         │
│ Average review time: 2.3 hours                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘

Report saved: governance-report-Q4-2025.pdf
```

### 4. Agent Guardrails

Control what AI agents can do:

```yaml
# Agent governance policies
agents:
  # Default agent permissions
  default:
    can_create_beads: true
    can_close_beads: false  # Humans close
    can_modify_priority: false
    can_modify_blocking: false
    max_files_per_change: 10

  # Per-agent overrides
  claude-code:
    inherit: default
    can_close_beads: true  # Trusted agent
    allowed_repos: ["*"]

  cursor:
    inherit: default
    allowed_repos: ["frontend", "docs"]  # Limited scope

  custom-agent:
    inherit: default
    allowed_repos: ["internal-tools"]
    require_approval_for:
      - create_bead
      - modify_any
```

**Enforcement:**

```bash
# Agent tries to close bead in restricted repo
$ AB_AGENT=cursor bd close billing-123

Error: Agent 'cursor' not authorized for repo 'billing'
  Policy: agents.cursor.allowed_repos = ["frontend", "docs"]

  Options:
    1. Use an authorized agent (claude-code)
    2. Request policy exception
    3. Have human close the bead
```

### 5. Onboarding Automation

Automate new repo setup:

```yaml
# Repo onboarding template
onboarding:
  template: standard

  steps:
    - name: Initialize beads
      action: bd init

    - name: Configure hooks
      action: install-hooks
      hooks: [pre-commit, post-commit]

    - name: Apply policies
      action: apply-policy
      policy: global

    - name: Create initial beads
      action: create-beads
      beads:
        - title: "Repository setup checklist"
          type: task
          priority: P1
          checklist:
            - "Configure CI/CD"
            - "Add CODEOWNERS"
            - "Set up branch protection"
            - "Add to AllBeads manifest"

    - name: Notify team
      action: notify
      channel: "#engineering"
      message: "New repo {repo} onboarded to AllBeads governance"
```

**Onboarding command:**

```bash
$ ab sheriff onboard github.com/acme/new-service

Onboarding new-service to AllBeads governance...

Step 1/5: Initialize beads
  ✓ Created .beads/ directory
  ✓ Configured git hooks

Step 2/5: Configure hooks
  ✓ Installed pre-commit hook
  ✓ Installed post-commit hook

Step 3/5: Apply policies
  ✓ Applied 'global' policy
  ✓ Repository classified as 'standard'

Step 4/5: Create initial beads
  ✓ Created ns-001: Repository setup checklist

Step 5/5: Notify team
  ✓ Posted to #engineering

Onboarding complete! new-service is now governed by AllBeads.
```

### 6. Drift Detection

Detect when repos drift from policy:

```bash
$ ab sheriff drift

Checking for policy drift...

✗ acme/api: DRIFTED
  - Missing required hook: pre-commit
  - Policy version: 1.2 (current: 1.5)
  - Last sync: 45 days ago

⚠ acme/frontend: PARTIAL
  - Policy version: 1.4 (current: 1.5)
  - Minor: beads.require_description_min_length changed

✓ acme/billing: COMPLIANT
✓ acme/auth: COMPLIANT

Drift Summary: 1 drifted, 1 partial, 2 compliant

Run 'ab sheriff remediate' to fix drift automatically.
```

### 7. Git Hooks for Proactive Policy Enforcement

**Goal:** Catch policy violations *before* they're committed, not after.

Instead of only running policy checks in Sheriff daemon (post-commit), integrate with git hooks to provide immediate feedback during the development workflow.

#### Architecture

```
Developer workflow with hooks:

┌─────────────────────────────────────────────────────────────┐
│  Developer: bd update beads-123 --status=in_progress        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Modified: .beads/issues.jsonl                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Developer: git commit -m "Update bead status"              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Git pre-commit hook triggers                               │
│  → Runs: bd check --pre-commit                              │
│  → Loads policies from .beads/policies.yaml                 │
│  → Runs all enabled policies against current state          │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────────┐
│  ✓ All policies pass    │     │  ✗ Policy violation         │
│  → Commit proceeds      │     │  → Commit BLOCKED           │
│  → Changes committed    │     │  → Show violations          │
└─────────────────────────┘     │  → Exit code 1              │
                                └─────────────────────────────┘
```

#### The `bd check` Command

New command for running policy checks on-demand:

```bash
# Check all policies against current state
$ bd check

Checking governance policies...

✓ require-description: PASS (128/128 beads)
✗ max-in-progress: FAIL (4/3 beads)
    Violation: User @thrashr888 has 4 beads in progress (max: 3)
    Beads: ab-001, ab-002, ab-003, ab-004
✓ dependency-cycle-check: PASS (no cycles detected)
✓ require-priority: PASS (128/128 beads)

Summary: 3 passed, 1 failed

Run with --fix to get suggestions for resolving violations.
```

**Command options:**

```bash
# Run in strict mode (exit non-zero on any violation)
bd check --strict

# Check specific policy only
bd check --policy=max-in-progress

# Check with auto-fix suggestions
bd check --fix

# Check in pre-commit mode (optimized, no output if passing)
bd check --pre-commit

# Check specific bead
bd check --bead=ab-123

# Output JSON for scripting
bd check --format=json
```

**Pre-commit mode behavior:**

```bash
$ bd check --pre-commit
# If passing: no output, exit 0
# If failing: brief error, exit 1

Error: Policy violations detected

✗ max-in-progress: You have 4 beads in progress (max: 3)
  → Close one of: ab-001, ab-002, ab-003, ab-004
  → Or run: bd check --fix for suggestions

Commit blocked. Fix violations and try again.
```

#### The `bd hooks install` Command

Install git hooks for policy enforcement:

```bash
# Install default hooks (pre-commit with bd check)
$ bd hooks install

Installing git hooks...

  ✓ Created .git/hooks/pre-commit
  ✓ Hook will run: bd check --pre-commit --strict

Hooks installed successfully.

Test the hook:
  bd update <bead-id> --status=in_progress
  git add .beads/issues.jsonl
  git commit -m "test"
```

**Command options:**

```bash
# Install specific hooks
bd hooks install --hook=pre-commit
bd hooks install --hook=commit-msg

# Install all available hooks
bd hooks install --all

# Show what would be installed (dry run)
bd hooks install --dry-run

# Uninstall hooks
bd hooks uninstall

# List installed hooks
bd hooks list

# Test hooks without committing
bd hooks test
```

**Hook types:**

| Hook | When | What it does |
|------|------|--------------|
| `pre-commit` | Before commit | Runs `bd check --pre-commit --strict` |
| `commit-msg` | After commit message entered | Validates bead references (e.g., `Fixes: ab-123`) |
| `post-commit` | After commit | Updates bead metadata (last_modified, commit_hash) |
| `pre-push` | Before push | Runs full policy check + sync validation |

#### Example: Blocking a Commit

**Scenario:** Developer tries to start 4th bead when max is 3

```bash
$ bd update ab-004 --status=in_progress
✓ Updated ab-004 status to in_progress

$ git add .beads/issues.jsonl

$ git commit -m "Start work on ab-004"

Running governance checks...

✗ Policy violation: max-in-progress

  Rule: max_in_progress (max_count: 3)
  Current: 4 beads in progress for @thrashr888

  Beads in progress:
    • ab-001 (P1): Implement user authentication
    • ab-002 (P2): Add caching layer
    • ab-003 (P2): Refactor API handlers
    • ab-004 (P3): Update documentation

  Resolution options:
    1. Close one of the above beads
    2. Adjust policy in .beads/policies.yaml (requires approval)
    3. Skip this check: git commit --no-verify (not recommended)

Commit blocked. Exit code: 1
```

**Resolution:**

```bash
$ bd close ab-003
✓ Closed ab-003

$ git add .beads/issues.jsonl

$ git commit -m "Start work on ab-004"

Running governance checks...
✓ All policies passed

[main a1b2c3d] Start work on ab-004
 1 file changed, 2 insertions(+), 1 deletion(-)
```

#### Hook Configuration

Hooks can be configured in `.beads/policies.yaml`:

```yaml
# .beads/policies.yaml

policies:
  - name: require-description
    enabled: true
    severity: error
    enforce_in_hooks: true  # Block commit if violated

  - name: max-in-progress
    enabled: true
    type: max_in_progress
    config:
      max_count: 3
    enforce_in_hooks: true

  - name: require-labels
    enabled: true
    type: require_labels
    config:
      min_count: 1
    enforce_in_hooks: false  # Warning only, don't block

hooks:
  # Hook-specific settings
  pre_commit:
    enabled: true
    policies: [require-description, max-in-progress]
    strict: true  # Exit non-zero on any violation
    timeout: 5s   # Max time for check

  commit_msg:
    enabled: true
    require_bead_reference: false

  post_commit:
    enabled: true
    auto_update_metadata: true

  pre_push:
    enabled: false
    run_full_check: true
```

#### Integration with Existing Hook System

This builds on the existing PluginHooks infrastructure:

```rust
// src/hooks/mod.rs

pub struct HookRunner {
    policy_checker: PolicyChecker,
    hook_config: HookConfig,
}

impl HookRunner {
    pub fn run_pre_commit(&self) -> Result<HookResult> {
        // Load current state
        let beads = load_beads()?;

        // Run policy checks
        let results = self.policy_checker.check_beads(&beads)?;

        // Filter by enforce_in_hooks
        let blocking_violations: Vec<_> = results
            .iter()
            .filter(|r| !r.passed && r.policy.enforce_in_hooks)
            .collect();

        if !blocking_violations.is_empty() {
            return Ok(HookResult::Blocked {
                violations: blocking_violations,
                exit_code: 1,
            });
        }

        Ok(HookResult::Allowed)
    }
}
```

#### CLI Commands Summary

```bash
# Check policies
bd check                              # Run all checks
bd check --strict                     # Exit non-zero on failure
bd check --policy=max-in-progress     # Check specific policy
bd check --pre-commit                 # Pre-commit mode (optimized)
bd check --fix                        # Show fix suggestions

# Manage hooks
bd hooks install                      # Install default hooks
bd hooks install --all                # Install all hooks
bd hooks uninstall                    # Remove hooks
bd hooks list                         # Show installed hooks
bd hooks test                         # Test hooks without committing

# Hook status (for sheriff drift detection)
bd hooks status                       # Check if hooks are installed
```

#### Benefits

1. **Immediate Feedback** - Developers know about violations before push
2. **Prevents Bad State** - Can't commit beads that violate policies
3. **Local Enforcement** - Works without Sheriff daemon running
4. **Configurable** - Choose which policies block vs warn
5. **Fast** - Pre-commit checks optimized for speed (<1s typical)
6. **Bypass Option** - `--no-verify` for emergencies (logged in audit)

#### Sheriff Integration

Sheriff daemon tracks hook compliance:

```bash
$ ab sheriff drift

Checking for policy drift...

✗ acme/api: DRIFTED
  - Missing required hook: pre-commit
  - Policy version: 1.2 (current: 1.5)

⚠ acme/frontend: HOOKS NOT INSTALLED
  - Install with: bd hooks install
  - Required by policy: governance.yaml

✓ acme/billing: COMPLIANT (hooks installed and passing)
✓ acme/auth: COMPLIANT (hooks installed and passing)
```

Sheriff can also auto-install hooks during onboarding:

```yaml
# Onboarding template
onboarding:
  steps:
    - name: Configure hooks
      action: install-hooks
      hooks: [pre-commit, post-commit]
      verify: true  # Test hooks before completing
```

---

## Architecture

### Sheriff Daemon Evolution

```
Current Sheriff:
┌─────────────────────────────────────────┐
│              Sheriff Daemon              │
├─────────────────────────────────────────┤
│  • Poll repos for bead changes          │
│  • Sync state to Boss repo              │
│  • Create shadow beads                  │
└─────────────────────────────────────────┘

Governance Sheriff:
┌─────────────────────────────────────────────────────────────┐
│                    Sheriff Governance Daemon                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Sync Engine │  │Policy Engine │  │ Check Runner │       │
│  │  (existing)  │  │   (new)      │  │    (new)     │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Agent Guard  │  │   Reporter   │  │  Onboarder   │       │
│  │    (new)     │  │    (new)     │  │    (new)     │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│                                                              │
│  ┌────────────────────────────────────────────────────┐     │
│  │              Event Bus (Agent Mail)                 │     │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Policy Evaluation Flow

```
1. Bead Operation Attempted
   ↓
2. Sheriff intercepts (via hook or API)
   ↓
3. Policy Engine evaluates:
   - Who is the actor? (human, agent, system)
   - What operation? (create, update, close)
   - Which repo/context?
   - What are the applicable policies?
   ↓
4. Decision:
   - ALLOW: Operation proceeds
   - DENY: Operation blocked with reason
   - REQUIRE_APPROVAL: Queued for review
   ↓
5. Audit log updated
   ↓
6. Metrics collected
```

### Storage

```
~/.config/allbeads/
├── governance.yaml          # Policy definitions
├── governance.db            # SQLite: audit log, metrics
├── checks/                  # Custom check scripts
│   ├── security-review.sh
│   └── test-coverage.sh
├── reports/                 # Generated reports
│   ├── 2025-Q4.pdf
│   └── 2025-Q4.json
└── templates/               # Onboarding templates
    └── standard.yaml
```

---

## CLI Commands

### Policy Management

```bash
# View current policies
ab sheriff policy list

# Validate policy file
ab sheriff policy validate governance.yaml

# Apply policy to repo
ab sheriff policy apply github.com/acme/api

# Check policy compliance
ab sheriff policy check --repo=api
```

### Governance Checks (Sheriff-level)

```bash
# Run all checks across all repos (Sheriff daemon)
ab sheriff check

# Run specific check
ab sheriff check --id=stale-beads

# Run checks for specific repo
ab sheriff check --repo=api

# Run in CI mode (exit code reflects status)
ab sheriff check --ci
```

### Policy Checks (Repo-level)

For individual repository use:

```bash
# Check policies in current repo
bd check

# Run in strict mode (exit non-zero on any violation)
bd check --strict

# Check specific policy only
bd check --policy=max-in-progress

# Check with auto-fix suggestions
bd check --fix

# Pre-commit mode (optimized, quiet if passing)
bd check --pre-commit

# Check specific bead
bd check --bead=ab-123

# Output formats
bd check --format=json
bd check --format=yaml
```

### Git Hooks Management

```bash
# Install default hooks
bd hooks install

# Install specific hook
bd hooks install --hook=pre-commit
bd hooks install --hook=commit-msg

# Install all hooks
bd hooks install --all

# Dry run (show what would be installed)
bd hooks install --dry-run

# Uninstall hooks
bd hooks uninstall
bd hooks uninstall --hook=pre-commit

# List installed hooks
bd hooks list

# Test hooks without committing
bd hooks test
bd hooks test --hook=pre-commit

# Check hook status
bd hooks status
```

### Reporting

```bash
# Generate compliance report
ab sheriff report --period=2025-Q4

# Generate for specific repos
ab sheriff report --repos=billing,auth,payment

# Different formats
ab sheriff report --format=pdf
ab sheriff report --format=html
ab sheriff report --format=json

# Scheduled reports (cron-friendly)
ab sheriff report --scheduled --email=compliance@acme.com
```

### Agent Management

```bash
# List registered agents
ab sheriff agents list

# View agent permissions
ab sheriff agents show claude-code

# Audit agent activity
ab sheriff agents audit --agent=cursor --period=7d

# Revoke agent access
ab sheriff agents revoke custom-agent --repo=billing
```

### Onboarding

```bash
# Onboard new repo
ab sheriff onboard github.com/acme/new-repo

# Onboard with specific template
ab sheriff onboard github.com/acme/new-repo --template=critical

# Dry run
ab sheriff onboard github.com/acme/new-repo --dry-run

# Batch onboard
ab sheriff onboard --from-manifest=repos.yaml
```

### Drift Management

```bash
# Check for drift
ab sheriff drift

# Auto-remediate drift
ab sheriff drift --fix

# Show detailed drift
ab sheriff drift --verbose
```

---

## Integration Points

### With Aiki (if integrated)

```yaml
# Governance can reference Aiki provenance
policies:
  repos:
    billing:
      aiki_integration:
        require_signed_changes: true
        require_review_pass: true
        max_review_iterations: 3
```

### With CI/CD

```yaml
# GitHub Actions example
name: AllBeads Governance
on: [push, pull_request]

jobs:
  governance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install AllBeads
        run: cargo install allbeads

      - name: Run governance checks
        run: ab sheriff check --ci

      - name: Upload report
        if: github.ref == 'refs/heads/main'
        run: ab sheriff report --format=json --output=governance.json
```

### With Slack/Teams

```yaml
# Notifications
notifications:
  slack:
    webhook: ${SLACK_WEBHOOK}
    channels:
      critical: "#security-alerts"
      warning: "#engineering"
      info: "#allbeads"

  events:
    - policy_violation: critical
    - check_failure: warning
    - onboarding_complete: info
```

---

## Implementation Phases

### Phase 0: Git Hooks Integration (1-2 weeks) **← CURRENT**

**Goal:** Enable proactive policy enforcement via git hooks

- [ ] `bd check` command implementation
  - [ ] Load policies from `.beads/policies.yaml`
  - [ ] Run all enabled policies
  - [ ] `--strict` mode (exit non-zero on violations)
  - [ ] `--pre-commit` mode (optimized, quiet if passing)
  - [ ] `--policy=NAME` to check specific policy
  - [ ] `--fix` to show resolution suggestions
  - [ ] `--format=json` for scripting
- [ ] `bd hooks` command implementation
  - [ ] `hooks install` - Install git hooks
  - [ ] `hooks uninstall` - Remove hooks
  - [ ] `hooks list` - Show installed hooks
  - [ ] `hooks test` - Test hooks without committing
  - [ ] `hooks status` - Check hook installation
- [ ] Hook script generation
  - [ ] `pre-commit` hook template
  - [ ] `commit-msg` hook template (bead reference validation)
  - [ ] `post-commit` hook template (metadata updates)
  - [ ] `pre-push` hook template (full checks)
- [ ] Hook configuration in policies.yaml
  - [ ] `enforce_in_hooks` flag per policy
  - [ ] `hooks` section with per-hook settings
  - [ ] Timeout configuration
- [ ] Testing and dogfooding
  - [ ] Install hooks in AllBeads repo
  - [ ] Test with actual workflow
  - [ ] Verify performance (<1s for typical checks)

### Phase 1: Policy Engine (4-6 weeks) **✓ COMPLETE**

- [x] Policy YAML schema definition
- [x] Policy loading and validation
- [x] Basic policy evaluation (allow/deny)
- [x] Audit logging to SQLite (PolicyStorage)
- [x] Built-in policy types (RequireDescription, MaxInProgress, etc.)
- [ ] `ab sheriff policy` commands (Sheriff-level, deferred)

### Phase 2: Checks Runner (4-6 weeks) **✓ COMPLETE**

- [x] Built-in check implementations (6 rules)
- [x] Policy checker integration with Sheriff
- [x] Check scheduling in daemon (poll cycle)
- [x] Check results storage (SQLite)
- [ ] Custom check script execution (deferred)
- [ ] `ab sheriff check` commands (Sheriff-level, deferred)
- [ ] CI mode with exit codes (deferred)

### Phase 3: Agent Guardrails (3-4 weeks)

- [ ] Agent identification in hooks
- [ ] Permission evaluation
- [ ] Approval queue for restricted ops
- [ ] `ab sheriff agents` commands

### Phase 4: Reporting (3-4 weeks)

- [ ] Report data aggregation
- [ ] PDF generation (via wkhtmltopdf or similar)
- [ ] HTML/JSON export
- [ ] Scheduled report cron
- [ ] Email delivery

### Phase 5: Onboarding & Drift (2-3 weeks)

- [ ] Onboarding templates
- [ ] Automated repo setup
- [ ] Drift detection algorithm
- [ ] Auto-remediation
- [ ] Notifications

---

## Success Metrics

### Adoption
- [ ] 10+ repos under governance
- [ ] 3+ different policy profiles in use
- [ ] Weekly governance checks running

### Compliance
- [ ] 95%+ policy compliance rate
- [ ] <24h mean time to remediate violations
- [ ] Zero undetected drift >7 days

### Efficiency
- [ ] 80% reduction in manual compliance checks
- [ ] Onboarding time <10 minutes per repo
- [ ] Reports generated in <5 minutes

---

## Open Questions

### Q1: Centralized vs Distributed Policies?

**Option A:** Single governance.yaml in Boss repo
- Pros: Single source of truth, easy to audit
- Cons: Requires Boss repo access for policy changes

**Option B:** Policy per repo with inheritance
- Pros: Repo teams can customize
- Cons: Drift risk, harder to audit

**Recommendation:** Hybrid - global policies centralized, repos can add (not remove) constraints

### Q2: Real-time vs Batch Checks?

**Option A:** Check on every bead operation
- Pros: Immediate feedback
- Cons: Performance overhead, complexity

**Option B:** Batch checks on Sheriff sync cycle
- Pros: Simple, predictable
- Cons: Delayed feedback

**Recommendation:** Critical checks real-time, others batch

### Q3: How Strict on Agent Guardrails?

**Option A:** Advisory only (warn but allow)
- Pros: Non-blocking, gradual adoption
- Cons: Violations still happen

**Option B:** Enforcing (block unauthorized actions)
- Pros: True governance
- Cons: May frustrate agents/developers

**Recommendation:** Configurable per policy (advisory/enforcing mode)

---

## Appendix: Example Policies

### Startup (Minimal Governance)

```yaml
organization: startup-inc
policies:
  global:
    beads:
      require_priority: true
    workflow:
      max_wip_per_author: 5
```

### Enterprise (Strict Governance)

```yaml
organization: megacorp
policies:
  global:
    beads:
      require_priority: true
      require_type: true
      require_description_min_length: 100
      require_acceptance_criteria: true
    agents:
      require_human_review_for:
        - priority: [P0, P1]
        - paths: ["**/security/**", "**/auth/**", "**/payment/**"]
    workflow:
      require_blocking_reason: true
      max_wip_per_author: 2
      stale_threshold_days: 7
    compliance:
      audit_retention_days: 2555
      require_sign_off: true

  repos:
    "billing":
      classification: sox-critical
      override:
        agents:
          allowed_agents: []  # No AI allowed
        compliance:
          require_sign_off: [cfo, security-lead]
```

### Open Source Project

```yaml
organization: oss-project
policies:
  global:
    beads:
      require_type: true
    agents:
      allowed_agents: ["*"]  # Any agent welcome
    workflow:
      stale_threshold_days: 30  # More relaxed
```

---

*This spec outlines Sheriff's evolution into a governance engine. Implementation should validate core policy evaluation before adding advanced features.*
