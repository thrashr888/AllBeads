//! Coding Agent Support
//!
//! Support for multiple coding agents like Claude Code, Cursor, GitHub Copilot, and Aider.
//! Each agent has its own configuration format and file locations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported coding agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingAgent {
    /// Claude Code (CLAUDE.md, .claude-plugin/)
    Claude,
    /// Cursor (.cursorrules)
    Cursor,
    /// GitHub Copilot (.github/copilot-instructions.md)
    Copilot,
    /// Aider (.aider.conf.yml)
    Aider,
    /// Codex CLI (future)
    Codex,
    /// Gemini CLI (future)
    Gemini,
}

impl CodingAgent {
    /// Get all supported agents
    pub fn all() -> &'static [CodingAgent] {
        &[
            CodingAgent::Claude,
            CodingAgent::Cursor,
            CodingAgent::Copilot,
            CodingAgent::Aider,
        ]
    }

    /// Parse agent name from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some(Self::Claude),
            "cursor" => Some(Self::Cursor),
            "copilot" | "github-copilot" | "github_copilot" => Some(Self::Copilot),
            "aider" => Some(Self::Aider),
            "codex" => Some(Self::Codex),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Copilot => "GitHub Copilot",
            Self::Aider => "Aider",
            Self::Codex => "Codex CLI",
            Self::Gemini => "Gemini CLI",
        }
    }

    /// Get short name
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
            Self::Aider => "aider",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }

    /// Get the configuration file paths for this agent
    pub fn config_paths(&self) -> Vec<&'static str> {
        match self {
            Self::Claude => vec!["CLAUDE.md", ".claude-plugin/"],
            Self::Cursor => vec![".cursorrules", ".cursor/rules"],
            Self::Copilot => vec![".github/copilot-instructions.md"],
            Self::Aider => vec![".aider.conf.yml", ".aider/"],
            Self::Codex => vec![".codex/"],
            Self::Gemini => vec![".gemini/"],
        }
    }

    /// Check if agent is configured in the given path
    pub fn is_configured(&self, project_path: &Path) -> bool {
        for config_path in self.config_paths() {
            if project_path.join(config_path).exists() {
                return true;
            }
        }
        false
    }

    /// Get the primary config file path
    pub fn primary_config(&self) -> &'static str {
        match self {
            Self::Claude => "CLAUDE.md",
            Self::Cursor => ".cursorrules",
            Self::Copilot => ".github/copilot-instructions.md",
            Self::Aider => ".aider.conf.yml",
            Self::Codex => ".codex/config.yml",
            Self::Gemini => ".gemini/config.yml",
        }
    }

    /// Generate initial configuration content
    pub fn initial_config(&self, project_name: &str) -> String {
        match self {
            Self::Claude => format!(
                r#"# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**{}** - [Brief description of the project]

## Build Commands

```bash
# Build the project
# cargo build / npm install / etc.

# Run tests
# cargo test / npm test / etc.

# Run the project
# cargo run / npm start / etc.
```

## Architecture

[Describe the project architecture, key modules, and design decisions]

## Development Guidelines

[Add project-specific coding guidelines, conventions, and best practices]
"#,
                project_name
            ),
            Self::Cursor => format!(
                r#"# Cursor Rules for {}

## Project Context
[Brief description of the project]

## Code Style
- Follow consistent formatting
- Write clear, self-documenting code
- Add comments for complex logic

## Architecture Guidelines
[Describe key architectural decisions]

## Testing Requirements
- Write tests for new features
- Ensure existing tests pass

## Common Commands
- Build: [command]
- Test: [command]
- Run: [command]
"#,
                project_name
            ),
            Self::Copilot => format!(
                r#"# GitHub Copilot Instructions for {}

## Project Overview
[Brief description]

## Coding Standards
- Follow the project's existing code style
- Use meaningful variable and function names
- Write documentation for public APIs

## Architecture
[Key architectural patterns and decisions]

## Testing
- Write unit tests for new functionality
- Integration tests where appropriate
"#,
                project_name
            ),
            Self::Aider => format!(
                r#"# Aider Configuration

model: claude-3-5-sonnet-20241022
edit-format: diff

# Project: {}

# Environment
auto-commits: false
dirty-commits: false

# Files to include/exclude
# include:
#   - src/**/*.rs
# exclude:
#   - target/**
"#,
                project_name
            ),
            Self::Codex | Self::Gemini => format!(
                "# {} Configuration\n\nproject: {}\n",
                self.display_name(),
                project_name
            ),
        }
    }
}

/// Agent configuration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent: CodingAgent,
    pub configured: bool,
    pub config_path: Option<String>,
    pub has_allbeads_context: bool,
}

