//! Repository onboarding implementation
//!
//! Implements the 9-stage onboarding workflow from SPEC-onboarding.md

use crate::config::{AllBeadsConfig, AuthStrategy, BossContext};
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Repository information discovered during onboarding
pub struct RepoInfo {
    /// Repository name (extracted from URL or path)
    pub name: String,
    /// Local path where repository exists or will be cloned
    pub path: PathBuf,
    /// Repository URL (if applicable)
    pub url: Option<String>,
    /// Organization/user (extracted from URL)
    pub organization: Option<String>,
    /// Whether repository already exists locally
    pub exists_locally: bool,
}

/// Stage 1: Discover and validate repository
pub fn discover_repository(
    target: &str,
    custom_path: Option<&str>,
    config: &AllBeadsConfig,
) -> Result<RepoInfo> {
    // Check if target is a URL (starts with http:// or https:// or git@)
    let is_url = target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("git@");

    // Check if target is a GitHub shorthand (owner/repo format)
    let is_github_shorthand = !is_url
        && !target.starts_with('/')
        && !target.starts_with('.')
        && target.matches('/').count() == 1
        && target.split('/').all(|part| !part.is_empty());

    if is_url {
        // Parse URL
        let (name, organization) = parse_repo_url(target)?;

        // Determine local path
        let path = if let Some(custom) = custom_path {
            PathBuf::from(custom)
        } else {
            config.workspace_directory().join(&name)
        };

        // Check if already exists
        let exists = path.exists() && path.join(".git").exists();

        Ok(RepoInfo {
            name,
            path,
            url: Some(target.to_string()),
            organization: Some(organization),
            exists_locally: exists,
        })
    } else if is_github_shorthand {
        // Convert owner/repo to GitHub URL
        let parts: Vec<&str> = target.split('/').collect();
        let organization = parts[0].to_string();
        let name = parts[1].to_string();
        let url = format!("https://github.com/{}/{}.git", organization, name);

        // Determine local path
        let path = if let Some(custom) = custom_path {
            PathBuf::from(custom)
        } else {
            config.workspace_directory().join(&name)
        };

        // Check if already exists
        let exists = path.exists() && path.join(".git").exists();

        Ok(RepoInfo {
            name,
            path,
            url: Some(url),
            organization: Some(organization),
            exists_locally: exists,
        })
    } else {
        // Treat as local path
        let path = PathBuf::from(target);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(&path)
        };

        // Verify it's a git repository
        if !path.join(".git").exists() {
            return Err(crate::AllBeadsError::Config(format!(
                "Not a git repository: {}",
                path.display()
            )));
        }

        // Extract repo name from path
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                crate::AllBeadsError::Config(format!("Invalid path: {}", path.display()))
            })?
            .to_string();

        // Try to detect git remote URL
        let (url, organization) = detect_git_remote(&path);

        Ok(RepoInfo {
            name,
            path,
            url,
            organization,
            exists_locally: true,
        })
    }
}

/// Detect git remote URL from a local repository
///
/// Runs `git remote get-url origin` to get the remote URL, then parses
/// out the organization/owner from the URL.
fn detect_git_remote(path: &Path) -> (Option<String>, Option<String>) {
    use std::process::Command;

    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if url.is_empty() {
                return (None, None);
            }

            // Try to parse organization from URL
            let organization = if let Ok((_, org)) = parse_repo_url(&url) {
                Some(org)
            } else {
                None
            };

            (Some(url), organization)
        }
        _ => (None, None),
    }
}

/// Parse repository URL to extract name and organization
fn parse_repo_url(url: &str) -> Result<(String, String)> {
    // Handle SSH format: git@github.com:org/repo.git
    if url.starts_with("git@") {
        if let Some(colon_pos) = url.find(':') {
            let path = &url[colon_pos + 1..];
            let path = path.trim_end_matches(".git");
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                return Ok((parts[1].to_string(), parts[0].to_string()));
            }
        }
    }

    // Handle HTTPS format: https://github.com/org/repo.git
    if url.starts_with("http://") || url.starts_with("https://") {
        if let Some(domain_start) = url.find("://") {
            let after_protocol = &url[domain_start + 3..];
            if let Some(slash_pos) = after_protocol.find('/') {
                let path = &after_protocol[slash_pos + 1..];
                let path = path.trim_end_matches(".git");
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 2 {
                    return Ok((parts[1].to_string(), parts[0].to_string()));
                }
            }
        }
    }

    Err(crate::AllBeadsError::Config(format!(
        "Could not parse repository URL: {}",
        url
    )))
}

