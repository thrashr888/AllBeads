# SPEC-onboarding: Repository Onboarding & Offboarding Workflows

**Status**: Draft
**Created**: 2026-01-14
**Epic**: ab-pp6i (Onboard workflow improvements)
**Related**: ab-49s (Beads uninstaller)

## Problem Statement

Currently, onboarding a repository into the AllBeads ecosystem requires manual steps:
1. Clone the repository
2. Run `bd init` in the repo directory
3. Manually add context to `~/.config/allbeads/config.yaml`
4. Manually configure .claude/settings.json for skills
5. Manually install Git hooks
6. Optionally configure integrations (JIRA/GitHub)

This manual process is error-prone, inconsistent, and doesn't guide users through best practices. Similarly, there's no clean way to offboard a repository when it's no longer needed.

## Goals

1. **Streamlined Onboarding**: Single command to onboard any repository
2. **Interactive Guidance**: Step-by-step wizard with sensible defaults
3. **Complete Setup**: Achieve 100% onboarding score (BSICH) where applicable
4. **Flexible Input**: Support URL, local path, or current directory
5. **Clean Offboarding**: Multiple levels of removal based on user intent

## Non-Goals

- Creating new GitHub repositories (separate feature: ab-k2g)
- Managing CI/CD configuration (out of scope)
- Language-specific skill recommendations (requires agentic analysis)
- Replacing or duplicating beads' initialization system

## Separation of Concerns: AllBeads vs Beads

**Beads Responsibilities (Rig Level):**
- Repository-level issue tracking (`.beads/` directory)
- Database management (SQLite or JSONL-only modes)
- Git hooks for auto-sync (pre-commit, post-commit)
- Merge drivers for conflict resolution
- Local workflow (create, update, close issues)
- Initialization via `bd init` with multiple modes

**AllBeads Responsibilities (Boss Level):**
- Multi-repository aggregation and federation
- Context management (adding repos to AllBeads config)
- Boss-level integrations (JIRA/GitHub sync across repos)
- Skills marketplace configuration (.claude/settings.json)
- Cross-repo dashboard (TUI)
- Clone automation and workspace management

**Key Principle**: AllBeads delegates to `bd init` for all Rig-level setup (beads, hooks, database). AllBeads focuses on Boss-level orchestration (multi-repo aggregation, integrations, visualization).

## Solution Overview

### Onboarding Command

```bash
# Primary interface
ab onboard <target> [options]

# Examples
ab onboard https://github.com/thrashr888/my-project
ab onboard git@github.com:thrashr888/my-project.git
ab onboard .
ab onboard /path/to/repo

# Options
--non-interactive    # Use defaults, no prompts
--skip-clone         # Assume already cloned
--skip-skills        # Don't add marketplace skills
--skip-hooks         # Don't install Git hooks
--skip-issues        # Don't import/scan for issues
--context-name NAME  # Override auto-detected name
```

### Offboarding Command

```bash
# Primary interface
ab offboard <context-name> [options]

# Examples
ab offboard my-project
ab offboard my-project --level=soft
ab offboard my-project --level=hard

# Levels
--level=soft    # Remove from AllBeads config only (default)
--level=medium  # Remove config + hooks, keep .beads/
--level=hard    # Remove everything including .beads/ directory
```

## Detailed Workflow

### Stage 1: Discovery & Validation

**Input Types:**
- GitHub URL (HTTPS or SSH)
- GitHub Enterprise URL (e.g., github.ibm.com)
- Local path (absolute or relative)
- Current directory (`.`)

**Validation:**
1. Parse URL/path to extract org/repo/local location
2. Verify Git repository (check for .git/)
3. Detect if already onboarded (check AllBeads config)
4. Detect organization from URL

**Output:**
```
→ Detected repository: thrashr888/my-project
→ Organization: thrashr888
→ Location: https://github.com/thrashr888/my-project
```

### Stage 2: Clone (if needed)