/// Detect all configured agents in a project
pub fn detect_agents(project_path: &Path) -> Vec<AgentStatus> {
    CodingAgent::all()
        .iter()
        .map(|agent| {
            let configured = agent.is_configured(project_path);
            let config_path = if configured {
                agent
                    .config_paths()
                    .iter()
                    .find(|p| project_path.join(p).exists())
                    .map(|s| s.to_string())
            } else {
                None
            };

            // Check if AllBeads context marker exists in config
            let has_allbeads_context = if let Some(ref path) = config_path {
                let full_path = project_path.join(path);
                if full_path.is_file() {
                    std::fs::read_to_string(&full_path)
                        .map(|content| content.contains("AllBeads"))
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            };

            AgentStatus {
                agent: *agent,
                configured,
                config_path,
                has_allbeads_context,
            }
        })
        .collect()
}

/// Initialize an agent configuration
pub fn init_agent(
    agent: CodingAgent,
    project_path: &Path,
    overwrite: bool,
) -> Result<PathBuf, String> {
    let config_path = project_path.join(agent.primary_config());

    // Check if already exists
    if config_path.exists() && !overwrite {
        return Err(format!(
            "Configuration already exists at {}. Use --yes to overwrite.",
            config_path.display()
        ));
    }

    // Create parent directories if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Get project name from directory
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    // Generate and write config
    let content = agent.initial_config(project_name);
    std::fs::write(&config_path, content).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(config_path)
}

/// Context info to sync to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllBeadsContext {
    pub project_name: String,
    pub beads_prefix: Option<String>,
    pub open_issues: usize,
    pub ready_issues: usize,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
}

/// Generate AllBeads context section for agent configs
pub fn generate_context_section(context: &AllBeadsContext) -> String {
    let mut section = String::new();
    section.push_str("\n## AllBeads Context\n\n");
    section.push_str(&format!("Project: {}\n", context.project_name));

    if let Some(ref prefix) = context.beads_prefix {
        section.push_str(&format!("Beads Prefix: {}\n", prefix));
    }

    section.push_str(&format!("Open Issues: {}\n", context.open_issues));
    section.push_str(&format!("Ready to Work: {}\n", context.ready_issues));

    if !context.languages.is_empty() {
        section.push_str(&format!("Languages: {}\n", context.languages.join(", ")));
    }

    if !context.frameworks.is_empty() {
        section.push_str(&format!("Frameworks: {}\n", context.frameworks.join(", ")));
    }

    section.push_str("\nUse `bd ready` to see available work.\n");
    section.push_str("Use `bd show <id>` to view issue details.\n");

    section
}

/// Sync AllBeads context to an agent's config
pub fn sync_agent_context(
    agent: CodingAgent,
    project_path: &Path,
    context: &AllBeadsContext,
) -> Result<(), String> {
    let config_path = project_path.join(agent.primary_config());

    if !config_path.exists() {
        return Err(format!(
            "{} not configured. Run 'ab agent init {}' first.",
            agent.display_name(),
            agent.short_name()
        ));
    }

    // Read existing config
    let existing = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    // Remove old AllBeads context section if present
    let content = if let Some(start) = existing.find("## AllBeads Context") {
        let before = &existing[..start];
        // Find next section or end
        let after_start = start + "## AllBeads Context".len();
        let next_section = existing[after_start..]
            .find("\n## ")
            .map(|i| after_start + i)
            .unwrap_or(existing.len());
        let after = &existing[next_section..];
        format!("{}{}", before.trim_end(), after)
    } else {
        existing
    };

    // Add new context section
    let context_section = generate_context_section(context);
    let new_content = format!("{}\n{}", content.trim_end(), context_section);

    std::fs::write(&config_path, new_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Preview what the agent config would look like
pub fn preview_agent_config(agent: CodingAgent, project_path: &Path) -> Result<String, String> {
    let config_path = project_path.join(agent.primary_config());

    if config_path.exists() {
        std::fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))
    } else {
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");
        Ok(agent.initial_config(project_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_parse() {
        assert_eq!(CodingAgent::parse("claude"), Some(CodingAgent::Claude));
        assert_eq!(CodingAgent::parse("cursor"), Some(CodingAgent::Cursor));
        assert_eq!(CodingAgent::parse("copilot"), Some(CodingAgent::Copilot));
        assert_eq!(CodingAgent::parse("aider"), Some(CodingAgent::Aider));
        assert_eq!(CodingAgent::parse("unknown"), None);
    }

    #[test]
    fn test_agent_config_paths() {
        assert!(!CodingAgent::Claude.config_paths().is_empty());
        assert!(!CodingAgent::Cursor.config_paths().is_empty());
    }

    #[test]
    fn test_initial_config() {
        let config = CodingAgent::Claude.initial_config("test-project");
        assert!(config.contains("test-project"));
        assert!(config.contains("CLAUDE.md"));
    }

    #[test]
    fn test_context_section() {
        let context = AllBeadsContext {
            project_name: "test".to_string(),
            beads_prefix: Some("test".to_string()),
            open_issues: 5,
            ready_issues: 3,
            languages: vec!["rust".to_string()],
            frameworks: vec![],
        };
        let section = generate_context_section(&context);
        assert!(section.contains("AllBeads Context"));
        assert!(section.contains("Open Issues: 5"));
    }
}
