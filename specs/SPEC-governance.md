# SPEC-governance.md

## Overview

This specification defines the governance system for AllBeads, enabling centralized policy enforcement across managed repositories. The Boss repository imposes policies externally rather than relying on individual repos to manage their own compliance.

**Epic**: ab-zi5 - Sheriff enhancements for agent governance
**Related**: ab-tz3 (external policy), ab-mw8 (org scanner), ab-res (agent detection), ab-s79 (usage tracking)

## Design Principles

1. **External Imposition**: Policies are defined at the Boss level, not within individual repos
2. **Graduated Enforcement**: Advisory → Soft Mandatory → Hard Mandatory (per-policy, inspired by HCP Terraform)
3. **Agent Agnostic**: Support multiple AI agents (Claude, Copilot, Cursor, Aider, etc.)
4. **Override with Justification**: Soft mandatory can be overridden, but requires explicit justification
5. **Git-Native**: All policy state stored in git, syncs with Sheriff

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Boss Repository                          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  .allbeads/governance/                                  ││
│  │    policies.yaml       # Policy definitions             ││
│  │    exemptions.yaml     # Per-repo exemptions            ││
│  │    audit-log.jsonl     # Audit trail                    ││
│  └─────────────────────────────────────────────────────────┘│
│                              │                               │
│                    Sheriff Daemon                            │
│                              │                               │
│         ┌────────────────────┼────────────────────┐         │
│         ▼                    ▼                    ▼         │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   Rig A     │     │   Rig B     │     │   Rig C     │   │
│  │  (compliant)│     │ (warning)   │     │ (violation) │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Policy Model

### Policy Definition

Policies are defined in `~/.config/allbeads/governance/policies.yaml`:

```yaml
# Policy schema version
version: 1

# Global settings
settings:
  # How often Sheriff checks policies (in addition to sync)
  check_interval: 1h
  # Default enforcement for new policies
  default_enforcement: soft_mandatory

# Policy definitions
policies:
  # Require beads initialization
  require-beads:
    enabled: true
    enforcement: soft_mandatory  # advisory | soft_mandatory | hard_mandatory
    description: "Repository must have beads initialized"
    check:
      type: file_exists
      paths:
        - .beads/
        - .beads/issues.jsonl

  # Require agent configuration
  require-agent-config:
    enabled: true
    enforcement: advisory  # Nice to have, don't block
    description: "Repository must have agent configuration"
    check:
      type: file_exists_any
      paths:
        - CLAUDE.md
        - .claude/CLAUDE.md
        - agents.md
        - .github/copilot-instructions.md
        - .cursorrules

  # Require minimum onboarding score
  minimum-onboarding-score:
    enabled: true
    enforcement: advisory
    description: "Repository must meet minimum onboarding score"
    check:
      type: onboarding_score
      minimum: 50  # percentage

  # Restrict to approved agents only
  approved-agents-only:
    enabled: false  # opt-in
    enforcement: soft_mandatory
    description: "Only approved AI agents may be configured"
    check:
      type: agent_allowlist
      allowed:
        - claude
        - copilot
      denied:
        - cursor  # example: org doesn't allow Cursor

  # Require specific files
  require-readme:
    enabled: true
    enforcement: advisory  # Nice to have
    description: "Repository must have a README"
    check:
      type: file_exists_any
      paths:
        - README.md
        - README.rst
        - README.txt
        - README

  # No secrets in config
  no-secrets-in-config:
    enabled: true
    enforcement: hard_mandatory  # Security: never allow override
    description: "No secrets or tokens in configuration files"
    check:
      type: pattern_absent
      patterns:
        - "(?i)(api[_-]?key|secret|token|password)\\s*[:=]\\s*['\"][^'\"]{8,}"
      files:
        - "**/*.yaml"
        - "**/*.yml"
        - "**/*.json"
        - "**/*.toml"
      exclude:
        - "**/node_modules/**"
        - "**/.git/**"

  # Require CI/CD
  require-ci:
    enabled: false  # opt-in
    enforcement: advisory
    description: "Repository should have CI/CD configured"
    check:
      type: file_exists_any
      paths:
        - .github/workflows/*.yml
        - .github/workflows/*.yaml
        - .gitlab-ci.yml
        - Jenkinsfile
        - .circleci/config.yml
```

