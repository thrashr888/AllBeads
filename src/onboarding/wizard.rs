//! Guided onboarding wizard for repository setup
//!
//! Provides a step-by-step interactive wizard for onboarding repositories
//! into the AllBeads ecosystem with Health Checks status tracking.

use crate::config::{AllBeadsConfig, AuthStrategy, BossContext};
use crate::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Agent tooling metadata collected from a repository
///
/// Tracks MCP servers and agent rules configured in the repository:
/// - MCP servers from .claude/settings.json
/// - Cursor rules from .cursorrules
/// - GitHub Copilot instructions from .github/copilot-instructions.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentTooling {
    /// MCP servers configured in .claude/settings.json
    pub mcp_servers: Vec<String>,
    /// Whether .cursorrules file exists
    pub has_cursor_rules: bool,
    /// Whether .github/copilot-instructions.md exists
    pub has_copilot_rules: bool,
    /// Whether AGENTS.md exists (generic agent instructions)
    pub has_agents_md: bool,
}

impl AgentTooling {
    /// Detect agent tooling configuration for a repository path
    pub fn detect(repo_path: &Path) -> Self {
        let has_cursor_rules = repo_path.join(".cursorrules").exists();
        let has_copilot_rules = repo_path.join(".github/copilot-instructions.md").exists();
        let has_agents_md = repo_path.join("AGENTS.md").exists();
        let mcp_servers = Self::detect_mcp_servers(repo_path);

        Self {
            mcp_servers,
            has_cursor_rules,
            has_copilot_rules,
            has_agents_md,
        }
    }

    /// Parse MCP servers from .claude/settings.json
    fn detect_mcp_servers(repo_path: &Path) -> Vec<String> {
        let settings_path = repo_path.join(".claude/settings.json");
        if !settings_path.exists() {
            return Vec::new();
        }

        if let Ok(content) = std::fs::read_to_string(&settings_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // MCP servers are typically in mcpServers key
                if let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                    return servers.keys().cloned().collect();
                }
            }
        }

        Vec::new()
    }

    /// Check if any agent tooling is configured
    pub fn has_any(&self) -> bool {
        !self.mcp_servers.is_empty()
            || self.has_cursor_rules
            || self.has_copilot_rules
            || self.has_agents_md
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.mcp_servers.is_empty() {
            parts.push(format!("{} MCP", self.mcp_servers.len()));
        }
        if self.has_cursor_rules {
            parts.push("Cursor".to_string());
        }
        if self.has_copilot_rules {
            parts.push("Copilot".to_string());
        }
        if self.has_agents_md {
            parts.push("AGENTS.md".to_string());
        }
        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Health Checks status indicator for repository onboarding
///
/// Tracks the setup status of key AllBeads ecosystem components:
/// - Beads: Issue tracking initialized
/// - Skills: Claude skills marketplace configured
/// - Integration: External systems (JIRA/GitHub) connected
/// - CI/CD: Continuous integration workflows detected
/// - Hooks: Git hooks installed for automation
#[derive(Debug, Clone, Default)]
pub struct HealthChecks {
    pub beads: bool,       // Beads tracking initialized
    pub skills: bool,      // Skills/marketplace configured
    pub integration: bool, // Integration (JIRA/GitHub) configured
    pub cicd: bool,        // CI/CD detected
    pub hooks: bool,       // Git hooks installed
    /// Agent tooling metadata (MCP servers, rules files)
    pub agent_tooling: AgentTooling,
}

impl HealthChecks {
    /// Get compact display string with status indicators
    pub fn display(&self) -> String {
        format!(
            "Beads:{} Skills:{} Integration:{} CI/CD:{} Hooks:{}",
            if self.beads { "✓" } else { "✗" },
            if self.skills { "✓" } else { "✗" },
            if self.integration { "✓" } else { "✗" },
            if self.cicd { "✓" } else { "?" },
            if self.hooks { "✓" } else { "✗" },
        )
    }

    /// Get onboarding score (0-100)
    pub fn score(&self) -> u8 {
        let mut score = 0;
        if self.beads {
            score += 30;
        }
        if self.skills {
            score += 20;
        }
        if self.integration {
            score += 20;
        }
        if self.cicd {
            score += 15;
        }
        if self.hooks {
            score += 15;
        }
        score
    }

    /// Count how many checks are passing
    pub fn passing_count(&self) -> usize {
        let mut count = 0;
        if self.beads {
            count += 1;
        }
        if self.skills {
            count += 1;
        }
        if self.integration {
            count += 1;
        }
        if self.cicd {
            count += 1;
        }
        if self.hooks {
            count += 1;
        }
        count
    }

    /// Total number of checks
    pub fn total_checks(&self) -> usize {
        5
    }

    /// Detect health checks for a repository path
    pub fn detect(repo_path: &Path) -> Self {
        let beads = repo_path.join(".beads").exists();
        let skills = repo_path.join(".claude/settings.json").exists();
        let cicd = repo_path.join(".github/workflows").exists();
        let hooks = repo_path.join(".git/hooks/pre-commit").exists();
        let agent_tooling = AgentTooling::detect(repo_path);

        Self {
            beads,
            skills,
            integration: false, // Can't detect from filesystem alone
            cicd,
            hooks,
            agent_tooling,
        }
    }
}

/// Beads initialization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeadsInitMode {
    Standard, // SQLite database + git hooks
    NoDb,     // JSONL only, no SQLite
    Stealth,  // Personal, git-ignored
    Team,     // Team workflow setup
    Skip,     // Skip beads initialization
}

