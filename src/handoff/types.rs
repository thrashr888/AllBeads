//! Types for agent handoff
//!
//! Defines the agent types and handoff metadata stored in beads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Supported agent types for handoff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    // Terminal-native agents
    Claude,
    OpenCode,
    Codex,
    Gemini,
    Aider,
    Cody,

    // IDE-based agents
    Cursor,
    Kiro,
    Antigravity,
    Copilot,

    // Web agents
    Jules,
    ChatGptCodex,

    // Generic fallback
    Other,
}

impl AgentType {
    /// Get the CLI command name for this agent
    pub fn command(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Aider => "aider",
            Self::Cody => "cody",
            Self::Cursor => "cursor",
            Self::Kiro => "kiro",
            Self::Antigravity => "antigravity",
            Self::Copilot => "code",
            Self::Jules => "jules",
            Self::ChatGptCodex => "codex", // Same CLI as OpenAI Codex
            Self::Other => "agent",
        }
    }

    /// Get the prompt argument format for this agent
    pub fn prompt_args(&self, prompt: &str) -> Vec<String> {
        match self {
            Self::Claude => vec![prompt.to_string()],
            Self::OpenCode => vec!["--prompt".to_string(), prompt.to_string()],
            Self::Codex => vec![
                "exec".to_string(),
                "--full-auto".to_string(),
                prompt.to_string(),
            ],
            Self::Gemini => vec!["-p".to_string(), prompt.to_string()],
            Self::Aider => vec!["--message".to_string(), prompt.to_string()],
            Self::Cody => vec!["chat".to_string(), prompt.to_string()],
            Self::Cursor => vec!["agent".to_string(), prompt.to_string()],
            Self::Kiro => vec!["chat".to_string(), prompt.to_string()],
            Self::Antigravity => vec!["chat".to_string(), prompt.to_string()],
            Self::Copilot => vec!["chat".to_string(), prompt.to_string()],
            Self::Jules => vec!["new".to_string(), prompt.to_string()], // jules new "prompt"
            Self::ChatGptCodex => vec![],                               // Web-only agent
            Self::Other => vec![prompt.to_string()],
        }
    }

    /// Check if this is a web-only agent (no local CLI available)
    pub fn is_web_agent(&self) -> bool {
        matches!(self, Self::ChatGptCodex)
    }

    /// Check if this agent has a web fallback URL
    pub fn has_web_fallback(&self) -> bool {
        matches!(self, Self::Jules | Self::ChatGptCodex)
    }

    /// Check if this is an IDE-based agent
    pub fn is_ide_agent(&self) -> bool {
        matches!(
            self,
            Self::Cursor | Self::Kiro | Self::Antigravity | Self::Copilot
        )
    }

    /// Check if this agent runs in a sandbox that prevents git operations
    ///
    /// Sandboxed agents typically can't write to `.git/` directories, which
    /// prevents branch creation, commits, and pushes. For these agents,
    /// AllBeads should handle git operations before/after the agent runs.
    pub fn is_sandboxed(&self) -> bool {
        matches!(self, Self::Codex)
    }

    /// Get the web URL for this agent (for web agents)
    ///
    /// Returns the base URL where the agent can be accessed. The prompt will be
    /// URL-encoded and appended as a query parameter if supported.
    pub fn web_url(&self) -> Option<&'static str> {
        match self {
            Self::Jules => Some("https://idx.google.com/jules"),
            Self::ChatGptCodex => Some("https://chatgpt.com/codex"),
            _ => None,
        }
    }

    /// Build the full URL with prompt for web agents
    pub fn build_web_url(&self, prompt: &str, repo_url: Option<&str>) -> Option<String> {
        let base = self.web_url()?;

        // URL-encode the prompt (first 500 chars to avoid URL length limits)
        let truncated = if prompt.len() > 500 {
            &prompt[..500]
        } else {
            prompt
        };
        let encoded_prompt = urlencoding::encode(truncated);

        match self {
            Self::Jules => {
                // Jules might accept repo URL + prompt
                if let Some(repo) = repo_url {
                    Some(format!(
                        "{}?repo={}&task={}",
                        base,
                        urlencoding::encode(repo),
                        encoded_prompt
                    ))
                } else {
                    Some(format!("{}?task={}", base, encoded_prompt))
                }
            }
            Self::ChatGptCodex => {
                // ChatGPT/Codex with prompt
                Some(format!("{}?q={}", base, encoded_prompt))
            }
            _ => None,
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex (OpenAI)",
            Self::Gemini => "Gemini CLI",
            Self::Aider => "Aider",
            Self::Cody => "Cody",
            Self::Cursor => "Cursor",
            Self::Kiro => "Kiro (AWS)",
            Self::Antigravity => "Antigravity",
            Self::Copilot => "VS Code Copilot",
            Self::Jules => "Jules (Google)",
            Self::ChatGptCodex => "ChatGPT Codex",
            Self::Other => "Other Agent",
        }
    }

    /// Check if this agent's CLI is installed
    pub fn is_installed(&self) -> bool {
        use std::process::Command;

        // Web agents are always "available"
        if self.is_web_agent() {
            return true;
        }

        let cmd = self.command();

        // Different agents use different version check methods
        let args = match self {
            Self::Jules => vec!["version"],
            Self::Cursor => vec!["agent", "--version"],
            _ => vec!["--version"],
        };

        Command::new(cmd)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get all agent types
    pub fn all() -> Vec<Self> {
        vec![
            Self::Claude,
            Self::OpenCode,
            Self::Codex,
            Self::Gemini,
            Self::Aider,
            Self::Cody,
            Self::Cursor,
            Self::Kiro,
            Self::Antigravity,
            Self::Copilot,
            Self::Jules,
            Self::ChatGptCodex,
        ]
    }
}

