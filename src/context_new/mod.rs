//! GitHub repository creation module
//!
//! Implements `ab context new` - create new GitHub repos with AllBeads pre-configured.

use crate::{AllBeadsError, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for creating a new repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRepoConfig {
    /// Repository name
    pub name: String,
    /// Repository description (optional)
    pub description: Option<String>,
    /// Whether the repo should be private
    pub private: bool,
    /// Organization to create in (None = user account)
    pub org: Option<String>,
    /// .gitignore template name
    pub gitignore: Option<String>,
    /// License template name
    pub license: Option<String>,
    /// AllBeads template to apply after creation
    pub template: Option<String>,
    /// Whether to initialize beads
    pub init_beads: bool,
    /// List of agents to configure
    pub init_agents: Vec<String>,
    /// Custom clone path (None = default workspace)
    pub clone_path: Option<String>,
    /// Don't clone locally (create on GitHub only)
    pub no_clone: bool,
}

impl Default for NewRepoConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            private: false,
            org: None,
            gitignore: None,
            license: None,
            template: None,
            init_beads: true,
            init_agents: vec!["claude".to_string()],
            clone_path: None,
            no_clone: false,
        }
    }
}

/// Result of creating a new repository
#[derive(Debug)]
pub struct NewRepoResult {
    /// Repository name
    pub repo_name: String,
    /// Full repository name (owner/repo)
    pub full_name: String,
    /// HTML URL for the repository
    pub html_url: String,
    /// Clone URL (SSH preferred)
    pub clone_url: String,
    /// Local path if cloned
    pub local_path: Option<PathBuf>,
    /// Whether beads was initialized
    pub beads_initialized: bool,
    /// List of agents that were configured
    pub agents_configured: Vec<String>,
}

/// Interactive prompt for new repository configuration
pub struct NewRepoPrompt {
    config: NewRepoConfig,
}

impl NewRepoPrompt {
    /// Create a new prompt instance
    pub fn new() -> Self {
        Self {
            config: NewRepoConfig::default(),
        }
    }

    /// Run the full interactive wizard
    pub fn run(self) -> Result<NewRepoConfig> {
        println!("\n{}", crate::style::header("Create a new repository"));
        println!();

        let mut config = self.config;

        // Repository name (required)
        config.name = prompt_required("Repository name")?;

        // Description (optional)
        config.description = prompt_optional("Description (optional)")?;

        // Visibility
        config.private = prompt_yes_no("Private repository?", false)?;

        // Organization (optional)
        config.org = prompt_optional("Organization (leave empty for personal account)")?;

        // .gitignore template
        println!("\nCommon .gitignore templates: Rust, Node, Python, Go, Java");
        config.gitignore = prompt_optional(".gitignore template")?;

        // License
        println!("\nCommon licenses: MIT, Apache-2.0, GPL-3.0, BSD-3-Clause");
        config.license = prompt_optional("License")?;

        // AllBeads configuration
        println!("\n{}", crate::style::subheader("AllBeads Configuration"));
        config.init_beads = prompt_yes_no("Initialize beads?", true)?;

        // Agents
        println!("\nAvailable agents: claude, cursor, copilot, aider");
        let agents_str = prompt_with_default("Agents to configure (comma-separated)", "claude")?;
        config.init_agents = agents_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Clone path
        config.clone_path = prompt_optional("Clone path (leave empty for default workspace)")?;

        // Confirm
        println!("\n{}", crate::style::subheader("Summary"));
        println!("  Name:        {}", config.name);
        if let Some(ref desc) = config.description {
            println!("  Description: {}", desc);
        }
        println!(
            "  Visibility:  {}",
            if config.private { "Private" } else { "Public" }
        );
        if let Some(ref org) = config.org {
            println!("  Organization: {}", org);
        }
        if let Some(ref gi) = config.gitignore {
            println!("  .gitignore:  {}", gi);
        }
        if let Some(ref lic) = config.license {
            println!("  License:     {}", lic);
        }
        println!(
            "  Beads:       {}",
            if config.init_beads { "Yes" } else { "No" }
        );
        println!("  Agents:      {}", config.init_agents.join(", "));
        println!();

        if !prompt_yes_no("Create repository?", true)? {
            return Err(AllBeadsError::Config("Cancelled by user".to_string()));
        }

        Ok(config)
    }

