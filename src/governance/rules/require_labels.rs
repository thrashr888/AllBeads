//! Rule: Require labels

use super::{CheckResult, PolicyRule};
use crate::governance::policy::PolicyConfig;
use crate::graph::Bead;

/// Rule that requires beads to have a minimum number of labels
pub struct RequireLabelsRule {
    min_count: usize,
}

impl RequireLabelsRule {
    pub fn new(min_count: usize) -> Self {
        Self { min_count }
    }
}

impl PolicyRule for RequireLabelsRule {
    fn check_bead(&self, bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        if bead.labels.len() >= self.min_count {
            None // Pass
        } else {
            Some(
                CheckResult::fail(
                    "require-labels",
                    format!(
                        "Bead {} has {} label(s), requires at least {}",
                        bead.id.as_str(),
                        bead.labels.len(),
                        self.min_count
                    ),
                )
                .with_affected_beads(vec![bead.id.as_str().to_string()]),
            )
        }
    }

    fn name(&self) -> &'static str {
        "require-labels"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BeadId, Priority, Status};
    use std::collections::HashSet;

    fn make_bead(id: &str, labels: Vec<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: "Test".to_string(),
            description: None,
            status: Status::Open,
            priority: Priority::P2,
            labels: labels.into_iter().map(String::from).collect::<HashSet<_>>(),
            dependencies: vec![],
            blocks: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            assignee: None,
            issue_type: crate::graph::IssueType::Task,
            notes: None,
            aiki_tasks: Vec::new(),
        }
    }

    #[test]
    fn test_bead_with_labels_passes() {
        let rule = RequireLabelsRule::new(1);
        let bead = make_bead("test-1", vec!["bug"]);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_bead_without_labels_fails() {
        let rule = RequireLabelsRule::new(1);
        let bead = make_bead("test-1", vec![]);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_some());
        assert!(!result.unwrap().passed);
    }

    #[test]
    fn test_bead_with_insufficient_labels_fails() {
        let rule = RequireLabelsRule::new(2);
        let bead = make_bead("test-1", vec!["bug"]);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_some());
        assert!(!result.unwrap().passed);
    }
}