**Detection:**
- If URL provided and not found locally → clone
- If local path provided and exists → skip
- If current directory → skip

**Interactive Prompt:**
```
Repository not found locally.
Clone to: /Users/thrashr888/Workspace/my-project
  [y] Yes (recommended)
  [c] Choose different path
  [s] Skip (I'll clone manually)
```

**Non-interactive:**
- Auto-clone to `~/Workspace/<repo-name>` or configured workspace directory

**Output:**
```
→ Cloning to /Users/thrashr888/Workspace/my-project...
✓ Cloned successfully
```

### Stage 3: Initialize Beads

**Purpose:**
Delegate to `bd init` to let beads handle its own initialization with user-chosen mode.

**Detection:**
- Check for `.beads/` directory
- If exists, ask to reinitialize or skip
- Check if `bd` CLI is available

**Interactive Prompt:**
```
Initialize beads tracking?
  [1] Standard mode (SQLite database + git hooks)
  [2] No-DB mode (JSONL only, no SQLite)
  [3] Stealth mode (personal, git-ignored)
  [4] Team mode (team workflow setup)
  [5] Skip beads setup

Choice [1]:
```

**Actions:**
1. Based on user choice, run one of:
   - `bd init` (standard)
   - `bd init --no-db` (no database)
   - `bd init --stealth` (personal mode)
   - `bd init --team` (team workflow wizard)
2. Beads handles:
   - Creating `.beads/` directory structure
   - Initializing `issues.jsonl` and/or SQLite database
   - Installing git hooks (pre-commit, post-commit for auto-sync)
   - Setting up merge drivers
3. AllBeads only observes the result

**Non-interactive:**
- Default to `bd init --quiet` (standard mode)

**Output:**
```
→ Running: bd init
→ Initializing beads (.beads/ directory)...
→ Installing git hooks (auto-sync enabled)...
✓ Beads initialized (standard mode)
→ Current status: [B]eads ✓  [S]kills ✗  [I]ntegrations ✗  [C]I/CD ?  [H]ooks ✓
```

**Note on Hooks:**
Beads installs its own git hooks for database syncing. AllBeads doesn't need additional hooks at the Rig (repository) level - AllBeads operates at the Boss level to aggregate across multiple Rigs.

### Stage 4: Populate Issues (Interactive)

**Detection:**
- Check for GitHub Issues (requires GitHub API)
- Check for JIRA integration possibility
- Analyze repository with Janitor for potential issues

**Interactive Prompt:**
```
Would you like to populate initial issues?
  [1] Import from GitHub Issues (found 23 issues)
  [2] Import from JIRA (requires configuration)
  [3] Scan with Janitor for potential issues
  [4] Start with empty state
  [5] Skip for now

Choice [4]:
```

**Non-interactive:**
- Default to empty state

**For Option 1 (GitHub Import):**
```
→ You selected: [1] Import from GitHub Issues
→ Importing 23 issues...
  • Converting GitHub Issue #123 → beads-abc1
  • Converting GitHub Issue #124 → beads-abc2
  ...
✓ Imported 23 issues (15 open, 8 closed)
```

**For Option 3 (Janitor Scan):**
```
→ You selected: [3] Scan with Janitor
→ Analyzing repository...
→ Found potential issues:
  • TODO comments (12 found)
  • Missing documentation (3 files)
  • Code smells (5 locations)
Create issues for these? [y/n]: y
✓ Created 20 issues from analysis
```

### Stage 5: Add Skills (Marketplace Configuration)

**Purpose:**
Configure `.claude/settings.json` with AllBeads and Beads marketplaces so Claude Code can access skills.

**Detection:**
- Check if `.claude/` directory exists
- Check if `settings.json` exists
- Check if marketplaces already configured

**Interactive Prompt:**
```
Would you like to add AllBeads skill marketplaces?
This allows Claude Code to use 'bd' commands and AllBeads features.
  [y] Yes (recommended)
  [n] Skip for now

Choice [y]:
```

