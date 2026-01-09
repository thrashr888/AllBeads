//! Bead data structure
//!
//! Represents a single issue/task/epic in the beads system.
//! Matches the beads JSONL schema for compatibility.

use super::BeadId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Issue status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
}

impl Default for Status {
    fn default() -> Self {
        Self::Open
    }
}

/// Issue priority (0 = critical, 4 = backlog)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    P0 = 0, // Critical
    P1 = 1, // High
    P2 = 2, // Medium
    P3 = 3, // Low
    P4 = 4, // Backlog
}

impl Default for Priority {
    fn default() -> Self {
        Self::P2
    }
}

impl From<u8> for Priority {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            4.. => Self::P4,
        }
    }
}

/// Issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
    MergeRequest,
    Molecule,
    Gate,
}

impl Default for IssueType {
    fn default() -> Self {
        Self::Task
    }
}

/// Core bead structure representing an issue/task/epic
///
/// This matches the beads JSONL schema for compatibility with the `bd` CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    /// Unique identifier (e.g., "ab-ldr")
    pub id: BeadId,

    /// Issue title
    pub title: String,

    /// Optional detailed description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Current status
    pub status: Status,

    /// Priority level (0-4)
    pub priority: Priority,

    /// Type of issue
    #[serde(rename = "issue_type")]
    pub issue_type: IssueType,

    /// Creation timestamp (RFC3339 format)
    pub created_at: String,

    /// Last update timestamp (RFC3339 format)
    pub updated_at: String,

    /// Creator username
    pub created_by: String,

    /// Optional assignee
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Dependencies (beads this one depends on)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<BeadId>,

    /// Beads that depend on this one (blocked by this)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<BeadId>,

    /// Labels/tags
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub labels: HashSet<String>,

    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Bead {
    /// Create a new bead with required fields
    pub fn new(
        id: impl Into<BeadId>,
        title: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            status: Status::default(),
            priority: Priority::default(),
            issue_type: IssueType::default(),
            created_at: now.clone(),
            updated_at: now,
            created_by: created_by.into(),
            assignee: None,
            dependencies: Vec::new(),
            blocks: Vec::new(),
            labels: HashSet::new(),
            notes: None,
        }
    }

    /// Check if this bead is blocked by dependencies
    pub fn is_blocked(&self) -> bool {
        !self.dependencies.is_empty() && self.status != Status::Closed
    }

    /// Check if this bead is ready to work (no blockers)
    pub fn is_ready(&self) -> bool {
        self.dependencies.is_empty() && self.status == Status::Open
    }

    /// Add a dependency (this bead depends on another)
    pub fn add_dependency(&mut self, dep: impl Into<BeadId>) {
        let dep_id = dep.into();
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
        self.update_timestamp();
    }

    /// Add a label/tag
    pub fn add_label(&mut self, label: impl Into<String>) {
        self.labels.insert(label.into());
        self.update_timestamp();
    }

    /// Update the timestamp to now
    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_creation() {
        let bead = Bead::new("ab-123", "Test Issue", "alice");

        assert_eq!(bead.id.as_str(), "ab-123");
        assert_eq!(bead.title, "Test Issue");
        assert_eq!(bead.created_by, "alice");
        assert_eq!(bead.status, Status::Open);
        assert_eq!(bead.priority, Priority::P2);
        assert_eq!(bead.issue_type, IssueType::Task);
    }

    #[test]
    fn test_bead_is_ready() {
        let mut bead = Bead::new("ab-123", "Test", "alice");
        assert!(bead.is_ready());

        bead.add_dependency("ab-456");
        assert!(!bead.is_ready());
        assert!(bead.is_blocked());
    }

    #[test]
    fn test_bead_serialization() {
        let bead = Bead::new("ab-123", "Test Issue", "alice");
        let json = serde_json::to_string(&bead).unwrap();

        assert!(json.contains("ab-123"));
        assert!(json.contains("Test Issue"));
        assert!(json.contains("alice"));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::P0 < Priority::P1);
        assert!(Priority::P1 < Priority::P2);
        assert!(Priority::P2 < Priority::P3);
        assert!(Priority::P3 < Priority::P4);
    }

    #[test]
    fn test_labels() {
        let mut bead = Bead::new("ab-123", "Test", "alice");
        bead.add_label("bug");
        bead.add_label("p1");

        assert!(bead.labels.contains("bug"));
        assert!(bead.labels.contains("p1"));
        assert_eq!(bead.labels.len(), 2);
    }
}
