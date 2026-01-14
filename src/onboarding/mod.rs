//! Repository onboarding tracking and assistance
//!
//! This module helps track and guide the onboarding process for repositories
//! into the AllBeads ecosystem, from initial tracking to full adoption.

pub mod workflow;

use crate::config::BossContext;
use crate::git::BossRepo;
use crate::Result;
use std::path::PathBuf;

pub use workflow::OnboardingWorkflow;

/// Onboarding stage for a repository
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OnboardingStage {
    /// Repository is tracked but not cloned locally yet
    NotCloned,

    /// Repository is cloned but beads not initialized
    Cloned,

    /// Beads initialized (.beads/ directory exists)
    BeadsInitialized,

    /// Repository has issues tracked in beads
    HasIssues,

    /// Repository has skills configured
    HasSkills,

    /// Repository has integration configured (JIRA/GitHub)
    IntegrationConfigured,

    /// Fully onboarded and actively maintained
    FullyOnboarded,
}

impl OnboardingStage {
    /// Get human-readable name for this stage
    pub fn name(&self) -> &'static str {
        match self {
            OnboardingStage::NotCloned => "Not Cloned",
            OnboardingStage::Cloned => "Cloned",
            OnboardingStage::BeadsInitialized => "Beads Initialized",
            OnboardingStage::HasIssues => "Has Issues",
            OnboardingStage::HasSkills => "Has Skills",
            OnboardingStage::IntegrationConfigured => "Integration Configured",
            OnboardingStage::FullyOnboarded => "Fully Onboarded",
        }
    }

    /// Get emoji indicator for this stage
    pub fn emoji(&self) -> &'static str {
        match self {
            OnboardingStage::NotCloned => "📍",
            OnboardingStage::Cloned => "📦",
            OnboardingStage::BeadsInitialized => "🔧",
            OnboardingStage::HasIssues => "📝",
            OnboardingStage::HasSkills => "🎯",
            OnboardingStage::IntegrationConfigured => "🔗",
            OnboardingStage::FullyOnboarded => "✅",
        }
    }

    /// Get the next steps for this stage
    pub fn next_steps(&self, context_name: &str) -> Vec<String> {
        match self {
            OnboardingStage::NotCloned => vec![
                format!("Run: cargo run -- list -C {}", context_name),
                "This will automatically clone the repository".to_string(),
            ],
            OnboardingStage::Cloned => vec![
                "Navigate to the repository directory".to_string(),
                "Run: bd init".to_string(),
                "This will initialize beads tracking".to_string(),
            ],
            OnboardingStage::BeadsInitialized => vec![
                "Create your first issue: bd create --title=\"Initial setup\" --type=task --priority=2".to_string(),
                "Or import existing issues from JIRA/GitHub".to_string(),
            ],
            OnboardingStage::HasIssues => vec![
                "Consider installing skills for enhanced functionality".to_string(),
                "Skills provide specialized agent capabilities".to_string(),
            ],
            OnboardingStage::HasSkills => vec![
                "Configure integration: ab jira status / ab github status".to_string(),
                "Set up bi-directional sync with external systems".to_string(),
            ],
            OnboardingStage::IntegrationConfigured => vec![
                "Maintain regular usage and keep issues updated".to_string(),
                "Review bd stats and bd ready regularly".to_string(),
                "Consider adding to Sheriff daemon for auto-sync".to_string(),
            ],
            OnboardingStage::FullyOnboarded => vec![
                "Repository is fully onboarded!".to_string(),
                "Continue maintaining and using beads for issue tracking".to_string(),
            ],
        }
    }

    /// Get progress percentage (0-100)
    pub fn progress(&self) -> u8 {
        match self {
            OnboardingStage::NotCloned => 0,
            OnboardingStage::Cloned => 15,
            OnboardingStage::BeadsInitialized => 30,
            OnboardingStage::HasIssues => 50,
            OnboardingStage::HasSkills => 70,
            OnboardingStage::IntegrationConfigured => 85,
            OnboardingStage::FullyOnboarded => 100,
        }
    }
}

/// Onboarding status for a specific repository
#[derive(Debug, Clone)]
pub struct OnboardingStatus {
    /// Context name
    pub context_name: String,

    /// Current onboarding stage
    pub stage: OnboardingStage,

    /// Local path (if cloned)
    pub path: Option<PathBuf>,

    /// Repository URL
    pub url: String,

    /// Organization (extracted from URL)
    pub organization: Option<String>,