**Actions:**
1. Create `.claude/` directory if missing
2. Create or update `settings.json` with:
   ```json
   {
     "enabledPlugins": {
       "open-prose@prose": true,
       "allbeads@allbeads-marketplace": true,
       "beads@beads-marketplace": true
     },
     "extraKnownMarketplaces": {
       "allbeads-marketplace": {
         "source": {
           "source": "github",
           "repo": "thrashr888/AllBeads"
         }
       },
       "beads-marketplace": {
         "source": {
           "source": "github",
           "repo": "steveyegge/beads"
         }
       }
     }
   }
   ```

**Output:**
```
→ Creating .claude/ directory...
→ Adding AllBeads and Beads marketplaces to settings.json...
✓ Skills configured
→ Current status: [B]eads ✓  [S]kills ✓  [I]ntegrations ✗  [C]I/CD ?  [H]ooks ✗
```

**Note on Language-Specific Skills:**
Language or project-specific skills (rust-analyzer, testing frameworks, etc.) are NOT automatically added as this requires agentic analysis. Users can add these manually or through Claude Code skills later.

### Stage 6: Integrations (Optional)

**Detection:**
- Check if GitHub integration already configured
- Check if JIRA integration already configured
- Check repository for .jira or similar config files

**Interactive Prompt:**
```
Would you like to configure external integrations?
  [g] GitHub Issues sync
  [j] JIRA sync
  [n] Skip (you can add later with 'ab github' or 'ab jira')

Choice [n]:
```

**Actions for GitHub:**
```
→ You selected: [g] GitHub Issues
→ Configuring GitHub integration...
  GitHub URL: https://github.com
  Owner: thrashr888
  Repo: my-project
  Token: (using GITHUB_TOKEN env var)
→ Testing connection...
✓ GitHub integration configured
```

**Actions for JIRA:**
```
→ You selected: [j] JIRA
→ Configuring JIRA integration...
  JIRA URL: https://mycompany.atlassian.net
  Project Key: PROJ
  Token: (using JIRA_TOKEN env var)
→ Testing connection...
✓ JIRA integration configured
```

**Default Behavior:**
Skip integrations by default. They can be added later.

**Output:**
```
→ Skipping integrations (you can add later)
→ Current status: [B]eads ✓  [S]kills ✓  [I]ntegrations ✗  [C]I/CD ?  [H]ooks ✓
```

### Stage 7: CI/CD Detection

**Detection:**
- Check for `.github/workflows/*.yml`
- Do NOT attempt to create or modify CI/CD

**Output:**
```
→ Detected CI/CD: GitHub Actions (3 workflows)
→ Current status: [B]eads ✓  [S]kills ✓  [I]ntegrations ✗  [C]I/CD ✓  [H]ooks ✓
```

**Note:**
CI/CD is informational only. AllBeads does not create or manage CI/CD configurations as it's highly project-specific.

### Stage 8: Add to AllBeads Config

**Actions:**
1. Extract context name from repo name (or use --context-name override)
2. Detect auth strategy based on URL scheme:
   - SSH URL (git@) → SshAgent
   - HTTPS URL → PersonalAccessToken or GhEnterpriseToken
3. Add context to `~/.config/allbeads/config.yaml`:
   ```yaml
   contexts:
     - name: my-project
       type: git
       url: https://github.com/thrashr888/my-project.git
       path: /Users/thrashr888/Workspace/my-project
       auth_strategy: ssh_agent
       integrations: {}
   ```

**Interactive Prompt:**
```
→ Adding context to AllBeads config...
  Context name: my-project
  Organization: thrashr888
  Auth strategy: [auto-detected: SSH Agent]

  Is this correct? [y/n]: y
```

**Output:**
```
✓ Context added to AllBeads config
```

### Stage 9: Summary & Next Steps