    /// Run with pre-filled defaults, prompting only for missing required fields
    #[allow(clippy::too_many_arguments)]
    pub fn run_with_defaults(
        self,
        name: Option<String>,
        description: Option<String>,
        private: bool,
        org: Option<String>,
        gitignore: Option<String>,
        license: Option<String>,
        template: Option<String>,
        init_beads: bool,
        init_agents: String,
        clone_path: Option<String>,
        no_clone: bool,
    ) -> Result<NewRepoConfig> {
        let mut config = self.config;

        // Only prompt for name if not provided
        config.name = if let Some(n) = name {
            n
        } else {
            prompt_required("Repository name")?
        };

        config.description = description;
        config.private = private;
        config.org = org;
        config.gitignore = gitignore;
        config.license = license;
        config.template = template;
        config.init_beads = init_beads;
        config.init_agents = init_agents
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config.clone_path = clone_path;
        config.no_clone = no_clone;

        Ok(config)
    }
}

impl Default for NewRepoPrompt {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new repository on GitHub with full AllBeads setup
pub fn create_new_repository(config: &NewRepoConfig) -> Result<NewRepoResult> {
    // Step 1: Verify GitHub CLI is available and authenticated
    verify_gh_cli()?;

    println!(
        "\n{}",
        crate::style::subheader("Creating repository on GitHub...")
    );

    // Step 2: Create repository using gh CLI
    let (full_name, html_url, clone_url) = create_github_repo(config)?;
    println!("{} Created {}", crate::style::success("✓"), html_url);

    let repo_name = config.name.clone();
    let mut local_path: Option<PathBuf> = None;
    let mut beads_initialized = false;
    let mut agents_configured: Vec<String> = Vec::new();

    // Step 3: Clone if requested
    if !config.no_clone {
        let clone_dir = if let Some(ref path) = config.clone_path {
            PathBuf::from(path)
        } else {
            // Default to ~/Workspace/<repo_name>
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Workspace")
                .join(&repo_name)
        };

        println!(
            "{} Cloning to {}...",
            crate::style::info("→"),
            clone_dir.display()
        );

        clone_repository(&clone_url, &clone_dir)?;
        local_path = Some(clone_dir.clone());
        println!(
            "{} Cloned to {}",
            crate::style::success("✓"),
            clone_dir.display()
        );

        // Step 4: Initialize beads if requested
        if config.init_beads {
            println!("{} Initializing beads...", crate::style::info("→"));
            init_beads(&clone_dir)?;
            beads_initialized = true;
            println!("{} Beads initialized", crate::style::success("✓"));
        }

        // Step 5: Configure agents
        if !config.init_agents.is_empty() {
            println!("{} Configuring agents...", crate::style::info("→"));
            agents_configured = configure_agents(&clone_dir, &config.init_agents)?;
            if !agents_configured.is_empty() {
                println!(
                    "{} Configured: {}",
                    crate::style::success("✓"),
                    agents_configured.join(", ")
                );
            }
        }

        // Step 6: Commit and push configuration
        if beads_initialized || !agents_configured.is_empty() {
            println!("{} Committing configuration...", crate::style::info("→"));
            commit_and_push(&clone_dir)?;
            println!("{} Pushed to GitHub", crate::style::success("✓"));
        }
    }

    Ok(NewRepoResult {
        repo_name,
        full_name,
        html_url,
        clone_url,
        local_path,
        beads_initialized,
        agents_configured,
    })
}

/// Verify gh CLI is installed and authenticated
fn verify_gh_cli() -> Result<()> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map_err(|e| {
            AllBeadsError::Config(format!(
                "GitHub CLI (gh) not found. Install it with: brew install gh\nError: {}",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!(
            "GitHub CLI not authenticated. Run: gh auth login\n{}",
            stderr
        )));
    }

    Ok(())
}

/// Create a repository on GitHub using gh CLI
fn create_github_repo(config: &NewRepoConfig) -> Result<(String, String, String)> {
    let mut args = vec!["repo", "create"];

    // Add name (with org if specified)
    let repo_spec = if let Some(ref org) = config.org {
        format!("{}/{}", org, config.name)
    } else {
        config.name.clone()
    };
    args.push(&repo_spec);

    // Add visibility
    if config.private {
        args.push("--private");
    } else {
        args.push("--public");
    }

    // Add description
    let desc_arg;
    if let Some(ref desc) = config.description {
        desc_arg = format!("--description={}", desc);
        args.push(&desc_arg);
    }

    // Add gitignore
    let gi_arg;
    if let Some(ref gi) = config.gitignore {
        gi_arg = format!("--gitignore={}", gi);
        args.push(&gi_arg);
    }

    // Add license
    let lic_arg;
    if let Some(ref lic) = config.license {
        lic_arg = format!("--license={}", lic);
        args.push(&lic_arg);
    }

    // Confirm creation (no prompts)
    args.push("--confirm");

    let output = Command::new("gh")
        .args(&args)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to run gh repo create: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!(
            "Failed to create repository: {}",
            stderr
        )));
    }

    // Parse the URL from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let html_url = stdout.trim().to_string();

    // Get the full repo info using gh repo view
    let view_output = Command::new("gh")
        .args(["repo", "view", &repo_spec, "--json", "nameWithOwner,sshUrl"])
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to get repo info: {}", e)))?;

    let view_json: serde_json::Value = serde_json::from_slice(&view_output.stdout)
        .map_err(|e| AllBeadsError::Config(format!("Failed to parse repo info: {}", e)))?;

    let full_name = view_json["nameWithOwner"]
        .as_str()
        .unwrap_or(&repo_spec)
        .to_string();
    let clone_url = view_json["sshUrl"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("git@github.com:{}.git", full_name));

    Ok((full_name, html_url, clone_url))
}