### Exemptions

Per-repo exemptions in `~/.config/allbeads/governance/exemptions.yaml`:

```yaml
exemptions:
  # Exempt a specific repo from a policy
  - repo: legacy-service
    policy: require-beads
    reason: "Legacy repo, will be deprecated Q2"
    expires: 2026-06-30
    approved_by: paul

  # Exempt multiple repos
  - repos:
      - docs-site
      - marketing-landing
    policy: require-agent-config
    reason: "Non-code repositories"
    approved_by: paul

  # Exempt from all policies (use sparingly)
  - repo: experiments
    policy: "*"
    reason: "Experimental sandbox"
    expires: 2026-03-01
```

## Agent Detection

### Supported Agents

| Agent | Detection Markers | Config Files |
|-------|-------------------|--------------|
| Claude | CLAUDE.md, .claude/, .claude-plugin/ | CLAUDE.md |
| Copilot | .github/copilot-instructions.md | copilot-instructions.md |
| Cursor | .cursorrules, .cursor/ | .cursorrules |
| Aider | .aider*, .aider.conf.yml | .aider.conf.yml |
| Cody | .cody/, cody.json | cody.json |
| Continue | .continue/, config.json | .continue/config.json |
| Windsurf | .windsurf/ | .windsurf/rules.md |

### Agent Detection Logic

```rust
pub struct AgentDetection {
    pub agent: AgentType,
    pub confidence: Confidence,  // High, Medium, Low
    pub config_path: Option<PathBuf>,
    pub evidence: Vec<String>,
}

pub enum AgentType {
    Claude,
    Copilot,
    Cursor,
    Aider,
    Cody,
    Continue,
    Windsurf,
    Unknown(String),
}

pub enum Confidence {
    High,    // Config file exists
    Medium,  // Indirect markers (lock files, etc.)
    Low,     // Heuristic detection
}
```

### Detection Implementation

```rust
/// Detect agents in a repository
pub fn detect_agents(repo_path: &Path) -> Vec<AgentDetection> {
    let mut detections = Vec::new();

    // Claude detection
    if repo_path.join("CLAUDE.md").exists()
        || repo_path.join(".claude").exists()
    {
        detections.push(AgentDetection {
            agent: AgentType::Claude,
            confidence: Confidence::High,
            config_path: find_claude_config(repo_path),
            evidence: vec!["CLAUDE.md or .claude/ found".into()],
        });
    }

    // Copilot detection
    if repo_path.join(".github/copilot-instructions.md").exists() {
        detections.push(AgentDetection {
            agent: AgentType::Copilot,
            confidence: Confidence::High,
            config_path: Some(repo_path.join(".github/copilot-instructions.md")),
            evidence: vec!["copilot-instructions.md found".into()],
        });
    }

    // ... other agents

    detections
}
```

## GitHub Organization Scanner

### Purpose

Scan a GitHub user or organization to identify:
1. Repositories not yet managed by AllBeads
2. Repositories with agent configurations (adoption opportunities)
3. Repository metadata for prioritization

### Command Interface

```bash
# Scan user's repos
ab scan github --user thrashr888

# Scan organization
ab scan github --org mycompany

# Filter options
ab scan github --org mycompany \
  --min-stars 5 \
  --language rust \
  --activity 90d \
  --exclude-forks \
  --exclude-archived

# Output formats
ab scan github --org mycompany --json
ab scan github --org mycompany --csv
```

### Scanner Output