/// Stage 2: Clone repository
/// Check if repository is safe to onboard
///
/// Returns an error if:
/// - Working directory has uncommitted changes (dirty)
/// - Not on main or master branch
///
/// This prevents accidental commits to feature branches or conflicts with local work.
pub fn check_repo_safe_to_onboard(path: &Path) -> Result<()> {
    // Check if working directory is clean
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()?;

    if status_output.status.success() {
        let status = String::from_utf8_lossy(&status_output.stdout);
        // Filter out untracked files in .beads/ and .claude/ since we're about to create those
        let dirty_files: Vec<&str> = status
            .lines()
            .filter(|line| {
                !line.contains(".beads/") && !line.contains(".claude/") && !line.trim().is_empty()
            })
            .collect();

        if !dirty_files.is_empty() {
            return Err(crate::AllBeadsError::Config(format!(
                "Repository has uncommitted changes:\n  {}\n\nPlease commit or stash changes before onboarding.",
                dirty_files.join("\n  ")
            )));
        }
    }

    // Check current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()?;

    if branch_output.status.success() {
        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        if branch != "main" && branch != "master" {
            return Err(crate::AllBeadsError::Config(format!(
                "Repository is on branch '{}', not main/master.\n\nPlease switch to the main branch before onboarding:\n  git checkout main",
                branch
            )));
        }
    }

    Ok(())
}

pub fn clone_repository(url: &str, path: &Path, non_interactive: bool) -> Result<()> {
    if !non_interactive {
        println!("  Clone to: {}", path.display());
        print!("  Proceed? [Y/n]: ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            return Err(crate::AllBeadsError::Config(
                "Clone cancelled by user".to_string(),
            ));
        }
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Clone using git (capture output to avoid terminal corruption from progress)
    println!("  Cloning {}...", url);
    let output = Command::new("git")
        .args(["clone", "--progress", url, &path.display().to_string()])
        .stderr(std::process::Stdio::piped())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::AllBeadsError::Config(format!(
            "Git clone failed: {}",
            stderr.trim()
        )));
    }

    println!("  ✓ Cloned successfully");
    Ok(())
}

