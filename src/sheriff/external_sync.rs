//! External System Synchronization
//!
//! Handles bi-directional sync between AllBeads and external issue trackers
//! (JIRA, GitHub Issues) during the Sheriff's poll cycle.
//!
//! # Sync Flow
//!
//! 1. **Ingress**: Pull issues with `ai-agent` label from external systems
//! 2. **Diff**: Compare external state with local Shadow Beads
//! 3. **Egress**: Push status changes back to external systems (future)

use crate::config::{GitHubIntegration, JiraIntegration};
use crate::graph::{BeadId, ShadowBead, Status};
use crate::integrations::{GitHubAdapter, GitHubSyncStats, JiraAdapter, JiraSyncStats};
use crate::Result;
use std::collections::HashMap;
use tracing::{debug, error, info};

/// Configuration for external sync
#[derive(Debug, Clone, Default)]
pub struct ExternalSyncConfig {
    /// JIRA configuration
    pub jira: Option<JiraIntegration>,

    /// GitHub configuration
    pub github: Option<GitHubIntegration>,

    /// Label filter for pulling issues (default: "ai-agent")
    pub label_filter: String,

    /// Whether to enable two-way sync (push status back)
    pub two_way_sync: bool,
}

impl ExternalSyncConfig {
    /// Create a new external sync config
    pub fn new() -> Self {
        Self {
            jira: None,
            github: None,
            label_filter: "ai-agent".to_string(),
            two_way_sync: true,
        }
    }

    /// Set JIRA configuration
    pub fn with_jira(mut self, config: JiraIntegration) -> Self {
        self.jira = Some(config);
        self
    }

    /// Set GitHub configuration
    pub fn with_github(mut self, config: GitHubIntegration) -> Self {
        self.github = Some(config);
        self
    }

    /// Set the label filter
    pub fn with_label_filter(mut self, label: impl Into<String>) -> Self {
        self.label_filter = label.into();
        self
    }

    /// Enable or disable two-way sync
    pub fn with_two_way_sync(mut self, enabled: bool) -> Self {
        self.two_way_sync = enabled;
        self
    }
}

/// Result of an external sync cycle
#[derive(Debug, Clone, Default)]
pub struct ExternalSyncResult {
    /// JIRA sync statistics
    pub jira_stats: Option<JiraSyncStats>,

    /// GitHub sync statistics
    pub github_stats: Option<GitHubSyncStats>,

    /// Shadow beads created from external sources
    pub shadows_created: Vec<ShadowBead>,

    /// Shadow beads updated from external sources
    pub shadows_updated: Vec<BeadId>,

    /// Errors encountered
    pub errors: Vec<String>,
}

impl ExternalSyncResult {
    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.shadows_created.len() + self.shadows_updated.len()
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// External system syncer
///
/// Handles bi-directional sync with JIRA and GitHub Issues.
pub struct ExternalSyncer {
    /// Configuration
    config: ExternalSyncConfig,

    /// JIRA adapter
    jira_adapter: Option<JiraAdapter>,

    /// GitHub adapter
    github_adapter: Option<GitHubAdapter>,

    /// Existing shadow beads from external sources (keyed by external ref)
    external_shadows: HashMap<String, ShadowBead>,
}

impl ExternalSyncer {
    /// Create a new external syncer
    pub fn new(config: ExternalSyncConfig) -> Self {
        let jira_adapter = config
            .jira
            .as_ref()
            .and_then(|c| match JiraAdapter::new(c.clone()) {
                Ok(adapter) => Some(adapter),
                Err(e) => {
                    tracing::warn!("Failed to create JIRA adapter: {}", e);
                    None
                }
            });

        let github_adapter =
            config
                .github
                .as_ref()
                .and_then(|c| match GitHubAdapter::new(c.clone()) {
                    Ok(adapter) => Some(adapter),
                    Err(e) => {
                        tracing::warn!("Failed to create GitHub adapter: {}", e);
                        None
                    }
                });

        Self {
            config,
            jira_adapter,
            github_adapter,
            external_shadows: HashMap::new(),
        }
    }

