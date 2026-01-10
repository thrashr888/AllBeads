//! Shadow Bead data structure
//!
//! Represents a pointer to a bead in a member Rig repository.
//! Shadow Beads live in the Boss repo and provide cross-repo coordination.

use super::{BeadId, RigId, Status};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// URI format for pointing to beads in member repositories
///
/// Format: `bead://repo-name/bead-id`
/// Example: `bead://auth-service/auth-5fm`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadUri(String);

impl BeadUri {
    /// Create a new BeadUri
    pub fn new(rig_id: &RigId, bead_id: &BeadId) -> Self {
        Self(format!("bead://{}/{}", rig_id.as_str(), bead_id.as_str()))
    }

    /// Parse a URI string
    pub fn from_string(uri: impl Into<String>) -> Self {
        Self(uri.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the rig ID from the URI
    pub fn rig_id(&self) -> Option<RigId> {
        if !self.0.starts_with("bead://") {
            return None;
        }
        let parts: Vec<&str> = self.0.trim_start_matches("bead://").split('/').collect();
        parts.first().map(|s| RigId::new(*s))
    }

    /// Extract the bead ID from the URI
    pub fn bead_id(&self) -> Option<BeadId> {
        if !self.0.starts_with("bead://") {
            return None;
        }
        let parts: Vec<&str> = self.0.trim_start_matches("bead://").split('/').collect();
        parts.get(1).map(|s| BeadId::new(*s))
    }
}

impl std::fmt::Display for BeadUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Shadow bead pointing to a bead in a member repository
///
/// Shadow Beads are created in the Boss repo for Epic-level beads in member Rigs.
/// They provide a lightweight representation with cross-repo dependency tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowBead {
    /// Unique identifier in the Boss repo (e.g., "boss-mig-1")
    pub id: BeadId,

    /// Pointer to the native bead in the member Rig
    pub pointer: BeadUri,

    /// Summary/title mirrored from the native bead
    pub summary: String,

    /// Status mirrored from the native bead
    pub status: Status,

    /// Context this shadow bead belongs to (work, personal, etc.)
    pub context: String,

    /// Cross-repo dependencies (URIs to other shadow beads or native beads)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_repo_dependencies: Vec<BeadUri>,

    /// Cross-repo blocking relationships
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_repo_blocks: Vec<BeadUri>,

    /// Labels/tags
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub labels: HashSet<String>,

    /// Last sync timestamp
    pub last_synced: String,

    /// Optional notes about this shadow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// External reference for integration tracking (e.g., "jira:PROJ-123", "github:owner/repo#456")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
}

impl ShadowBead {
    /// Create a new shadow bead
    pub fn new(
        id: impl Into<BeadId>,
        rig_id: &RigId,
        native_bead_id: &BeadId,
        summary: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            pointer: BeadUri::new(rig_id, native_bead_id),
            summary: summary.into(),
            status: Status::Open,
            context: context.into(),
            cross_repo_dependencies: Vec::new(),
            cross_repo_blocks: Vec::new(),
            labels: HashSet::new(),
            last_synced: chrono::Utc::now().to_rfc3339(),
            notes: None,
            external_ref: None,
        }
    }

    /// Add a cross-repo dependency
    pub fn add_cross_repo_dependency(&mut self, uri: BeadUri) {
        if !self.cross_repo_dependencies.contains(&uri) {
            self.cross_repo_dependencies.push(uri);
        }
        self.update_sync_time();
    }

    /// Update the last synced timestamp
    pub fn update_sync_time(&mut self) {
        self.last_synced = chrono::Utc::now().to_rfc3339();
    }

    /// Check if this shadow bead has cross-repo blockers
    pub fn has_cross_repo_blockers(&self) -> bool {
        !self.cross_repo_dependencies.is_empty()
    }

    /// Create a ShadowBead from an external source (JIRA, GitHub, etc.)
    ///
    /// This is a convenience constructor for integration adapters.
    pub fn from_external(
        id: impl Into<BeadId>,
        summary: impl Into<String>,
        external_uri: impl Into<String>,
    ) -> ShadowBeadBuilder {
        ShadowBeadBuilder::new(id.into(), summary.into(), external_uri.into())
    }
}

/// Builder for ShadowBead from external sources
pub struct ShadowBeadBuilder {
    id: BeadId,
    summary: String,
    pointer: BeadUri,
    status: Status,
    context: String,
    description: String,
    priority: Option<u8>,
    issue_type: Option<String>,
    external_ref: Option<String>,
    labels: HashSet<String>,
}