impl BeadsInitMode {
    pub fn label(&self) -> &'static str {
        match self {
            BeadsInitMode::Standard => "Standard (SQLite database + git hooks)",
            BeadsInitMode::NoDb => "No-DB mode (JSONL only, no SQLite)",
            BeadsInitMode::Stealth => "Stealth mode (personal, git-ignored)",
            BeadsInitMode::Team => "Team mode (team workflow setup)",
            BeadsInitMode::Skip => "Skip beads initialization",
        }
    }
}

/// Issue import source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueImportSource {
    GitHub,
    Jira,
    Janitor,
    Empty,
    Skip,
}

impl IssueImportSource {
    pub fn label(&self) -> &'static str {
        match self {
            IssueImportSource::GitHub => "Import from GitHub Issues",
            IssueImportSource::Jira => "Import from JIRA (requires configuration)",
            IssueImportSource::Janitor => "Scan with Janitor for potential issues",
            IssueImportSource::Empty => "Start with empty state",
            IssueImportSource::Skip => "Skip issue import",
        }
    }
}

/// Guided onboarding wizard
pub struct OnboardingWizard {
    /// Path to the repository
    pub repo_path: PathBuf,
    /// Repository name
    pub repo_name: String,
    /// Remote URL (if available)
    pub remote_url: Option<String>,
    /// Organization (extracted from URL)
    pub organization: Option<String>,
    /// Current health checks status
    pub status: HealthChecks,
    /// Theme for dialoguer
    theme: ColorfulTheme,
}

impl OnboardingWizard {
    /// Create a new wizard for a repository
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = std::fs::canonicalize(path.as_ref()).map_err(|e| {
            crate::AllBeadsError::Config(format!(
                "Failed to resolve path '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;

        // Verify it's a git repository
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            return Err(crate::AllBeadsError::Config(format!(
                "'{}' is not a git repository (no .git directory)",
                repo_path.display()
            )));
        }

        // Get repository name
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                crate::AllBeadsError::Config("Could not determine repository name".to_string())
            })?
            .to_string();

        // Try to get remote URL and extract organization
        let remote_url = Self::get_remote_url(&repo_path).ok();
        let organization = remote_url.as_deref().and_then(Self::extract_org);

        // Detect current status
        let status = Self::detect_status(&repo_path);

        Ok(Self {
            repo_path,
            repo_name,
            remote_url,
            organization,
            status,
            theme: ColorfulTheme::default(),
        })
    }

    /// Run the guided wizard
    pub fn run(&mut self) -> Result<()> {
        self.show_header();

        // Stage 1: Beads initialization
        self.stage_beads()?;

        // Stage 2: Issue import
        self.stage_issues()?;

        // Stage 3: Skills marketplace
        self.stage_skills()?;

        // Stage 4: Add to AllBeads
        self.stage_context()?;

        // Final summary
        self.show_summary();

        Ok(())
    }

    fn show_header(&self) {
        println!();
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║               🚀 AllBeads Onboarding Wizard                   ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("  Repository:    {}", self.repo_name);
        println!("  Path:          {}", self.repo_path.display());
        if let Some(ref url) = self.remote_url {
            println!("  Remote:        {}", url);
        }
        if let Some(ref org) = self.organization {
            println!("  Organization:  {}", org);
        }
        println!();
        println!("  Current Status: {}", self.status.display());
        println!("  Onboarding Score: {}%", self.status.score());
        println!();
        println!("─────────────────────────────────────────────────────────────────");
        println!();
    }

