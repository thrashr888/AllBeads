//! Rule: Require assignee

use super::{CheckResult, PolicyRule};
use crate::governance::policy::PolicyConfig;
use crate::graph::{Bead, Status};

/// Rule that requires open beads to have an assignee
pub struct RequireAssigneeRule;

impl PolicyRule for RequireAssigneeRule {
    fn check_bead(&self, bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        // Only check non-closed beads
        if bead.status == Status::Closed {
            return None;
        }

        if bead.assignee.is_some() {
            None // Pass
        } else {
            Some(
                CheckResult::fail(
                    "require-assignee",
                    format!("Bead {} has no assignee", bead.id.as_str()),
                )
                .with_affected_beads(vec![bead.id.as_str().to_string()]),
            )
        }
    }

    fn name(&self) -> &'static str {
        "require-assignee"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BeadId, Priority};
    use std::collections::HashSet;

    fn make_bead(id: &str, status: Status, assignee: Option<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: "Test".to_string(),
            description: None,
            status,
            priority: Priority::P2,
            labels: HashSet::new(),
            dependencies: vec![],
            blocks: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            assignee: assignee.map(String::from),
            issue_type: crate::graph::IssueType::Task,
            notes: None,
            aiki_tasks: Vec::new(),
        }
    }

    #[test]
    fn test_open_bead_with_assignee_passes() {
        let rule = RequireAssigneeRule;
        let bead = make_bead("test-1", Status::Open, Some("alice"));
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_open_bead_without_assignee_fails() {
        let rule = RequireAssigneeRule;
        let bead = make_bead("test-1", Status::Open, None);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_some());
        assert!(!result.unwrap().passed);
    }

    #[test]
    fn test_closed_bead_without_assignee_passes() {
        let rule = RequireAssigneeRule;
        let bead = make_bead("test-1", Status::Closed, None);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_none()); // Closed beads are not checked
    }
}