```
GitHub Organization Scan: mycompany
═══════════════════════════════════════════════════════════════

Found 47 repositories, 12 already managed by AllBeads

Unmanaged Repositories (35):
  Priority: High (has agent config, active)
  ├── api-gateway          ★42  Rust   Updated 2d ago   [Claude, Copilot]
  ├── auth-service         ★28  Go     Updated 5d ago   [Copilot]
  └── data-pipeline        ★15  Python Updated 1d ago   [Claude]

  Priority: Medium (active, no agent config)
  ├── frontend-app         ★89  TS     Updated 1d ago
  ├── mobile-sdk           ★34  Swift  Updated 3d ago
  └── ... (8 more)

  Priority: Low (inactive or small)
  ├── old-prototype        ★2   JS     Updated 180d ago
  └── ... (19 more)

Recommendations:
  • 3 repos have Claude config - consider onboarding first
  • 5 active repos have no agent config - opportunity for adoption
  • 19 repos inactive >90 days - consider archiving

Run: ab onboard <repo> to start onboarding
```

### Scanner Data Model

```rust
pub struct ScanResult {
    pub timestamp: DateTime<Utc>,
    pub source: ScanSource,  // GitHub user/org
    pub repositories: Vec<ScannedRepo>,
    pub summary: ScanSummary,
}

pub struct ScannedRepo {
    pub name: String,
    pub full_name: String,  // org/repo
    pub url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub is_fork: bool,
    pub is_archived: bool,
    pub last_push: DateTime<Utc>,
    pub created_at: DateTime<Utc>,

    // AllBeads-specific
    pub managed: bool,           // In AllBeads contexts?
    pub detected_agents: Vec<AgentType>,
    pub onboarding_priority: Priority,
}

pub enum Priority {
    High,    // Has agents, active
    Medium,  // Active, no agents
    Low,     // Inactive or small
    Skip,    // Archived, fork, etc.
}
```

## Policy Enforcement

### Check Execution

Sheriff runs policy checks:
1. During each sync cycle
2. On-demand via `ab governance check`
3. As pre-commit hook (optional)

### Enforcement Levels

Inspired by HCP Terraform's policy enforcement model:

| Level | Behavior | Override | Use Case |
|-------|----------|----------|----------|
| **Advisory** | Warn only, never blocks | N/A | "Nice to have" (require-readme) |
| **Soft Mandatory** | Blocks by default, can override | `--override` flag | Standard policies (require-beads) |
| **Hard Mandatory** | Always blocks, no override | None | Security (no-secrets-in-config) |

Enforcement is defined **per-policy** because different policies have genuinely different importance levels. A missing README is not the same as exposed secrets.

**Run-level modifiers** for CI/development scenarios:

```bash
# Normal: respects each policy's enforcement level
ab governance check

# Advisory mode: treat all policies as advisory (CI dry-run)
ab governance check --advisory-only

# Strict mode: treat soft mandatory as hard mandatory (pre-merge gates)
ab governance check --strict

# Override soft mandatory violations (with justification)
ab governance check --override="Deploying hotfix, will address in follow-up"
```

### Violation Handling

```rust
pub struct PolicyViolation {
    pub policy_id: String,
    pub repo: String,
    pub enforcement: Enforcement,
    pub message: String,
    pub details: Option<String>,
    pub remediation: Option<String>,
    pub detected_at: DateTime<Utc>,
}

pub enum Enforcement {
    Advisory,       // Log and continue, never blocks
    SoftMandatory,  // Blocks unless --override provided
    HardMandatory,  // Always blocks, no override possible
}
```

### Audit Trail

All policy checks logged to `~/.config/allbeads/governance/audit-log.jsonl`:

```jsonl
{"ts":"2026-01-14T15:30:00Z","type":"check","repo":"api-gateway","policy":"require-beads","result":"pass"}
{"ts":"2026-01-14T15:30:00Z","type":"check","repo":"frontend","policy":"require-beads","result":"fail","severity":"warning"}
{"ts":"2026-01-14T15:30:01Z","type":"exemption_applied","repo":"legacy","policy":"require-beads","expires":"2026-06-30"}
```

## CLI Commands

### Governance Commands

```bash
# Check all repos against policies
ab governance check

# Check specific repo
ab governance check --repo api-gateway

# Show policy status
ab governance status

# List violations
ab governance violations
ab governance violations --severity error

# Add exemption
ab governance exempt api-gateway require-beads \
  --reason "Legacy system" \
  --expires 2026-06-30

# Remove exemption
ab governance unexempt api-gateway require-beads

# Show audit log
ab governance audit
ab governance audit --repo api-gateway --days 7
```

