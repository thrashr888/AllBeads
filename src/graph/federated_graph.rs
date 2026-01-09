//! Federated Graph
//!
//! Unified graph containing beads, shadow beads, and cross-repo dependencies.

use super::{Bead, BeadId, Rig, RigId, ShadowBead, Status};
use std::collections::{HashMap, HashSet};

/// Federated graph aggregating beads across multiple contexts
///
/// The FederatedGraph is the central data structure that unifies:
/// - Native beads from the Boss repository
/// - Shadow beads pointing to member Rig repositories
/// - Cross-repo dependency relationships
/// - Multi-context work isolation (work, personal, etc.)
#[derive(Debug, Clone, Default)]
pub struct FederatedGraph {
    /// Native beads in the Boss repository (keyed by BeadId)
    pub beads: HashMap<BeadId, Bead>,

    /// Shadow beads pointing to member Rigs (keyed by BeadId)
    pub shadow_beads: HashMap<BeadId, ShadowBead>,

    /// Member Rig configurations (keyed by RigId)
    pub rigs: HashMap<RigId, Rig>,

    /// Index: BeadId -> Set of BeadIds that depend on it
    /// (inverse of dependencies - "what depends on me?")
    dependents_index: HashMap<BeadId, HashSet<BeadId>>,

    /// Index: Context -> Set of BeadIds in that context
    context_index: HashMap<String, HashSet<BeadId>>,

    /// Index: Label -> Set of BeadIds with that label
    label_index: HashMap<String, HashSet<BeadId>>,
}

impl FederatedGraph {
    /// Create a new empty federated graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a native bead to the graph
    pub fn add_bead(&mut self, bead: Bead) {
        let id = bead.id.clone();

        // Update indices
        for dep_id in &bead.dependencies {
            self.dependents_index
                .entry(dep_id.clone())
                .or_insert_with(HashSet::new)
                .insert(id.clone());
        }

        for label in &bead.labels {
            self.label_index
                .entry(label.clone())
                .or_insert_with(HashSet::new)
                .insert(id.clone());
        }

        // Note: Native beads don't have a context field, so we don't index by context

        self.beads.insert(id, bead);
    }

    /// Add a shadow bead to the graph
    pub fn add_shadow_bead(&mut self, shadow: ShadowBead) {
        let id = shadow.id.clone();

        // Index by context
        self.context_index
            .entry(shadow.context.clone())
            .or_insert_with(HashSet::new)
            .insert(id.clone());

        // Index by labels
        for label in &shadow.labels {
            self.label_index
                .entry(label.clone())
                .or_insert_with(HashSet::new)
                .insert(id.clone());
        }

        self.shadow_beads.insert(id, shadow);
    }

    /// Add a member Rig to the graph
    pub fn add_rig(&mut self, rig: Rig) {
        self.rigs.insert(rig.id.clone(), rig);
    }

    /// Get a bead by ID (checks both beads and shadow beads)
    pub fn get_bead(&self, id: &BeadId) -> Option<&Bead> {
        self.beads.get(id)
    }

    /// Get a shadow bead by ID
    pub fn get_shadow_bead(&self, id: &BeadId) -> Option<&ShadowBead> {
        self.shadow_beads.get(id)
    }

    /// Get a rig by ID
    pub fn get_rig(&self, id: &RigId) -> Option<&Rig> {
        self.rigs.get(id)
    }

    /// Query beads by status
    pub fn beads_by_status(&self, status: Status) -> Vec<&Bead> {
        self.beads
            .values()
            .filter(|b| b.status == status)
            .collect()
    }

    /// Query shadow beads by status
    pub fn shadow_beads_by_status(&self, status: Status) -> Vec<&ShadowBead> {
        self.shadow_beads
            .values()
            .filter(|s| s.status == status)
            .collect()
    }

