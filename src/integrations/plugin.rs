//! Plugin Architecture for External Integrations
//!
//! Provides structures for adding custom integrations beyond JIRA and GitHub.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Direction of synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Pull from external system to local beads
    Inbound,
    /// Push from local beads to external system
    Outbound,
    /// Bi-directional sync
    Bidirectional,
}

/// Result of a sync operation
#[derive(Debug, Clone, Default)]
pub struct PluginSyncResult {
    /// Number of items created locally
    pub created_local: u32,
    /// Number of items updated locally
    pub updated_local: u32,
    /// Number of items created in external system
    pub created_remote: u32,
    /// Number of items updated in external system
    pub updated_remote: u32,
    /// Number of items skipped
    pub skipped: u32,
    /// Number of errors
    pub errors: u32,
    /// Error messages
    pub error_messages: Vec<String>,
}

/// Status mapping between external system and beads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMapping {
    /// External status name
    pub external: String,
    /// Local bead status
    pub local: String,
}

/// Plugin configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Whether the plugin is enabled
    pub enabled: bool,

    /// Plugin-specific settings
    #[serde(default)]
    pub settings: HashMap<String, String>,

    /// Status mappings
    #[serde(default)]
    pub status_mappings: Vec<StatusMapping>,

    /// Label filter (only sync issues with these labels)
    #[serde(default)]
    pub label_filter: Vec<String>,
}

/// External issue representation (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIssue {
    /// Unique identifier in external system
    pub id: String,

    /// Human-readable key (e.g., "PROJ-123", "#456")
    pub key: String,

    /// Issue title/summary
    pub title: String,

    /// Issue description/body
    #[serde(default)]
    pub description: Option<String>,

    /// Status name
    pub status: String,

    /// Priority (0-4, lower is higher priority)
    #[serde(default)]
    pub priority: Option<u8>,

    /// Issue type (bug, task, feature, epic)
    #[serde(default)]
    pub issue_type: Option<String>,

    /// Labels/tags
    #[serde(default)]
    pub labels: Vec<String>,

    /// Assignee username
    #[serde(default)]
    pub assignee: Option<String>,

    /// URL to view in external system
    #[serde(default)]
    pub url: Option<String>,

    /// Last updated timestamp (ISO 8601)
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Created timestamp (ISO 8601)
    #[serde(default)]
    pub created_at: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ExternalIssue {
    /// Create a new external issue
    pub fn new(id: impl Into<String>, key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            title: title.into(),
            description: None,
            status: "open".to_string(),
            priority: None,
            issue_type: None,
            labels: Vec::new(),
            assignee: None,
            url: None,
            updated_at: None,
            created_at: None,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_issue_creation() {
        let issue = ExternalIssue::new("123", "PROJ-123", "Test issue");
        assert_eq!(issue.id, "123");
        assert_eq!(issue.key, "PROJ-123");
        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.status, "open");
    }

    #[test]
    fn test_plugin_config() {
        let config = PluginConfig {
            enabled: true,
            settings: HashMap::new(),
            status_mappings: vec![StatusMapping {
                external: "To Do".to_string(),
                local: "open".to_string(),
            }],
            label_filter: vec!["ai-agent".to_string()],
        };

        assert!(config.enabled);
        assert_eq!(config.label_filter.len(), 1);
    }
}
