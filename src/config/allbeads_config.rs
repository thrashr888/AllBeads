//! AllBeads configuration file handling
//!
//! Loads and manages the ~/.config/allbeads/config.yaml file with multi-context support.

use super::boss_context::BossContext;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Agent Mail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMailConfig {
    /// Port for the Agent Mail server
    #[serde(default = "default_mail_port")]
    pub port: u16,

    /// Storage path for mail database
    pub storage: PathBuf,
}

fn default_mail_port() -> u16 {
    8085
}

impl Default for AgentMailConfig {
    fn default() -> Self {
        // Always use ~/.config for consistency across platforms (macOS, Linux)
        let mut storage = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        storage.push(".config");
        storage.push("allbeads");
        storage.push("mail.db");

        Self {
            port: default_mail_port(),
            storage,
        }
    }
}

/// Visualization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConfig {
    /// Default view mode (kanban, graph, mail, swarm)
    #[serde(default = "default_view_mode")]
    pub default_view: String,

    /// Theme (light, dark)
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Refresh interval in seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u32,
}

fn default_view_mode() -> String {
    "kanban".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_refresh_interval() -> u32 {
    60
}

impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            default_view: default_view_mode(),
            theme: default_theme(),
            refresh_interval: default_refresh_interval(),
        }
    }
}

/// Onboarding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingConfig {
    /// Default agent configuration to create during onboarding
    /// Options: "claude", "agent-skills", "cursor", "copilot", "aider", "kiro", "all"
    /// Default: "claude"
    #[serde(default = "default_onboarding_agent")]
    pub default_agent: String,
}

fn default_onboarding_agent() -> String {
    "claude".to_string()
}

impl Default for OnboardingConfig {
    fn default() -> Self {
        Self {
            default_agent: default_onboarding_agent(),
        }
    }
}

/// AllBeads configuration
///
/// Represents the complete ~/.config/allbeads/config.yaml file with multiple
/// Boss contexts, Agent Mail settings, and visualization preferences.
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

    /// Onboarding settings
    #[serde(default)]
    pub onboarding: OnboardingConfig,

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
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            agent_mail: AgentMailConfig::default(),
            visualization: VisualizationConfig::default(),
            onboarding: OnboardingConfig::default(),
            workspace_directory: default_workspace_dir(),
        }
    }

    /// Load configuration from the default path (~/.config/allbeads/config.yaml)
    pub fn load_default() -> Result<Self> {
        let path = Self::default_path();
        Self::load(&path)
    }

    /// Load configuration from a specific path
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(crate::AllBeadsError::Config(format!(
                "Config file not found: {}",
                path.display()
            )));
        }

        tracing::info!(path = %path.display(), "Loading AllBeads configuration");

        let content = fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;

        tracing::debug!(
            contexts = config.contexts.len(),
            mail_port = config.agent_mail.port,
            "Configuration loaded successfully"
        );

        Ok(config)
    }

    /// Save configuration to the default path
    pub fn save_default(&self) -> Result<()> {
        let path = Self::default_path();
        self.save(&path)
    }

    /// Save configuration to a specific path
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        tracing::info!(path = %path.display(), "Saving AllBeads configuration");

        let yaml = serde_yaml::to_string(self)?;
        fs::write(path, yaml)?;

        Ok(())
    }

    /// Get the default config path (~/.config/allbeads/config.yaml)
    pub fn default_path() -> PathBuf {
        // Always use ~/.config for consistency across platforms (macOS, Linux)
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".config");
        path.push("allbeads");
        path.push("config.yaml");
        path
    }

    /// Get a context by name
    pub fn get_context(&self, name: &str) -> Option<&BossContext> {
        self.contexts.iter().find(|c| c.name == name)
    }

    /// Get a mutable reference to a context by name
    pub fn get_context_mut(&mut self, name: &str) -> Option<&mut BossContext> {
        self.contexts.iter_mut().find(|c| c.name == name)
    }

    /// Add a new context
    pub fn add_context(&mut self, context: BossContext) {
        self.contexts.push(context);
    }

    /// Remove a context by name
    pub fn remove_context(&mut self, name: &str) -> Option<BossContext> {
        if let Some(index) = self.contexts.iter().position(|c| c.name == name) {
            Some(self.contexts.remove(index))
        } else {
            None
        }
    }

    /// Get all context names
    pub fn context_names(&self) -> Vec<&str> {
        self.contexts.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get the workspace directory for cloning repositories
    pub fn workspace_directory(&self) -> &Path {
        &self.workspace_directory
    }
}

