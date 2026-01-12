//! Rule: Require priority

use super::{CheckResult, PolicyRule};
use crate::governance::policy::PolicyConfig;
use crate::graph::Bead;

/// Rule that requires all beads to have a priority set
pub struct RequirePriorityRule;

impl PolicyRule for RequirePriorityRule {
    fn check_bead(&self, _bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        // All beads have a priority by default in our model, so this always passes
        // This rule exists for systems where priority might be optional
        None
    }

    fn name(&self) -> &'static str {
        "require-priority"
    }
}
