//! Interactive repository onboarding workflow

use crate::config::{AllBeadsConfig, AuthStrategy, BossContext};
use crate::Result;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Interactive onboarding workflow for a repository
pub struct OnboardingWorkflow {
    /// Path to the repository
    repo_path: PathBuf,

    /// Repository name
    repo_name: String,

    /// Remote URL (if available)
    remote_url: Option<String>,

    /// Skip interactive prompts
    non_interactive: bool,

    /// Skip steps
    skip_init: bool,
    skip_claude: bool,
    skip_context: bool,
}

impl OnboardingWorkflow {
    /// Create a new onboarding workflow
    pub fn new(
        path: impl AsRef<Path>,
        non_interactive: bool,
        skip_init: bool,
        skip_claude: bool,
        skip_context: bool,
    ) -> Result<Self> {
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

        // Try to get remote URL
        let remote_url = Self::get_remote_url(&repo_path).ok();

        Ok(Self {
            repo_path,
            repo_name,
            remote_url,
            non_interactive,
            skip_init,
            skip_claude,
            skip_context,
        })
    }

    /// Run the onboarding workflow
    pub fn run(&self) -> Result<()> {
        eprintln!("\n🚀 AllBeads Repository Onboarding\n");
        eprintln!("Repository: {}", self.repo_name);
        eprintln!("Path: {}", self.repo_path.display());
        if let Some(ref url) = self.remote_url {
            eprintln!("Remote: {}", url);
        }
        eprintln!();

        // Step 1: Initialize beads
        if !self.skip_init {
            self.step_init_beads()?;
        }

        // Step 2: Setup CLAUDE.md
        if !self.skip_claude {
            self.step_setup_claude()?;
        }

        // Step 3: Add to AllBeads contexts
        if !self.skip_context {
            self.step_add_context()?;
        }

        // Step 4: Summary and next steps
        self.show_summary()?;

        eprintln!("\n✅ Onboarding complete!\n");

        Ok(())
    }

