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

**Detection:**
- Check for `.beads/` directory
- If exists, ask to reinitialize or skip

**Interactive Prompt:**
```
Initialize beads tracking?
  [y] Yes (recommended)
  [n] No (skip beads setup)
```

**Actions:**
1. Run `bd init` equivalent
2. Create `.beads/` directory structure
3. Initialize `issues.jsonl`
4. Create initial database

**Output:**
```
→ Initializing beads (.beads/ directory)...
✓ Beads initialized
→ Current status: [B]eads ✓  [S]kills ✗  [I]ntegrations ✗  [C]I/CD ?  [H]ooks ✗
```

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

### Stage 6: Install Git Hooks

**Detection:**
- Check `.git/hooks/` directory
- Check for existing hooks (pre-commit, post-commit)

**Interactive Prompt:**
```
Would you like to install AllBeads Git hooks?
Hooks will:
  • pre-commit: Run 'bd check' to validate issues before commits
  • post-commit: Auto-sync beads changes to remote
  [y] Yes (recommended)
  [n] Skip for now

Choice [y]:
```

**Actions:**
1. Create `.git/hooks/pre-commit`:
   ```bash
   #!/bin/bash
   # AllBeads pre-commit hook
   bd check || exit 1
   ```
2. Create `.git/hooks/post-commit`:
   ```bash
   #!/bin/bash
   # AllBeads post-commit hook
   bd sync --quiet || true
   ```
3. Make hooks executable

**Output:**
```
→ Installing Git hooks...
  ✓ pre-commit (runs 'bd check' before commits)
  ✓ post-commit (auto-sync beads changes)
✓ Hooks installed
→ Current status: [B]eads ✓  [S]kills ✓  [I]ntegrations ✗  [C]I/CD ?  [H]ooks ✓
```

### Stage 7: Integrations (Optional)

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

### Stage 8: CI/CD Detection

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

### Stage 9: Add to AllBeads Config

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

### Stage 10: Summary & Next Steps

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
User Input → Discovery → Clone → Init Beads → Import Issues →
Configure Skills → Install Hooks → Detect CI/CD → Add Config → Summary
```

**Offboarding:**
```
User Input → Verify Context → Confirm Level → Remove Config →
Remove Hooks → Remove Beads (optional) → Clean Claude Config (optional) → Summary
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

### Hook Installation

**pre-commit hook:**
```bash
#!/bin/bash
# AllBeads pre-commit hook
# Validates beads state before allowing commit

if command -v bd &> /dev/null; then
    bd check || exit 1
else
    echo "Warning: 'bd' command not found, skipping beads check"
fi
```

**post-commit hook:**
```bash
#!/bin/bash
# AllBeads post-commit hook
# Auto-syncs beads changes after commit

if command -v bd &> /dev/null; then
    bd sync --quiet || true
fi
```

**Implementation:**
```rust
/// Install AllBeads Git hooks in a repository
/// Appends to existing hooks if present, doesn't overwrite
pub fn install_hooks(repo_path: &Path) -> Result<()> {
    let hooks_dir = repo_path.join(".git/hooks");

    // Ensure hooks directory exists
    if !hooks_dir.exists() {
        return Err(anyhow!("Not a git repository"));
    }

    // Install pre-commit (append if exists)
    let pre_commit = hooks_dir.join("pre-commit");
    install_hook(&pre_commit, PRECOMMIT_HOOK_TEMPLATE)?;

    // Install post-commit (append if exists)
    let post_commit = hooks_dir.join("post-commit");
    install_hook(&post_commit, POSTCOMMIT_HOOK_TEMPLATE)?;

    Ok(())
}

/// Install or append to a Git hook file
fn install_hook(hook_path: &Path, content: &str) -> Result<()> {
    if hook_path.exists() {
        // Existing hook - read and check if our content is already there
        let existing = fs::read_to_string(hook_path)?;

        // Check if AllBeads hook already present
        if existing.contains("AllBeads") {
            tracing::info!("AllBeads hook already installed: {}", hook_path.display());
            return Ok(());
        }

        // Append to existing hook
        let mut new_content = existing;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n# AllBeads hook\n");
        new_content.push_str(content);
        fs::write(hook_path, new_content)?;
        tracing::info!("Appended AllBeads hook to: {}", hook_path.display());
    } else {
        // No existing hook - create new
        fs::write(hook_path, format!("#!/bin/bash\n\n{}", content))?;
        set_executable(hook_path)?;
        tracing::info!("Created AllBeads hook: {}", hook_path.display());
    }

    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}
```

### Issue Import

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

2. **Hook Conflicts**: Follow beads' approach from [INSTALLING.md](https://github.com/steveyegge/beads/blob/main/docs/INSTALLING.md#cli--hooks-recommended-for-claude-code):
   - Check for existing hooks
   - If hooks exist, append to them (don't overwrite)
   - Preserve existing hook functionality

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