    /// Number of issues (if available)
    pub issue_count: Option<usize>,

    /// Has skills configured
    pub has_skills: bool,

    /// Has integration configured
    pub has_integration: bool,

    /// Has GitHub Actions configured
    pub has_ci: bool,

    /// Has Git hooks installed
    pub has_hooks: bool,
}

impl OnboardingStatus {
    /// Check if beads is actively being used (has issues)
    pub fn has_beads_usage(&self) -> bool {
        self.stage >= OnboardingStage::HasIssues
    }
}

impl OnboardingStatus {
    /// Detect onboarding status for a Boss context
    pub fn detect(context: &BossContext) -> Result<Self> {
        let repo = BossRepo::from_context(context.clone())?;
        let repo_status = repo.status()?;

        // Determine current stage
        let stage = match repo_status {
            crate::git::RepoStatus::NotCloned => OnboardingStage::NotCloned,
            _ => {
                if !repo.has_beads_dir() {
                    OnboardingStage::Cloned
                } else if !repo.has_issues_jsonl() {
                    OnboardingStage::BeadsInitialized
                } else {
                    // Count issues
                    let issue_count = Self::count_issues(&repo)?;

                    // Check for skills
                    let has_skills = Self::has_skills(&repo);

                    // Check for integration (from context config)
                    let has_integration = Self::has_integration_from_context(context);

                    // Check for CI
                    let has_ci = Self::has_ci(&repo);

                    if has_integration {
                        if issue_count > 5 && has_skills && has_ci {
                            OnboardingStage::FullyOnboarded
                        } else {
                            OnboardingStage::IntegrationConfigured
                        }
                    } else if has_skills {
                        OnboardingStage::HasSkills
                    } else {
                        OnboardingStage::HasIssues
                    }
                }
            }
        };

        let path = if repo_status != crate::git::RepoStatus::NotCloned {
            Some(repo.path().to_path_buf())
        } else {
            None
        };

        let issue_count = if repo.has_issues_jsonl() {
            Some(Self::count_issues(&repo)?)
        } else {
            None
        };

        let has_skills = Self::has_skills(&repo);
        let has_integration = Self::has_integration_from_context(context);
        let has_ci = Self::has_ci(&repo);
        let has_hooks = Self::has_hooks(&repo);

        Ok(OnboardingStatus {
            context_name: context.name.clone(),
            stage,
            path,
            url: context.url.clone(),
            organization: context.organization(),
            issue_count,
            has_skills,
            has_integration,
            has_ci,
            has_hooks,
        })
    }

    /// Count issues in the repository
    fn count_issues(repo: &BossRepo) -> Result<usize> {
        use crate::storage::JsonlReader;

        if !repo.has_issues_jsonl() {
            return Ok(0);
        }

        let mut reader = JsonlReader::open(repo.issues_jsonl_path())?;
        let beads: Vec<crate::graph::Bead> = reader.read_all()?;
        Ok(beads.len())
    }

    /// Check if repository has skills configured
    fn has_skills(repo: &BossRepo) -> bool {
        // Check for .claude-plugin directory or CLAUDE.md with skills
        let claude_plugin = repo.path().join(".claude-plugin");
        let claude_md = repo.path().join("CLAUDE.md");

        claude_plugin.exists() || claude_md.exists()
    }

    /// Check if repository has integration configured (from BossContext)
    fn has_integration_from_context(context: &BossContext) -> bool {
        // Check if context has JIRA or GitHub integration configured
        context.integrations.jira.is_some() || context.integrations.github.is_some()
    }