    /// Step 1: Initialize beads tracking
    fn step_init_beads(&self) -> Result<()> {
        eprintln!("📦 Step 1: Initialize Beads Tracking\n");

        let beads_dir = self.repo_path.join(".beads");
        if beads_dir.exists() {
            eprintln!("  ✓ Beads already initialized (.beads/ directory exists)");
            eprintln!();
            return Ok(());
        }

        if !self.non_interactive {
            eprintln!("  Initialize beads tracking in this repository?");
            if !self.confirm("  Initialize beads? [Y/n]: ")? {
                eprintln!("  ⊘ Skipped beads initialization");
                eprintln!();
                return Ok(());
            }
        }

        eprintln!("  Running: bd init");
        let output = Command::new("bd")
            .arg("init")
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                crate::AllBeadsError::Config(format!(
                    "Failed to run 'bd init': {}. Is 'bd' installed?",
                    e
                ))
            })?;

        if output.status.success() {
            eprintln!("  ✓ Beads initialized successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::AllBeadsError::Config(format!(
                "bd init failed: {}",
                stderr
            )));
        }

        eprintln!();
        Ok(())
    }

    /// Step 2: Setup CLAUDE.md
    fn step_setup_claude(&self) -> Result<()> {
        eprintln!("📝 Step 2: Setup CLAUDE.md\n");

        let claude_md = self.repo_path.join("CLAUDE.md");
        if claude_md.exists() {
            eprintln!("  ✓ CLAUDE.md already exists");
            eprintln!();
            return Ok(());
        }

        if !self.non_interactive {
            eprintln!("  CLAUDE.md provides guidance to AI agents working in this repository.");
            eprintln!("  Create a starter CLAUDE.md file?");
            if !self.confirm("  Create CLAUDE.md? [Y/n]: ")? {
                eprintln!("  ⊘ Skipped CLAUDE.md creation");
                eprintln!();
                return Ok(());
            }
        }

        let template = self.generate_claude_template();
        std::fs::write(&claude_md, template).map_err(|e| {
            crate::AllBeadsError::Config(format!("Failed to write CLAUDE.md: {}", e))
        })?;

        eprintln!("  ✓ Created CLAUDE.md");
        eprintln!("  ℹ  Edit CLAUDE.md to add project-specific guidance");
        eprintln!();

        Ok(())
    }

    /// Step 3: Add repository to AllBeads contexts
    fn step_add_context(&self) -> Result<()> {
        eprintln!("🔗 Step 3: Add to AllBeads Contexts\n");

        // Load AllBeads config
        let config_file = AllBeadsConfig::default_path();
        let mut config = if config_file.exists() {
            AllBeadsConfig::load(&config_file)?
        } else {
            eprintln!("  ⚠️  AllBeads not configured. Run 'ab setup' first.");
            eprintln!();
            return Ok(());
        };

        // Check if already added
        if config.get_context(&self.repo_name).is_some() {
            eprintln!("  ✓ Repository already in AllBeads contexts");
            eprintln!();
            return Ok(());
        }

        if !self.non_interactive {
            eprintln!("  Add this repository to AllBeads contexts for aggregation?");
            if !self.confirm("  Add to contexts? [Y/n]: ")? {
                eprintln!("  ⊘ Skipped context registration");
                eprintln!();
                return Ok(());
            }
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
        let remote_url = self.remote_url.as_ref().unwrap_or(&"".to_string()).clone();
        let mut context = BossContext::new(&self.repo_name, &remote_url, auth_strategy);
        context.path = Some(self.repo_path.clone());

        config.add_context(context);
        config.save(&config_file)?;

        eprintln!("  ✓ Added to AllBeads contexts as '{}'", self.repo_name);
        eprintln!();

        Ok(())
    }

    /// Show summary and next steps
    fn show_summary(&self) -> Result<()> {
        eprintln!("📚 Next Steps:\n");

        eprintln!("1. Create your first issue:");
        eprintln!("   cd {}", self.repo_path.display());
        eprintln!("   bd create --title=\"Initial setup\" --type=task --priority=2");
        eprintln!();

        eprintln!("2. View your issues in the TUI:");
        eprintln!("   ab tui");
        eprintln!();

        eprintln!("3. Check onboarding status:");
        eprintln!("   ab context onboarding");
        eprintln!();

        eprintln!("4. Optional: Run janitor to discover potential issues:");
        eprintln!("   ab janitor {}", self.repo_path.display());
        eprintln!();

        eprintln!("5. Optional: Setup integration with JIRA or GitHub:");
        eprintln!("   ab jira status");
        eprintln!("   ab github status");

        Ok(())
    }

    /// Generate CLAUDE.md template
    fn generate_claude_template(&self) -> String {
        format!(
            r#"# CLAUDE.md

This file provides guidance to Claude Code and other AI agents working in this repository.

## Project Overview

**{}** - [Add a brief description of what this project does]

### Tech Stack

- [List key technologies, frameworks, languages]
- [e.g., Python 3.11, FastAPI, PostgreSQL]

## Development Commands

### Setup
```bash
# Clone and setup
git clone [url]
cd {}
# [Add setup commands like: pip install -r requirements.txt]
```

### Running
```bash
# [Add commands to run the project]
# [e.g., python main.py, npm start, cargo run]
```

### Testing
```bash
# [Add test commands]
# [e.g., pytest, cargo test, npm test]
```

## Architecture

[Describe the high-level architecture]

### Directory Structure

```
.
├── [key directories]
└── [and their purposes]
```

## Beads Issue Tracking

This repository uses `bd` (beads) for issue tracking.

```bash
# Create issues
bd create --title="Feature X" --type=feature --priority=2

# List issues
bd list --status=open

# Update status
bd update <id> --status=in_progress
```

## Development Guidelines

- [Add project-specific coding standards]
- [Testing requirements]
- [Commit message conventions]
- [Any other important guidelines]

## Additional Notes

[Add any other context that would help AI agents work effectively in this codebase]
"#,
            self.repo_name, self.repo_name
        )
    }

    /// Get git remote URL
    fn get_remote_url(repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "remote", "get-url", "origin"])
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

    /// Prompt for confirmation
    fn confirm(&self, prompt: &str) -> Result<bool> {
        if self.non_interactive {
            return Ok(true);
        }

        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| crate::AllBeadsError::Config(format!("Failed to read input: {}", e)))?;

        let input = input.trim().to_lowercase();
        Ok(input.is_empty() || input == "y" || input == "yes")
    }
}