**Output:**
```
✓ Onboarding complete!

  Repository:      thrashr888/my-project
  Local Path:      /Users/thrashr888/Workspace/my-project
  Organization:    thrashr888

  Status:          [B]eads ✓  [S]kills ✓  [I]ntegrations ✗  [C]I/CD ✓  [H]ooks ✓
  Onboarding Score: 85% (Integration optional)

  Next steps:
    • Create your first issue:  bd create --title="Initial setup"
    • View all contexts:        ab tui (then Tab to Contexts view)
    • Add GitHub integration:   ab github status
    • Add JIRA integration:     ab jira status
```

## Offboarding Workflow

### Offboarding Levels

**Level: soft (default)**
- Remove context from `~/.config/allbeads/config.yaml`
- Leave all repository files intact
- Use case: Stop tracking in AllBeads but keep using beads locally

**Level: medium**
- Remove context from config
- Remove Git hooks (.git/hooks/pre-commit, post-commit)
- Leave `.beads/` directory intact
- Use case: Stop AllBeads automation but preserve issue history

**Level: hard**
- Remove context from config
- Remove Git hooks
- Remove `.beads/` directory entirely
- Remove `.claude/settings.json` AllBeads configuration
- Use case: Complete removal, no traces left

### Offboarding Command Flow

```bash
ab offboard my-project
```

**Stage 1: Verify Context**
```
→ Found context: my-project
  Path: /Users/thrashr888/Workspace/my-project
  Issues: 23 (15 open, 8 closed)
```

**Stage 2: Confirm Removal**
```
This will remove my-project from AllBeads.
Choose removal level:
  [1] Soft   - Remove from config only (keep everything in repo)
  [2] Medium - Remove config + hooks (keep .beads/)
  [3] Hard   - Remove everything including .beads/
  [c] Cancel

Choice [1]: 2
```

**Stage 3: Execute Removal (Medium Level Example)**
```
→ Offboarding my-project (medium level)...

  [1/3] Removing from AllBeads config...
        ✓ Context removed from ~/.config/allbeads/config.yaml

  [2/3] Removing Git hooks...
        ✓ Removed .git/hooks/pre-commit
        ✓ Removed .git/hooks/post-commit

  [3/3] Keeping .beads/ directory (23 issues preserved)
        → You can still use 'bd' commands in this repository

✓ Offboarding complete

  Removed:     Config, Hooks
  Preserved:   .beads/ (23 issues), .claude/settings.json

  To fully remove AllBeads, run:
    ab offboard my-project --level=hard
```

**Stage 4: Hard Level Confirmation (if selected)**
```
⚠️  WARNING: Hard removal will DELETE 23 issues permanently!

Type the context name to confirm: my-project

→ Offboarding my-project (hard level)...

  [1/4] Removing from AllBeads config...
        ✓ Context removed

  [2/4] Removing Git hooks...
        ✓ Hooks removed

  [3/4] Removing .beads/ directory...
        ⚠️  Deleting 23 issues
        ✓ .beads/ directory removed

  [4/4] Cleaning .claude/settings.json...
        ✓ Removed AllBeads marketplaces

✓ Offboarding complete (hard level)

  All AllBeads traces removed from repository.
```

## Implementation Details

### Command Structure

**New CLI Commands:**
- `ab onboard <target> [options]`
- `ab offboard <context-name> [options]`

**Location:**
- `src/cli/onboard.rs` - Onboarding workflow
- `src/cli/offboard.rs` - Offboarding workflow

### Data Flow

**Onboarding:**
```
User Input → Discovery → Clone → Init Beads (bd init handles hooks) → Import Issues →
Configure Skills → Integrations → Detect CI/CD → Add Config → Summary
```

**Offboarding:**
```
User Input → Verify Context → Confirm Level → Remove Config →
Remove Beads (optional, includes hooks) → Clean Claude Config (optional) → Summary
```

### Configuration Changes

