//! Policy checker - executes policy rules against beads

use super::policy::{Policy, PolicyType};
use super::rules::{
    CheckResult, CycleDetectionRule, MaxInProgressRule, PolicyRule, RequireAssigneeRule,
    RequireDescriptionRule, RequireLabelsRule, RequirePriorityRule,
};
use crate::graph::FederatedGraph;

/// Policy checker that runs policies against the federated graph
pub struct PolicyChecker {
    policies: Vec<Policy>,
}

impl PolicyChecker {
    /// Create a new policy checker
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Create a policy checker with default policies
    pub fn with_defaults() -> Self {
        Self {
            policies: super::policy::default_policies(),
        }
    }

    /// Add a policy to the checker
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Set all policies
    pub fn set_policies(&mut self, policies: Vec<Policy>) {
        self.policies = policies;
    }

    /// Get all policies
    pub fn policies(&self) -> &[Policy] {
        &self.policies
    }

    /// Get enabled policies
    pub fn enabled_policies(&self) -> Vec<&Policy> {
        self.policies.iter().filter(|p| p.enabled).collect()
    }

    /// Run all enabled policies against the graph
    pub fn check_graph(&self, graph: &FederatedGraph) -> Vec<CheckResult> {
        let mut results = Vec::new();

        for policy in self.enabled_policies() {
            let rule = Self::get_rule(&policy.policy_type);
            let result = rule.check_graph(graph, policy);
            results.push(result);
        }

        results
    }

    /// Get the rule implementation for a policy type
    fn get_rule(policy_type: &PolicyType) -> Box<dyn PolicyRule> {
        match policy_type {
            PolicyType::RequireDescription => Box::new(RequireDescriptionRule),
            PolicyType::MaxInProgress { max_count } => {
                Box::new(MaxInProgressRule::new(*max_count))
            }
            PolicyType::RequireLabels { min_count } => {
                Box::new(RequireLabelsRule::new(*min_count))
            }
            PolicyType::DependencyCycleCheck => Box::new(CycleDetectionRule),
            PolicyType::RequirePriority => Box::new(RequirePriorityRule),
            PolicyType::RequireAssignee => Box::new(RequireAssigneeRule),
            PolicyType::Custom { .. } => {
                // Custom rules would need a registry, for now return a no-op
                Box::new(RequirePriorityRule) // Placeholder
            }
        }
    }

    /// Get a summary of check results
    pub fn summarize(results: &[CheckResult]) -> CheckSummary {
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.iter().filter(|r| !r.passed).count();
        let total_affected: usize = results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.affected_beads.len())
            .sum();

        CheckSummary {
            total_checks: results.len(),
            passed,
            failed,
            total_affected_beads: total_affected,
        }
    }
}

impl Default for PolicyChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of check results
#[derive(Debug, Clone)]
pub struct CheckSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_affected_beads: usize,
}

impl CheckSummary {
    /// Check if all policies passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Bead, BeadId, Priority, Status};
    use std::collections::HashSet;

    fn make_bead(id: &str, description: Option<&str>, assignee: Option<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: format!("Test {}", id),
            description: description.map(String::from),
            status: Status::Open,
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
    fn test_checker_with_defaults() {
        let checker = PolicyChecker::with_defaults();
        assert!(!checker.policies().is_empty());
    }

    #[test]
    fn test_check_graph() {
        let mut checker = PolicyChecker::new();
        checker.add_policy(Policy::new(
            "require-description",
            PolicyType::RequireDescription,
        ));

        let graph = make_graph(vec![
            make_bead("test-1", Some("Has description"), None),
            make_bead("test-2", None, None), // Missing description
        ]);

        let results = checker.check_graph(&graph);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed); // Should fail because one bead is missing description
    }

    #[test]
    fn test_disabled_policy_not_checked() {
        let mut checker = PolicyChecker::new();
        checker.add_policy(
            Policy::new("require-description", PolicyType::RequireDescription).with_enabled(false),
        );

        let graph = make_graph(vec![make_bead("test-1", None, None)]);

        let results = checker.check_graph(&graph);
        assert!(results.is_empty()); // Disabled policy should not run
    }

    #[test]
    fn test_summarize() {
        let results = vec![
            CheckResult::pass("test1", "Passed"),
            CheckResult::fail("test2", "Failed").with_affected_beads(vec!["a".to_string()]),
            CheckResult::fail("test3", "Failed")
                .with_affected_beads(vec!["b".to_string(), "c".to_string()]),
        ];

        let summary = PolicyChecker::summarize(&results);
        assert_eq!(summary.total_checks, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.total_affected_beads, 3);
        assert!(!summary.all_passed());
    }
}