/// Clone the repository to the specified path
fn clone_repository(url: &str, path: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AllBeadsError::Config(format!("Failed to create directory: {}", e)))?;
    }

    let output = Command::new("git")
        .args(["clone", url, path.to_str().unwrap_or(".")])
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to clone: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!("Clone failed: {}", stderr)));
    }

    Ok(())
}

/// Initialize beads in the repository
fn init_beads(path: &Path) -> Result<()> {
    let output = Command::new("bd")
        .args(["init"])
        .current_dir(path)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to run bd init: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't fail if already initialized
        if !stderr.contains("already") {
            return Err(AllBeadsError::Config(format!("bd init failed: {}", stderr)));
        }
    }

    Ok(())
}

/// Configure AI agents in the repository
fn configure_agents(path: &Path, agents: &[String]) -> Result<Vec<String>> {
    let mut configured = Vec::new();

    for agent in agents {
        match agent.to_lowercase().as_str() {
            "claude" => {
                let claude_md = path.join("CLAUDE.md");
                if !claude_md.exists() {
                    let content = generate_claude_md(path)?;
                    std::fs::write(&claude_md, content).map_err(|e| {
                        AllBeadsError::Config(format!("Failed to write CLAUDE.md: {}", e))
                    })?;
                    configured.push("claude".to_string());
                }
            }
            "cursor" => {
                let cursorrules = path.join(".cursorrules");
                if !cursorrules.exists() {
                    let content = generate_cursorrules(path)?;
                    std::fs::write(&cursorrules, content).map_err(|e| {
                        AllBeadsError::Config(format!("Failed to write .cursorrules: {}", e))
                    })?;
                    configured.push("cursor".to_string());
                }
            }
            "copilot" => {
                let copilot_dir = path.join(".github");
                let copilot_file = copilot_dir.join("copilot-instructions.md");
                if !copilot_file.exists() {
                    std::fs::create_dir_all(&copilot_dir).map_err(|e| {
                        AllBeadsError::Config(format!("Failed to create .github: {}", e))
                    })?;
                    let content = generate_copilot_instructions(path)?;
                    std::fs::write(&copilot_file, content).map_err(|e| {
                        AllBeadsError::Config(format!(
                            "Failed to write copilot-instructions.md: {}",
                            e
                        ))
                    })?;
                    configured.push("copilot".to_string());
                }
            }
            "aider" => {
                let aider_conf = path.join(".aider.conf.yml");
                if !aider_conf.exists() {
                    let content = generate_aider_conf()?;
                    std::fs::write(&aider_conf, content).map_err(|e| {
                        AllBeadsError::Config(format!("Failed to write .aider.conf.yml: {}", e))
                    })?;
                    configured.push("aider".to_string());
                }
            }
            _ => {
                eprintln!("{} Unknown agent: {}", crate::style::warning("⚠"), agent);
            }
        }
    }

    Ok(configured)
}

