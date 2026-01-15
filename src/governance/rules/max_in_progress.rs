//! Rule: Max in-progress beads per assignee

use super::{CheckResult, PolicyRule};
use crate::governance::policy::{Policy, PolicyConfig};
use crate::graph::{Bead, FederatedGraph, Status};
use std::collections::HashMap;

/// Rule that limits concurrent in-progress beads per assignee
pub struct MaxInProgressRule {
    max_count: usize,
}

impl MaxInProgressRule {
    pub fn new(max_count: usize) -> Self {
        Self { max_count }
    }
}

impl PolicyRule for MaxInProgressRule {
    fn check_bead(&self, _bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        // This rule operates on the graph level, not individual beads
        None
    }

    fn check_graph(&self, graph: &FederatedGraph, policy: &Policy) -> CheckResult {
        // Count in-progress beads per assignee
        let mut by_assignee: HashMap<String, Vec<String>> = HashMap::new();

        for bead in graph.beads.values() {
            if bead.status == Status::InProgress {
                let assignee = bead
                    .assignee
                    .clone()
                    .unwrap_or_else(|| "(unassigned)".to_string());
                by_assignee
                    .entry(assignee)
                    .or_default()
                    .push(bead.id.as_str().to_string());
            }
        }

        // Check for violations
        let mut violations: Vec<String> = Vec::new();
        let mut violation_messages: Vec<String> = Vec::new();

        for (assignee, beads) in by_assignee {
            if beads.len() > self.max_count {
                violation_messages.push(format!(
                    "{} has {} in-progress (max: {})",
                    assignee,
                    beads.len(),
                    self.max_count
                ));
                violations.extend(beads);
            }
        }

        if violations.is_empty() {
            CheckResult::pass(&policy.name, "All assignees within in-progress limits")
        } else {
            CheckResult::fail(&policy.name, violation_messages.join("; "))
                .with_affected_beads(violations)
        }
    }

    fn name(&self) -> &'static str {
        "max-in-progress"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::policy::PolicyType;
    use crate::graph::{BeadId, Priority};
    use std::collections::HashSet;

    fn make_bead(id: &str, status: Status, assignee: Option<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: format!("Test {}", id),
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

    fn make_graph(beads: Vec<Bead>) -> FederatedGraph {
        let mut graph = FederatedGraph::new();
        for bead in beads {
            graph.beads.insert(bead.id.clone(), bead);
        }
        graph
    }

    #[test]
    fn test_within_limit_passes() {
        let rule = MaxInProgressRule::new(3);
        let policy = Policy::new(
            "max-in-progress",
            PolicyType::MaxInProgress { max_count: 3 },
        );

        let graph = make_graph(vec![
            make_bead("test-1", Status::InProgress, Some("alice")),
            make_bead("test-2", Status::InProgress, Some("alice")),
            make_bead("test-3", Status::Open, Some("alice")),
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(result.passed);
    }

    #[test]
    fn test_over_limit_fails() {
        let rule = MaxInProgressRule::new(2);
        let policy = Policy::new(
            "max-in-progress",
            PolicyType::MaxInProgress { max_count: 2 },
        );

        let graph = make_graph(vec![
            make_bead("test-1", Status::InProgress, Some("alice")),
            make_bead("test-2", Status::InProgress, Some("alice")),
            make_bead("test-3", Status::InProgress, Some("alice")),
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(!result.passed);
        assert!(result.message.contains("alice"));
        assert!(result.message.contains("3 in-progress"));
    }

    #[test]
    fn test_different_assignees_separate_counts() {
        let rule = MaxInProgressRule::new(2);
        let policy = Policy::new(
            "max-in-progress",
            PolicyType::MaxInProgress { max_count: 2 },
        );

        let graph = make_graph(vec![
            make_bead("test-1", Status::InProgress, Some("alice")),
            make_bead("test-2", Status::InProgress, Some("alice")),
            make_bead("test-3", Status::InProgress, Some("bob")),
            make_bead("test-4", Status::InProgress, Some("bob")),
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(result.passed); // Both have exactly 2, which is the limit
    }
}