    /// Query beads by context (shadow beads only, as native beads don't have context)
    pub fn beads_by_context(&self, context: &str) -> Vec<&ShadowBead> {
        self.context_index
            .get(context)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.shadow_beads.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query beads by label (searches both beads and shadow beads)
    pub fn beads_by_label(&self, label: &str) -> (Vec<&Bead>, Vec<&ShadowBead>) {
        let ids = self.label_index.get(label);
        if ids.is_none() {
            return (Vec::new(), Vec::new());
        }

        let ids = ids.unwrap();
        let beads = ids
            .iter()
            .filter_map(|id| self.beads.get(id))
            .collect();
        let shadows = ids
            .iter()
            .filter_map(|id| self.shadow_beads.get(id))
            .collect();

        (beads, shadows)
    }

    /// Get all beads that depend on the given bead ID
    pub fn get_dependents(&self, id: &BeadId) -> Vec<&Bead> {
        self.dependents_index
            .get(id)
            .map(|deps| deps.iter().filter_map(|dep_id| self.beads.get(dep_id)).collect())
            .unwrap_or_default()
    }

    /// Check if a bead is ready to work (no blocking dependencies)
    pub fn is_bead_ready(&self, id: &BeadId) -> bool {
        if let Some(bead) = self.beads.get(id) {
            bead.is_ready()
        } else if let Some(shadow) = self.shadow_beads.get(id) {
            // For shadow beads, check if all cross-repo dependencies are closed
            shadow.cross_repo_dependencies.iter().all(|uri| {
                // Extract bead ID from URI and check if it's closed
                uri.bead_id()
                    .and_then(|dep_id| {
                        self.beads
                            .get(&dep_id)
                            .map(|b| b.status == Status::Closed)
                            .or_else(|| {
                                self.shadow_beads
                                    .get(&dep_id)
                                    .map(|s| s.status == Status::Closed)
                            })
                    })
                    .unwrap_or(false) // If dependency not found, assume not ready
            })
        } else {
            false
        }
    }

    /// Get all beads that are ready to work (no blocking dependencies)
    pub fn ready_beads(&self) -> Vec<&Bead> {
        self.beads
            .values()
            .filter(|b| b.is_ready() && b.status != Status::Closed)
            .collect()
    }

    /// Get statistics about the graph
    pub fn stats(&self) -> GraphStats {
        let total_beads = self.beads.len();
        let total_shadows = self.shadow_beads.len();
        let total_rigs = self.rigs.len();

        let open_beads = self.beads.values().filter(|b| b.status == Status::Open).count();
        let in_progress_beads = self
            .beads
            .values()
            .filter(|b| b.status == Status::InProgress)
            .count();
        let blocked_beads = self.beads.values().filter(|b| b.status == Status::Blocked).count();
        let closed_beads = self.beads.values().filter(|b| b.status == Status::Closed).count();

        GraphStats {
            total_beads,
            total_shadows,
            total_rigs,
            open_beads,
            in_progress_beads,
            blocked_beads,
            closed_beads,
        }
    }

    /// Remove a bead from the graph
    pub fn remove_bead(&mut self, id: &BeadId) -> Option<Bead> {
        // Clean up indices
        if let Some(bead) = self.beads.get(id) {
            for label in &bead.labels {
                if let Some(ids) = self.label_index.get_mut(label) {
                    ids.remove(id);
                }
            }
        }

        // Remove from dependents index
        self.dependents_index.remove(id);

        self.beads.remove(id)
    }

    /// Remove a shadow bead from the graph
    pub fn remove_shadow_bead(&mut self, id: &BeadId) -> Option<ShadowBead> {
        // Clean up indices
        if let Some(shadow) = self.shadow_beads.get(id) {
            if let Some(ids) = self.context_index.get_mut(&shadow.context) {
                ids.remove(id);
            }
            for label in &shadow.labels {
                if let Some(ids) = self.label_index.get_mut(label) {
                    ids.remove(id);
                }
            }
        }

        self.shadow_beads.remove(id)
    }
}

/// Statistics about the federated graph
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub total_beads: usize,
    pub total_shadows: usize,
    pub total_rigs: usize,
    pub open_beads: usize,
    pub in_progress_beads: usize,
    pub blocked_beads: usize,
    pub closed_beads: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federated_graph_creation() {
        let graph = FederatedGraph::new();
        assert_eq!(graph.beads.len(), 0);
        assert_eq!(graph.shadow_beads.len(), 0);
        assert_eq!(graph.rigs.len(), 0);
    }