    fn stage_beads(&mut self) -> Result<()> {
        println!("📦 Stage 1: Initialize Beads Tracking");
        println!();

        if self.status.beads {
            println!("  ✓ Beads already initialized (.beads/ directory exists)");
            println!();
            return Ok(());
        }

        let modes = [
            BeadsInitMode::Standard,
            BeadsInitMode::NoDb,
            BeadsInitMode::Stealth,
            BeadsInitMode::Team,
            BeadsInitMode::Skip,
        ];

        let selection = Select::with_theme(&self.theme)
            .with_prompt("Choose beads initialization mode")
            .items(modes.iter().map(|m| m.label()).collect::<Vec<_>>())
            .default(0)
            .interact()
            .unwrap_or(4);

        let mode = modes[selection];

        if mode == BeadsInitMode::Skip {
            println!("  ⊘ Skipped beads initialization");
            println!();
            return Ok(());
        }

        let args = match mode {
            BeadsInitMode::Standard => vec!["init"],
            BeadsInitMode::NoDb => vec!["init", "--no-db"],
            BeadsInitMode::Stealth => vec!["init", "--stealth"],
            BeadsInitMode::Team => vec!["init", "--team"],
            BeadsInitMode::Skip => unreachable!(),
        };

        println!("  Running: bd {}", args.join(" "));
        let output = Command::new("bd")
            .args(&args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                crate::AllBeadsError::Config(format!(
                    "Failed to run 'bd': {}. Is 'bd' installed?",
                    e
                ))
            })?;

        if output.status.success() {
            println!("  ✓ Beads initialized successfully");
            self.status.beads = true;
            // Hooks are installed with bd init
            self.status.hooks = true;
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  ⚠ bd init returned: {}", stderr.trim());
        }

        println!();
        println!("  Updated Status: {}", self.status.display());
        println!();