**AllBeadsConfig additions:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllBeadsConfig {
    /// Boss repository contexts (work, personal, etc.)
    pub contexts: Vec<BossContext>,

    /// Agent Mail configuration
    #[serde(default)]
    pub agent_mail: AgentMailConfig,

    /// Visualization settings
    #[serde(default)]
    pub visualization: VisualizationConfig,

    /// Default workspace directory for cloning repositories
    /// Defaults to ~/Workspace if not specified
    #[serde(default = "default_workspace_dir")]
    pub workspace_directory: PathBuf,
}

fn default_workspace_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("Workspace");
    path
}

impl AllBeadsConfig {
    /// Add a new context from onboarding
    pub fn add_context(&mut self, context: BossContext) -> Result<()>;

    /// Remove a context by name
    pub fn remove_context(&mut self, name: &str) -> Result<()>;

    /// Get the workspace directory (with fallback to default)
    pub fn workspace_directory(&self) -> &Path {
        &self.workspace_directory
    }
}
```

**BossContext additions:**
```rust
impl BossContext {
    /// Create from repository URL with auto-detection
    pub fn from_url(url: &str, local_path: Option<PathBuf>) -> Result<Self>;

    /// Detect auth strategy from URL
    pub fn detect_auth_strategy(url: &str) -> AuthStrategy;
}
```

### Skills Configuration

**File: `.claude/settings.json`**

Template for skills setup:
```json
{
  "enabledPlugins": {
    "open-prose@prose": true,
    "allbeads@allbeads-marketplace": true,
    "beads@beads-marketplace": true
  },
  "extraKnownMarketplaces": {
    "allbeads-marketplace": {
      "source": {
        "source": "github",
        "repo": "thrashr888/AllBeads"
      }
    },
    "beads-marketplace": {
      "source": {
        "source": "github",
        "repo": "steveyegge/beads"
      }
    }
  }
}
```

**Implementation:**
```rust
/// Configure Claude skills for a repository
pub fn configure_claude_skills(repo_path: &Path) -> Result<()> {
    let claude_dir = repo_path.join(".claude");
    let settings_file = claude_dir.join("settings.json");

    // Create .claude/ directory if missing
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
    }

    // Load existing settings or create new
    let mut settings = if settings_file.exists() {
        serde_json::from_str(&fs::read_to_string(&settings_file)?)?
    } else {
        json!({})
    };

    // Add AllBeads and Beads marketplaces
    // ... (merge with existing config)

    fs::write(&settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(())
}
```

### Issue Import

**Note on Hooks:**
Hook installation is handled by `bd init` during Stage 3. AllBeads doesn't install additional hooks at the Rig level - it operates at the Boss level to aggregate across repositories.

**GitHub Issues Import:**
```rust
/// Import issues from GitHub
pub async fn import_github_issues(
    org: &str,
    repo: &str,
    beads_repo: &mut BeadsRepo,
) -> Result<usize> {
    let client = GitHubClient::new()?;
    let issues = client.list_issues(org, repo).await?;

    let mut count = 0;
    for gh_issue in issues {
        let bead = Bead::from_github_issue(gh_issue);
        beads_repo.create(bead)?;
        count += 1;
    }

    Ok(count)
}
```

**Janitor Scan:**
```rust
/// Scan repository with Janitor for potential issues
pub fn scan_with_janitor(repo_path: &Path) -> Result<Vec<Bead>> {
    let analyzer = JanitorAnalyzer::new();
    let findings = analyzer.analyze(repo_path)?;

    let beads: Vec<Bead> = findings
        .into_iter()
        .map(|finding| Bead::from_janitor_finding(finding))
        .collect();

    Ok(beads)
}
```

## Testing Strategy

### Unit Tests

**Test onboarding stages:**
- URL parsing and validation
- Auth strategy detection
- Skills configuration creation
- Hook installation

**Test offboarding levels:**
- Soft removal (config only)
- Medium removal (config + hooks)
- Hard removal (everything)

### Integration Tests

**Full onboarding flow:**
```rust
#[test]
fn test_full_onboarding_flow() {
    let temp_dir = TempDir::new().unwrap();
    let repo_url = "https://github.com/test/repo.git";

    // Run onboarding
    let result = onboard_repository(repo_url, &temp_dir, OnboardOptions {
        non_interactive: true,
        skip_clone: false,
        skip_skills: false,
        skip_hooks: false,
        ..Default::default()
    });

    assert!(result.is_ok());

    // Verify results
    assert!(temp_dir.join(".beads").exists());
    assert!(temp_dir.join(".claude/settings.json").exists());
    assert!(temp_dir.join(".git/hooks/pre-commit").exists());
}
```

## Resolved Design Decisions

1. **Workspace Directory**: Use configured default workspace directory in AllBeads config. Default to `~/Workspace` if not set. Allow override with `--path` option.

2. **Safety Checks (Implemented)**: Before onboarding an existing repository:
   - Check for uncommitted changes (excluding `.beads/` and `.claude/` directories)
   - Verify current branch is `main` or `master`
   - Refuse to onboard if either check fails with clear error message
   - This prevents accidental commits of unrelated work during onboarding

3. **Git Remote Detection (Implemented)**: When onboarding local paths:
   - Automatically detect `git remote get-url origin` for the repository URL
   - Parse organization/owner from the URL for context configuration
   - Works with both SSH and HTTPS remotes

4. **Dependency Direction for Onboarding Beads**:
   - When creating epic + task beads during onboarding, the **epic depends on tasks**
   - Use `bd dep add <epic> <task>` - epic can't close until tasks are done
   - This makes tasks appear as "ready" while epic appears as "blocked"
   - **Anti-pattern**: Don't make tasks depend on epic (this blocks the tasks)

5. **Plugin Configuration (Simplified)**:
   - Only auto-enable `beads` and `allbeads` plugins (core functionality)
   - Don't auto-enable other marketplace plugins
   - Create an informational bead suggesting available marketplace plugins
   - Let users decide which additional plugins to enable

2. **Beads Initialization**: Delegate to `bd init` for all beads setup:
   - Respects user choice of mode (standard, no-db, stealth, team)
   - Beads handles hook installation, merge drivers, git config
   - AllBeads only adds Boss-level config (context, integrations, skills)
   - Separation of concerns: beads = Rig level, AllBeads = Boss level

3. **Settings.json Merge**: Mutate existing JSON intelligently:
   - Load existing settings.json if present
   - Add/merge into `enabledPlugins` object
   - Add/merge into `extraKnownMarketplaces` object
   - Preserve all other existing configuration

4. **Progress Indicators**: Yes, use Unicode/ANSI animated progress indicators for:
   - Cloning repositories
   - Importing issues (show count)
   - Long-running operations
   - Use spinners (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) and progress bars (█░░░░)

5. **Dry Run Mode**: Not needed yet. Defer to future if requested.

6. **Batch Onboarding**: Not needed yet. Single repo at a time for now.

7. **Template Repositories**: Not applicable for onboarding existing repos. This is for ab-k2g (new repo creation) only.

## Success Metrics

- Onboarding time reduced from ~10 manual steps to single command
- 100% onboarding score achievable in under 2 minutes
- Zero error rate for standard GitHub repositories
- Clean offboarding with no residual files (hard level)

## Future Enhancements

- **Interactive TUI**: Replace CLI prompts with full TUI (ab-jwb2)
- **Guided Wizard**: Step-by-step visual wizard (ab-xgjx)
- **Repository Templates**: Onboard from AllBeads templates (ab-k2g integration)
- **Bulk Operations**: Onboard multiple repos at once
- **Language Detection**: Automatic language-specific skill recommendations
- **Smart Defaults**: Learn user preferences over time

## Related Work

- **ab-pp6i**: Onboard workflow improvements (this spec)
- **ab-jwb2**: Build TUI GitHub repo search and picker
- **ab-xgjx**: Create guided onboarding wizard
- **ab-49s**: Beads uninstaller for context repos
- **ab-k2g**: GitHub templates for quick repo setup
