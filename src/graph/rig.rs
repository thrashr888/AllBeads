//! Rig configuration
//!
//! Represents a member repository with its metadata and authentication.
//! Uses Builder pattern for flexible construction.

use super::RigId;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Authentication strategy for git operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStrategy {
    /// Use SSH agent for authentication
    SshAgent,
    /// GitHub Enterprise token (from environment variable)
    GhEnterpriseToken,
    /// Personal access token (from environment variable)
    PersonalAccessToken,
}

/// Rig configuration for a member repository
///
/// Represents a member repository that contributes beads to the federated graph.
/// Each Rig has its own `.beads/` directory and may have specialized agent personas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rig {
    /// Unique identifier (typically the repository name)
    pub id: RigId,

    /// Local path to the repository
    pub path: PathBuf,

    /// Remote URL (e.g., "https://github.com/org/repo" or "git@github.com:org/repo")
    pub remote: String,

    /// Git branch to track
    #[serde(default = "default_branch")]
    pub branch: String,

    /// Authentication strategy
    pub auth_strategy: AuthStrategy,

    /// Optional agent persona (e.g., "security-specialist", "ux-designer")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,

    /// Bead prefix for this rig (e.g., "auth", "ui")
    pub prefix: String,

    /// Optional JIRA project mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jira_project: Option<String>,

    /// Context this rig belongs to (work, personal, etc.)
    pub context: String,
}

fn default_branch() -> String {
    "main".to_string()
}

impl Rig {
    /// Create a new Rig builder
    pub fn builder() -> RigBuilder {
        RigBuilder::default()
    }
}

/// Builder for Rig configuration
#[derive(Debug, Default)]
pub struct RigBuilder {
    id: Option<RigId>,
    path: Option<PathBuf>,
    remote: Option<String>,
    branch: Option<String>,
    auth_strategy: Option<AuthStrategy>,
    persona: Option<String>,
    prefix: Option<String>,
    jira_project: Option<String>,
    context: Option<String>,
}

impl RigBuilder {
    /// Set the rig ID
    pub fn id(mut self, id: impl Into<RigId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the local path
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the remote URL
    pub fn remote(mut self, remote: impl Into<String>) -> Self {
        self.remote = Some(remote.into());
        self
    }

    /// Set the branch (defaults to "main")
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set the authentication strategy
    pub fn auth_strategy(mut self, strategy: AuthStrategy) -> Self {
        self.auth_strategy = Some(strategy);
        self
    }

    /// Set the agent persona
    pub fn persona(mut self, persona: impl Into<String>) -> Self {
        self.persona = Some(persona.into());
        self
    }

    /// Set the bead prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set the JIRA project mapping
    pub fn jira_project(mut self, project: impl Into<String>) -> Self {
        self.jira_project = Some(project.into());
        self
    }

    /// Set the context
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Build the Rig, returning an error if required fields are missing
    pub fn build(self) -> Result<Rig> {
        let id = self
            .id
            .ok_or_else(|| crate::AllBeadsError::Config("Rig id is required".to_string()))?;
        let path = self
            .path
            .ok_or_else(|| crate::AllBeadsError::Config("Rig path is required".to_string()))?;
        let remote = self
            .remote
            .ok_or_else(|| crate::AllBeadsError::Config("Rig remote is required".to_string()))?;
        let auth_strategy = self.auth_strategy.ok_or_else(|| {
            crate::AllBeadsError::Config("Rig auth_strategy is required".to_string())
        })?;
        let prefix = self
            .prefix
            .ok_or_else(|| crate::AllBeadsError::Config("Rig prefix is required".to_string()))?;
        let context = self
            .context
            .ok_or_else(|| crate::AllBeadsError::Config("Rig context is required".to_string()))?;

        Ok(Rig {
            id,
            path,
            remote,
            branch: self.branch.unwrap_or_else(default_branch),
            auth_strategy,
            persona: self.persona,
            prefix,
            jira_project: self.jira_project,
            context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rig_builder_complete() {
        let rig = Rig::builder()
            .id("auth-service")
            .path("/Users/alice/projects/auth-service")
            .remote("git@github.com:org/auth-service.git")
            .branch("main")
            .auth_strategy(AuthStrategy::SshAgent)
            .persona("security-specialist")
            .prefix("auth")
            .jira_project("SEC")
            .context("work")
            .build()
            .unwrap();

        assert_eq!(rig.id.as_str(), "auth-service");
        assert_eq!(rig.prefix, "auth");
        assert_eq!(rig.context, "work");
        assert_eq!(rig.persona, Some("security-specialist".to_string()));
        assert_eq!(rig.jira_project, Some("SEC".to_string()));
    }

    #[test]
    fn test_rig_builder_minimal() {
        let rig = Rig::builder()
            .id("personal-blog")
            .path("/Users/alice/blog")
            .remote("git@github.com:alice/blog.git")
            .auth_strategy(AuthStrategy::SshAgent)
            .prefix("blog")
            .context("personal")
            .build()
            .unwrap();

        assert_eq!(rig.id.as_str(), "personal-blog");
        assert_eq!(rig.branch, "main"); // Default branch
        assert!(rig.persona.is_none());
        assert!(rig.jira_project.is_none());
    }

    #[test]
    fn test_rig_builder_missing_required() {
        let result = Rig::builder()
            .id("incomplete")
            .path("/some/path")
            // Missing: remote, auth_strategy, prefix, context
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_rig_serialization() {
        let rig = Rig::builder()
            .id("test-rig")
            .path("/test")
            .remote("git@test.com:test.git")
            .auth_strategy(AuthStrategy::SshAgent)
            .prefix("test")
            .context("work")
            .build()
            .unwrap();

        let json = serde_json::to_string(&rig).unwrap();
        assert!(json.contains("test-rig"));
        assert!(json.contains("ssh_agent"));
    }

    #[test]
    fn test_auth_strategy_variants() {
        let ssh = Rig::builder()
            .id("ssh-rig")
            .path("/test")
            .remote("git@test.com:test.git")
            .auth_strategy(AuthStrategy::SshAgent)
            .prefix("test")
            .context("work")
            .build()
            .unwrap();

        let token = Rig::builder()
            .id("token-rig")
            .path("/test")
            .remote("https://github.com/test.git")
            .auth_strategy(AuthStrategy::GhEnterpriseToken)
            .prefix("test")
            .context("work")
            .build()
            .unwrap();

        assert_eq!(ssh.auth_strategy, AuthStrategy::SshAgent);
        assert_eq!(token.auth_strategy, AuthStrategy::GhEnterpriseToken);
    }
}
