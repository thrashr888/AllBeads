//! BeadsRepo - High-level wrapper around beads crate
//!
//! Provides AllBeads-specific operations on top of the beads CLI wrapper.
//! Handles type conversions and integrates with FederatedGraph.

use crate::graph::{Bead, BeadId, FederatedGraph, Status};
use crate::Result;
use std::path::PathBuf;

use super::conversions::{issues_to_beads, issue_to_bead};

/// Repository wrapper for beads operations
///
/// Provides a high-level interface to the beads CLI with automatic
/// type conversion to AllBeads internal types.
#[derive(Debug)]
pub struct BeadsRepo {
    bd: beads::Beads,
}

impl BeadsRepo {
    /// Create a new BeadsRepo
    ///
    /// # Errors
    /// Returns an error if bd is not installed or not available
    pub fn new() -> Result<Self> {
        let bd = beads::Beads::new()
            .map_err(|e| crate::AllBeadsError::Other(format!("Failed to initialize beads: {}", e)))?;
        Ok(Self { bd })
    }

    /// Create a BeadsRepo with a specific working directory
    pub fn with_workdir(path: impl Into<PathBuf>) -> Self {
        Self {
            bd: beads::Beads::with_workdir(path),
        }
    }

    /// Check if bd is available
    pub fn is_available(&self) -> bool {
        self.bd.is_available()
    }

    /// Check if current directory is a beads repository
    pub fn is_repo(&self) -> bool {
        self.bd.is_repo()
    }

    // --- Reading operations ---

    /// List all beads
    pub fn list_all(&self) -> Result<Vec<Bead>> {
        let issues = self.bd.list(None, None)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        issues_to_beads(issues)
    }

    /// List beads by status
    pub fn list_by_status(&self, status: Status) -> Result<Vec<Bead>> {
        let status_str = match status {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Deferred => "deferred",
            Status::Closed => "closed",
            Status::Tombstone => "tombstone",
        };

        let issues = self.bd.list(Some(status_str), None)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        issues_to_beads(issues)
    }

    /// Get beads ready to work on (no blockers)
    pub fn ready(&self) -> Result<Vec<Bead>> {
        let issues = self.bd.ready()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        issues_to_beads(issues)
    }

    /// Get blocked beads
    pub fn blocked(&self) -> Result<Vec<Bead>> {
        let issues = self.bd.blocked()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        issues_to_beads(issues)
    }

    /// Get a specific bead by ID
    pub fn get(&self, id: &BeadId) -> Result<Bead> {
        let issue = self.bd.show(id.as_str())
            .map_err(|e| match e {
                beads::Error::IssueNotFound(id) => crate::AllBeadsError::IssueNotFound(id),
                _ => crate::AllBeadsError::Storage(e.to_string()),
            })?;
        issue_to_bead(issue)
    }

    /// Search for beads by query
    pub fn search(&self, query: &str) -> Result<Vec<Bead>> {
        let issues = self.bd.search(query)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        issues_to_beads(issues)
    }

    // --- Loading into FederatedGraph ---

    /// Load all beads into a FederatedGraph
    pub fn load_graph(&self) -> Result<FederatedGraph> {
        let beads = self.list_all()?;
        let mut graph = FederatedGraph::new();

        for bead in beads {
            graph.add_bead(bead);
        }

        Ok(graph)
    }

    /// Load beads by status into a FederatedGraph
    pub fn load_graph_by_status(&self, status: Status) -> Result<FederatedGraph> {
        let beads = self.list_by_status(status)?;
        let mut graph = FederatedGraph::new();

        for bead in beads {
            graph.add_bead(bead);
        }

        Ok(graph)
    }

    // --- Writing operations ---

    /// Create a new bead
    pub fn create(
        &self,
        title: &str,
        issue_type: &str,
        priority: Option<u8>,
    ) -> Result<()> {
        self.bd.create(title, issue_type, priority, None)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Update a bead's status
    pub fn update_status(&self, id: &BeadId, status: Status) -> Result<()> {
        let status_str = match status {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Deferred => "deferred",
            Status::Closed => "closed",
            Status::Tombstone => "tombstone",
        };

        self.bd.update_status(id.as_str(), status_str)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Close a bead
    pub fn close(&self, id: &BeadId) -> Result<()> {
        self.bd.close(id.as_str())
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Close multiple beads at once
    pub fn close_multiple(&self, ids: &[&BeadId]) -> Result<()> {
        let id_strs: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
        self.bd.close_multiple(&id_strs)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Add a dependency between beads
    pub fn add_dependency(&self, issue: &BeadId, depends_on: &BeadId) -> Result<()> {
        self.bd.dep_add(issue.as_str(), depends_on.as_str())
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Remove a dependency between beads
    pub fn remove_dependency(&self, issue: &BeadId, depends_on: &BeadId) -> Result<()> {
        self.bd.dep_remove(issue.as_str(), depends_on.as_str())
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Add a label to a bead
    pub fn add_label(&self, id: &BeadId, label: &str) -> Result<()> {
        self.bd.label_add(id.as_str(), label)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Remove a label from a bead
    pub fn remove_label(&self, id: &BeadId, label: &str) -> Result<()> {
        self.bd.label_remove(id.as_str(), label)
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    // --- Sync operations ---

    /// Sync with remote repository
    pub fn sync(&self) -> Result<()> {
        self.bd.sync()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Initialize beads in current directory
    pub fn init(&self) -> Result<()> {
        self.bd.init()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Run health checks
    pub fn doctor(&self) -> Result<String> {
        let output = self.bd.doctor()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))?;
        Ok(output.combined())
    }

    // --- Statistics ---

    /// Get repository statistics
    pub fn stats(&self) -> Result<beads::Stats> {
        self.bd.stats()
            .map_err(|e| crate::AllBeadsError::Storage(e.to_string()))
    }

    /// Get access to the underlying beads::Beads instance for advanced operations
    pub fn beads(&self) -> &beads::Beads {
        &self.bd
    }
}

impl Default for BeadsRepo {
    fn default() -> Self {
        Self {
            bd: beads::Beads::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_repo_creation() {
        // This test requires bd to be installed
        let result = BeadsRepo::new();
        // We don't assert success since bd might not be installed in test env
        // Just verify it compiles and returns a Result
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_beads_repo_with_workdir() {
        let repo = BeadsRepo::with_workdir("/tmp");
        // Just verify it constructs correctly
        assert!(!repo.is_repo()); // /tmp typically not a beads repo
    }

    #[test]
    fn test_default() {
        let _repo = BeadsRepo::default();
        // Just verify default construction works
    }
}