        Ok(())
    }

    fn stage_issues(&mut self) -> Result<()> {
        println!("📝 Stage 2: Populate Issues");
        println!();

        if !self.status.beads {
            println!("  ⊘ Skipping issue import (beads not initialized)");
            println!();
            return Ok(());
        }

        let sources = [
            IssueImportSource::GitHub,
            IssueImportSource::Jira,
            IssueImportSource::Janitor,
            IssueImportSource::Empty,
            IssueImportSource::Skip,
        ];

        let selection = Select::with_theme(&self.theme)
            .with_prompt("How would you like to populate initial issues?")
            .items(sources.iter().map(|s| s.label()).collect::<Vec<_>>())
            .default(3)
            .interact()
            .unwrap_or(4);

        let source = sources[selection];

        match source {
            IssueImportSource::GitHub => {
                println!("  → GitHub import requires 'ab github pull' configuration");
                println!("    Run 'ab github status' to check your setup");
            }
            IssueImportSource::Jira => {
                println!("  → JIRA import requires configuration");
                println!("    Run 'ab jira status' to check your setup");
            }
            IssueImportSource::Janitor => {
                println!("  Running: ab janitor {}", self.repo_path.display());
                let output = Command::new("ab")
                    .args(["janitor", self.repo_path.to_str().unwrap(), "--dry-run"])
                    .output();

                if let Ok(output) = output {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!(
                        "  {}",
                        stdout.lines().take(10).collect::<Vec<_>>().join("\n  ")
                    );
                    if stdout.lines().count() > 10 {
                        println!("  ...");
                    }
                }
            }
            IssueImportSource::Empty => {
                println!("  ✓ Starting with empty issue state");
            }
            IssueImportSource::Skip => {
                println!("  ⊘ Skipped issue import");
            }
        }

        println!();
        Ok(())
    }

    fn stage_skills(&mut self) -> Result<()> {
        println!("🎯 Stage 3: Configure Skills Marketplace");
        println!();

        let claude_dir = self.repo_path.join(".claude");
        let settings_file = claude_dir.join("settings.json");

        if settings_file.exists() {
            // Check if already configured
            if let Ok(content) = std::fs::read_to_string(&settings_file) {
                if content.contains("allbeads-marketplace") {
                    println!("  ✓ Skills marketplace already configured");
                    self.status.skills = true;
                    println!();
                    return Ok(());
                }
            }
        }

        let configure = Confirm::with_theme(&self.theme)
            .with_prompt("Add AllBeads and Beads skill marketplaces to .claude/settings.json?")
            .default(true)
            .interact()
            .unwrap_or(false);

        if !configure {
            println!("  ⊘ Skipped skills configuration");
            println!();
            return Ok(());
        }

        // Create .claude directory if needed
        if !claude_dir.exists() {
            std::fs::create_dir_all(&claude_dir).map_err(|e| {
                crate::AllBeadsError::Config(format!("Failed to create .claude/: {}", e))
            })?;
        }

        // Create or update settings.json
        let settings = serde_json::json!({
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
        });

        std::fs::write(
            &settings_file,
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .map_err(|e| {
            crate::AllBeadsError::Config(format!("Failed to write settings.json: {}", e))
        })?;

        println!("  ✓ Skills marketplace configured");
        self.status.skills = true;

        println!();
        println!("  Updated Status: {}", self.status.display());
        println!();

        Ok(())
    }

    fn stage_context(&mut self) -> Result<()> {
        println!("🔗 Stage 4: Add to AllBeads Contexts");
        println!();

        let config_file = AllBeadsConfig::default_path();
        let mut config = if config_file.exists() {
            AllBeadsConfig::load(&config_file)?
        } else {
            println!("  ⚠ AllBeads not configured. Run 'ab init' first.");
            println!();
            return Ok(());
        };

        // Check if already added
        if config.get_context(&self.repo_name).is_some() {
            println!("  ✓ Repository already in AllBeads contexts");
            println!();
            return Ok(());
        }

        // Customize context name
        let context_name: String = Input::with_theme(&self.theme)
            .with_prompt("Context name")
            .default(self.repo_name.clone())
            .interact_text()
            .unwrap_or_else(|_| self.repo_name.clone());

        let add = Confirm::with_theme(&self.theme)
            .with_prompt(format!("Add '{}' to AllBeads contexts?", context_name))
            .default(true)
            .interact()
            .unwrap_or(false);

        if !add {
            println!("  ⊘ Skipped context registration");
            println!();
            return Ok(());
        }

        // Determine auth strategy
        let auth_strategy = if let Some(ref url) = self.remote_url {
            if url.starts_with("https://") {
                AuthStrategy::PersonalAccessToken
            } else {
                AuthStrategy::SshAgent
            }
        } else {
            AuthStrategy::SshAgent
        };

        // Create context
        let remote_url = self.remote_url.clone().unwrap_or_default();
        let mut context = BossContext::new(&context_name, &remote_url, auth_strategy);
        context.path = Some(self.repo_path.clone());

        config.add_context(context);
        config.save(&config_file)?;

        println!("  ✓ Added to AllBeads contexts as '{}'", context_name);
        println!();

        Ok(())
    }

    fn show_summary(&self) {
        // Detect CI/CD
        let has_ci = self.repo_path.join(".github/workflows").exists();
        let mut final_status = self.status.clone();
        final_status.cicd = has_ci;

        println!("═══════════════════════════════════════════════════════════════════");
        println!();
        println!("  ✅ Onboarding Complete!");
        println!();
        println!("  Repository:       {}", self.repo_name);
        println!("  Path:             {}", self.repo_path.display());
        println!("  Final Status:     {}", final_status.display());
        println!("  Onboarding Score: {}%", final_status.score());
        println!();
        println!("  📚 Next Steps:");
        println!();
        println!("    1. Create your first issue:");
        println!("       bd create --title=\"Initial setup\" --type=task --priority=2");
        println!();
        println!("    2. View in TUI dashboard:");
        println!("       ab tui");
        println!();
        println!("    3. Check onboarding status:");
        println!("       ab context onboarding");
        println!();

        if !final_status.integration {
            println!("    4. Optional - Setup integrations:");
            println!("       ab github status");
            println!("       ab jira status");
            println!();
        }

        println!("═══════════════════════════════════════════════════════════════════");
        println!();
    }

    fn get_remote_url(repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "remote",
                "get-url",
                "origin",
            ])
            .output()
            .map_err(|e| crate::AllBeadsError::Config(format!("Failed to run git: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(crate::AllBeadsError::Config(
                "No 'origin' remote found".to_string(),
            ))
        }
    }

    fn extract_org(url: &str) -> Option<String> {
        // Parse GitHub URL to extract organization
        // git@github.com:org/repo.git
        // https://github.com/org/repo.git
        if url.contains("github.com") {
            let parts: Vec<&str> = if url.starts_with("git@") {
                url.split(':').next_back()?.split('/').collect()
            } else {
                // Use rsplit().next() since split on &str doesn't impl DoubleEndedIterator
                url.rsplit("github.com/").next()?.split('/').collect()
            };
            if parts.len() >= 2 {
                return Some(parts[0].to_string());
            }
        }
        None
    }

    fn detect_status(repo_path: &Path) -> HealthChecks {
        HealthChecks::detect(repo_path)
    }
}