    #[test]
    fn test_add_and_query_beads() {
        let mut graph = FederatedGraph::new();

        let bead = Bead::new("ab-test", "Test Bead", "test-user");

        graph.add_bead(bead);

        assert_eq!(graph.beads.len(), 1);
        assert!(graph.get_bead(&BeadId::new("ab-test")).is_some());
    }

    #[test]
    fn test_query_by_status() {
        let mut graph = FederatedGraph::new();

        let mut bead1 = Bead::new("ab-1", "Open Task", "user");
        bead1.status = Status::Open;

        let mut bead2 = Bead::new("ab-2", "Closed Task", "user");
        bead2.status = Status::Closed;

        graph.add_bead(bead1);
        graph.add_bead(bead2);

        let open_beads = graph.beads_by_status(Status::Open);
        assert_eq!(open_beads.len(), 1);
        assert_eq!(open_beads[0].id.as_str(), "ab-1");

        let closed_beads = graph.beads_by_status(Status::Closed);
        assert_eq!(closed_beads.len(), 1);
        assert_eq!(closed_beads[0].id.as_str(), "ab-2");
    }

    #[test]
    fn test_query_by_label() {
        let mut graph = FederatedGraph::new();

        let mut bead = Bead::new("ab-1", "Test", "user");
        bead.add_label("bug");
        bead.add_label("frontend");

        graph.add_bead(bead);

        let (beads, _) = graph.beads_by_label("bug");
        assert_eq!(beads.len(), 1);

        let (beads, _) = graph.beads_by_label("frontend");
        assert_eq!(beads.len(), 1);

        let (beads, _) = graph.beads_by_label("backend");
        assert_eq!(beads.len(), 0);
    }

    #[test]
    fn test_shadow_bead_context_query() {
        let mut graph = FederatedGraph::new();

        let rig_id = RigId::new("test-rig");
        let native_id = BeadId::new("native-123");
        let shadow = ShadowBead::new("ab-shadow", &rig_id, &native_id, "Test Shadow", "work");

        graph.add_shadow_bead(shadow);

        let work_beads = graph.beads_by_context("work");
        assert_eq!(work_beads.len(), 1);

        let personal_beads = graph.beads_by_context("personal");
        assert_eq!(personal_beads.len(), 0);
    }

    #[test]
    fn test_dependents_tracking() {
        let mut graph = FederatedGraph::new();

        let bead1 = Bead::new("ab-1", "Foundation", "user");

        let mut bead2 = Bead::new("ab-2", "Depends on 1", "user");
        bead2.add_dependency(BeadId::new("ab-1"));

        graph.add_bead(bead1);
        graph.add_bead(bead2);

        let dependents = graph.get_dependents(&BeadId::new("ab-1"));
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].id.as_str(), "ab-2");
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = FederatedGraph::new();

        let mut bead1 = Bead::new("ab-1", "Open", "user");
        bead1.status = Status::Open;

        let mut bead2 = Bead::new("ab-2", "Closed", "user");
        bead2.status = Status::Closed;

        graph.add_bead(bead1);
        graph.add_bead(bead2);

        let stats = graph.stats();
        assert_eq!(stats.total_beads, 2);
        assert_eq!(stats.open_beads, 1);
        assert_eq!(stats.closed_beads, 1);
    }

    #[test]
    fn test_remove_bead() {
        let mut graph = FederatedGraph::new();

        let bead = Bead::new("ab-test", "Test", "user");
        graph.add_bead(bead);

        assert_eq!(graph.beads.len(), 1);

        let removed = graph.remove_bead(&BeadId::new("ab-test"));
        assert!(removed.is_some());
        assert_eq!(graph.beads.len(), 0);
    }
}