impl ShadowBeadBuilder {
    /// Create a new builder
    pub fn new(id: BeadId, summary: String, external_uri: String) -> Self {
        Self {
            id,
            summary,
            pointer: BeadUri::from_string(external_uri),
            status: Status::Open,
            context: String::new(),
            description: String::new(),
            priority: None,
            issue_type: None,
            external_ref: None,
            labels: HashSet::new(),
        }
    }

    /// Set the status
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        let status_str = status.into();
        self.status = match status_str.to_lowercase().as_str() {
            "open" => Status::Open,
            "in_progress" | "in progress" => Status::InProgress,
            "blocked" => Status::Blocked,
            "closed" | "done" => Status::Closed,
            _ => Status::Open,
        };
        self
    }

    /// Set the priority (0-4)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the issue type
    pub fn with_issue_type(mut self, issue_type: impl Into<String>) -> Self {
        self.issue_type = Some(issue_type.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// Set the external reference
    pub fn with_external_ref(mut self, external_ref: impl Into<String>) -> Self {
        self.external_ref = Some(external_ref.into());
        self
    }

    /// Add a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.insert(label.into());
        self
    }

    /// Build the ShadowBead
    pub fn build(self) -> ShadowBead {
        ShadowBead {
            id: self.id,
            pointer: self.pointer,
            summary: self.summary,
            status: self.status,
            context: self.context,
            cross_repo_dependencies: Vec::new(),
            cross_repo_blocks: Vec::new(),
            labels: self.labels,
            last_synced: chrono::Utc::now().to_rfc3339(),
            notes: None,
            external_ref: self.external_ref,
        }
    }
}

/// Convenience wrapper for creating ShadowBead from external sources
impl ShadowBead {
    /// Alias for from_external to make the integration code cleaner
    pub fn external(
        id: impl Into<BeadId>,
        summary: impl Into<String>,
        external_uri: impl Into<String>,
    ) -> ShadowBeadBuilder {
        Self::from_external(id, summary, external_uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_uri_creation() {
        let rig_id = RigId::new("auth-service");
        let bead_id = BeadId::new("auth-5fm");
        let uri = BeadUri::new(&rig_id, &bead_id);

        assert_eq!(uri.as_str(), "bead://auth-service/auth-5fm");
    }

    #[test]
    fn test_bead_uri_parsing() {
        let uri = BeadUri::from_string("bead://auth-service/auth-5fm");

        assert_eq!(uri.rig_id().unwrap().as_str(), "auth-service");
        assert_eq!(uri.bead_id().unwrap().as_str(), "auth-5fm");
    }

    #[test]
    fn test_shadow_bead_creation() {
        let rig_id = RigId::new("auth-service");
        let native_id = BeadId::new("auth-5fm");
        let shadow = ShadowBead::new(
            "boss-mig-1",
            &rig_id,
            &native_id,
            "Refactor JWT Logic",
            "work",
        );

        assert_eq!(shadow.id.as_str(), "boss-mig-1");
        assert_eq!(shadow.summary, "Refactor JWT Logic");
        assert_eq!(shadow.context, "work");
        assert_eq!(shadow.pointer.as_str(), "bead://auth-service/auth-5fm");
    }

    #[test]
    fn test_cross_repo_dependencies() {
        let rig_id = RigId::new("auth-service");
        let native_id = BeadId::new("auth-5fm");
        let mut shadow = ShadowBead::new("boss-mig-1", &rig_id, &native_id, "Test", "work");

        assert!(!shadow.has_cross_repo_blockers());

        let dep_uri = BeadUri::from_string("bead://frontend-web/fe-123");
        shadow.add_cross_repo_dependency(dep_uri);

        assert!(shadow.has_cross_repo_blockers());
        assert_eq!(shadow.cross_repo_dependencies.len(), 1);
    }

    #[test]
    fn test_shadow_bead_serialization() {
        let rig_id = RigId::new("auth-service");
        let native_id = BeadId::new("auth-5fm");
        let shadow = ShadowBead::new("boss-mig-1", &rig_id, &native_id, "Test", "work");

        let json = serde_json::to_string(&shadow).unwrap();
        assert!(json.contains("boss-mig-1"));
        assert!(json.contains("bead://auth-service/auth-5fm"));
    }
}
