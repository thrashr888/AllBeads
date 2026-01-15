//! Type conversions between beads crate and AllBeads internal types
//!
//! Provides conversion traits to translate between:
//! - beads::Issue <-> graph::Bead
//! - beads::Status <-> graph::Status
//! - beads::IssueType <-> graph::IssueType

use crate::graph::{Bead, BeadId, IssueType, Priority, Status};
use crate::Result;

/// Convert beads crate Status to AllBeads Status
impl From<beads::Status> for Status {
    fn from(status: beads::Status) -> Self {
        match status {
            beads::Status::Open => Status::Open,
            beads::Status::InProgress => Status::InProgress,
            beads::Status::Blocked => Status::Blocked,
            beads::Status::Deferred => Status::Deferred,
            beads::Status::Closed => Status::Closed,
            beads::Status::Tombstone => Status::Tombstone,
        }
    }
}

/// Convert AllBeads Status to beads crate Status
impl From<Status> for beads::Status {
    fn from(status: Status) -> Self {
        match status {
            Status::Open => beads::Status::Open,
            Status::InProgress => beads::Status::InProgress,
            Status::Blocked => beads::Status::Blocked,
            Status::Deferred => beads::Status::Deferred,
            Status::Closed => beads::Status::Closed,
            Status::Tombstone => beads::Status::Tombstone,
        }
    }
}

/// Convert beads crate IssueType to AllBeads IssueType
impl From<beads::IssueType> for IssueType {
    fn from(issue_type: beads::IssueType) -> Self {
        match issue_type {
            beads::IssueType::Bug => IssueType::Bug,
            beads::IssueType::Feature => IssueType::Feature,
            beads::IssueType::Task => IssueType::Task,
            beads::IssueType::Epic => IssueType::Epic,
            beads::IssueType::Chore => IssueType::Chore,
            beads::IssueType::MergeRequest => IssueType::MergeRequest,
            beads::IssueType::Molecule => IssueType::Molecule,
            beads::IssueType::Gate => IssueType::Gate,
        }
    }
}

/// Convert AllBeads IssueType to beads crate IssueType
impl From<IssueType> for beads::IssueType {
    fn from(issue_type: IssueType) -> Self {
        match issue_type {
            IssueType::Bug => beads::IssueType::Bug,
            IssueType::Feature => beads::IssueType::Feature,
            IssueType::Task => beads::IssueType::Task,
            IssueType::Epic => beads::IssueType::Epic,
            IssueType::Chore => beads::IssueType::Chore,
            IssueType::MergeRequest => beads::IssueType::MergeRequest,
            IssueType::Molecule => beads::IssueType::Molecule,
            IssueType::Gate => beads::IssueType::Gate,
        }
    }
}

/// Convert string status to AllBeads Status
pub fn parse_status(s: &str) -> Result<Status> {
    match s {
        "open" => Ok(Status::Open),
        "in_progress" => Ok(Status::InProgress),
        "blocked" => Ok(Status::Blocked),
        "deferred" => Ok(Status::Deferred),
        "closed" => Ok(Status::Closed),
        _ => Err(crate::AllBeadsError::Parse(format!(
            "Invalid status: {}",
            s
        ))),
    }
}

/// Convert string issue type to AllBeads IssueType
pub fn parse_issue_type(s: &str) -> Result<IssueType> {
    match s {
        "bug" => Ok(IssueType::Bug),
        "feature" => Ok(IssueType::Feature),
        "task" => Ok(IssueType::Task),
        "epic" => Ok(IssueType::Epic),
        "chore" => Ok(IssueType::Chore),
        "merge_request" => Ok(IssueType::MergeRequest),
        "molecule" => Ok(IssueType::Molecule),
        "gate" => Ok(IssueType::Gate),
        _ => Err(crate::AllBeadsError::Parse(format!(
            "Invalid issue type: {}",
            s
        ))),
    }
}

/// Convert beads::Issue to AllBeads Bead
pub fn issue_to_bead(issue: beads::Issue) -> Result<Bead> {
    let status = parse_status(&issue.status)?;
    let issue_type = parse_issue_type(&issue.issue_type)?;

    let priority = issue
        .priority
        .and_then(|p| match p {
            0 => Some(Priority::P0),
            1 => Some(Priority::P1),
            2 => Some(Priority::P2),
            3 => Some(Priority::P3),
            4 => Some(Priority::P4),
            _ => None,
        })
        .unwrap_or(Priority::P2);

    let bead = Bead {
        id: BeadId::new(issue.id),
        title: issue.title,
        description: issue.description,
        status,
        priority,
        issue_type,
        created_at: issue
            .created_at
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: issue
            .updated_at
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        created_by: "unknown".to_string(), // beads::Issue doesn't track creator
        assignee: issue.assignee,
        dependencies: issue.depends_on.into_iter().map(BeadId::new).collect(),
        blocks: issue.blocks.into_iter().map(|d| BeadId::new(d.id)).collect(),
        labels: issue.labels.into_iter().collect(),
        notes: None,
        aiki_tasks: Vec::new(),
        handoff: None,
    };

    Ok(bead)
}

/// Convert multiple beads::Issue to AllBeads Beads
pub fn issues_to_beads(issues: Vec<beads::Issue>) -> Result<Vec<Bead>> {
    issues.into_iter().map(issue_to_bead).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_conversion() {
        assert_eq!(Status::from(beads::Status::Open), Status::Open);
        assert_eq!(Status::from(beads::Status::InProgress), Status::InProgress);
        assert_eq!(Status::from(beads::Status::Blocked), Status::Blocked);
        assert_eq!(Status::from(beads::Status::Closed), Status::Closed);
    }

    #[test]
    fn test_status_roundtrip() {
        let original = Status::InProgress;
        let beads_status: beads::Status = original.into();
        let back: Status = beads_status.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_issue_type_conversion() {
        assert_eq!(IssueType::from(beads::IssueType::Bug), IssueType::Bug);
        assert_eq!(IssueType::from(beads::IssueType::Epic), IssueType::Epic);
    }

    #[test]
    fn test_parse_status() {
        assert!(parse_status("open").is_ok());
        assert!(parse_status("in_progress").is_ok());
        assert!(parse_status("invalid").is_err());
    }

    #[test]
    fn test_parse_issue_type() {
        assert!(parse_issue_type("bug").is_ok());
        assert!(parse_issue_type("feature").is_ok());
        assert!(parse_issue_type("invalid").is_err());
    }
}