    /// Set the authentication token for JIRA
    pub fn with_jira_token(mut self, token: impl Into<String>) -> Self {
        if let Some(ref mut adapter) = self.jira_adapter {
            adapter.set_auth_token(token.into());
        }
        self
    }

    /// Set the authentication token for GitHub
    pub fn with_github_token(mut self, token: impl Into<String>) -> Self {
        if let Some(ref mut adapter) = self.github_adapter {
            adapter.set_auth_token(token.into());
        }
        self
    }

    /// Load existing external shadows
    pub fn load_external_shadows(&mut self, shadows: Vec<ShadowBead>) {
        self.external_shadows.clear();
        for shadow in shadows {
            if let Some(ref external_ref) = shadow.external_ref {
                self.external_shadows.insert(external_ref.clone(), shadow);
            }
        }
        debug!(
            "Loaded {} external shadow beads",
            self.external_shadows.len()
        );
    }

    /// Execute a full sync cycle
    pub async fn sync_cycle(&mut self) -> ExternalSyncResult {
        let mut result = ExternalSyncResult::default();
        let label_filter = self.config.label_filter.clone();
        let two_way_sync = self.config.two_way_sync;

        // Sync from JIRA
        if let Some(ref jira_adapter) = self.jira_adapter {
            match Self::sync_jira_internal(
                jira_adapter,
                &label_filter,
                &self.external_shadows,
                two_way_sync,
            )
            .await
            {
                Ok((stats, shadows, updated)) => {
                    result.jira_stats = Some(stats);
                    result.shadows_created.extend(shadows);
                    result.shadows_updated.extend(updated);
                }
                Err(e) => {
                    let msg = format!("JIRA sync failed: {}", e);
                    error!("{}", msg);
                    result.errors.push(msg);
                }
            }
        }

        // Sync from GitHub
        if let Some(ref github_adapter) = self.github_adapter {
            match Self::sync_github_internal(
                github_adapter,
                &label_filter,
                &self.external_shadows,
                two_way_sync,
            )
            .await
            {
                Ok((stats, shadows, updated)) => {
                    result.github_stats = Some(stats);
                    result.shadows_created.extend(shadows);
                    result.shadows_updated.extend(updated);
                }
                Err(e) => {
                    let msg = format!("GitHub sync failed: {}", e);
                    error!("{}", msg);
                    result.errors.push(msg);
                }
            }
        }

        info!(
            "External sync complete: {} created, {} updated, {} errors",
            result.shadows_created.len(),
            result.shadows_updated.len(),
            result.errors.len()
        );

        result
    }

    /// Internal JIRA sync (static method to avoid borrow issues)
    async fn sync_jira_internal(
        adapter: &JiraAdapter,
        label_filter: &str,
        external_shadows: &HashMap<String, ShadowBead>,
        two_way_sync: bool,
    ) -> Result<(JiraSyncStats, Vec<ShadowBead>, Vec<BeadId>)> {
        let mut stats = JiraSyncStats::default();
        let mut new_shadows = Vec::new();
        let mut updated_ids = Vec::new();

        // Pull issues with ai-agent label
        let issues = adapter.pull_agent_issues(label_filter).await?;

        info!("Pulled {} issues from JIRA", issues.len());

        for issue in issues {
            let external_ref = format!("jira:{}", issue.key);

            if let Some(existing) = external_shadows.get(&external_ref) {
                // Check if status changed
                let external_status = adapter.map_jira_status(&issue.fields.status.name);
                if existing.status != external_status {
                    stats.updated += 1;
                    updated_ids.push(existing.id.clone());
                    debug!(
                        "JIRA issue {} status changed: {:?} -> {:?}",
                        issue.key, existing.status, external_status
                    );
                }
            } else {
                // Create new shadow bead
                let shadow = adapter.issue_to_shadow_bead(&issue);
                stats.created += 1;
                new_shadows.push(shadow);
                debug!("Created shadow bead for JIRA issue {}", issue.key);
            }
        }

        // Two-way sync: log what would be pushed back
        if two_way_sync {
            for shadow in external_shadows.values() {
                if let Some(ref external_ref) = shadow.external_ref {
                    if external_ref.starts_with("jira:") {
                        debug!(
                            "Would push status {:?} to JIRA for {}",
                            shadow.status, external_ref
                        );
                    }
                }
            }
        }

        Ok((stats, new_shadows, updated_ids))
    }