    /// Check if repository has CI/CD configured
    fn has_ci(repo: &BossRepo) -> bool {
        // Check for GitHub Actions workflows
        let gh_actions = repo.path().join(".github/workflows");

        // Check if workflows directory exists and has any .yml or .yaml files
        if gh_actions.exists() {
            if let Ok(entries) = std::fs::read_dir(gh_actions) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "yml" || ext == "yaml" {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if repository has beads Git hooks installed
    fn has_hooks(repo: &BossRepo) -> bool {
        // Check for the main beads hook (pre-commit)
        let git_hooks_dir = repo.path().join(".git/hooks");
        let pre_commit_hook = git_hooks_dir.join("pre-commit");

        // Check if pre-commit hook exists and contains AllBeads marker
        if pre_commit_hook.exists() {
            if let Ok(content) = std::fs::read_to_string(&pre_commit_hook) {
                // Check if it's an AllBeads hook by looking for our marker
                return content.contains("AllBeads") || content.contains("bd check");
            }
        }

        false
    }
}

/// Aggregate onboarding status across all contexts
#[derive(Debug)]
pub struct OnboardingReport {
    /// Individual status for each context
    pub statuses: Vec<OnboardingStatus>,

    /// Overall statistics
    pub stats: OnboardingStats,
}

/// Statistics about onboarding across all contexts
#[derive(Debug, Default)]
pub struct OnboardingStats {
    pub total_contexts: usize,
    pub not_cloned: usize,
    pub cloned: usize,
    pub beads_initialized: usize,
    pub has_issues: usize,
    pub has_skills: usize,
    pub integration_configured: usize,
    pub fully_onboarded: usize,
}

impl OnboardingReport {
    /// Create a report from multiple contexts
    pub fn from_contexts(contexts: &[BossContext]) -> Result<Self> {
        let mut statuses = Vec::new();
        let mut stats = OnboardingStats {
            total_contexts: contexts.len(),
            ..Default::default()
        };

        for context in contexts {
            if let Ok(status) = OnboardingStatus::detect(context) {
                // Update stats
                match status.stage {
                    OnboardingStage::NotCloned => stats.not_cloned += 1,
                    OnboardingStage::Cloned => stats.cloned += 1,
                    OnboardingStage::BeadsInitialized => stats.beads_initialized += 1,
                    OnboardingStage::HasIssues => stats.has_issues += 1,
                    OnboardingStage::HasSkills => stats.has_skills += 1,
                    OnboardingStage::IntegrationConfigured => stats.integration_configured += 1,
                    OnboardingStage::FullyOnboarded => stats.fully_onboarded += 1,
                }

                statuses.push(status);
            }
        }

        // Sort by stage (least to most advanced) then by name
        statuses.sort_by(|a, b| {
            a.stage
                .cmp(&b.stage)
                .then_with(|| a.context_name.cmp(&b.context_name))
        });

        Ok(OnboardingReport { statuses, stats })
    }

    /// Print the report in a human-readable format
    pub fn print(&self) {
        eprintln!("\n🚀 AllBeads Onboarding Status\n");

        // Print summary stats
        eprintln!("Summary:");
        eprintln!("  Total contexts: {}", self.stats.total_contexts);
        eprintln!("  ✅ Fully onboarded: {}", self.stats.fully_onboarded);
        eprintln!("  🔗 Integration configured: {}", self.stats.integration_configured);
        eprintln!("  🎯 Has skills: {}", self.stats.has_skills);
        eprintln!("  📝 Has issues: {}", self.stats.has_issues);
        eprintln!("  🔧 Beads initialized: {}", self.stats.beads_initialized);
        eprintln!("  📦 Cloned: {}", self.stats.cloned);
        eprintln!("  📍 Not cloned: {}", self.stats.not_cloned);

        // Print details for each context
        eprintln!("\nRepository Details:\n");

        let mut current_stage: Option<OnboardingStage> = None;

        for status in &self.statuses {
            // Print stage header if changed
            if current_stage != Some(status.stage) {
                eprintln!(
                    "\n{} {} ({}% complete)",
                    status.stage.emoji(),
                    status.stage.name(),
                    status.stage.progress()
                );
                eprintln!("{}", "─".repeat(60));
                current_stage = Some(status.stage);
            }

            // Print context details
            eprintln!("  {}", status.context_name);
            eprintln!("    URL: {}", status.url);

            if let Some(ref path) = status.path {
                eprintln!("    Path: {}", path.display());
            }

            if let Some(count) = status.issue_count {
                eprintln!("    Issues: {}", count);
            }

            if status.has_skills {
                eprintln!("    Skills: ✓");
            }

            if status.has_integration {
                eprintln!("    Integration: ✓");
            }

            if status.has_ci {
                eprintln!("    CI/CD: ✓ (GitHub Actions)");
            }

            // Print next steps
            let steps = status.stage.next_steps(&status.context_name);
            if !steps.is_empty() {
                eprintln!("    Next steps:");
                for step in steps {
                    eprintln!("      • {}", step);
                }
            }

            eprintln!();
        }
    }

    /// Print a compact summary
    pub fn print_summary(&self) {
        let progress = if self.stats.total_contexts > 0 {
            (self.stats.fully_onboarded * 100) / self.stats.total_contexts
        } else {
            0
        };

        eprintln!(
            "Onboarding: {}/{} fully onboarded ({}%)",
            self.stats.fully_onboarded, self.stats.total_contexts, progress
        );
    }
}
