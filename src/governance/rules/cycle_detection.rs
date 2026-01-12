//! Rule: Dependency cycle detection

use super::{CheckResult, PolicyRule};
use crate::governance::policy::{Policy, PolicyConfig};
use crate::graph::{Bead, BeadId, FederatedGraph};
use std::collections::{HashMap, HashSet};

/// Rule that detects circular dependencies
pub struct CycleDetectionRule;

impl PolicyRule for CycleDetectionRule {
    fn check_bead(&self, _bead: &Bead, _config: &PolicyConfig) -> Option<CheckResult> {
        // This rule operates on the graph level
        None
    }

    fn check_graph(&self, graph: &FederatedGraph, policy: &Policy) -> CheckResult {
        // Build adjacency list from dependencies
        let mut adj: HashMap<&BeadId, Vec<&BeadId>> = HashMap::new();

        for bead in graph.beads.values() {
            adj.entry(&bead.id).or_default();
            for dep in &bead.dependencies {
                adj.entry(&bead.id).or_default().push(dep);
            }
        }

        // Find cycles using DFS
        let mut visited: HashSet<&BeadId> = HashSet::new();
        let mut rec_stack: HashSet<&BeadId> = HashSet::new();
        let mut cycles: Vec<String> = Vec::new();

        for node in adj.keys() {
            if !visited.contains(node) {
                let mut path = Vec::new();
                if let Some(cycle) = detect_cycle_dfs(node, &adj, &mut visited, &mut rec_stack, &mut path) {
                    cycles.push(cycle);
                }
            }
        }

        if cycles.is_empty() {
            CheckResult::pass(&policy.name, "No circular dependencies detected")
        } else {
            CheckResult::fail(
                &policy.name,
                format!("Found {} cycle(s): {}", cycles.len(), cycles.join(", ")),
            )
            .with_affected_beads(cycles)
        }
    }

    fn name(&self) -> &'static str {
        "dependency-cycle-check"
    }
}

fn detect_cycle_dfs<'a>(
    node: &'a BeadId,
    adj: &HashMap<&'a BeadId, Vec<&'a BeadId>>,
    visited: &mut HashSet<&'a BeadId>,
    rec_stack: &mut HashSet<&'a BeadId>,
    path: &mut Vec<&'a BeadId>,
) -> Option<String> {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = detect_cycle_dfs(neighbor, adj, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - build the cycle string
                let cycle_start = path.iter().position(|n| *n == *neighbor).unwrap();
                let cycle_path: Vec<String> = path[cycle_start..]
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect();
                return Some(format!("{} -> {}", cycle_path.join(" -> "), neighbor.as_str()));
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::policy::PolicyType;
    use crate::graph::{Priority, Status};
    use std::collections::HashSet;

    fn make_bead(id: &str, deps: Vec<&str>) -> Bead {
        Bead {
            id: BeadId::new(id),
            title: format!("Test {}", id),
            description: None,
            status: Status::Open,
            priority: Priority::P2,
            labels: HashSet::new(),
            dependencies: deps.into_iter().map(BeadId::new).collect(),
            blocks: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            assignee: None,
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
    fn test_no_cycle() {
        let rule = CycleDetectionRule;
        let policy = Policy::new("cycle-check", PolicyType::DependencyCycleCheck);

        let graph = make_graph(vec![
            make_bead("a", vec!["b"]),
            make_bead("b", vec!["c"]),
            make_bead("c", vec![]),
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(result.passed);
    }

    #[test]
    fn test_simple_cycle() {
        let rule = CycleDetectionRule;
        let policy = Policy::new("cycle-check", PolicyType::DependencyCycleCheck);

        let graph = make_graph(vec![
            make_bead("a", vec!["b"]),
            make_bead("b", vec!["c"]),
            make_bead("c", vec!["a"]), // Cycle: a -> b -> c -> a
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(!result.passed);
    }

    #[test]
    fn test_self_cycle() {
        let rule = CycleDetectionRule;
        let policy = Policy::new("cycle-check", PolicyType::DependencyCycleCheck);

        let graph = make_graph(vec![
            make_bead("a", vec!["a"]), // Self-reference
        ]);

        let result = rule.check_graph(&graph, &policy);
        assert!(!result.passed);
    }
}