/// Generate CLAUDE.md content
fn generate_claude_md(path: &Path) -> Result<String> {
    let repo_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    Ok(format!(
        r#"# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**{name}** - [Add project description here]

## Build Commands

```bash
# Build the project
[Add build command]

# Run tests
[Add test command]

# Run the project
[Add run command]
```

## Architecture Overview

[Describe your project architecture here]

## Development Workflow

[Describe your development workflow here]

## Beads Issue Tracking

This repository uses `bd` (beads) for issue tracking.

### Essential Beads Commands

```bash
# Create issues
bd create --title="Implement feature X" --type=feature --priority=1

# List and filter
bd list --status=open
bd ready                    # Show unblocked work

# Update work
bd update <id> --status=in_progress
bd close <id>

# Dependencies
bd dep add <issue> <depends-on>
```

Use beads for tracking multi-session work and complex features with dependencies.
"#,
        name = repo_name
    ))
}

/// Generate .cursorrules content
fn generate_cursorrules(path: &Path) -> Result<String> {
    let repo_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    Ok(format!(
        r#"# Cursor Rules for {name}

## Project Context
This project uses beads (bd) for issue tracking.

## Code Style
- Follow existing code style
- Write clear, concise code
- Add comments for complex logic

## Issue Tracking
- Reference issues with bd commands
- Use `bd list --status=open` to see current work
- Use `bd update <id> --status=in_progress` when starting work
"#,
        name = repo_name
    ))
}

/// Generate copilot-instructions.md content
fn generate_copilot_instructions(path: &Path) -> Result<String> {
    let repo_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    Ok(format!(
        r#"# GitHub Copilot Instructions for {name}

## Project Overview
[Add project description]

## Issue Tracking
This project uses beads (bd) for issue tracking.
Use `bd list` to see open issues.
"#,
        name = repo_name
    ))
}

/// Generate .aider.conf.yml content
fn generate_aider_conf() -> Result<String> {
    Ok(r#"# Aider configuration
auto-commits: true
gitignore: true
"#
    .to_string())
}

/// Commit and push configuration changes
fn commit_and_push(path: &PathBuf) -> Result<()> {
    // Stage all changes
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(path)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to stage changes: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!("git add failed: {}", stderr)));
    }

    // Check if there are changes to commit
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to check git status: {}", e)))?;

    let status_output = String::from_utf8_lossy(&status.stdout);
    if status_output.trim().is_empty() {
        // Nothing to commit
        return Ok(());
    }

    // Commit
    let output = Command::new("git")
        .args([
            "commit",
            "-m",
            "Initialize AllBeads configuration\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        ])
        .current_dir(path)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to commit: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't fail if nothing to commit
        if !stderr.contains("nothing to commit") {
            return Err(AllBeadsError::Config(format!(
                "git commit failed: {}",
                stderr
            )));
        }
    }

    // Push
    let output = Command::new("git")
        .args(["push"])
        .current_dir(path)
        .output()
        .map_err(|e| AllBeadsError::Config(format!("Failed to push: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!(
            "git push failed: {}",
            stderr
        )));
    }

    Ok(())
}

// === Helper functions for prompting ===

fn prompt_required(prompt: &str) -> Result<String> {
    loop {
        print!("  {}: ", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
        println!("  {} This field is required", crate::style::error("✗"));
    }
}

fn prompt_optional(prompt: &str) -> Result<Option<String>> {
    print!("  {}: ", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool> {
    let default_str = if default { "[Y/n]" } else { "[y/N]" };
    print!("  {} {}: ", prompt, default_str);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else if trimmed == "y" || trimmed == "yes" {
        Ok(true)
    } else if trimmed == "n" || trimmed == "no" {
        Ok(false)
    } else {
        Ok(default)
    }
}

fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("  {} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_repo_config_default() {
        let config = NewRepoConfig::default();
        assert!(config.name.is_empty());
        assert!(config.description.is_none());
        assert!(!config.private);
        assert!(config.org.is_none());
        assert!(config.init_beads);
        assert_eq!(config.init_agents, vec!["claude".to_string()]);
    }

    #[test]
    fn test_generate_claude_md() {
        let path = PathBuf::from("/tmp/test-project");
        let content = generate_claude_md(&path).unwrap();
        assert!(content.contains("test-project"));
        assert!(content.contains("CLAUDE.md"));
        assert!(content.contains("beads"));
    }

    #[test]
    fn test_generate_cursorrules() {
        let path = PathBuf::from("/tmp/test-project");
        let content = generate_cursorrules(&path).unwrap();
        assert!(content.contains("test-project"));
        assert!(content.contains("bd"));
    }

    #[test]
    fn test_generate_aider_conf() {
        let content = generate_aider_conf().unwrap();
        assert!(content.contains("auto-commits"));
    }
}
