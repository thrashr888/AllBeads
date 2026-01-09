//! Boss Context configuration
//!
//! Represents a single Boss repository context (work, personal, etc.) with
//! authentication, integrations, and member Rigs.

use crate::graph::Rig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Authentication strategy for a Boss context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStrategy {
    /// Use SSH agent for authentication
    SshAgent,

    /// GitHub Enterprise token from environment variable
    GhEnterpriseToken,

    /// Personal access token from environment variable
    PersonalAccessToken,
}

/// JIRA integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIntegration {
    /// JIRA instance URL
    pub url: String,

    /// JIRA project key
    pub project: String,

    /// Optional authentication token (from env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,
}

/// GitHub integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIntegration {
    /// GitHub instance URL (e.g., "https://github.com" or "https://github.ibm.com")
    pub url: String,

    /// Organization or user
    pub owner: String,

    /// Optional repository filter pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_pattern: Option<String>,
}

/// Integration configurations for a context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Integrations {
    /// JIRA integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jira: Option<JiraIntegration>,

    /// GitHub integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<GitHubIntegration>,
}

/// A Boss repository context
///
/// Represents a single Boss repository (work, personal, etc.) that aggregates
/// beads from multiple member Rig repositories. Each context has its own
/// authentication and integration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossContext {
    /// Context name (e.g., "work", "personal")
    pub name: String,

    /// Repository type (currently only "git" supported)
    #[serde(rename = "type")]
    pub repo_type: String,

    /// Git repository URL
    pub url: String,

    /// Local path to the Boss repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    /// Authentication strategy
    pub auth_strategy: AuthStrategy,

    /// Environment variables required for this context
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env_vars: HashMap<String, String>,

    /// External system integrations
    #[serde(default, skip_serializing_if = "is_default_integrations")]
    pub integrations: Integrations,

    /// Member Rig repositories (loaded at runtime)
    #[serde(skip)]
    pub rigs: Vec<Rig>,
}

fn is_default_integrations(integrations: &Integrations) -> bool {
    integrations.jira.is_none() && integrations.github.is_none()
}

impl BossContext {
    /// Create a new Boss context
    pub fn new(name: impl Into<String>, url: impl Into<String>, auth_strategy: AuthStrategy) -> Self {
        Self {
            name: name.into(),
            repo_type: "git".to_string(),
            url: url.into(),
            path: None,
            auth_strategy,
            env_vars: HashMap::new(),
            integrations: Integrations::default(),
            rigs: Vec::new(),
        }
    }

    /// Set the local path for this context
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add an environment variable
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set JIRA integration
    pub fn with_jira(mut self, url: impl Into<String>, project: impl Into<String>) -> Self {
        self.integrations.jira = Some(JiraIntegration {
            url: url.into(),
            project: project.into(),
            token_env: None,
        });
        self
    }

    /// Set GitHub integration
    pub fn with_github(mut self, url: impl Into<String>, owner: impl Into<String>) -> Self {
        self.integrations.github = Some(GitHubIntegration {
            url: url.into(),
            owner: owner.into(),
            repo_pattern: None,
        });
        self
    }

    /// Add a member Rig to this context
    pub fn add_rig(&mut self, rig: Rig) {
        self.rigs.push(rig);
    }

    /// Get the local path, computing it if not set
    pub fn get_path(&self) -> PathBuf {
        if let Some(ref path) = self.path {
            path.clone()
        } else {
            // Default to ~/.config/allbeads/{context_name}
            let mut path = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            path.push("allbeads");
            path.push(&self.name);
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_context_creation() {
        let context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::GhEnterpriseToken,
        );

        assert_eq!(context.name, "work");
        assert_eq!(context.url, "https://github.com/org/boss.git");
        assert_eq!(context.auth_strategy, AuthStrategy::GhEnterpriseToken);
        assert_eq!(context.repo_type, "git");
    }

    #[test]
    fn test_boss_context_builder() {
        let context = BossContext::new(
            "personal",
            "git@github.com:user/boss.git",
            AuthStrategy::SshAgent,
        )
        .with_path("/home/user/boss")
        .with_env_var("GITHUB_TOKEN", "$MY_TOKEN")
        .with_jira("https://jira.example.com", "PROJ")
        .with_github("https://github.com", "myorg");

        assert_eq!(context.name, "personal");
        assert!(context.path.is_some());
        assert_eq!(context.env_vars.len(), 1);
        assert!(context.integrations.jira.is_some());
        assert!(context.integrations.github.is_some());
    }

    #[test]
    fn test_boss_context_serialization() {
        let context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::GhEnterpriseToken,
        )
        .with_env_var("GITHUB_TOKEN", "$IBM_TOKEN");

        let yaml = serde_yaml::to_string(&context).unwrap();
        assert!(yaml.contains("name: work"));
        assert!(yaml.contains("auth_strategy: gh_enterprise_token"));
    }

    #[test]
    fn test_get_path_default() {
        let context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::SshAgent,
        );

        let path = context.get_path();
        assert!(path.ends_with("allbeads/work"));
    }

    #[test]
    fn test_get_path_custom() {
        let context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::SshAgent,
        )
        .with_path("/custom/path");

        let path = context.get_path();
        assert_eq!(path, PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_add_rig() {
        let mut context = BossContext::new(
            "work",
            "https://github.com/org/boss.git",
            AuthStrategy::SshAgent,
        );

        let rig = crate::graph::Rig::builder()
            .id("test-rig")
            .path("/test")
            .remote("git@test.com:test.git")
            .auth_strategy(crate::graph::RigAuthStrategy::SshAgent)
            .prefix("test")
            .context("work")
            .build()
            .unwrap();

        context.add_rig(rig);
        assert_eq!(context.rigs.len(), 1);
    }
}