/// Detect which agents are installed on the system
///
/// Returns a list of (AgentType, is_installed) tuples
pub fn detect_installed_agents() -> Vec<(AgentType, bool)> {
    AgentType::all()
        .into_iter()
        .map(|agent| {
            let installed = agent.is_installed();
            (agent, installed)
        })
        .collect()
}

/// Get just the installed agents
pub fn get_installed_agents() -> Vec<AgentType> {
    detect_installed_agents()
        .into_iter()
        .filter(|(_, installed)| *installed)
        .map(|(agent, _)| agent)
        .collect()
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" => Ok(Self::Claude),
            "opencode" => Ok(Self::OpenCode),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "aider" => Ok(Self::Aider),
            "cody" => Ok(Self::Cody),
            "cursor" | "cursor-agent" => Ok(Self::Cursor),
            "kiro" => Ok(Self::Kiro),
            "antigravity" => Ok(Self::Antigravity),
            "copilot" | "vscode" | "code" => Ok(Self::Copilot),
            "jules" => Ok(Self::Jules),
            "chatgpt-codex" | "chatgpt" => Ok(Self::ChatGptCodex),
            _ => Err(format!("Unknown agent type: {}", s)),
        }
    }
}

/// Information about a handoff to an agent
///
/// Stored in the bead to track which agent is working on it
/// and provide a link back to the agent's task/session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandoff {
    /// Type of agent handling this bead
    pub agent_type: AgentType,

    /// When the handoff occurred
    pub handed_off_at: DateTime<Utc>,

    /// For web agents: URL to check status (Jules task, Codex task, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_url: Option<String>,

    /// For CLI agents: which repo/worktree was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<PathBuf>,

    /// Brief note about the handoff
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl AgentHandoff {
    /// Create a new handoff record for a CLI agent
    pub fn cli(agent_type: AgentType, workdir: PathBuf) -> Self {
        Self {
            agent_type,
            handed_off_at: Utc::now(),
            task_url: None,
            workdir: Some(workdir),
            note: Some("Handed off via ab handoff".to_string()),
        }
    }

    /// Create a new handoff record for a web agent
    pub fn web(agent_type: AgentType, task_url: String) -> Self {
        Self {
            agent_type,
            handed_off_at: Utc::now(),
            task_url: Some(task_url),
            workdir: None,
            note: Some("Handed off via ab handoff".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!("claude".parse::<AgentType>().unwrap(), AgentType::Claude);
        assert_eq!("gemini".parse::<AgentType>().unwrap(), AgentType::Gemini);
        assert_eq!("cursor".parse::<AgentType>().unwrap(), AgentType::Cursor);
        assert_eq!("jules".parse::<AgentType>().unwrap(), AgentType::Jules);
    }

    #[test]
    fn test_agent_type_command() {
        assert_eq!(AgentType::Claude.command(), "claude");
        assert_eq!(AgentType::Gemini.command(), "gemini");
        assert_eq!(AgentType::Cursor.command(), "cursor");
    }

    #[test]
    fn test_agent_type_prompt_args() {
        let prompt = "Fix the bug";
        assert_eq!(AgentType::Claude.prompt_args(prompt), vec!["Fix the bug"]);
        assert_eq!(
            AgentType::Gemini.prompt_args(prompt),
            vec!["-p", "Fix the bug"]
        );
        assert_eq!(
            AgentType::OpenCode.prompt_args(prompt),
            vec!["--prompt", "Fix the bug"]
        );
        assert_eq!(
            AgentType::Codex.prompt_args(prompt),
            vec!["exec", "--full-auto", "Fix the bug"]
        );
    }

    #[test]
    fn test_agent_handoff_serialization() {
        let handoff = AgentHandoff::cli(AgentType::Claude, PathBuf::from("/tmp/test"));
        let json = serde_json::to_string(&handoff).unwrap();
        assert!(json.contains("claude"));
        assert!(json.contains("/tmp/test"));
    }
}
