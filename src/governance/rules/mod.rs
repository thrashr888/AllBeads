//! Policy rules implementations

mod cycle_detection;
mod max_in_progress;
mod require_assignee;
mod require_description;
mod require_labels;
mod require_priority;

pub use cycle_detection::CycleDetectionRule;
pub use max_in_progress::MaxInProgressRule;
pub use require_assignee::RequireAssigneeRule;
pub use require_description::RequireDescriptionRule;
pub use require_labels::RequireLabelsRule;
pub use require_priority::RequirePriorityRule;

use crate::governance::policy::{Policy, PolicyConfig};
use crate::graph::{Bead, FederatedGraph};

/// Result of checking a policy rule
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Name of the policy that was checked
    pub policy_name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Human-readable message about the result
    pub message: String,
    /// Optional affected bead IDs
    pub affected_beads: Vec<String>,
    /// Timestamp of when the check was run
    pub timestamp: String,
}

impl CheckResult {
    /// Create a passing check result
    pub fn pass(policy_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            policy_name: policy_name.into(),
            passed: true,
            message: message.into(),
            affected_beads: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a failing check result
    pub fn fail(policy_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            policy_name: policy_name.into(),
            passed: false,
            message: message.into(),
            affected_beads: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add affected beads to the result
    pub fn with_affected_beads(mut self, beads: Vec<String>) -> Self {
        self.affected_beads = beads;
        self
    }
}

/// Trait for policy rules
pub trait PolicyRule: Send + Sync {
    /// Check a single bead against this rule
    fn check_bead(&self, bead: &Bead, config: &PolicyConfig) -> Option<CheckResult>;

    /// Check the entire graph against this rule
    /// Default implementation checks each bead individually
    fn check_graph(&self, graph: &FederatedGraph, policy: &Policy) -> CheckResult {
        let mut failures = Vec::new();
        let mut total_checked = 0;

        for bead in graph.beads.values() {
            total_checked += 1;
            if let Some(result) = self.check_bead(bead, &policy.config) {
                if !result.passed {
                    failures.extend(result.affected_beads);
                }
            }
        }

        if failures.is_empty() {
            CheckResult::pass(&policy.name, format!("All {} beads passed", total_checked))
        } else {
            CheckResult::fail(
                &policy.name,
                format!("{} bead(s) failed check", failures.len()),
            )
            .with_affected_beads(failures)
        }
    }

    /// Get the name of this rule
    fn name(&self) -> &'static str;
}