impl Default for AllBeadsConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::boss_context::AuthStrategy;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_creation() {
        let config = AllBeadsConfig::new();
        assert_eq!(config.contexts.len(), 0);
        assert_eq!(config.agent_mail.port, 8085);
        assert_eq!(config.visualization.default_view, "kanban");
    }

    #[test]
    fn test_config_with_contexts() {
        let mut config = AllBeadsConfig::new();

        let work_context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::GhEnterpriseToken,
        );

        let personal_context = BossContext::new(
            "personal",
            "git@github.com:user/boss.git",
            AuthStrategy::SshAgent,
        );

        config.add_context(work_context);
        config.add_context(personal_context);

        assert_eq!(config.contexts.len(), 2);
        assert_eq!(config.context_names(), vec!["work", "personal"]);
    }

    #[test]
    fn test_get_context() {
        let mut config = AllBeadsConfig::new();

        let context = BossContext::new(
            "test",
            "https://github.com/test.git",
            AuthStrategy::SshAgent,
        );

        config.add_context(context);

        assert!(config.get_context("test").is_some());
        assert!(config.get_context("missing").is_none());
    }

    #[test]
    fn test_remove_context() {
        let mut config = AllBeadsConfig::new();

        let context = BossContext::new(
            "test",
            "https://github.com/test.git",
            AuthStrategy::SshAgent,
        );

        config.add_context(context);
        assert_eq!(config.contexts.len(), 1);

        let removed = config.remove_context("test");
        assert!(removed.is_some());
        assert_eq!(config.contexts.len(), 0);
    }

    #[test]
    fn test_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Create config
        let mut config = AllBeadsConfig::new();
        let context = BossContext::new(
            "test",
            "https://github.com/test.git",
            AuthStrategy::SshAgent,
        );
        config.add_context(context);

        // Save
        config.save(path).unwrap();

        // Load
        let loaded = AllBeadsConfig::load(path).unwrap();
        assert_eq!(loaded.contexts.len(), 1);
        assert_eq!(loaded.contexts[0].name, "test");
    }

    #[test]
    fn test_default_path() {
        let path = AllBeadsConfig::default_path();
        assert!(path.ends_with("allbeads/config.yaml"));
    }

    #[test]
    fn test_load_missing_file() {
        let result = AllBeadsConfig::load("/nonexistent/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let mut config = AllBeadsConfig::new();

        let context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::GhEnterpriseToken,
        )
        .with_env_var("GITHUB_TOKEN", "$IBM_TOKEN");

        config.add_context(context);

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("contexts:"));
        assert!(yaml.contains("name: work"));
        assert!(yaml.contains("agent_mail:"));
        assert!(yaml.contains("visualization:"));
    }

    #[test]
    fn test_workspace_directory_default() {
        let config = AllBeadsConfig::new();
        let workspace = config.workspace_directory();

        // Should end with "Workspace"
        assert!(workspace.ends_with("Workspace"));
    }

    #[test]
    fn test_workspace_directory_custom() {
        let mut config = AllBeadsConfig::new();
        config.workspace_directory = PathBuf::from("/custom/workspace");

        assert_eq!(config.workspace_directory(), Path::new("/custom/workspace"));
    }

    #[test]
    fn test_workspace_directory_serialization() {
        let config = AllBeadsConfig::new();
        let yaml = serde_yaml::to_string(&config).unwrap();

        // workspace_directory should be in the YAML
        assert!(yaml.contains("workspace_directory:"));
    }
}
