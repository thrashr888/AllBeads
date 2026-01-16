//! Handoff configuration
//!
//! Reads and writes handoff preferences from .beads/config.yaml

use super::AgentType;
use serde::{Deserialize, Serialize};
use std::fs;

/// Handoff-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandoffConfig {
    /// Preferred agent for handoff (saved after first use)
    #[serde(rename = "preferred-agent", skip_serializing_if = "Option::is_none")]
    pub preferred_agent: Option<String>,

    /// Enable worktree creation for isolated handoffs
    #[serde(rename = "worktree-enabled", default)]
    pub worktree_enabled: bool,
}

/// Full beads config with handoff section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BeadsConfig {
    #[serde(flatten)]
    other: serde_yaml::Mapping,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    handoff: Option<HandoffConfig>,
}

/// Get the path to the beads config file
pub fn config_path() -> Option<std::path::PathBuf> {
    // Look for .beads/config.yaml in current directory or parents
    let mut current = std::env::current_dir().ok()?;

    loop {
        let config = current.join(".beads/config.yaml");
        if config.exists() {
            return Some(config);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Load handoff config from .beads/config.yaml
pub fn load_config() -> Option<HandoffConfig> {
    let path = config_path()?;
    let content = fs::read_to_string(&path).ok()?;

    // Parse the YAML and extract handoff section
    let config: BeadsConfig = serde_yaml::from_str(&content).ok()?;
    config.handoff
}

/// Get the preferred agent from config
pub fn get_preferred_agent() -> Option<AgentType> {
    let config = load_config()?;
    let agent_name = config.preferred_agent?;
    agent_name.parse().ok()
}

/// Save the preferred agent to config
pub fn save_preferred_agent(agent: AgentType) -> std::io::Result<()> {
    let path = config_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "No .beads/config.yaml found")
    })?;

    let content = fs::read_to_string(&path)?;

    // Check if handoff section exists
    if content.contains("handoff:") {
        // Update existing handoff section
        let mut new_content = String::new();
        let mut in_handoff = false;
        let mut updated = false;

        for line in content.lines() {
            if line.trim().starts_with("handoff:") {
                in_handoff = true;
                new_content.push_str(line);
                new_content.push('\n');
            } else if in_handoff && line.trim().starts_with("preferred-agent:") {
                new_content.push_str(&format!("  preferred-agent: \"{}\"\n", agent.command()));
                updated = true;
            } else if in_handoff
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && !line.is_empty()
            {
                // Exiting handoff section
                if !updated {
                    new_content.push_str(&format!("  preferred-agent: \"{}\"\n", agent.command()));
                }
                in_handoff = false;
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        // If we're still in handoff section at end of file and haven't updated
        if in_handoff && !updated {
            new_content.push_str(&format!("  preferred-agent: \"{}\"\n", agent.command()));
        }

        fs::write(&path, new_content)?;
    } else {
        // Add new handoff section at end
        let mut content = content;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!(
            "\n# Agent handoff settings\nhandoff:\n  preferred-agent: \"{}\"\n",
            agent.command()
        ));
        fs::write(&path, content)?;
    }

    Ok(())
}

/// Get worktree enabled setting
pub fn is_worktree_enabled() -> bool {
    load_config().map(|c| c.worktree_enabled).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_config_default() {
        let config = HandoffConfig::default();
        assert!(config.preferred_agent.is_none());
        assert!(!config.worktree_enabled);
    }
}