### Agent Commands

```bash
# Detect agents in a repo
ab agents detect /path/to/repo
ab agents detect .

# List agents across all contexts
ab agents list

# Show agent usage summary
ab agents summary
```

### Scan Commands

```bash
# Scan GitHub for repos
ab scan github --user thrashr888
ab scan github --org mycompany

# Compare with managed repos
ab scan compare

# Generate onboarding recommendations
ab scan recommend
```

## TUI Integration

### Governance Tab

Add a Governance tab to the TUI showing:

```
┌─ Governance ─────────────────────────────────────────────────┐
│                                                              │
│  Policy Status                    Violations                 │
│  ══════════════                   ══════════                 │
│  ✓ require-beads: 12/15 pass      ⚠ frontend: require-beads  │
│  ✓ require-agent: 10/15 pass      ⚠ mobile: require-beads    │
│  ○ approved-agents: disabled      ⚠ utils: require-beads     │
│  ✓ require-readme: 15/15 pass     ⚠ frontend: require-agent  │
│                                   ⚠ mobile: require-agent    │
│                                                              │
│  Agent Adoption                   Unmanaged Repos            │
│  ══════════════                   ═══════════════            │
│  Claude:  8 repos                 api-gateway (Claude)       │
│  Copilot: 5 repos                 auth-service (Copilot)     │
│  Cursor:  2 repos                 data-pipeline              │
│  None:    5 repos                 + 32 more...               │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Agent Detection (ab-res)
- Implement agent detection logic for all supported agents
- Add `ab agents detect` command
- Store detection results in context metadata

### Phase 2: Policy Framework (ab-tz3)
- Create policy YAML schema and parser
- Implement core policy checks (file_exists, pattern_absent)
- Add `ab governance check` command
- Add exemption support

### Phase 3: GitHub Scanner (ab-mw8)
- Implement GitHub API integration for org/user scanning
- Add priority scoring algorithm
- Add `ab scan github` command
- Generate onboarding recommendations

### Phase 4: Usage Tracking (ab-s79)
- Track agent detection over time
- Build adoption metrics
- Add `ab agents summary` command
- Sheriff integration for continuous monitoring

### Phase 5: TUI & Enforcement
- Add Governance tab to TUI
- Implement configurable blocking
- Add audit log viewer

## Configuration

### Default Policies Location

```
~/.config/allbeads/governance/
├── policies.yaml      # Policy definitions
├── exemptions.yaml    # Per-repo exemptions
└── audit-log.jsonl    # Audit trail
```

### Environment Variables

```bash
# Skip governance checks (for CI)
AB_SKIP_GOVERNANCE=1

# Custom policies path
AB_GOVERNANCE_PATH=/path/to/governance/

# GitHub token for scanning
GITHUB_TOKEN=ghp_xxx
```

## Success Metrics

1. **Visibility**: 100% of managed repos have governance status visible
2. **Adoption**: Track agent adoption rate across repos over time
3. **Compliance**: Measure policy compliance rates
4. **Coverage**: % of org repos managed by AllBeads

## Resolved Design Decisions

1. **Enforcement Model**: Per-policy enforcement with run-level override
   - Advisory: warn only, never blocks
   - Soft Mandatory: blocks by default, can override with justification
   - Hard Mandatory: always blocks, no override (security policies)
   - Inspired by HCP Terraform's policy enforcement model

2. **Real-time vs Batch**: Batch during sync, on-demand for CLI
   - Sheriff checks policies during each sync cycle
   - `ab governance check` for ad-hoc checks

3. **GitHub Enterprise**: Yes, reuse existing auth strategy pattern
   - Works with both github.com and GHE instances

## Open Questions

1. **Policy Inheritance**: Should policies support inheritance/composition?
   - Recommendation: Keep simple for v1, consider for v2

## Dependencies

- GitHub API access (for org scanning)
- Existing AllBeads context/config infrastructure
- Sheriff daemon for continuous monitoring
