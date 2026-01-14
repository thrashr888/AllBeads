//! Rule: Require description

use super::{CheckResult, PolicyRule};
use crate::governance::policy::PolicyConfig;
use crate::graph::Bead;

/// Rule that requires all beads to have a description
pub struct RequireDescriptionRule;

impl PolicyRule for RequireDescriptionRule {
    fn check_bead(&self, bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        let has_description = bead
            .description
            .as_ref()
            .is_some_and(|d| !d.trim().is_empty());

        if has_description {
            None // Pass - no result needed for individual beads
        } else {
            Some(
                CheckResult::fail(
                    "require-description",
                    format!("Bead {} is missing a description", bead.id.as_str()),
                )
                .with_affected_beads(vec![bead.id.as_str().to_string()]),
            )
        }
    }

    fn name(&self) -> &'static str {
        "require-description"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BeadId, Priority, Status};
    use std::collections::HashSet;

    fn make_bead(id: &str, description: Option<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: "Test".to_string(),
            description: description.map(|s| s.to_string()),
            status: Status::Open,
            priority: Priority::P2,
            labels: HashSet::new(),
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
    fn test_bead_with_description_passes() {
        let rule = RequireDescriptionRule;
        let bead = make_bead("test-1", Some("This is a description"));
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_none()); // None means pass
    }

    #[test]
    fn test_bead_without_description_fails() {
        let rule = RequireDescriptionRule;
        let bead = make_bead("test-1", None);
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(!result.passed);
        assert!(result.affected_beads.contains(&"test-1".to_string()));
    }

    #[test]
    fn test_bead_with_empty_description_fails() {
        let rule = RequireDescriptionRule;
        let bead = make_bead("test-1", Some("   "));
        let result = rule.check_bead(&bead, &PolicyConfig::default());
        assert!(result.is_some());
        assert!(!result.unwrap().passed);
    }
}