    /// Internal GitHub sync (static method to avoid borrow issues)
    async fn sync_github_internal(
        adapter: &GitHubAdapter,
        label_filter: &str,
        external_shadows: &HashMap<String, ShadowBead>,
        two_way_sync: bool,
    ) -> Result<(GitHubSyncStats, Vec<ShadowBead>, Vec<BeadId>)> {
        let mut stats = GitHubSyncStats::default();
        let mut new_shadows = Vec::new();
        let mut updated_ids = Vec::new();

        // Pull issues with ai-agent label
        let issues = adapter.pull_agent_issues(label_filter).await?;

        info!("Pulled {} issues from GitHub", issues.len());

        for issue in issues {
            let external_ref = format!(
                "github:{}/{}#{}",
                issue.repository.owner.login, issue.repository.name, issue.number
            );

            if let Some(existing) = external_shadows.get(&external_ref) {
                // Check if status changed
                let external_status = adapter.map_github_state(&issue.state);
                if existing.status != external_status {
                    stats.updated += 1;
                    updated_ids.push(existing.id.clone());
                    debug!(
                        "GitHub issue #{} status changed: {:?} -> {:?}",
                        issue.number, existing.status, external_status
                    );
                }
            } else {
                // Create new shadow bead
                let shadow = adapter.issue_to_shadow_bead(&issue);
                stats.created += 1;
                new_shadows.push(shadow);
                debug!("Created shadow bead for GitHub issue #{}", issue.number);
            }
        }

        // Two-way sync: log what would be pushed back
        if two_way_sync {
            for shadow in external_shadows.values() {
                if let Some(ref external_ref) = shadow.external_ref {
                    if external_ref.starts_with("github:") {
                        debug!(
                            "Would push status {:?} to GitHub for {}",
                            shadow.status, external_ref
                        );
                    }
                }
            }
        }

        Ok((stats, new_shadows, updated_ids))
    }

    /// Get current external shadows
    pub fn external_shadows(&self) -> &HashMap<String, ShadowBead> {
        &self.external_shadows
    }

    /// Check if JIRA sync is configured
    pub fn has_jira(&self) -> bool {
        self.jira_adapter.is_some()
    }

    /// Check if GitHub sync is configured
    pub fn has_github(&self) -> bool {
        self.github_adapter.is_some()
    }
}

/// Events for external sync
#[derive(Debug, Clone)]
pub enum ExternalSyncEvent {
    /// Sync started
    Started,

    /// Sync completed
    Completed(ExternalSyncResult),

    /// Shadow bead created from external source
    ShadowCreated(ShadowBead),

    /// Shadow bead updated from external source
    ShadowUpdated(BeadId),

    /// Status pushed to external system
    StatusPushed {
        external_ref: String,
        status: Status,
    },

    /// Error occurred
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_sync_config() {
        let config = ExternalSyncConfig::new()
            .with_label_filter("custom-label")
            .with_two_way_sync(false);

        assert_eq!(config.label_filter, "custom-label");
        assert!(!config.two_way_sync);
    }

    #[test]
    fn test_external_sync_result() {
        let mut result = ExternalSyncResult::default();
        assert_eq!(result.total_changes(), 0);
        assert!(!result.has_errors());

        result.errors.push("Test error".to_string());
        assert!(result.has_errors());
    }

    #[test]
    fn test_external_syncer_creation() {
        let config = ExternalSyncConfig::new();
        let syncer = ExternalSyncer::new(config);

        assert!(!syncer.has_jira());
        assert!(!syncer.has_github());
    }
}