/// Stage 3: Initialize beads via bd init
pub fn initialize_beads(path: &Path, non_interactive: bool) -> Result<()> {
    let beads_dir = path.join(".beads");
    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Check if fully initialized (has both .beads/ and database)
    if beads_dir.exists() && db_path.exists() {
        println!("  ✓ Beads already initialized");
        return Ok(());
    }

    // Check if bd command is available
    let bd_check = Command::new("bd").arg("--version").output();
    if bd_check.is_err() {
        return Err(crate::AllBeadsError::Config(
            "'bd' command not found. Please install beads CLI first.".to_string(),
        ));
    }

    // Check if this is a cloned repo (has JSONL but no database)
    let is_cloned_repo = jsonl_path.exists() && !db_path.exists();

    if non_interactive {
        if is_cloned_repo {
            // Cloned repo - just need to create database from existing JSONL
            println!("  Running: bd init --quiet (creating database from existing JSONL)");
        } else {
            println!("  Running: bd init --quiet");
        }
        let status = Command::new("bd")
            .arg("init")
            .arg("--quiet")
            .current_dir(path)
            .status()?;

        if !status.success() {
            return Err(crate::AllBeadsError::Config(format!(
                "bd init failed with status: {}",
                status
            )));
        }

        println!("  ✓ Beads initialized (standard mode)");
    } else {
        // Interactive mode selection
        println!("  Initialize beads tracking?");
        println!("    [1] Standard mode (SQLite database + git hooks)");
        println!("    [2] No-DB mode (JSONL only, no SQLite)");
        println!("    [3] Stealth mode (personal, git-ignored)");
        println!("    [4] Team mode (team workflow setup)");
        println!("    [5] Skip beads setup");
        print!("  Choice [1]: ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let choice = if choice.is_empty() { "1" } else { choice };

        match choice {
            "1" => {
                println!("  Running: bd init");
                let status = Command::new("bd").arg("init").current_dir(path).status()?;
                if !status.success() {
                    return Err(crate::AllBeadsError::Config("bd init failed".to_string()));
                }
                println!("  ✓ Beads initialized (standard mode)");
            }
            "2" => {
                println!("  Running: bd init --no-db");
                let status = Command::new("bd")
                    .args(["init", "--no-db"])
                    .current_dir(path)
                    .status()?;
                if !status.success() {
                    return Err(crate::AllBeadsError::Config(
                        "bd init --no-db failed".to_string(),
                    ));
                }
                println!("  ✓ Beads initialized (no-db mode)");
            }
            "3" => {
                println!("  Running: bd init --stealth");
                let status = Command::new("bd")
                    .args(["init", "--stealth"])
                    .current_dir(path)
                    .status()?;
                if !status.success() {
                    return Err(crate::AllBeadsError::Config(
                        "bd init --stealth failed".to_string(),
                    ));
                }
                println!("  ✓ Beads initialized (stealth mode)");
            }
            "4" => {
                println!("  Running: bd init --team");
                let status = Command::new("bd")
                    .args(["init", "--team"])
                    .current_dir(path)
                    .status()?;
                if !status.success() {
                    return Err(crate::AllBeadsError::Config(
                        "bd init --team failed".to_string(),
                    ));
                }
                println!("  ✓ Beads initialized (team mode)");
            }
            "5" => {
                println!("  Skipped beads initialization");
                return Ok(());
            }
            _ => {
                return Err(crate::AllBeadsError::Config(format!(
                    "Invalid choice: {}",
                    choice
                )));
            }
        }
    }

    Ok(())
}

/// Onboarding issue to create
#[derive(Debug)]
pub struct OnboardingIssue {
    pub title: String,
    pub description: String,
    pub priority: u8,
    pub labels: Vec<String>,
}

/// Stage 4: Populate onboarding issues
///
/// Creates a bead for the specified agent configuration if it doesn't exist.
/// The agent_type parameter controls which config to create:
/// - "claude": CLAUDE.md for Claude Code
/// - "agent-skills": .agent/skills/ directory (Open Agent Skills spec)
/// - "cursor": .cursorrules for Cursor AI
/// - "copilot": .github/copilot-instructions.md for GitHub Copilot
/// - "aider": .aider.conf.yml for Aider
/// - "kiro": .kiro/ directory for AWS Kiro
/// - "all": All agent configurations (legacy behavior)
pub fn populate_onboarding_issues(path: &Path, agent_type: &str) -> Result<Vec<OnboardingIssue>> {
    let mut issues = Vec::new();

    match agent_type {
        "claude" => {
            let claude_md = path.join("CLAUDE.md");
            if !claude_md.exists() {
                issues.push(OnboardingIssue {
                    title: "Initialize Claude Code configuration".to_string(),
                    description: "Create CLAUDE.md with project-specific instructions. \
                                 This file helps Claude understand your codebase architecture, \
                                 coding conventions, and key patterns."
                        .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "agent-skills" => {
            let agent_skills = path.join(".agent/skills");
            if !agent_skills.exists() {
                issues.push(OnboardingIssue {
                    title: "Initialize Open Agent Skills".to_string(),
                    description: "Create .agent/skills/ directory with SKILL.md files following \
                                 the Open Agent Skills specification (agentskills.io). \
                                 This format is supported by multiple AI agents."
                        .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "cursor" => {
            let cursorrules = path.join(".cursorrules");
            let cursor_dir = path.join(".cursor");
            if !cursorrules.exists() && !cursor_dir.exists() {
                issues.push(OnboardingIssue {
                    title: "Add Cursor configuration".to_string(),
                    description:
                        "Create .cursorrules file with project-specific rules for Cursor AI. \
                                 Include coding standards, preferred patterns, and project context."
                            .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "copilot" => {
            let copilot_instructions = path.join(".github/copilot-instructions.md");
            if !copilot_instructions.exists() {
                issues.push(OnboardingIssue {
                    title: "Add GitHub Copilot instructions".to_string(),
                    description:
                        "Create .github/copilot-instructions.md with project-specific guidance. \
                                 Document coding standards and patterns Copilot should follow."
                            .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "aider" => {
            let aider_conf = path.join(".aider.conf.yml");
            let aiderignore = path.join(".aiderignore");
            if !aider_conf.exists() && !aiderignore.exists() {
                issues.push(OnboardingIssue {
                    title: "Add Aider configuration".to_string(),
                    description: "Create .aider.conf.yml with model preferences and .aiderignore. \
                                 Configure aider for optimal performance with this codebase."
                        .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "kiro" => {
            let kiro_dir = path.join(".kiro");
            if !kiro_dir.exists() {
                issues.push(OnboardingIssue {
                    title: "Add AWS Kiro configuration".to_string(),
                    description:
                        "Create .kiro/ directory with specs and steering files for AWS Kiro. \
                                 Define project requirements and agent behavior guidelines."
                            .to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }
        }
        "all" | _ => {
            // Legacy behavior: create issues for all missing configs
            let claude_md = path.join("CLAUDE.md");
            if !claude_md.exists() {
                issues.push(OnboardingIssue {
                    title: "Initialize Claude Code configuration".to_string(),
                    description: "Create CLAUDE.md with project-specific instructions.".to_string(),
                    priority: 2,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }

            let cursorrules = path.join(".cursorrules");
            let cursor_dir = path.join(".cursor");
            if !cursorrules.exists() && !cursor_dir.exists() {
                issues.push(OnboardingIssue {
                    title: "Add Cursor configuration".to_string(),
                    description: "Create .cursorrules file with project-specific rules."
                        .to_string(),
                    priority: 3,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }

            let kiro_dir = path.join(".kiro");
            if !kiro_dir.exists() {
                issues.push(OnboardingIssue {
                    title: "Add AWS Kiro configuration".to_string(),
                    description: "Create .kiro/ directory with specs and steering files."
                        .to_string(),
                    priority: 3,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }

            let aider_conf = path.join(".aider.conf.yml");
            let aiderignore = path.join(".aiderignore");
            if !aider_conf.exists() && !aiderignore.exists() {
                issues.push(OnboardingIssue {
                    title: "Add Aider configuration".to_string(),
                    description: "Create .aider.conf.yml with model preferences.".to_string(),
                    priority: 3,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }

            let copilot_instructions = path.join(".github/copilot-instructions.md");
            if !copilot_instructions.exists() {
                issues.push(OnboardingIssue {
                    title: "Add GitHub Copilot instructions".to_string(),
                    description: "Create .github/copilot-instructions.md with project guidance."
                        .to_string(),
                    priority: 3,
                    labels: vec!["onboarding".to_string(), "agent-config".to_string()],
                });
            }

            let claude_commands = path.join(".claude/commands");
            let claude_skills = path.join(".claude-plugin");
            if !claude_commands.exists() && !claude_skills.exists() {
                issues.push(OnboardingIssue {
                    title: "Add project-specific skills".to_string(),
                    description: "Create custom slash commands in .claude/commands/ or skills in .claude-plugin/.".to_string(),
                    priority: 3,
                    labels: vec!["onboarding".to_string(), "customization".to_string()],
                });
            }
        }
    }

    Ok(issues)
}

/// Create beads from onboarding issues using bd CLI
/// Returns (epic_id, task_count)
pub fn create_onboarding_beads(
    path: &Path,
    issues: &[OnboardingIssue],
) -> Result<(Option<String>, usize)> {
    if issues.is_empty() {
        return Ok((None, 0));
    }

    // Check if we need --no-db mode (JSONL exists but no database)
    let jsonl_path = path.join(".beads/issues.jsonl");
    let db_path = path.join(".beads/beads.db");
    let use_no_db = jsonl_path.exists() && !db_path.exists();

    // First, create the parent epic for Agent Onboarding
    let mut epic_cmd = Command::new("bd");
    if use_no_db {
        epic_cmd.arg("--no-db");
    }
    epic_cmd
        .arg("create")
        .arg("--title")
        .arg("Agent Onboarding")
        .arg("--body")
        .arg("Configure AI agents for this repository. Complete these tasks to set up agent configurations for Claude, Cursor, Kiro, Aider, and GitHub Copilot.")
        .arg("--priority")
        .arg("2")
        .arg("--type")
        .arg("epic")
        .arg("--label")
        .arg("onboarding")
        .current_dir(path);

    let epic_output = epic_cmd.output()?;
    let epic_id = if epic_output.status.success() {
        // Parse the epic ID from output (e.g., "✓ Created issue: proj-abc")
        let stdout = String::from_utf8_lossy(&epic_output.stdout);
        stdout
            .lines()
            .find(|l| l.contains("Created issue:"))
            .and_then(|l| l.split(':').last())
            .map(|s| s.trim().to_string())
    } else {
        None
    };

    // Create individual tasks
    let mut created = 0;
    let mut task_ids: Vec<String> = Vec::new();

    for issue in issues {
        let labels = issue.labels.join(",");
        let mut cmd = Command::new("bd");

        if use_no_db {
            cmd.arg("--no-db");
        }

        cmd.arg("create")
            .arg("--title")
            .arg(&issue.title)
            .arg("--body")
            .arg(&issue.description)
            .arg("--priority")
            .arg(issue.priority.to_string())
            .arg("--type")
            .arg("task")
            .arg("--label")
            .arg(&labels)
            .current_dir(path);

        let output = cmd.output()?;
        if output.status.success() {
            created += 1;
            // Parse task ID for dependency linking
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(task_id) = stdout
                .lines()
                .find(|l| l.contains("Created issue:"))
                .and_then(|l| l.split(':').last())
                .map(|s| s.trim().to_string())
            {
                task_ids.push(task_id);
            }
        }
    }

    // Link epic to tasks as dependencies (tasks block epic - epic can't close until tasks done)
    if let Some(ref epic) = epic_id {
        for task_id in &task_ids {
            let mut dep_cmd = Command::new("bd");
            if use_no_db {
                dep_cmd.arg("--no-db");
            }
            // bd dep add <issue> <depends-on>
            // epic depends on task (task blocks epic)
            dep_cmd
                .arg("dep")
                .arg("add")
                .arg(epic)
                .arg(task_id)
                .current_dir(path);
            let _ = dep_cmd.status(); // Ignore errors - dependency is optional
        }
    }

    Ok((epic_id, created))
}

/// Stage 5: Configure skills marketplace
///
/// Configures Claude plugin marketplaces from the provided list.
/// Only enables beads and allbeads plugins - other plugins from marketplaces
/// are made available but not auto-enabled (users can enable what they need).
///
/// Each marketplace can be specified as:
/// - GitHub repo: "owner/repo" or "github.com/owner/repo"
/// - Git URL: "git@github.com:owner/repo.git" or "https://github.com/owner/repo.git"
pub fn configure_skills(path: &Path, configured_marketplaces: &[String]) -> Result<()> {
    let claude_dir = path.join(".claude");
    let settings_file = claude_dir.join("settings.json");

    // Create .claude directory if it doesn't exist
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
        println!("  Created .claude/ directory");
    }

    // Load or create settings.json
    let mut settings = if settings_file.exists() {
        let content = fs::read_to_string(&settings_file)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    let settings = settings.as_object_mut().ok_or_else(|| {
        crate::AllBeadsError::Config("settings.json is not an object".to_string())
    })?;

    // Ensure both sections exist first
    if !settings.contains_key("enabledPlugins") {
        settings.insert("enabledPlugins".to_string(), serde_json::json!({}));
    }
    if !settings.contains_key("extraKnownMarketplaces") {
        settings.insert("extraKnownMarketplaces".to_string(), serde_json::json!({}));
    }

    let mut marketplace_count = 0;

    // Add all configured marketplaces
    {
        let marketplaces = settings
            .get_mut("extraKnownMarketplaces")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| {
                crate::AllBeadsError::Config("extraKnownMarketplaces is not an object".to_string())
            })?;

        for marketplace_url in configured_marketplaces {
            let (marketplace_name, source_config) = parse_marketplace_url(marketplace_url);
            marketplaces.insert(marketplace_name.clone(), source_config);
            println!("    - {}", marketplace_name);
            marketplace_count += 1;
        }
    }

    // Only auto-enable beads and allbeads - the core plugins for this project
    {
        let enabled_plugins = settings
            .get_mut("enabledPlugins")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| {
                crate::AllBeadsError::Config("enabledPlugins is not an object".to_string())
            })?;

        enabled_plugins.insert(
            "beads@beads-marketplace".to_string(),
            serde_json::json!(true),
        );
        enabled_plugins.insert(
            "allbeads@allbeads-marketplace".to_string(),
            serde_json::json!(true),
        );
    }

    // Write settings.json
    let content = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_file, content)?;

    println!(
        "  ✓ Configured .claude/settings.json ({} marketplaces)",
        marketplace_count
    );
    println!("    Enabled: beads, allbeads");

    Ok(())
}

/// Generate an onboarding issue suggesting the user review available marketplace plugins
///
/// Creates a single bead listing configured marketplaces and suggesting users
/// consider which plugins to enable for their workflow.
pub fn marketplace_suggestion_issue(configured_marketplaces: &[String]) -> Option<OnboardingIssue> {
    if configured_marketplaces.is_empty() {
        return None;
    }

    // Build list of marketplace names
    let marketplace_names: Vec<String> = configured_marketplaces
        .iter()
        .map(|url| {
            let (name, _) = parse_marketplace_url(url);
            name
        })
        .collect();

    let marketplace_list = marketplace_names
        .iter()
        .map(|name| format!("- {}", name))
        .collect::<Vec<_>>()
        .join("\n");

    Some(OnboardingIssue {
        title: "Review and enable marketplace plugins".to_string(),
        description: format!(
            "The following plugin marketplaces have been configured for this project:\n\n\
            {}\n\n\
            Consider which plugins would be useful for your workflow and enable them:\n\n\
            ```bash\n\
            # List available plugins\n\
            claude plugin marketplace list\n\n\
            # Enable a plugin\n\
            claude plugin enable <plugin>@<marketplace>\n\
            ```\n\n\
            Note: beads and allbeads plugins are enabled by default.",
            marketplace_list
        ),
        priority: 4, // Low priority - informational
        labels: vec!["onboarding".to_string(), "plugins".to_string()],
    })
}

/// Parse a marketplace URL into a name and source configuration
///
/// Supports formats:
/// - "owner/repo" → github source
/// - "git@github.com:owner/repo.git" → github source
/// - "https://github.com/owner/repo.git" → github source
fn parse_marketplace_url(url: &str) -> (String, serde_json::Value) {
    // Try to extract owner/repo from various URL formats
    let repo = if url.starts_with("git@github.com:") {
        // git@github.com:owner/repo.git
        let without_prefix = url.strip_prefix("git@github.com:").unwrap_or(url);
        without_prefix
            .strip_suffix(".git")
            .unwrap_or(without_prefix)
            .to_string()
    } else if url.starts_with("https://github.com/") {
        // https://github.com/owner/repo.git or https://github.com/owner/repo
        let without_prefix = url.strip_prefix("https://github.com/").unwrap_or(url);
        without_prefix
            .strip_suffix(".git")
            .unwrap_or(without_prefix)
            .to_string()
    } else if url.contains('/') && !url.contains(':') && !url.contains("://") {
        // Simple owner/repo format
        url.to_string()
    } else {
        // Unknown format, use as-is
        url.to_string()
    };

    // Generate marketplace name from repo name
    // e.g., "thrashr888/AllBeads" → "allbeads-marketplace"
    // e.g., "steveyegge/beads" → "beads-marketplace"
    let name_part = repo.split('/').last().unwrap_or(&repo);
    let marketplace_name = format!(
        "{}-marketplace",
        name_part.to_lowercase().replace(".git", "")
    );

    let source_config = serde_json::json!({
        "source": {
            "source": "github",
            "repo": repo
        }
    });

    (marketplace_name, source_config)
}

/// Stage 7: Detect CI/CD
pub fn detect_ci_cd(path: &Path) -> Result<()> {
    let gh_actions = path.join(".github/workflows");

    if gh_actions.exists() {
        if let Ok(entries) = fs::read_dir(&gh_actions) {
            let workflows: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "yml" || ext == "yaml")
                        .unwrap_or(false)
                })
                .collect();

            if !workflows.is_empty() {
                println!(
                    "  ✓ Detected CI/CD: GitHub Actions ({} workflows)",
                    workflows.len()
                );
                return Ok(());
            }
        }
    }

    println!("  No CI/CD detected");
    Ok(())
}

/// Stage 8: Add to AllBeads config
pub fn add_to_allbeads_config(
    context_name: &str,
    repo_info: &RepoInfo,
    config: &AllBeadsConfig,
) -> Result<()> {
    // Check if context already exists
    if config.get_context(context_name).is_some() {
        println!("  ⚠ Context '{}' already exists in config", context_name);
        println!("  Skipping config update");
        return Ok(());
    }

    // Determine auth strategy
    let auth_strategy = if repo_info
        .url
        .as_ref()
        .map(|u| u.starts_with("git@"))
        .unwrap_or(false)
    {
        AuthStrategy::SshAgent
    } else {
        AuthStrategy::PersonalAccessToken
    };

    // Create new context
    let url = repo_info
        .url
        .as_ref()
        .ok_or_else(|| crate::AllBeadsError::Config("No URL available for context".to_string()))?;

    let new_context = BossContext::new(context_name, url, auth_strategy.clone())
        .with_path(repo_info.path.clone());

    // Load config, add context, save
    let config_path = AllBeadsConfig::default_path();
    let mut config = AllBeadsConfig::load(&config_path)?;
    config.add_context(new_context);
    config.save(&config_path)?;

    println!("  ✓ Added context '{}' to AllBeads config", context_name);
    println!("    Path: {}", repo_info.path.display());
    println!("    Auth: {:?}", auth_strategy);

    Ok(())
}

/// Stage 9: Print summary
pub fn print_onboarding_summary(
    repo_info: &RepoInfo,
    context_name: &str,
    skip_beads: bool,
    skip_skills: bool,
) {
    println!("Repository:      {}", repo_info.name);
    println!("Local Path:      {}", repo_info.path.display());
    if let Some(ref org) = repo_info.organization {
        println!("Organization:    {}", org);
    }
    println!();

    println!("Status:");
    let beads = if skip_beads { "✗" } else { "✓" };
    let skills = if skip_skills { "✗" } else { "✓" };
    println!("  [{}] Beads initialized", beads);
    println!("  [{}] Skills configured", skills);
    println!("  [✓] Added to AllBeads as '@{}'", context_name);
    println!();

    println!("Next steps:");
    println!("  • Create your first issue:  bd create --title=\"Initial setup\"");
    println!("  • View all contexts:        ab tui (Tab to Contexts view)");
    println!("  • Add GitHub integration:   ab github status");
    println!("  • Add JIRA integration:     ab jira status");
}

/// Stage 9: Commit and push onboarding changes
pub fn commit_and_push_onboarding(path: &Path, non_interactive: bool) -> Result<()> {
    // Check if there are changes to commit
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()?;

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.trim().is_empty() {
        println!("  No changes to commit");
        return Ok(());
    }

    // Show what will be committed
    let files_to_add = [".beads/", ".claude/", "AGENTS.md", ".gitattributes"];
    let mut has_changes = false;

    for file in &files_to_add {
        let file_path = path.join(file);
        if file_path.exists() {
            has_changes = true;
        }
    }

    if !has_changes {
        println!("  No onboarding files to commit");
        return Ok(());
    }

    if !non_interactive {
        println!("  The following files will be committed:");
        for file in &files_to_add {
            if path.join(file).exists() {
                println!("    + {}", file);
            }
        }
        print!("  Commit and push? [Y/n]: ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input == "n" || input == "no" {
            println!("  Skipping commit");
            return Ok(());
        }
    }

    // Add files
    println!("  Adding files...");
    for file in &files_to_add {
        if path.join(file).exists() {
            let _ = Command::new("git")
                .args(["add", file])
                .current_dir(path)
                .output();
        }
    }

    // Commit
    println!("  Committing...");
    let commit_output = Command::new("git")
        .args([
            "commit",
            "-m",
            "Initialize AllBeads onboarding\n\n- Add beads tracking (.beads/)\n- Configure Claude skills (.claude/)\n- Add AGENTS.md",
        ])
        .current_dir(path)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stderr.contains("nothing to commit") {
            println!("  No changes to commit");
            return Ok(());
        }
        println!("  Warning: Commit may have failed: {}", stderr.trim());
    } else {
        println!("  ✓ Committed onboarding files");
    }

    // Push
    println!("  Pushing to remote...");
    let push_output = Command::new("git")
        .args(["push"])
        .current_dir(path)
        .stderr(std::process::Stdio::piped())
        .output()?;

    if push_output.status.success() {
        println!("  ✓ Pushed to remote");
    } else {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        println!("  Warning: Push may have failed: {}", stderr.trim());
        println!("  You can push manually later with: git push");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_url_https() {
        let url = "https://github.com/user/repo.git";
        let parsed = parse_repo_url(url);
        assert!(parsed.is_ok());
        let (name, org) = parsed.unwrap();
        assert_eq!(name, "repo");
        assert_eq!(org, "user");
    }

    #[test]
    fn test_parse_repo_url_ssh() {
        let url = "git@github.com:user/repo.git";
        let parsed = parse_repo_url(url);
        assert!(parsed.is_ok());
        let (name, org) = parsed.unwrap();
        assert_eq!(name, "repo");
        assert_eq!(org, "user");
    }

    #[test]
    fn test_parse_repo_url_without_git_suffix() {
        let url = "https://github.com/user/repo";
        let parsed = parse_repo_url(url);
        assert!(parsed.is_ok());
        let (name, _) = parsed.unwrap();
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_repo_url_invalid() {
        let url = "not-a-valid-url";
        let parsed = parse_repo_url(url);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_repo_info_struct() {
        let info = RepoInfo {
            name: "test-repo".to_string(),
            path: PathBuf::from("/tmp/test-repo"),
            url: Some("https://github.com/user/test-repo.git".to_string()),
            organization: Some("user".to_string()),
            exists_locally: false,
        };

        assert_eq!(info.name, "test-repo");
        assert!(!info.exists_locally);
        assert!(info.url.is_some());
        assert!(info.organization.is_some());
    }

    #[test]
    fn test_parse_marketplace_url_simple() {
        let (name, config) = parse_marketplace_url("steveyegge/beads");
        assert_eq!(name, "beads-marketplace");
        assert_eq!(config["source"]["repo"], "steveyegge/beads");
    }

    #[test]
    fn test_parse_marketplace_url_ssh() {
        let (name, config) = parse_marketplace_url("git@github.com:openprose/prose.git");
        assert_eq!(name, "prose-marketplace");
        assert_eq!(config["source"]["repo"], "openprose/prose");
    }

    #[test]
    fn test_parse_marketplace_url_https() {
        let (name, config) = parse_marketplace_url("https://github.com/thrashr888/AllBeads.git");
        assert_eq!(name, "allbeads-marketplace");
        assert_eq!(config["source"]["repo"], "thrashr888/AllBeads");
    }

    #[test]
    fn test_parse_marketplace_url_https_no_git() {
        let (name, config) = parse_marketplace_url("https://github.com/anthropics/skills");
        assert_eq!(name, "skills-marketplace");
        assert_eq!(config["source"]["repo"], "anthropics/skills");
    }
}
